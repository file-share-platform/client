//Author Josiah Bull, Copyright 2021

use crate::errors::ServerError;
use crate::{NAME, DEFAULT_SHARE_TIME_HOURS};
use serde::{Serialize, Deserialize};
use std::collections::hash_map::DefaultHasher;
use reqwest::header::{USER_AGENT as UserAgent, CONTENT_TYPE as ContentType};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use crate::errors::RequestError;
use crate::hash::ComputerIdentifier;
use std::time::{SystemTime, UNIX_EPOCH};
use whoami;

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

impl RequestBody {
    fn to_str(&self) -> String {
        return serde_json::to_string(self).unwrap();
    }

    pub fn new(path: &str) -> Result<RequestBody, RequestError> {
        //TODO make this fill itself out with some reasonable defaults later down the line.

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
    pub fn set_path(mut self, new_path: &str) -> RequestBody {
        self.path = new_path.to_owned();
        return self;
    }
    pub fn set_usr(mut self, new_usr: &str) -> RequestBody {
        self.usr = new_usr.to_owned();
        return self;
    }
    pub fn set_exp(mut self, new_exp: &u64) -> RequestBody {
        self.exp = new_exp.to_owned();
        return self;
    }
    pub fn set_restrict_wget(mut self, new_wget: &bool) -> RequestBody {
        self.restrict_wget = new_wget.to_owned();
        return self;
    }
    pub fn set_restrict_website(mut self, new_website: &bool) -> RequestBody {
        self.restrict_website = new_website.to_owned();
        return self;
    }
    pub fn set_name(mut self, new_name: &str) -> RequestBody {
        self.name = new_name.to_owned();
        return self;
    }
    pub fn validate(&self) -> bool {
        //TODO Implement validation
        return true;
    }
}

pub async fn check_heartbeat(address: &str) -> Result<bool, ServerError> {
    let body = reqwest::get(address).await?;
    return Ok(true);
}

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