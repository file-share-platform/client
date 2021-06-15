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

// const 


#[macro_use] extern crate rocket;
use rocket::http::Status;
use rocket::response::{content, status};
use std::path::PathBuf;
mod structs;
use structs::{Share};

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
#[get("/download/<id>/<fileName>")]
fn download(id: String, fileName: String) -> &'static str {
    return "hello, world!"
}


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
fn share(share: Share) -> &'static str {
    return "hello, world!"
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
    rocket::build().mount("/", routes![heartbeat, share])
}
