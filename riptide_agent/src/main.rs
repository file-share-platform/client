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

#![warn(
    missing_docs,
    missing_debug_implementations,
    missing_copy_implementations,
    // clippy::missing_docs_in_private_items, //TODO
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications,
    deprecated
)]

mod error;

use std::{sync::Arc, time::Duration};

use error::AgentError;
use futures::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use riptide_config::Config;
use riptide_database::{establish_connection, get_share_by_id, Share};
use tokio::{fs, net::TcpStream, sync::RwLock, time::Instant};
use tokio_tungstenite::{
    tungstenite::{protocol::WebSocketConfig, Message as TungsteniteMessage},
    MaybeTlsStream, WebSocketStream,
};
use ws_com_framework::{error::ErrorKind, Message};

const MIN_RECONNECT_DELAY: usize = 5000;

/// Self contained function to upload files to the server
async fn upload_file(metadata: Share, config: Arc<RwLock<Config>>, url: String) {
    let loc = (*config.read().await.file_store_location()).join(metadata.file_id.to_string());

    let mut a = 0;
    loop {
        let f = fs::File::open(&loc)
            .await
            .expect("File unexpectedly not available!");

        let f = f.into_std().await;

        let local_url = url.clone();
        let res = tokio::task::spawn_blocking(move || {
            ureq::post(&local_url)
                .set("Content-Type", "application/octet-stream")
                .send(f)
        })
        .await;

        match res {
            Ok(_) => break,
            Err(e) => {
                a += 1;
                if a >= *config.read().await.max_upload_attempts() {
                    error!("Failed to upload file to endpoint, error: {}", e);
                    break;
                }
            }
        }
    }
    debug!("File {} uploaded to: {}", metadata.file_name, url);
}

