//Author Josiah Bull, Copyright 2021
//!This module contains all of the error types and code for the project.
//!
//!It includes some implementations to and from standard error types where needed.

use std::fmt;

///Represents the errors that can occur when attempting generating the request body client-side.
#[derive(Debug)]
pub enum Error {
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
    ///Failed to create a hard link to the file
    HardLinkError(String),
    
    DBQuery(mobc_postgres::tokio_postgres::Error),
    
    DBPool(mobc::Error<mobc_postgres::tokio_postgres::Error>),

    DBInit(mobc_postgres::tokio_postgres::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &*self {
            Error::FileExtensionError => f.write_str("Failed to parse file extension."),
            Error::FileNameError => f.write_str("Failed to parse file name."),
            Error::FileSizeError(text) => f.write_str(&format!("FileSizeError {}", text)),
            Error::FileExistError(text) => f.write_str(&format!("FileExistError {}", text)),
            Error::RestrictionError => {
                f.write_str("Cannot set both restrict_wget and restrict_website at the same time!")
            }
            Error::TimeError => f.write_str("Expiry time set in the past."),
            Error::HardLinkError(text) => f.write_str(&text),
            Error::DBQuery(_) => todo!(),
            Error::DBPool(_) => todo!(),
            Error::DBInit(_) => todo!(),
        }
    }
}

impl<'r> From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Error {
        Error::FileSizeError(error.to_string())
    }
}
