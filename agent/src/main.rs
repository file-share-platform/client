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

mod db;
mod error;
mod uploader;

use db::DBPool;
use error::Error;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::fs;
use ws_com_framework::{message::Upload, File, Message, Receiver, Sender};

const CONFIG_PATH: &str = "/opt/file-share/file-share.toml";
const MIN_RECONNECT_DELAY: usize = 2000;

/// A copy of println!, which only prints when the global const DEBUG is true.
/// This makes debugging quick and easy to toggle.
macro_rules! debug {
    () => {
        if cfg!(debug_assertions) {
            println!();
        }
    };
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            println!($($arg)*);
        }
    }
}

/// When called, if application is in DEBUG mode will panic. Otherwise will merely print the error to the console.
macro_rules! debug_panic {
    ($fmt_string:expr) => {
        if cfg!(debug_assertions) {
            panic!($fmt_string);
        } else {
            println!($fmt_string);
        }
    };
    ($fmt_string:expr, $( $arg:expr ),*) => {
        if cfg!(debug_assertions) {
            panic!($fmt_string, $( $arg ),*);
        } else {
            println!($fmt_string, $( $arg ),*);
        }
    }
}

macro_rules! okie {
    ($fmt_string:expr) => {
        return Ok(Some($fmt_string.into()))
    };
}

#[derive(Serialize, Deserialize, Clone)]
struct Config {
    server_ip: String,
    prefix: String,
    max_upload_attempts: usize,
    home_dir_location: String,
    reconnect_delay: usize,
    id: Option<Id>,
}

impl Config {
    fn is_valid(&self) -> bool {
        //TODO
        true
    }
    fn set_id(&mut self, id: Id) {
        self.id = Some(id)
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            server_ip: "localhost:3030".to_owned(),
            prefix: "http".to_owned(),
            max_upload_attempts: 3,
            home_dir_location: "/opt/file-share".to_owned(),
            reconnect_delay: 60000,
            id: None,
        }
    }
}

/// Information required to connect to central api
#[derive(Serialize, Deserialize, Clone)]
struct Id {
    public_id: String,
    private_key: String,
}

fn file_to_body(f: tokio::fs::File) -> reqwest::Body {
    let stream = tokio_util::codec::FramedRead::new(f, tokio_util::codec::BytesCodec::new());
    reqwest::Body::wrap_stream(stream)
}

/// Self contained function to upload files to the server
async fn upload_file(metadata: File, cfg: Config, url: &str) {
    let loc = format!("{}/hard_links/{}", cfg.home_dir_location, metadata.id());
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
                if a >= cfg.max_upload_attempts {
                    debug_panic!("Failed to upload file to endpoint, error: {}", e);
                    break;
                }
                a += 1;
            }
        }
    }
    debug!("File {} uploaded to: {}", metadata.name(), url);
}

async fn handle_message(m: Message, db: DBPool, cfg: Config) -> Result<Option<Message>, Error> {
    match m {
        Message::UploadRequest(u) => {
            if let Some(f) = db::Search::PublicId(u.id().to_string()).find(&db).await? {
                upload_file(f, cfg, u.url()).await;
                Ok(None)
            } else {
                okie!(Message::Error(ws_com_framework::Error::FileDoesntExist))
            }
        }
        Message::MetadataRequest(r) => {
            if let Some(f) = db::Search::PublicId(r.id().to_string()).find(&db).await? {
                okie!(Message::MetadataResponse(Upload::new(
                    r.url().to_string(),
                    f,
                )))
            }
            okie!(ws_com_framework::Error::FileDoesntExist)
        }
        Message::Close(c) => Err(Error::Closed(c)),
        Message::AuthReq => {
            okie!(Message::AuthResponse(ws_com_framework::AuthKey {
                key: cfg.id.unwrap().private_key.as_bytes().try_into().unwrap()
            }))
        }
        e => {
            debug_panic!("Unsupported message, recieved! {:?}", e);
            Ok(None)
        }
    }
}

