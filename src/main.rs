//Author Josiah Bull
//This script is designed to be a counter part to the cli application.
//Represents a simple webserver which takes basic requests to a handful of endpoints.
//Implements some *very* basic encryption to prevent any old application explroing the endpoints.
//TODO: Implement some sort of external service which does a check to see if the port forward is running.
//Endpoints:
// heartbeat: A simple endpoint to check if the server is up and running, if there is a problem returns 500, 200 otherwise.
// share: Creates a share, returns a simple plain http/text response with the url of the created link.
// URL should look like: http://54.150.23.450/share/123d-d212-3dlk-dafe/yourFile/

//Global Config

const SAVE_PATH: &str = "/home/josiah/MEGA/share";
const SERVER_IP: &str = "127.0.0.1:8000";

#[macro_use] extern crate rocket;
use rocket::response::{content, status};
use rocket::http::{ContentType, Status};
use std::path::PathBuf;
mod structs;
use structs::{Share, Link, FileDownload, UserAgent};
use uuid::Uuid;
use std::fs::File;
use std::io::prelude::*;
use rocket::fs::NamedFile;
use rocket_dyn_templates::Template;

/// Loads the page for downloading a file! Also does a simple check to see if a request is coming from curl or wget.
/// 
/// Address: /download/:ID/:FILENAME/
/// Returns an html webpage, or a handle to the file
/// 
/// Request:
/// ```DOS
///     WGET http://:SERVER_URL/download/some-uuid-code-yuup/YourFile/
/// ```
/// 
#[get("/download/<id>/<file_name>")]
async fn download(id: u128, file_name: String, user_agent: UserAgent) -> Result<FileDownload, (Status, String)> {
    //We need to create a link, then check if the link exists
    let link: Link = Link::new(&file_name, id);
    println!("Some and things: {}", user_agent.agent);
    //Check if the file exists
    if !PathBuf::from(link.to_file()).exists() {
        return Err((Status::NotFound, "File not found".into()));
    }

    //Open file
    let mut file = match File::open(PathBuf::from(link.to_file())) {
        Ok(file) => file,
        Err(e) => return Err((Status::InternalServerError, e.to_string()))
    };
    //Read file
    let mut content: String = String::new();
    match file.read_to_string(&mut content) {
        Ok(_) => (),
        Err(e) => return Err((Status::InternalServerError, e.to_string()))
    };
    //Parse content
    let share: Share = match serde_json::from_str(&content) {
        Ok(share) => share,
        Err(e) => return Err((Status::InternalServerError, e.to_string())),
    };
    //Validate the share
    match share.validate() {
        Ok(_) => (),
        Err(e) => return Err((Status::InternalServerError, e.to_string())),
    };

    //Request is coming from wget or curl, and wget is enabled. Lets allow a download!
    if user_agent.agent.to_lowercase().contains("wget") || user_agent.agent.to_lowercase().contains("curl") {
        if share.restrict_wget {
            return Err((Status::BadRequest, "Bad Request Client".into()));
        }
        //Download File
        return Ok(FileDownload::Download (
            NamedFile::open(&share.path).await.unwrap(), //NB, while this could theoretically error share.validate() does a check that the file exists so it *shouldnt*.
            ContentType::new("application", "octet-stream"),
            rocket::http::Header::new("content-disposition", format!("attachment; filename=\"{}\"", &share.name)),
        ));
    } 

    //Otherwise, return them the page
    if share.restrict_website {
        return Err((Status::BadRequest, "Bad Request Client".into()));
    }
    return Ok(FileDownload::Page (
        Template::render("download", share.to_string()),
        ContentType::new("text", "html")
    ));
    // inner: NamedFile::open("/home/josiah/Documents/rust-sharing-server/www/static/download.html").await.unwrap(), //NB, Should never fail as this will link to templates
    // content_type: ContentType::new("text", "html"),
    // more: rocket::http::Header::new("content-disposition", "inline"),
}

// #[get("/download/<id>/<fileName>?force")]
// fn download_forced(id: String, fileName: String) -> Option<NamedFile> {

// }


/// Returns a url which can be used to download a file from anywhere!
/// 
/// Address: /share
/// Returns 200 or 404. 
/// 
/// Returns a link which will allow the file to be downloaded from anywhere!
/// 
/// Request:
/// ```JSON
/// POST http://:SERVER_URL/share/
/// {
///     "": ""
/// }
/// ```
/// 
/// Response (JSON):
/// ```JSON
/// {
///     "url": "http://:SERVER_URL/download/some-uuid-code-yuup/YourFile/"
/// }
/// ```
#[post("/share", data="<share>")]
fn share(share: Share) -> (Status, (ContentType, String)) {
    //Process the share into an available download
    //Create the link file, which is the file which holds the record of the shares we are looking to store.
    let link: Link = Link::new(&share.name, Uuid::new_v4().as_u128());

    //First check the file doesn't already exist!
    if PathBuf::from(link.to_file()).exists() {
        return (Status::BadRequest, (ContentType::new("text", "html"), String::from("This file has already been shared!")))
    }

    //Create the file
    println!("{}",link.to_file());
    let mut file = match File::create(link.to_file()) {
        Ok(file) => file,
        Err(e) => return (Status::InternalServerError, (ContentType::new("text", "html"), format!("Failed to create temporary file: {}", e.to_string()))),
    };

    //Write the relevant details to the file
    let file_content = match serde_json::to_string(&share) {
        Ok(data) => data,
        Err(e) => return (Status::InternalServerError, (ContentType::new("text", "html"), format!("Failed to serialize response: {}", e.to_string()))),
    };

    if file.write_all(file_content.as_bytes()).is_err() {
        return (Status::InternalServerError, (ContentType::new("text", "html"), String::from("Failed to write to link file")));
    }
    
    (Status::Ok, (ContentType::new("text", "html"), link.to_url()))
}

/// Returns the status of the server, is meant to be used to check if the server is alive.
/// 
/// Address: /heartbeat
/// Takes no arguments.
/// Returns 200 or 404.
/// 
/// 
/// Eventually may return some more useful information about the status of the server, but currently either returns if it's alive, or (obviously) won't if the server is dead.
/// 
/// Request:
/// ```
/// GET /heartbeat 
/// ```
/// 
/// Response (JSON):
/// ```
/// {
///     "status": "online"
/// }
/// ```
/// 
#[get("/heartbeat")]
fn heartbeat() -> status::Custom<content::Json<&'static str>> {
    status::Custom(Status::ImATeapot, content::Json("{ \"status\": \"online\" }")) //TODO implement some form of useful feedback (i.e. online, offline, degraded?)
}

//TODO
// #[catch(404)]
// fn not_found(req: &Request) -> String {

// }

#[doc(hidden)]
#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![heartbeat, share, download])
        .attach(Template::fairing())
}
