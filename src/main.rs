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
//!    Upon second failure, fail out to the user - a restart resets this counter.
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
use serde::{Deserialize, Serialize};
use websocket::ClientBuilder;
use ws_com_framework::{File, Message, Receiver, Sender};
use tokio::fs;

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
    port: i64,
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
            server_ip: "127.0.0.1".to_owned(),
            port: 3030,
            prefix: "http".to_owned(),
            max_upload_attempts: 3,
            home_dir_location: "/opt/file-share".to_owned(),
            reconnect_delay: MIN_RECONNECT_DELAY,
            id: None,
        }
    }
}

/// Information required to connect to central api
#[derive(Serialize, Deserialize, Clone)]
struct Id {
    id: i64,
    unique_id: uuid::Uuid,
}

fn file_to_body(f: tokio::fs::File) -> reqwest::Body {
    let stream = tokio_util::codec::FramedRead::new(f, tokio_util::codec::BytesCodec::new());
    let body = reqwest::Body::wrap_stream(stream);
    body
}

/// Self contained function to upload files to the server
async fn upload_file(metadata: File, cfg: Config, url: &str) {
    let loc = format!("{}/hard_links/{}", cfg.home_dir_location, metadata.id());
    let mut a = 0;
    loop {
        let f = fs::File::open(&loc).await.expect("File unexpectedly not available!");
        let res = reqwest::Client::new().post(url).body(file_to_body(f)).send().await;
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
        Message::Upload(u) => {
            if let Some(f) = db::Search::uuid(u.id()).find(&db).await? {
                // HACK This is very dangerous and should be migrated to a thread pool
                // to avoid an accidental DDOS of the users system via upload threads.
                // But it's *fine* for now.=
                upload_file(f, cfg, u.url()).await;
                return Ok(None);
            } else {
                okie!(Message::Error(ws_com_framework::Error::FileDoesntExist))
            };
        }
        Message::Metadata(r) => {
            if let Some(mut f) = db::Search::uuid(r.id()).find(&db).await? {
                f.set_stream_id(r.stream_id());
                okie!(f)
            }
            okie!(ws_com_framework::Error::FileDoesntExist)
        }
        Message::Close(c) => return Err(Error::Closed(c)),
        e => {
            debug_panic!("Unsupported message, recieved! {:?}", e);
            return Ok(None);
        }
    }
}

async fn handle_ws<F, R, S, Fut>(
    handle: F,
    (mut rx, mut tx): (Receiver<R>, Sender<S>),
    db: &DBPool,
    cfg: Config,
) -> Result<(), ()>
where
    F: Fn(Message, DBPool, Config) -> Fut,
    R: ws_com_framework::RxStream,
    S: ws_com_framework::TxStream,
    Fut: std::future::Future<Output = Result<Option<Message>, Error>>,
{
    //Loop to get messages
    while let Some(m) = rx.next().await {
        // TODO spin each message off into a handler of a thread pool
        // This will help to make large uploads be non-blocking
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

        let m = handle(m, db.clone(), cfg.clone()).await;
        if let Err(e) = m {
            debug_panic!("Error occured! {:?}", e);
            continue;
        }

        debug!("Sending response to Central-API: {:?}", m);

        if let Some(r) = m.unwrap() {
            if let Err(e) = tx.send(r).await {
                debug_panic!("Error occured! {:?}", e);
                continue;
            };
        };
    }
    Ok(())
}

/// Connect to a websocket on the server, and return the sender/receiver handles
async fn connect_sever(
    ip: &str,
) -> Result<
    (
        Receiver<websocket::receiver::Reader<std::net::TcpStream>>,
        Sender<websocket::sender::Writer<std::net::TcpStream>>,
    ),
    (),
