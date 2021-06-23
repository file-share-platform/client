use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum ServerError {
    NotFoundError,
}

impl Error for ServerError {
    fn description(&self) -> &str {
        match *self {
            ServerError::NotFoundError => 
                "Unable to contact server.",
        }
    }
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ServerError::NotFoundError => f.write_str("Unable to contact server."),
        }
    }
}

impl From<reqwest::Error> for ServerError {
    fn from(error: reqwest::Error) -> Self {
        ServerError::NotFoundError
    }
}

#[derive(Debug)]
pub enum RequestError {
    FileExtensionError
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            RequestError::FileExtensionError => f.write_str("Problem parsing file extension."),
        }
    }
}