async fn handle_ws<F, R, S, Fut>(
    handle: F,
    (mut tx_ws, mut rx_ws): (Sender<S>, Receiver<R>),
    db: &DBPool,
    cfg: Config,
) -> Result<(), ()>
where
    F: Fn(Message, DBPool, Config) -> Fut + Send + Sync + 'static + Copy,
    R: ws_com_framework::RxStream,
    S: ws_com_framework::TxStream + Send + 'static,
    Fut: std::future::Future<Output = Result<Option<Message>, Error>> + Send,
{
    let (tx_internal, mut rx_internal) = tokio::sync::mpsc::unbounded_channel::<Message>();
    tokio::task::spawn(async move {
        while let Some(m) = rx_internal.recv().await {
            if let Err(e) = tx_ws.send(m).await {
                debug_panic!("Error occured! {:?}", e);
                return;
            };
        }
    });

    //Loop to get messages
    while let Some(m) = rx_ws.next().await {
        let m: Message = match m {
            Ok(f) => f,
            Err(e) => {
                //TODO add some handling here
                debug_panic!("Error occured! {:?}", e);
                continue;
            }
        };

        debug!(
            "Message recieved from Central-API: {:?}\nProcessing now...",
            m
        );

        // Ugly, but this is required to pass owned values into the thread
        let db_o = db.clone();
        let cfg_o = cfg.clone();
        let tx_o = tx_internal.clone();
        tokio::task::spawn(async move {
            let m = handle(m, db_o, cfg_o).await;
            if let Err(e) = m {
                debug_panic!("Error occured! {:?}", e);
                return;
            }

            debug!("Sending response to Central-API: {:?}", m);

            if let Some(r) = m.unwrap() {
                if let Err(e) = tx_o.send(r) {
                    debug_panic!(
                        "Tried to send response through internal websocket, but failed \n{}",
                        e
                    );
                };
            };
        });
    }
    Ok(())
}

/// Connect to a websocket on the server, and return the sender/receiver handles
async fn connect_sever(
    ip: &str,
) -> Result<
    (
        Receiver<
            futures_util::stream::SplitStream<
                tokio_tungstenite::WebSocketStream<
                    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
                >,
            >,
        >,
        Sender<
            futures_util::stream::SplitSink<
                tokio_tungstenite::WebSocketStream<
                    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
                >,
                tokio_tungstenite::tungstenite::Message,
            >,
        >,
    ),
    Error,
> {
    debug!("Attempting to connect to {}", ip);

    let (client, _) = tokio_tungstenite::connect_async(ip)
        .await
        .expect("Failed to connect"); //TODO error handling

    debug!("Client succesfully connected to Central-Api at {}", ip);

    //Split streams into components, and wrapper them with communication framework
    let (rx, tx) = client.split();

    Ok((Receiver::new(tx), Sender::new(rx)))
}

/// We call to this in the event that we are not registered yet.
async fn register_server(ip: String) -> Result<Id, Error> {
    debug!("Attempting to register websocket with At IP {}", ip);

    let id = reqwest::Client::new()
        .post(&ip)
        .send()
        .await?
        .json::<Id>()
        .await?;

    Ok(id)
}

#[tokio::main]
async fn main() {
    debug!("Starting...");

    let mut cfg: Config = confy::load_path(CONFIG_PATH).expect("Failed to load config!");

    let db_pool = db::create_pool().expect("failed to create db pool");
    db::init_db(&db_pool).await.expect("failed to initalize db");

    loop {
        // Register websocket if not registered
        if cfg.clone().id.is_none() {
            let ip = format!(
                "{}://{}/api/v1/client/ws-register",
                cfg.prefix, cfg.server_ip
            );
            let id: Id = match register_server(ip).await {
                Ok(f) => f,
                Err(e) => {
                    debug_panic!("Failed to register websocket {:?}", e);
                    continue;
                }
            };
            cfg.set_id(id);
            debug!(
                "Registered websocket with id {}",
                cfg.clone().id.unwrap().public_id
            );
            if let Err(e) = confy::store_path(CONFIG_PATH, cfg.clone()) {
                debug_panic!(
                    "Failed to save config, quitting to prevent unintended errors: {}",
                    e
                );
                continue;
            };
        }

        if !cfg.is_valid() {
            debug_panic!("Invalid config detected!");
            continue;
        }

        let ip = format!(
            "ws://{}/api/v1/client/ws/{}",
            &cfg.server_ip,
            cfg.clone().id.unwrap().public_id
        );

        let (rx, tx) = match connect_sever(&ip).await {
            Ok(f) => f,
            Err(e) => {
                debug_panic!("Failed to connect to webserver {:?}", e);
                continue;
            }
        };

        //TODO validation here

        handle_ws(handle_message, (tx, rx), &db_pool, cfg.clone())
            .await
            .expect("Not Implemented"); //TODO

        tokio::time::sleep(std::time::Duration::from_millis(std::cmp::max(
            cfg.reconnect_delay as u64,
            MIN_RECONNECT_DELAY as u64,
        )))
        .await;
    }

    //TODO Implement a ctrl+c catch to close

    // debug!("Connection closed, Server Agent exiting....");
    // std::process::exit(0);
}

