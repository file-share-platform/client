use mobc_postgres::tokio_postgres;
use std::fmt::{self, Formatter};

#[derive(Debug)]
pub enum Error {
    DBPool(mobc::Error<tokio_postgres::Error>),
    DBQuery(tokio_postgres::Error),
    DBInit(tokio_postgres::Error),
    ReadFile(std::io::Error),
    Closed(String),
}

impl std::convert::From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::ReadFile(e)
    }
}
