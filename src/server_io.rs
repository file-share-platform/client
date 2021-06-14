use crate::errors::ServerError;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct RequestBody {
    path: String,
    usr: String,
    exp: u128,
    restrict_wget: bool,
    restrict_website: bool,
    name: String,
}

impl RequestBody {
    fn to_str(&self) -> String {
        return serde_json::to_string(self).unwrap();
    }

    pub fn default(path: &str) -> RequestBody {
        //TODO make this fill itself out with some reasonable defaults later down the line.
        return RequestBody {
            path: path.to_owned(),
            usr: "".to_owned(),
            exp: std::u128::MAX,
            restrict_wget: false,
            restrict_website: false,
            name: "File Share".to_owned(),
        }
    }
    pub fn set_path(mut self, new_path: &str) -> RequestBody {
        self.path = new_path.to_owned();
        return self;
    }
    pub fn set_usr(mut self, new_usr: &str) -> RequestBody {
        self.usr = new_usr.to_owned();
        return self;
    }
    pub fn set_exp(mut self, new_exp: &u128) -> RequestBody {
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
        .send()
        .await?;
    return Ok("hello".to_owned());
}