use serde::{Serialize, Deserialize};
use rocket::data::{self, Data, FromData, ToByteUnit};
use rocket::outcome::Outcome::*;
use rocket::http::{Status, ContentType, Header};
use rocket::request::{self, Request, FromRequest};
use rocket::fs::NamedFile;
use std::error::Error;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use rocket_dyn_templates::Template;
use derive_getters::Getters;

#[derive(Responder)]
#[response(status = 200)]
pub enum FileDownload {
    Download(NamedFile, ContentType, Header<'static>),
    Page(Template, ContentType)
}

pub struct UserAgent {
    pub agent: String
}

#[derive(Debug)]
pub enum ShareError {
    ParseError(String),
    TooLarge,
    Io(std::io::Error),
    ContentType,
    TimeError,
}

#[derive(Serialize, Deserialize, Getters, Clone)]
pub struct Share {
    uuid: String,
    usr: String,
    exp: u64,
    restrict_wget: bool,
    restrict_website: bool,
    name: String,
    #[serde(default)]
    crt: u64,
    size: u64,
    file_type: String,
}

impl Share {
    pub fn validate(&self) -> Result<(), ShareError> {
        //Check that the exp is actually after the created time stamp
        if self.exp < self.crt { //NB No error checking needed here.
            return Err(ShareError::TimeError);
        }        

        // Validate that restrict_wget and restrict_website aren't both set
        if self.restrict_wget && self.restrict_website {
            return Err(ShareError::ParseError("Both restrict_wget and restrict_website are set!".into()))
        }

        //TODO Implement validating the file type

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
        let share_ct = ContentType::new("text", "plain");
        if req.content_type() != Some(&share_ct) {
            return Failure((Status::UnsupportedMediaType, ShareError::ContentType));
        }

        let limit = req.limits().get("share").unwrap_or_else(|| 1024.bytes()); //Set the maximum size we'll unwrap
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
            Err(e) => return Failure((Status::BadRequest, ShareError::ParseError(format!("Unable to parse string with serde: {}", e.to_string())))),
        };

        //Set the time we received this request on the share.
        share.crt = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis() as u64;

        //Validate the share
        match share.validate() {
            Ok(_) => (),
            Err(e) => return Failure((Status::BadRequest, e)),
        };

        Success(share)
    }
}

impl crate::database::FromDatabase<rocket_sync_db_pools::rusqlite::Error> for Share {
    fn from_database(row: &rocket_sync_db_pools::rusqlite::Row<'_>) -> Result<Share, rocket_sync_db_pools::rusqlite::Error> {
        //SAFTEY: These should be safe, as the types with unwraps are disallowed from being null in the schema of the db.
        Ok(Share {
            //NB: Skip first col (0-th index) as that's the id
            uuid: row.get(1).unwrap(),
            usr: row.get(2).unwrap(),
            exp: row.get(3).unwrap(),
            restrict_wget: row.get(4).unwrap(),
            restrict_website: row.get(5).unwrap(),
            name: row.get(6).unwrap(),
            crt: row.get(7).unwrap(),
            size: row.get(8).unwrap(),
            file_type: row.get(9).unwrap(), 
        })
    }
}

impl Error for ShareError {
    fn description(&self) -> &str {
        match &*self {
            ShareError::ParseError(err) => &err,
            ShareError::TooLarge => "The share was too large",
            ShareError::ContentType => "Incorrect content type, expected application/JSON",
            ShareError::Io(_) => "Failed to read string",
            ShareError::TimeError => "The expiry date is set before the current time!",
        }
    }
}

impl fmt::Display for ShareError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &*self {
            ShareError::ParseError(err) => f.write_str(&err),
            ShareError::TooLarge => f.write_str("The share was too large"),
            ShareError::ContentType => f.write_str("Incorrect content type, expected application/JSON"),
            ShareError::Io(err) => f.write_str(&err.to_string()),
            ShareError::TimeError => f.write_str("The expiry date is set before the current time!"),
        }
    }
}