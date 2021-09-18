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

use websocket::ClientBuilder;
use ws_com_framework::{File, FileRequest, Message, Receiver, Sender};
use db::{DBCon, DBPool};
use error::Error;

const SERVER_IP: &str = "ws://127.0.0.1:3030/ws/1";
const SECURE_CONNECTION: bool = false; //TODO, not implemented
const DEBUG: bool = true;

/// A copy of println!, which only prints when the global const DEBUG is true.
/// This makes debugging quick and easy to toggle.
macro_rules! debug {
    () => {
        if DEBUG { println!(); }
    };
    ($fmt_string:expr) => {
        if DEBUG { 
            println!($fmt_string); 
        }
    };
    ($fmt_string:expr, $( $arg:expr ),*) => {
        if DEBUG { 
            println!($fmt_string, $( $arg ),*); 
        }
    }
}

/// When called, if application is in DEBUG mode will panic. Otherwise will merely print the error to the console.
macro_rules! debug_panic {
    ($fmt_string:expr) => {
        if DEBUG {
            panic!($fmt_string);
        } else {
            println!($fmt_string);
        }
    };
    ($fmt_string:expr, $( $arg:expr ),*) => {
        if DEBUG {
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

async fn handle_message(m: Message, db: DBPool) -> Result<Option<Message>, Error>
{
    match m {
        Message::Upload(u) => {
            if let Some(f) = db::Search::uuid(u.id()).find(&db).await? {
                //TODO start file upload
                return Ok(None)
            } else {
                okie!(Message::Error(ws_com_framework::Error::FileDoesntExist))
            };
        },
        Message::Metadata(f) => {
            if let Some(f) = db::Search::uuid(f.id()).find(&db).await? {
                return Ok(Some(f.into()))
            }
            return Ok(Some(ws_com_framework::Error::FileDoesntExist.into()))
        },
        Message::Close(c) => return Err(Error::Closed(c)),
        e => {
            debug_panic!("Unsupported message, recieved! {:?}", e);
            return Ok(None)
        },
    }
}

async fn handle_ws<F, R, S, Fut>(handle: F, (mut rx, mut tx): (Receiver<R>, Sender<S>), db: &DBPool) -> Result<(), ()>
where
    F: Fn(Message, DBPool) -> Fut,
    R: ws_com_framework::RxStream,
    S: ws_com_framework::TxStream,
    Fut: std::future::Future<Output = Result<Option<Message>, Error>>
{
    //Loop to get messages
    while let Some(m) = rx.next().await {
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

        handle(m, db.clone()).await.unwrap(); //TODO
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

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let mut db_pool = db::create_pool().expect("failed to create db pool");
    db::init_db(&db_pool).await.expect("failed ti initalize db");

    loop {
        let (mut rx, mut tx) = connect_sever(SERVER_IP).await.unwrap();


        let res = handle_ws(handle_message, (rx, tx),  &mut db_pool).await;

    }

    debug!("Connection closed, Server Agent exiting....");
    std::process::exit(0);
}

#[cfg(test)]
mod websocket_tests {
    use crate::{connect_sever, handle_ws, DEBUG};
    use futures::{FutureExt, StreamExt};
    use std::time::Duration;
    use tokio::sync::oneshot;
    use tokio::time::timeout;
    use warp;
    use warp::Filter;
    use ws_com_framework::{Message, Receiver, Sender};
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
    fn create_http_server() -> Result<(), ()> {
        Ok(())
    }

    /// Test that websocket is able to succesfully connect to a provided server, and send a simple message
    /// Will timeout and fail anyway after 10 seconds - this usually indicates the test has failed.
    #[tokio::test(flavor = "multi_thread")]
    async fn websocket_connect() {
        timeout(Duration::from_millis(10_000), async {
            let close_server_tx = create_websocket_server(([127, 0, 0, 1], 3030)).unwrap();

            let (mut rx, mut tx) = connect_sever("ws://127.0.0.1:3030/echo").await.unwrap();

            let msg = Message::Message("Hello, World!".into());

            tx.send(msg.clone()).await.unwrap();

            let m = rx.next().await.unwrap().unwrap();

            assert_eq!(msg, m);

            let _ = close_server_tx.send(());
        })
        .await
        .expect("Test failed due to timeout!");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 5)]
    async fn websocket_handle() {
        timeout(Duration::from_millis(5000), async {
            let close_server_tx = create_websocket_server(([127, 0, 0, 1], 3031)).unwrap();

            let (rx, mut tx) = connect_sever("ws://127.0.0.1:3031/echo").await.unwrap();

            let msg = Message::Message("Hello, World!".into());
            let e_msg = Message::Message("Hello, World!".into());

            //Send relevant sequences of messages
            tx.send(msg.clone()).await.unwrap();
            tx.send(e_msg.clone()).await.unwrap();

            tx.underlying().shutdown().unwrap();

            //This function should process the messages we sent (to ensure they're all getting through)
            fn handle<S>(m: Message, _: &mut S) -> Result<(), ()> {
                match m.clone() {
                    Message::Message(t) => {
                        if t != "Hello, World!".to_owned() {
                            panic!("Unexpected message recieved! {:?}", m);
                        }
                    }
                    _ => panic!("Unexpected message recieved! {:?}", m),
                }
                Ok(())
            }

            let (tx, _) = tokio::sync::mpsc::unbounded_channel::<Message>();
            let s = Sender::new(tx);

            handle_ws(handle, (rx, s)).await.unwrap();

            let _ = close_server_tx.send(());
        })
        .await
        .expect("Test failed due to timeout!");
    }
}
