//Author Josiah Bull, Copyright 2021

//!Handles communication with the file server.
use crate::errors::ServerError;
use crate::{NAME, DEFAULT_SHARE_TIME_HOURS, SIZE_LIMIT, SERVER_FILE_LOCATION};
use serde::{Serialize, Deserialize};
use reqwest::header::{USER_AGENT as UserAgent, CONTENT_TYPE as ContentType};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use crate::errors::ShareError;
use crate::common::*;
use std::fs::hard_link;
use uuid;

///Represents the data required to send a share request to the file server.
#[derive(Serialize, Deserialize, Debug)]
pub struct Share {
    id: String, //A uuid
    usr: String,
    exp: u64,
    restrict_wget: bool,
    restrict_website: bool,
    name: String,
    size: u64,
    file_type: String,
}
#[allow(dead_code)]
impl Share {
    ///Convert the file to a json string for saving.
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
    ///Returns a result which contains `Share` if successful, otherwise retursn a `ShareError`. Populates all values with reasonable defaults derived from the provided path.
    pub fn new(path_raw: &str) -> Result<Share, ShareError> {
        //Check file exists
        let path = Path::new(path_raw);
        if !path.exists() {
            return Err(ShareError::FileExistError("File doesn't exist!".into()));
        }
        
        if path.is_dir() {
            return Err(ShareError::FileExistError("File is a directory!".into()))
        }

        //Create the file extension. 
        let file_type: String = match path.extension().and_then(OsStr::to_str) {
            Some(file) => file.to_owned(),
            None => return Err(ShareError::FileExtensionError),
        };

        //Extract filename
        let name: String = match path.file_stem().and_then(OsStr::to_str) {
            Some(name) => name.to_owned(),
            None => return Err(ShareError::FileNameError)
        };
        
        //Collect size of file
        let size: u64 = PathBuf::from(path).metadata()?.len();
        if size > SIZE_LIMIT {
            return Err(ShareError::FileSizeError("Too Large!".into()));
        } 

        let id: String = uuid::Uuid::new_v4().to_string();

        //Create a hard link to the relevant file
        if let Err(e) = hard_link(path_raw, format!("{}/hard_links/{}", SERVER_FILE_LOCATION, id)) {
            return Err(ShareError::HardLinkError(e.to_string()));
        }

        Ok(Share {
            id,
            usr: whoami::realname(),
            exp: get_time() + (DEFAULT_SHARE_TIME_HOURS * 60 * 60 * 1000) as u64,
            restrict_wget: false,
            restrict_website: false,
            name,
            size,
            file_type,
        })
    }

    ///Returns the id of the current share file, as a readonly string slice.
    pub fn get_id(&self) -> &str {
        &self.id
    }

    ///Takes a `&str` representing the new user, and returns a `Share`. Has no validation, so it is recommended to validate your `&str` beforehand. Can be chained with other `set_???` functions.
    pub fn set_usr(mut self, new_usr: &str) -> Share {
        self.usr = new_usr.to_owned();
        self
    }
    ///Takes a `&u64` representing the new expiry time (in milliseconds since the epoch), and returns a `Share`. Has no validation, so it is recommended to validate your `&u64` beforehand. Can be chained with other `set_???` functions.
    pub fn set_exp(mut self, new_exp: &u64) -> Share {
        self.exp = new_exp.to_owned();
        self
    }
    ///Takes a `bool` representing whether or not this share can be accessed by wget, and returns a `Share`. Has no validation, so it is recommended to validate your `&bool` beforehand (e.g. ensure that you're not also setting `restrict_website`!). Can be chained with other `set_???` functions.
    pub fn set_restrict_wget(mut self, new_wget: bool) -> Share {
        self.restrict_wget = new_wget.to_owned();
        self
    }
    ///Takes a `bool` representing whether or not this share can be accessed by the website, and returns a `Share`. Has no validation, so it is recommended to validate your `&bool` beforehand (e.g. ensure that you're not also setting `restrict_wget`!). Can be chained with other `set_???` functions.
    pub fn set_restrict_website(mut self, new_website: bool) -> Share {
        self.restrict_website = new_website.to_owned();
        self
    }
    ///Takes a `&str` representing a new author name, and returns a `Share`. Has no validation, so it is recommended to validate your `&str` beforehand. Can be chained with other `set_???` functions.
    pub fn set_name(mut self, new_name: &str) -> Share {
        self.name = new_name.to_owned();
        self
    }
    ///Takes a `&u64` representing a file size override, and returns a `Share`. Has no validation, so it is recommended to validate your `&u64` beforehand. Can be chained with other `set_???` functions.
    pub fn set_size(mut self, new_size: &u64) -> Share {
        self.size = new_size.to_owned();
        self
    }
    ///Used to validate all values in the struct. Will test the paths, check exp dates are valid, and do a whole bunch of other checks to prevent problems when we attempt to send this file to the server.
    pub fn validate(&self) -> Result<(), ShareError> {  
        if self.restrict_website && self.restrict_wget {
            return Err(ShareError::RestrictionError);
        }

        if self.exp < get_time() {
            return Err(ShareError::TimeError);
        }

        Ok(())
    }
}

///Sends a "ping" to the server, returns a `Result` which is Ok bif the server is ready for a request, and returns `ServerError` otherwise.
pub async fn check_heartbeat(address: &str) -> Result<(), ServerError> {
    reqwest::get(address).await?;
    Ok(())
}

