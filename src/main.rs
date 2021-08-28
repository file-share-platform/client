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

const SAVE_PATH: &str = "/opt/fileShare";
const SERVER_IP: &str = "127.0.0.1:8000";

#[macro_use] extern crate rocket;
use rocket::response::{content, status};
use rocket::http::{ContentType, Status};

mod structs;
use structs::{Share, FileDownload, UserAgent};

mod database;
use database::{SharesDbConn, add_to_database, Search, setup};

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
/// It's 
#[get("/download/<uuid>/<file_name>?<force>")]
async fn download(uuid: String, file_name: String, user_agent: UserAgent, force: Option<String>, conn: SharesDbConn) -> Result<FileDownload, (Status, String)> {

    let share: Share = Search::Uuid(uuid).find_share(&conn).await?;

    //Request is coming from wget or curl, and wget is enabled. Lets allow a download!
    if user_agent.agent.to_lowercase().contains("wget") || user_agent.agent.to_lowercase().contains("curl") || force.is_some() {
        if *share.restrict_wget() && force.is_none() {
            return Err((Status::BadRequest, "Bad Request Client".into()));
        }
        //Download File
        return Ok(FileDownload::Download (
            NamedFile::open(format!("{}/hard_links/{}", SAVE_PATH, share.uuid())).await.unwrap(),
            ContentType::new("application", "octet-stream"),
            rocket::http::Header::new("content-disposition", format!("attachment; filename=\"{}\"", file_name)),
        ));
    } 

    //Otherwise, return them the page
    if *share.restrict_website() {
        return Err((Status::BadRequest, "Bad Request Client".into()));
    }
    Ok(FileDownload::Page (
        Template::render("download", share),
        ContentType::new("text", "html")
    ))
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
async fn share(share: Share, conn: SharesDbConn) -> Result<String, (Status, String)> {
    let response = format!("http://{}/download/{}/{}.{}", SERVER_IP, share.uuid(), share.name(), share.file_type());
    setup(&conn).await?;
    //Validate hard link file exists
    //TODO

    //Add share to database
    add_to_database(&conn, share).await?;
    
    Ok(response)
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
        .attach(SharesDbConn::fairing())
        .attach(Template::fairing())
}