async fn handle_message(
    m: Message,
    config: Arc<RwLock<Config>>,
) -> Result<Option<Message>, AgentError> {
    match m {
        Message::UploadTo {
            file_id,
            upload_url,
        } => {
            //XXX: use tokio_scoped to avoid the allocation here - or wrap config in an arc globally
            let database_location = config.read().await.database_location().clone();
            let item = tokio::task::spawn_blocking(move || {
                match establish_connection(&database_location) {
                    Ok(ref mut conn) => get_share_by_id(conn, &file_id),
                    Err(e) => Err(e),
                }
            })
            .await??;

            if let Some(f) = item {
                upload_file(f, config, upload_url).await;
                Ok(None)
            } else {
                let upload_id = upload_url
                    .split('/')
                    .last()
                    .expect("Upload URL is invalid, no / found!");
                Ok(Some(Message::Error {
                    kind: ErrorKind::FileDoesntExist,
                    reason: Some(upload_id.to_string()),
                }))
            }
        }
        Message::MetadataReq { file_id, upload_id } => {
            let database_location = config.read().await.database_location().clone();
            let item = tokio::task::spawn_blocking(move || {
                match establish_connection(&database_location) {
                    Ok(ref mut conn) => get_share_by_id(conn, &file_id),
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
        Message::AuthReq { public_id } => Ok(Some(Message::AuthRes {
            public_id,
            passcode: config.read().await.private_key().as_ref().unwrap().to_vec(),
        })),
        Message::StatusReq {
            public_id: _,
            upload_id,
        } => Ok(Some(Message::StatusRes {
            public_id: config.read().await.public_id().unwrap(),
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
    config: Arc<RwLock<Config>>,
    websocket: WebSocketStream<MaybeTlsStream<TcpStream>>,
) -> Result<bool, AgentError> {
    let websocket = Arc::new(RwLock::new(websocket));

    let mut handles = Vec::new();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<Option<Message>, AgentError>>(20);

    let mut res = Ok(false);
    loop {
        // if there are any messages in the channel, send them
        while let Ok(m) = rx.try_recv() {
            match m {
                Ok(Some(msg)) => {
                    let bin: Vec<u8> = match msg.try_into() {
                        Ok(d) => d,
                        Err(e) => {
                            res = Err(e.into());
                            break;
                        }
                    };
                    if let Err(e) = websocket
                        .write()
                        .await
                        .send(TungsteniteMessage::Binary(bin))
                        .await
                    {
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

        // try to receive and act on new messages
        match websocket.write().await.next().await {
            Some(Ok(TungsteniteMessage::Binary(msg))) => {
                let msg: Message = match msg.try_into() {
                    Ok(m) => m,
                    Err(e) => {
                        res = Err(e.into());
                        break;
                    }
                };

                let local_tx = tx.clone();
                let local_config = config.clone();
                let h = tokio::spawn(async move {
                    local_tx
                        .send(handle_message(msg, local_config).await)
                        .await
                        .unwrap();
                });
                handles.push(h);
            }
            Some(Ok(TungsteniteMessage::Ping(msg))) => {
                if let Err(e) = websocket
                    .write()
                    .await
                    .send(TungsteniteMessage::Pong(msg))
                    .await
                {
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

    // kill all the handles
    for h in handles {
        h.abort();
    }

    websocket.write_owned().await.close(None).await?;
    res
}

/// Remove expired shares from the database
async fn remove_expired_shares(
    config: Arc<RwLock<Config>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let database_location = config.read().await.database_location().clone();
    let shares: Vec<Share> = tokio::task::spawn_blocking(move || {
        let mut conn = establish_connection(&database_location)?;
        riptide_database::remove_expired_shares(&mut conn)
    })
    .await??;

    for share in shares {
        let path = (*config.read().await.file_store_location()).join(share.file_id.to_string());
        tokio::fs::remove_file(path).await?;
    }

    Ok(())
}

async fn run(config: Arc<RwLock<Config>>) {
    let reader = config.read().await;
    let ip = format!(
        "{}/api/v1/ws/{}",
        reader.websocket_address(),
        reader.public_id().unwrap()
    );
    let reconnect_delay = reader.reconnect_delay_minutes();
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
                if let Err(e) = handle_ws(config.clone(), t).await {
                    error!("error occurred when handling websocket: {}", e);
                }
            }
            Err(e) => {
                error!("Failed to connect to webserver {:?}", e);
            }
        };

        tokio::time::sleep(std::time::Duration::from_secs(std::cmp::max(
            reconnect_delay * 60,
            MIN_RECONNECT_DELAY as u64,
        )))
        .await;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    debug!("Validating config...");
    while !Config::exists() {
        warn!("Config file does not exist, please create one by using the `riptide` cli utility");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }

    debug!("Checking registration status...");
    while !Config::is_registered() {
        warn!("Agent is not registered, please register by using the `riptide` cli utility");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }

    debug!("Starting...");
    let config: Config = tokio::task::spawn_blocking(Config::load_config).await??;
    let config = Arc::new(RwLock::new(config));

    // spawn monitoring task to remove expired shares
    let monitor_config = config.clone();
    let monitor_handle = tokio::task::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            if let Err(e) = remove_expired_shares(monitor_config.clone()).await {
                error!("Failed to remove expired shares: {}", e);
            }
        }
    });

    let reload_timer = tokio::time::sleep(Duration::from_secs(5));

    let runner = run(config);
    tokio::pin!(monitor_handle);
    tokio::pin!(runner);
    tokio::pin!(reload_timer);

    loop {
        tokio::select! {
            biased;

            _ = tokio::signal::ctrl_c() => {
                info!("SIGINT recieved, shutting down");
                break;
            }

            _ = &mut reload_timer => {
                match Config::reload_requested() {
                    Ok(true) => {
                        info!("Reload requested, shutting down");
                        break; //HACK: instead of breaking, we should just reload the config properly
                    },
                    Ok(_) => {},
                    Err(e) => {
                        error!("Failed to check for reload request: {}", e);
                    }
                }
                reload_timer.as_mut().reset(Instant::now() + Duration::from_secs(5));
            }

            _ = &mut runner => {
                info!("Runner exited, shutting down");
                break;
            }

            _ = &mut monitor_handle => {
                error!("Monitor exited, shutting down");
                break;
            }
        }
    }

    debug!("Connection closed, Server Agent exiting....");
    Ok(())
}
