//Author Josiah Bull, Copyright 2021

//!Handles communication with the file server.
use crate::errors::ServerError;
use crate::{NAME, DEFAULT_SHARE_TIME_HOURS};
use serde::{Serialize, Deserialize};
use reqwest::header::{USER_AGENT as UserAgent, CONTENT_TYPE as ContentType};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use crate::errors::RequestError;
use crate::hash::ComputerIdentifier;
use std::time::{SystemTime, UNIX_EPOCH};
use whoami;

///Represents the data required to send a share request to the file server.
#[derive(Serialize, Deserialize, Debug)]
pub struct RequestBody {
    path: String,
    usr: String,
    exp: u64,
    restrict_wget: bool,
    restrict_website: bool,
    name: String,
    size: u64,
    file_type: String,
    computer: u64,
}
#[allow(dead_code)]
impl RequestBody {
    fn to_str(&self) -> String {
        return serde_json::to_string(self).unwrap();
    }
    ///Returns a result which contains `RequestBody` if successful, otherwise retursn a `RequestError`. Populates all values with reasonable defaults derived from the provided path.
    pub fn new(path: &str) -> Result<RequestBody, RequestError> {
        //Create the file extension. 
        let file_type: String = match Path::new(path).extension().and_then(OsStr::to_str) {
            Some(file) => file.to_owned(),
            None => return Err(RequestError::FileExtensionError),
        };

        //Extract filename
        let file_name: String = match Path::new(path).file_name().and_then(OsStr::to_str) {
            Some(name) => name.to_owned(),
            None => return Err(RequestError::FileNameError)
        };
        
        //Collect size of file
        let file_size: u64 = PathBuf::from(path).metadata()?.len();  

        return Ok( RequestBody {
            path: path.into(),
            usr: whoami::realname(),
            exp: (SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis() + DEFAULT_SHARE_TIME_HOURS * 60 * 60 * 1000) as u64,
            restrict_wget: false,
            restrict_website: false,
            name: file_name,
            size: file_size,
            file_type: file_type,
            computer: ComputerIdentifier::default().get_hash(None), //A hash which relates to some basic info about this computer
        });
    }
    ///Takes a `&str` representing the new path, and returns a `RequestBody`. Has no validation, so it is recommended to validate your `&str` beforehand. Can be chained with other `set_???` functions.
    pub fn set_path(mut self, new_path: &str) -> RequestBody {
        self.path = new_path.to_owned();
        return self;
    }
    ///Takes a `&str` representing the new user, and returns a `RequestBody`. Has no validation, so it is recommended to validate your `&str` beforehand. Can be chained with other `set_???` functions.
    pub fn set_usr(mut self, new_usr: &str) -> RequestBody {
        self.usr = new_usr.to_owned();
        return self;
    }
    ///Takes a `&u64` representing the new expiry time (in milliseconds since the epoch), and returns a `RequestBody`. Has no validation, so it is recommended to validate your `&u64` beforehand. Can be chained with other `set_???` functions.
    pub fn set_exp(mut self, new_exp: &u64) -> RequestBody {
        self.exp = new_exp.to_owned();
        return self;
    }
    ///Takes a `&bool` representing whether or not this share can be accessed by wget, and returns a `RequestBody`. Has no validation, so it is recommended to validate your `&bool` beforehand (e.g. ensure that you're not also setting `restrict_website`!). Can be chained with other `set_???` functions.
    pub fn set_restrict_wget(mut self, new_wget: &bool) -> RequestBody {
        self.restrict_wget = new_wget.to_owned();
        return self;
    }
    //Takes a `&bool` representing whether or not this share can be accessed by the website, and returns a `RequestBody`. Has no validation, so it is recommended to validate your `&bool` beforehand (e.g. ensure that you're not also setting `restrict_wget`!). Can be chained with other `set_???` functions.
    pub fn set_restrict_website(mut self, new_website: &bool) -> RequestBody {
        self.restrict_website = new_website.to_owned();
        return self;
    }
    ///Takes a `&str` representing a new author name, and returns a `RequestBody`. Has no validation, so it is recommended to validate your `&str` beforehand. Can be chained with other `set_???` functions.
    pub fn set_name(mut self, new_name: &str) -> RequestBody {
        self.name = new_name.to_owned();
        return self;
    }
    ///Used to validate all values in the struct. Will test the paths, check exp dates are valid, and do a whole bunch of other checks which may cause problems when we attempt to send this file to the server.
    pub fn validate(&self) -> Result<(), RequestError> {
        //TODO Implement validation
        Ok(())
    }
}

///Sends a "ping" to the server, returns a `Result` which is Ok if the server is ready for a request, and returns `ServerError` otherwise.
pub async fn check_heartbeat(address: &str) -> Result<(), ServerError> {
    reqwest::get(address).await?;
    return Ok(());
}

///Send a share to the server, expects to recieve a single line in response containing the link to the now-shared file.
pub async fn send_file(address: &str, data: RequestBody) -> Result<String, ServerError> {
    let client = reqwest::Client::new();
    let res = client.post(address)
        .body(data.to_str().to_owned())
        .header(UserAgent, format!("{}", NAME))
        .header(ContentType, "application/json")
        .send()
        .await?
        .text()
        .await?;
    return Ok(res);
}