> {
    let client = ClientBuilder::new(ip)
        .expect("Failed to construct client") //TODO don't panic here!
        .connect_insecure()
        .expect("Failed to connect to Central-Api"); //TODO don't panic here!

    debug!("Client succesfully connected to Central-Api at {}", ip);

    //Split streams into components, and wrapper them with communication framework
    let (rx, tx) = client.split().expect("Failed to split client streams");

    Ok((Receiver::new(rx), Sender::new(tx)))
}

/// We call to this in the event that we are not registered yet.
async fn register_server(ip: String) -> Result<Id, Error> {
    let unique_id = generate_unique_id();
    let b = format!(
        "{{
        \"unique_id\": \"{}\"
    }}",
        unique_id
    );

    debug!(
        "Attempting to register websocket with: \n{}\nAt IP {}",
        b, ip
    );

    #[derive(Deserialize)]
    struct IdRes {
        message: String,
    }
    let id = reqwest::Client::new()
        .post(ip)
        .body(b)
        .send()
        .await?
        .json::<IdRes>()
        .await?;

    Ok(Id {
        unique_id,
        id: id.message.parse()?,
    })
}

/// Generate a unique id to represent this PC
fn generate_unique_id() -> uuid::Uuid {
    uuid::Uuid::new_v4()
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let mut cfg: Config = confy::load_path(CONFIG_PATH).expect("Failed to load config!");

    let mut db_pool = db::create_pool().expect("failed to create db pool");
    db::init_db(&db_pool).await.expect("failed to initalize db");

    loop {
        tokio::time::sleep(std::time::Duration::from_millis(std::cmp::max(
            cfg.reconnect_delay as u64,
            MIN_RECONNECT_DELAY as u64,
        )))
        .await;
        // Register websocket if not registered
        if cfg.clone().id.is_none() {
            let ip = format!(
                "{}://{}:{}/ws-register",
                cfg.prefix, cfg.server_ip, cfg.port
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
                cfg.clone().id.unwrap().id
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
            "ws://{}:{}/ws/{}",
            &cfg.server_ip,
            &cfg.port,
            cfg.clone().id.unwrap().id
        );

        let (rx, tx) = match connect_sever(&ip).await {
            Ok(f) => f,
            Err(e) => {
                debug_panic!("Failed to connect to webserver {:?}", e);
                continue;
            }
        };

        handle_ws(handle_message, (rx, tx), &mut db_pool, cfg.clone())
            .await
            .expect("Not Implemented"); //TODO
    }

    debug!("Connection closed, Server Agent exiting....");
    std::process::exit(0);
}

#[cfg(test)]
mod websocket_tests {
    use crate::db;
    use crate::db::DBPool;
    use crate::{connect_sever, handle_ws, Config, register_server};
    use futures::{FutureExt, StreamExt};
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
                format!("{{
                    \"message\": \"10568\"
                }}")
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
            let close_server_tx = create_websocket_server(([127, 0, 0, 1], 3033)).unwrap();

            let (mut rx, mut tx) = connect_sever("ws://127.0.0.1:3033/echo").await.unwrap();

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

            let close_server_tx = create_websocket_server(([127, 0, 0, 1], 3031)).unwrap();

            let (rx, mut tx) = connect_sever("ws://127.0.0.1:3031/echo").await.unwrap();

            let msg = Message::Message("Hello, World!".into());
            let e_msg = Message::Message("Hello, World!".into());

            //Send relevant sequences of messages
            tx.send(msg.clone()).await.unwrap();
            tx.send(e_msg.clone()).await.unwrap();

            tx.underlying().shutdown().unwrap();

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

            handle_ws(handle, (rx, s), &db_pool, cfg).await.unwrap();

            let _ = close_server_tx.send(());
        })
        .await
        .expect("Test failed due to timeout!");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_register() {
        let close_server_tx = create_http_server(([127, 0, 0, 1], 3034)).unwrap();

        let res = register_server("http://127.0.0.1:3034/test-register".into()).await.unwrap();

        assert_eq!(res.id, 10568);

        let _ = close_server_tx.send(());
    }
}
