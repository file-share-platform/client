//Author Josiah Bull, Copyright 2021
//!This module contains all of the error types and code for the project.
//! 
//!It includes some implementations to and from standard error types where needed.

use std::error::Error;
use std::fmt;

///Represents errors that can occur when attempting to communicate with the file server.
#[derive(Debug)]
pub enum ServerError {
    ///404 Error, server wasn't able to be contacted.
    NotFoundError,
    ///The request was rejected by the server for some reason!
    RequestError(String),
    ///Failed to parse the struct to json representation using serde_json.
    ParseError(String),
}

impl Error for ServerError {
    fn description(&self) -> &str {
        match &*self {
            ServerError::NotFoundError => 
                "Unable to contact server.",
            ServerError::ParseError(text) =>
                &text,
            ServerError::RequestError(text) => 
                &text,
        }
    }
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &*self {
            ServerError::NotFoundError => f.write_str("Unable to contact server."),
            ServerError::ParseError(text) => f.write_str(&text),
            ServerError::RequestError(text) => f.write_str(&text),
        }
    }
}

impl From<reqwest::Error> for ServerError {
    fn from(error: reqwest::Error) -> Self {
        if let Some(code) = error.status() {
            if code == 404 {
                return ServerError::NotFoundError;
            }
            if code == 400 {
                return ServerError::RequestError(error.to_string());
            }
        }

        ServerError::NotFoundError
    }
}

impl From<serde_json::Error> for ServerError {
    fn from(error: serde_json::Error) -> ServerError {
        ServerError::ParseError(error.to_string())
    }
}

///Represents the errors that can occur when attempting generating the request body client-side.
#[derive(Debug)]
pub enum RequestError {
    ///An error occured trying to parse the file extension.
    FileExtensionError,
    ///An error occured trying to parse the file name.
    FileNameError,
    ///An error occured when trying to collect the file size, likely an IoError.
    FileSizeError(String),
    ///File Doesn't Exist
    FileExistError(String),
    ///Both restrict_wget and restrict_website have been set
    RestrictionError,
    ///Expiry is set to before the current time. 
    TimeError,
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &*self {
            RequestError::FileExtensionError => f.write_str("Failed to parse file extension."),
            RequestError::FileNameError => f.write_str("Failed to parse file name."),
            RequestError::FileSizeError(text) => f.write_str(&text),
            RequestError::FileExistError(text) => f.write_str(&text),
            RequestError::RestrictionError => f.write_str("Cannot set both restrict_wget and restrict_website at the same time!"),
            RequestError::TimeError => f.write_str("Expiry time set in the past."),
        }
    }
}

impl<'r> From<std::io::Error> for RequestError { 
    fn from(error: std::io::Error) -> RequestError {
        RequestError::FileSizeError(error.to_string())
    }
}
