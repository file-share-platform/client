use serde::{Serialize, Deserialize};
use rocket::data::{self, Data, FromData, ToByteUnit};
use rocket::outcome::Outcome::*;
use rocket::http::{Status, ContentType, Header};
use rocket::request::{self, Request, FromRequest};
use rocket::fs::NamedFile;
use std::error::Error;
use std::path::PathBuf;
use std::fmt;
use crate::{SERVER_IP, SAVE_PATH};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Responder)]
#[response(status = 200)]
pub struct FileDownload {
    pub inner: NamedFile,
    pub content_type: ContentType,
    pub more: Header<'static>,
}

pub struct UserAgent {
    pub agent: String
}


#[derive(Deserialize, Serialize)]
pub struct Link {
    file: String,
    uuid: u128,
}

impl Default for Link {
    fn default() -> Link {
        Link {
            file: String::default(),
            uuid: u128::default(),
        }
    }
}
impl Link {
    pub fn new(file: &str, uuid: u128) -> Link {
        Link {
            file: String::from(file),
            uuid,
        }
    }
    pub fn to_url(&self) -> String {
        return format!("http://{}/download/{}/{}", SERVER_IP, self.uuid, self.file);
    }
    pub fn to_file(&self) -> String {
        return format!("{}/files/{}{}.link", SAVE_PATH, self.uuid, self.file)
    }
}

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
    pub path: String,
    usr: String,
    exp: u128,
    pub restrict_wget: bool,
    pub restrict_website: bool,
    pub name: String,
    computer: String,
    created: Option<u128>,
}

impl Share {
    pub fn validate(&self) -> Result<(), ShareError> {
        //TODO Check that this request came from the same computer
    
        //Check that the file does exist on the drive
        if !PathBuf::from(&self.path).exists() {
            return Err(ShareError::FileDoesntExist);
        }

        //TODO Check that we haven't reached the exp of this request

        //TODO Validate that restrict_wget and restrict_website aren't both set

        Ok(())
    }
}
#[derive(Debug)]
pub enum UserAgentError {
    ParseError
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for UserAgent {
    type Error = UserAgentError;
    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let agent_name = match req.headers().get_one("User-Agent") {
            Some(name) => name,
            None => return Failure((Status::BadRequest, UserAgentError::ParseError)),
        };
        return Success(
            UserAgent {
                agent: agent_name.to_owned()
            }
        );
    }
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
        let mut share: Share = match serde_json::from_str(string) {
            Ok(share) => share,
            Err(e) => return Failure((Status::BadRequest, ShareError::ParseError)),
        };

        //Validate the share
        match share.validate() {
            Ok(_) => (),
            Err(e) => return Failure((Status::BadRequest, e)),
        };

        //Set the time we received this request on the share.
        share.created = Some(SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis() as u128);

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