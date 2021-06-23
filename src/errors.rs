//Author Josiah Bull, Copyright 2021
//This module contains all of the error types and code for the project.
//It includes some implementations to and from standard error types where needed.

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
    FileExtensionError,
    FileNameError,
    FileSizeError(String),
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &*self {
            RequestError::FileExtensionError => f.write_str("Failed to parse file extension."),
            RequestError::FileNameError => f.write_str("Failed to parse file name."),
            RequestError::FileSizeError(text) => f.write_str(&text)
        }
    }
}

impl From<std::io::Error> for RequestError {
    fn from(error: std::io::Error) -> Self {
        RequestError::FileSizeError(format!("Failed to parse file size: {}", error.to_string()))
    }
}