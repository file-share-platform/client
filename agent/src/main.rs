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
use futures::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use tokio::{fs, net::TcpStream};
use tokio_tungstenite::{
    tungstenite::{protocol::WebSocketConfig, Message as TungsteniteMessage},
    MaybeTlsStream, WebSocketStream,
};
use ws_com_framework::{error::ErrorKind, Message};

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
                a += 1;
                if a >= *config.max_upload_attempts() {
                    error!("Failed to upload file to endpoint, error: {}", e);
                    break;
                }
            }
        }
    }
    debug!("File {} uploaded to: {}", metadata.file_name, url);
}

async fn handle_message(m: Message, config: &Config) -> Result<Option<Message>, AgentError> {
    match m {
        Message::UploadTo {
            file_id,
            upload_url,
        } => {
            //XXX: use tokio_scoped to avoid the allocation here - or wrap config in an arc globally
            let database_location = config.database_location().clone();
            let item = tokio::task::spawn_blocking(move || {
                match establish_connection(&database_location) {
                    Ok(ref mut conn) => find_share_by_id(conn, &file_id),
                    Err(e) => Err(e),
                }
            })
            .await??;

            if let Some(f) = item {
                upload_file(f, config, &upload_url).await;
                Ok(None)
            } else {
                Ok(Some(Message::Error {
                    kind: ErrorKind::FileDoesntExist,
                    reason: None,
                }))
            }
        }
        Message::MetadataReq { file_id, upload_id } => {
            let database_location = config.database_location().clone();
            let item = tokio::task::spawn_blocking(move || {
                match establish_connection(&database_location) {
                    Ok(ref mut conn) => find_share_by_id(conn, &file_id),
                    Err(e) => Err(e),
                }
            })
            .await??;

            if let Some(f) = item {
                Ok(Some(Message::MetadataRes {
                    file_id: f.file_id as u32,
                    exp: f.exp as u64,
                    crt: f.crt as u64,
                    file_size: f.file_size as u64,
                    username: f.user_name,
                    file_name: f.file_name,
                    upload_id,
                }))
            } else {
                Ok(Some(Message::Error {
                    kind: ErrorKind::FileDoesntExist,
                    reason: None,
                }))
            }
        }
        Message::AuthReq { public_id } => {
            Ok(Some(Message::AuthRes {
                public_id,
                passcode: config.private_key().to_vec(), //XXX: set this up with a zeroing field
            }))
        }
        Message::StatusReq {
            public_id: _,
            upload_id,
        } => Ok(Some(Message::StatusRes {
            public_id: *config.public_id(),
            ready: true,
            uptime: 0, //TODO: record uptime, this should be time connected to the api - not the time the agent has been running
            upload_id,
            message: Some(String::from("Ready to upload")),
        })),

        Message::Ok => Ok(None),
        Message::Error { kind, reason } => {
            error!(
                "Error received from server, kind: {:?}, reason: {:?}",
                kind, reason
            );
            Ok(None)
        }

        e => {
            warn!("Unsupported message, received! {:?}", e);
            Ok(None)
        }
    }
}

async fn handle_ws(
    config: &Config,
    mut websocket: WebSocketStream<MaybeTlsStream<TcpStream>>,
) -> Result<bool, AgentError> {
    let mut res = Ok(false);
    loop {
        //Loop to get messages
        match websocket.next().await {
            Some(Ok(TungsteniteMessage::Binary(msg))) => {
                let msg: Message = match msg.try_into() {
                    Ok(m) => m,
                    Err(e) => {
                        res = Err(e.into());
                        break;
                    }
                };

                match handle_message(msg, config).await {
                    Ok(Some(msg)) => {
                        let bin: Vec<u8> = match msg.try_into() {
                            Ok(d) => d,
                            Err(e) => {
                                res = Err(e.into());
                                break;
                            }
                        };
                        if let Err(e) = websocket.send(TungsteniteMessage::Binary(bin)).await {
                            res = Err(e.into());
                            break;
                        }
                    }
                    Ok(None) => {}
                    Err(e) => {
                        res = Err(e);
                        break;
                    }
                }
            }
            Some(Ok(TungsteniteMessage::Ping(msg))) => {
                if let Err(e) = websocket.send(TungsteniteMessage::Pong(msg)).await {
                    res = Err(e.into());
                    break;
                }
            }
            Some(Ok(TungsteniteMessage::Pong(_))) => {
                info!("Pong recieved");
            }
            Some(Ok(TungsteniteMessage::Text(msg))) => {
                warn!("recieved text message from server: {}", msg)
            }
            Some(Ok(TungsteniteMessage::Close(e))) => {
                info!("got close message from server message: {:?}", e);
                res = Ok(false); //XXX: should we try to reconnect?
            }
            Some(Ok(TungsteniteMessage::Frame(_))) => {
                error!("recieved raw frame");
                res = Err(AgentError::BadFrame(String::from("got raw frame")));
                break;
            }
            Some(Err(e)) => {
                res = Err(e.into());
                break;
            }
            None => break,
        }
    }

    websocket.close(None).await?;
    res
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    debug!("Starting...");
    let config = Config::load_config_async().await?;

    let ip = format!("{}/ws/{}", config.websocket_address(), config.public_id());

    loop {
        match tokio_tungstenite::connect_async_tls_with_config(
            &ip,
            Some(WebSocketConfig {
                max_send_queue: None,
                max_message_size: Some(16 << 20),
                max_frame_size: Some(2 << 20),
                accept_unmasked_frames: false,
            }),
            None,
        )
        .await
        {
            Ok((t, _r)) => {
                if let Err(e) = handle_ws(&config, t).await {
                    error!("error occured when handling websocket: {}", e);
                    break;
                }
            }
            Err(e) => {
                error!("Failed to connect to webserver {:?}", e);
            }
        };

        tokio::time::sleep(std::time::Duration::from_millis(std::cmp::max(
            (config.reconnect_delay_minutes() * 60 * 1000) as u64,
            MIN_RECONNECT_DELAY as u64,
        )))
        .await;
    }

    debug!("Connection closed, Server Agent exiting....");
    Ok(())
}
