//! # Server Agent for the file sharing platform
//! This server application aims to handle incoming connections from the server agent.
//! It needs to serve requested files when needed, and provide information back to the server.
//! It has no direct connection to the user - but instead reads off a database that the cli
//! tool also modifies. This largely isolates it, and prevents us from needing to write
//! a communication layer between the the CLI tool and the server agent.
//! # Function
//! 1. Connect to the Central-API on first start, and attempt to request an ID.
//! 2. Recieve our ID, and store that in a config file.
//! 3. Attempt to connect to the websocket endpoint of the Central-API, on failure repeat steps 1 and 2 again.
//! 4. With a succesful websocket connection gracefully handle incoming requests from the Central-API, primarily:
//!     - Requests for metadata/file status.
//!     - File upload requests.
//!     - Health requests.
//!     - Closing websocket.
//! 5. In the event that the Central-API is not available for a connection or disconnects us, sleep for 1 minute then
//!    re-attempt the connection.

mod error;

use config::Config;
use database::{establish_connection, find_share_by_id, Share};
use error::AgentError;
use futures::{StreamExt, SinkExt};
use tokio::{fs, net::TcpStream};
use tokio_tungstenite::{WebSocketStream, MaybeTlsStream, tungstenite::Message as TungsteniteMessage};
use ws_com_framework::{Message, error::{EndOfConnection, ErrorKind}, message::ShareMetadata};
use log::{error, debug, warn, info};

const MIN_RECONNECT_DELAY: usize = 5000;

fn file_to_body(f: tokio::fs::File) -> reqwest::Body {
    let stream = tokio_util::codec::FramedRead::new(f, tokio_util::codec::BytesCodec::new());
    reqwest::Body::wrap_stream(stream)
}

/// Self contained function to upload files to the server
async fn upload_file(metadata: Share, config: &Config, url: &str) {
    let loc = (*config.file_store_location()).join(metadata.file_id.to_string());

    let mut a = 0;
    loop {
        let f = fs::File::open(&loc)
            .await
            .expect("File unexpectedly not available!");
        let res = reqwest::Client::new()
            .post(url)
            .body(file_to_body(f))
            .send()
            .await;
        match res {
            Ok(_) => break,
            Err(e) => {
                if a >= config.max_upload_attempts() {
                    error!("Failed to upload file to endpoint, error: {}", e);
                    break;
                }
                a += 1;
            }
        }
    }
    debug!("File {} uploaded to: {}", metadata.file_name, url);
}

async fn handle_message(
    m: Message,
    config: &Config,
) -> Result<Option<Message>, AgentError> {
    match m {
        Message::UploadTo(file_id, ref url) => {
            let item = tokio::task::spawn_blocking(move || {
                match establish_connection() {
                    Ok(conn) =>
                        find_share_by_id(&conn, &file_id),
                    Err(e) => Err(e.into()),
                }
            }).await??;

            if let Some(f) = item {
                upload_file(f, config, url).await;
                Ok(None)
            } else {
                Ok(Some(Message::Error(None, EndOfConnection::Continue, ErrorKind::FileDoesntExist)))
            }
        }
        Message::MetadataReq(file_id) => {
            let item = tokio::task::spawn_blocking(move || {
                match establish_connection() {
                    Ok(conn) =>
                        find_share_by_id(&conn, &file_id),
                    Err(e) => Err(e.into()),
                }
            }).await??;

            if let Some(f) = item {
                Ok(Some(Message::MetadataRes(ShareMetadata {
                    file_id: f.file_id as u32,
                    exp: f.exp as u64,
                    crt: f.crt as u64,
                    file_size: f.file_size as u64,
                    username: f.user_name,
                    file_name: f.file_name,
                })))
            } else {
                Ok(Some(Message::Error(None, EndOfConnection::Continue, ErrorKind::FileDoesntExist)))
            }
        }
        Message::AuthReq(pub_id) => Ok(Some(Message::AuthRes(pub_id, config.private_key().to_vec()))),
        e => {
            error!("Unsupported message, recieved! {:?}", e);
            Ok(None)
        }
    }
}

async fn handle_ws(config: &Config, mut websocket: WebSocketStream<MaybeTlsStream<TcpStream>>) -> Result<(), AgentError> {
    let mut res = Ok(());
    loop {
        //Loop to get messages
        match websocket.next().await {
            Some(Ok(TungsteniteMessage::Binary(msg))) => {
                let msg: Message = match msg.try_into() {
                    Ok(m) => m,
                    Err(e) => {
                        res = Err(e.into());
                        break;
                    },
                };

                match handle_message(msg, config).await {
                    Ok(Some(msg)) => {
                        let bin: Vec<u8> = match msg.try_into() {
                            Ok(d) => d,
                            Err(e) => {
                                res = Err(e.into());
                                break;
                            },
                        };
                        if let Err(e) = websocket.send(TungsteniteMessage::Binary(bin)).await {
                            res = Err(e.into());
                            break;
                        }
                    },
                    Ok(None) => {}
                    Err(e) => {
                        res = Err(e);
                        break;
                    },
                }
            },
            Some(Ok(TungsteniteMessage::Ping(msg))) => {
                if let Err(e) = websocket.send(TungsteniteMessage::Pong(msg)).await {
                    res = Err(e.into());
                    break;
                }
            },
            Some(Ok(TungsteniteMessage::Pong(_))) => {
                warn!("Agent was ponged? Should not happen.");
                break;
            },
            Some(Ok(TungsteniteMessage::Text(msg))) => {
                warn!("recieved text message from server: {}", msg)
            },
            Some(Ok(TungsteniteMessage::Close(_))) => {
                info!("got close message from server");
                break;
            },
            Some(Err(e)) => {
                res = Err(e.into());
                break;
            },
            None => break,
        }
    }

    if let Err(e) = res {

        websocket.close(None).await?;
        Err(e)
    } else {
        websocket.close(None).await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    debug!("Starting...");
    let config = Config::load_config_async().await?;

    let ip = format!(
        "{}/ws/{}",
        config.websocket_address(),
        config.public_id()
    );

    loop {
        //XXX: Do we need to handle the response here?
        match tokio_tungstenite::connect_async(&ip).await {
            Ok((t, _r)) =>  {
                handle_ws(&config, t)
                    .await
                    .expect("Not Implemented"); //TODO
            },
            Err(e) => {
                error!("Failed to connect to webserver {:?}", e);
                continue;
            }
        };

        tokio::time::sleep(std::time::Duration::from_millis(std::cmp::max(
            (config.reconnect_delay_minutes() * 60 * 1000) as u64,
            MIN_RECONNECT_DELAY as u64,
        )))
        .await;
    }

    //TODO Implement a ctrl+c catch to close

    // debug!("Connection closed, Server Agent exiting....");
    std::process::exit(0);
}