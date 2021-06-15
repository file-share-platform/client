use serde::{Serialize, Deserialize};
use rocket::data::{self, Data, FromData, ToByteUnit};
use rocket::outcome::Outcome::*;
use rocket::http::{Status, ContentType};
use rocket::request::{self, Request};
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum ShareError {
    ParseError,
    TooLarge,
    FileDoesntExist,
    Io(std::io::Error),
    WrongComputer,
    ContentType,
}

#[derive(Serialize, Deserialize)]
pub struct Share {
    path: String,
    usr: String,
    exp: u128,
    restrict_wget: bool,
    restrict_website: bool,
    name: String,
}

#[rocket::async_trait]
impl<'r> FromData<'r> for Share {
    type Error = ShareError;

    async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> data::Outcome<'r, Self> {
        //Ensure correct content type
        let share_ct = ContentType::new("application", "json");
        if req.content_type() != Some(&share_ct) {
            return Failure((Status::UnsupportedMediaType, ShareError::ContentType));
        }

        let limit = req.limits().get("share").unwrap_or(256.bytes()); //Set the maximum size we'll unwrap

        //Read the data
        let string = match data.open(limit).into_string().await {
            Ok(string) if string.is_complete() => string.into_inner(),
            Ok(_) => return Failure((Status::PayloadTooLarge, ShareError::TooLarge)),
            Err(e) => return Failure((Status::InternalServerError, ShareError::Io(e))),
        };

        let string = request::local_cache!(req, string);

        // Attempt to parse the string with serde into our struct
        let share: Share = match serde_json::from_str(string) {
            Ok(share) => share,
            Err(e) => return Failure((Status::BadRequest, ShareError::ParseError)),
        };

        Success(share)
    }
}

impl Error for ShareError {
    fn description(&self) -> &str {
        match &*self {
            ShareError::ParseError => "Error parsing the share into a struct from JSON. Is the format correct? Did you include all fields?",
            ShareError::TooLarge => "The share was too large",
            ShareError::FileDoesntExist => "The file this share referenced doesn't exist",
            ShareError::WrongComputer => "This request was sent from a differnet computer than the one the server is hosted on",
            ShareError::ContentType => "Incorrect content type, expected application/JSON",
            ShareError::Io(_) => "Failed to read string",
        }
    }
}

impl fmt::Display for ShareError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &*self {
            ShareError::ParseError => f.write_str("Unable to contact server."),
            ShareError::TooLarge => f.write_str("The share was too large"),
            ShareError::FileDoesntExist => f.write_str("The file this share referenced doesn't exist"),
            ShareError::WrongComputer => f.write_str("This request was sent from a differnet computer than the one the server is hosted on"),
            ShareError::ContentType => f.write_str("Incorrect content type, expected application/JSON"),
            ShareError::Io(err) => f.write_str(&err.to_string()),
        }
    }
}