use mobc_postgres::tokio_postgres;
use std::fmt::{self, Formatter};

#[derive(Debug)]
pub enum Error {
    DBPool(mobc::Error<tokio_postgres::Error>),
    DBQuery(tokio_postgres::Error),
    DBInit(tokio_postgres::Error),
    ReadFile(std::io::Error),
    Closed(String),
    Http(reqwest::Error),
    Conversion(String),
    WebSocket(websocket::WebSocketError),
}

impl std::convert::From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::ReadFile(e)
    }
}

impl std::convert::From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Error {
        Error::Http(e)
    }
}

impl std::convert::From<std::num::ParseIntError> for Error {
    fn from(e: std::num::ParseIntError) -> Error {
        Error::Conversion(e.to_string())
    }
}

impl std::convert::From<websocket::client::ParseError> for Error {
    fn from(e: websocket::client::ParseError) -> Error {
        Error::Conversion(e.to_string())
    }
}

impl std::convert::From<websocket::WebSocketError> for Error {
    fn from(e: websocket::WebSocketError) -> Error {
        Error::WebSocket(e)
    }
}

//TODO implement to_string