#[cfg(test)]
mod websocket_tests {
    use crate::db;
    use crate::db::DBPool;
    use crate::{connect_sever, handle_ws, register_server, Config};
    use futures::{FutureExt, SinkExt, StreamExt};
    use std::time::Duration;
    use tokio::sync::oneshot;
    use tokio::time::timeout;
    use warp;
    use warp::Filter;
    use ws_com_framework::{Message, Sender};
    /// Spool up a simple websocket server, which is useful in tests, set to echo.
    fn create_websocket_server(ip: ([u8; 4], u16)) -> Result<oneshot::Sender<()>, ()> {
        let echo_ws = warp::any()
            .and(warp::path("echo"))
            .and(warp::path::end())
            .and(warp::ws())
            .map(|ws: warp::ws::Ws| {
                ws.on_upgrade(|websocket| {
                    // Just echo all messages back...
                    let (tx, rx) = websocket.split();
                    rx.forward(tx).map(|result| {
                        if let Err(e) = result {
                            panic!("websocket error: {:?}", e);
                        }
                    })
                })
            });

        let routes = echo_ws;

        let (tx, rx) = oneshot::channel();
        let (_addr, server) = warp::serve(routes).bind_with_graceful_shutdown(ip, async {
            rx.await.ok();
        });

        tokio::task::spawn(server);

        Ok(tx)
    }

    /// Create a simple webserver which parses some basic http requests.
    fn create_http_server(ip: ([u8; 4], u16)) -> Result<oneshot::Sender<()>, ()> {
        let register = warp::post()
            .and(warp::path("test-register"))
            .and(warp::path::end())
            .map(|| {
                format!(
                    "
                    {{
                        \"public_id\": \"7N58aK\", 
                        \"private_key\": \"oVZBbqJm5vXCmfTP8wQA0n13FeKd5Ego\"
                    }}"
                )
            });

        let routes = register;

        let (tx, rx) = oneshot::channel();
        let (_, server) = warp::serve(routes).bind_with_graceful_shutdown(ip, async {
            rx.await.ok();
        });

        tokio::task::spawn(server);

        Ok(tx)
    }

    /// Test that websocket is able to succesfully connect to a provided server, and send a simple message
    /// Will timeout and fail anyway after 10 seconds - this usually indicates the test has failed.
    #[tokio::test(flavor = "multi_thread")]
    async fn websocket_connect() {
        timeout(Duration::from_millis(10_000), async {
            let close_server_tx = create_websocket_server(([127, 0, 0, 1], 2033)).unwrap();

            let (mut rx, mut tx) = connect_sever("ws://127.0.0.1:2033/echo").await.unwrap();

            let msg = Message::Message("Hello, World!".into());

            tx.send(msg.clone()).await.unwrap();

            let m = rx.next().await.unwrap().unwrap();

            assert_eq!(msg, m);

            let _ = close_server_tx.send(());
        })
        .await
        .expect("Test failed due to timeout!");
    }

    #[tokio::test]
    async fn test_db() {
        let db_pool = db::create_pool().expect("failed to create db pool");
        db::init_db(&db_pool).await.expect("failed to initalize db");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 5)]
    async fn websocket_handle() {
        timeout(Duration::from_millis(5000), async {
            let cfg = Config::default(); //TODO should write custom config here
            let db_pool = db::create_pool().expect("failed to create db pool");
            db::init_db(&db_pool).await.expect("failed to initalize db");

            let close_server_tx = create_websocket_server(([127, 0, 0, 1], 2031)).unwrap();

            let (rx, mut tx) = connect_sever("ws://127.0.0.1:2031/echo").await.unwrap();

            let msg = Message::Message("Hello, World!".into());
            let e_msg = Message::Message("Hello, World!".into());

            //Send relevant sequences of messages
            tx.send(msg.clone()).await.unwrap();
            tx.send(e_msg.clone()).await.unwrap();
            tx.underlying().close().await.unwrap();
            //This function should process the messages we sent (to ensure they're all getting through)
            async fn handle(
                m: Message,
                _: DBPool,
                _: Config,
            ) -> Result<Option<Message>, crate::error::Error> {
                match m.clone() {
                    Message::Message(t) => {
                        if t != "Hello, World!".to_owned() {
                            panic!("Unexpected message recieved! {:?}", m);
                        }
                    }
                    _ => panic!("Unexpected message recieved! {:?}", m),
                }
                Ok(None)
            }

            let (tx, _) = tokio::sync::mpsc::unbounded_channel::<Message>();
            let s = Sender::new(tx);

            handle_ws(handle, (s, rx), &db_pool, cfg).await.unwrap();

            let _ = close_server_tx.send(());
        })
        .await
        .expect("Test failed due to timeout!");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_register() {
        let close_server_tx = create_http_server(([127, 0, 0, 1], 2034)).unwrap();

        let res = register_server("http://127.0.0.1:2034/test-register".into())
            .await
            .unwrap();

        assert_eq!(res.public_id, "7N58aK".to_owned());
        assert_eq!(
            res.private_key,
            "oVZBbqJm5vXCmfTP8wQA0n13FeKd5Ego".to_owned()
        );

        let _ = close_server_tx.send(());
    }
}
