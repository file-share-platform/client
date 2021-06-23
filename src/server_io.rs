use crate::errors::ServerError;
use crate::NAME;
use serde::{Serialize, Deserialize};
use std::collections::hash_map::DefaultHasher;
use reqwest::header::{USER_AGENT as UserAgent, CONTENT_TYPE as ContentType};
use std::ffi::OsStr;
use std::path::Path;
use crate::errors::RequestError;
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
        //Create the file extension. TODO: Validate for empty extensions? Is that even going to be a problem?
        let file_type: String = match Path::new(path).extension().and_then(OsStr::to_str) {
            Some(file) => file.to_owned(),
            None => return Err(RequestError::FileExtensionError),
        };

        return Ok( RequestBody {
            path: path.into(),
            usr: "test_usr".into(),
            exp: std::u64::MAX,
            restrict_wget: false,
            restrict_website: false,
            name: "File_Share".into(),
            size: 15000,
            file_type: file_type,
            computer: 12312, //A hash which relates to some basic info about this computer
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