///Notify the local fileserver that a new share has been created
pub async fn notify_new_share(address: &str, file_name: Share) -> Result<String, ServerError> {
    let client = reqwest::Client::new();
    let res = client.post(address)
        .body(file_name.to_json_string()?)
        .header(UserAgent, NAME)
        .header(ContentType, "text")
        .send()
        .await?
        .text()
        .await?;
    Ok(res)
}

// #[cfg(test)]
// #[doc(hidden)]
// mod server_io_tests {
//     use crate::server_io::*;
//     use crate::errors::*;
//     use std::fs::File;
//     use std::io::prelude::*;

//     struct TestFile {
//         path: String
//     }
    

//     impl TestFile {
//         fn new(name: &str) -> Self {
//             let path = format!("test_files/{}", name);
//             let mut file = File::create(&path).expect("Failed to create file.");
//             file.write_all(b"This is a test file, it should be deleted.").expect("Failed to write to file");
//             TestFile {
//                 path
//             }
//         }

//         fn cleanup(self) {
//             std::fs::remove_file(self.path).expect("Failed to remove file! Please manually clean up!");
//         }
//     }

    
//     #[tokio::test]
//     async fn test_heartbeat_success() {
//         let output = check_heartbeat("https://www.google.com/").await;
//         assert_eq!(output.unwrap(), ());
//     }

    
//     #[tokio::test]
//     async fn test_heartbeat_failure() {
//         let output = check_heartbeat("http://ThisAddressWillNeverExist-afelakjedflakej/Things.com").await;

//         match output {
//             Ok(_) => panic!("This should not produce an ok value!"),
//             Err(e) => {
//                 match e {
//                     ServerError::NotFoundError => (),
//                     e => panic!("Expected error type of NotFoundError. Got : {}", e)
//                 }
//             }
//         }
//     }

//     #[test]
//     fn test_request_body_success() {
//         let file: TestFile = TestFile::new("foo.pdf");

//         let req_body: RequestBody = RequestBody::new(&file.path).expect("Failed to create request body!");

//         assert_eq!(req_body.restrict_wget, false);
//         assert_eq!(req_body.restrict_website, false);
//         assert_eq!(req_body.file_type, "pdf");
//         assert_eq!(req_body.name, "foo");

//         file.cleanup();
//     }

//     //Test modifiers for RequestBody
//     #[test]
//     fn test_request_body_modifiers() {
//         let file = TestFile::new("foo.txt");

//         let mut req_body: RequestBody = RequestBody::new(&file.path).expect("Failed to create request body!");
        
//         //Let set a whole bunch of values and check that they appear on the output!
//         req_body = req_body
//             .set_exp(&1234)
//             .set_name("Jane Doe")
//             .set_restrict_website(true)
//             .set_restrict_wget(false);
        
//         assert_eq!(req_body.restrict_website, true);
//         assert_eq!(req_body.restrict_wget, false);
//         assert_eq!(req_body.exp, 1234);
//         assert_eq!(req_body.name, "Jane Doe");

//         file.cleanup();
//     }

//     #[test]
//     fn test_request_body_extension_error() {
//         let file = TestFile::new("foo");

//         match RequestBody::new(&file.path).expect_err("This should have errored!") {
//             RequestError::FileExtensionError => (),
//             e => panic!("Expected error type of FileExtensionError. Got : {}", e)
//         }

//         file.cleanup();
//     }

//     #[test]
//     fn test_request_body_file_exist_error() {
//         match RequestBody::new("berries.txt").expect_err("This should have errored!") {
//             RequestError::FileExistError(_) => (),
//             e => panic!("Expected error type of FileExistError. Got : {}", e)
//         }
//     }

//     #[test]
//     fn test_request_body_file_size() {
//         //Not implemented
//     }

//     #[test]
//     fn test_request_body() {
//         let file = TestFile::new("foo.mp4");

//         let mut req_body: RequestBody = RequestBody::new(&file.path).expect("Failed to create request body!");
//         req_body = req_body
//             .set_restrict_wget(true)
//             .set_restrict_website(true);

//         match req_body.validate().expect_err("This should be an error!") {
//             RequestError::RestrictionError => (),
//             e => panic!("Expected error type of RestrictionError. Got : {}", e)
//         }

//         file.cleanup();
//     }

//     #[test]
//     fn test_validation_default() {
//         let file = TestFile::new("foo.mp3");

//         let req_body: RequestBody = RequestBody::new(&file.path).expect("Failed to create request body!");

//         req_body.validate().expect("Validation failed on the default body!");

//         file.cleanup();
//     }

//     #[test]
//     fn test_validation() {
//         let file = TestFile::new("foo.mp2");

//         let mut req_body: RequestBody = RequestBody::new(&file.path).expect("Failed to create request body!");
        
//         req_body = req_body.set_exp(&1234);
//         match req_body.validate().expect_err("This should be an error!") {
//             RequestError::TimeError => (),
//             e => panic!("Expected error type of TimeError. Got : {}", e)
//         }

//         req_body = req_body
//             .set_exp(&std::u64::MAX);
//         match req_body.validate().expect_err("This should be an error!") {
//             RequestError::FileExistError(_) => (),
//             e => panic!("Expected error type of FileExistError. Got : {}", e)
//         }

//         req_body = req_body
//             .set_restrict_website(true)
//             .set_restrict_wget(true);
//         match req_body.validate().expect_err("This should be an error!") {
//             RequestError::RestrictionError => (),
//             e => panic!("Expected error type of RestrictionError. Got : {}", e)
//         }

//         file.cleanup();
//     }

// }