#![allow(dead_code)]
//Author: Josiah Bull

//TODO check that xorg-dev is installed! It's needed for clipboard interaction on linux.

//This is a small cli applet
//Supported commands:
//share :file, puts a link to the file in the clipboard where it can be pasted to share.
//  --remove :file_id, removes a given file share.
//  --list, lists all currently shared files
//  --time, sets the amount of time (in hours) that the file should remain shared. Default is 2 hours.
//  --restrict-wget, disables users downloading the file with wget, will force them to use web interface.
//  --restrict-website, users will only be able to collect the file using curl or wget.
//  --help, displays this interface
//  --remove-all, removes all shares.

use std::env;
use std::fs;
use std::path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use clap::{Arg, App, ArgMatches};
use tokio::time;
use reqwest;
mod errors;
use errors::ServerError;
mod server_io;
use server_io::{send_file, check_heartbeat};

extern crate clipboard;

use clipboard::ClipboardProvider;
use clipboard::ClipboardContext;


const NAME: &str = "fileshare";
const VERSION: &str = "0.0.1";
const SIZE_LIMIT: u64 = 2147483648; //Set the file transfer limit default to 2 GB. Should be enough for most people.
const SERVER_BINARY_LOCATION: &str = "";
const MAX_SERVER_START_ATTEMPTS: u8 = 3;
const SERVER_IP_ADDRESS: &str = "127.0.0.1";

// async fn begin_file_share<'a>(args: ArgMatches<'a>) -> () {

// }
#[tokio::main]
async fn main() -> () {
    let args = App::new(NAME)
        .version(VERSION)
        .author("Josiah Bull <Josiah.Bull7@gmail.com>")
        .arg(Arg::with_name("FILE")
            .help("Sets the file to share.")
            .required(true)
            .takes_value(true)
            .index(1))
        .arg(Arg::with_name("list")
            .short("l")
            .long("list")
            .takes_value(false)
            .help("lists all currently shared files (NOT SUPPORTED)"))
        .arg(Arg::with_name("time")
            .short("t")
            .long("time")
            .takes_value(true)
            .help("sets the amount of time (in hours) that the file should remain shared. Default is 2 hours (NOT SUPPORTED)"))
        .arg(Arg::with_name("remove")
            .short("r")
            .long("remove")
            .takes_value(true)
            .help("removes the specified file (NOT SUPPORTED)"))
        .arg(Arg::with_name("remove-all")
            .long("remove-all")
            .takes_value(false)
            .help("removes all current file shares"))
        .arg(Arg::with_name("restrict-wget")
            .long("restrict-wget")
            .takes_value(false)
            .help("disables users downloading the file with wget, will force them to use web interface. (NOT SUPPORTED)"))
        .arg(Arg::with_name("restrict-website")
            .long("restrict-website")
            .takes_value(false)
            .help("users will only be able to collect the file using curl or wget. (NOT SUPPORTED)"))
        .arg(Arg::with_name("force")
            .long("force")
            .takes_value(false)
            .help("disables boundary checks set in the config file. (NOT SUPPORTED)"))
        .get_matches();

    let input_file: path::PathBuf = [env::current_dir().unwrap().to_str().unwrap(), args.value_of("FILE").unwrap()].iter().collect(); //TODO add some error checking here.

    //We have recieved the given file for sharing by the user
    //Check file exists
    if !input_file.exists() {
        return println!("Error, {} doesn't exist!", args.value_of("FILE").unwrap());
    }
    //Check file isn't directory
    if input_file.is_dir() {
        return println!("Error, you must provide a file, not a directory!");
    }
    //Check size of file doesn't exceed limit
    if input_file.metadata().unwrap().len() > SIZE_LIMIT && !args.is_present("force") {
        return println!("Error, your file exceeds the file sharing limit of 2GB! You may bypass this and try sharing anyway with `--force`, or adjust your config settings.");
    }
    //Server Checks

    //Check the server binary is where it should be.
    if !path::PathBuf::from(SERVER_BINARY_LOCATION).exists() {
        return println!("Error, can't find the binary for the server! There may have been an installation issue.");
    }

    //Check if server is running, if it's not then start it up. If we fail to start the server 3 times, fail out to the user.
    let mut start_attmpts: u8 = 0;
    loop {
        if start_attmpts >= MAX_SERVER_START_ATTEMPTS {
            return println!("Error, failed to start the file server! Is there a problem with the binary?");
        } else {
            start_attmpts += 1;
        }
        //Check that the server is up
        if check_heartbeat(&format!("{}/heartbeat", SERVER_IP_ADDRESS)).await.is_ok() {
            println!("File server is running!");
            break; //Server is up!
        } else {
            //Attempt to start server
            println!("The file server appears to not be started. Making attempt {} of {} to start server.", start_attmpts, MAX_SERVER_START_ATTEMPTS);
            Command::new(SERVER_BINARY_LOCATION)
                .spawn()
                .expect("failed to start the server");
            std::thread::sleep(std::time::Duration::from_millis(2000));
        }
    }
    
    //Lets copy the file to tmp, ready for sharing!
    let current_time = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs();
    let tmp_location: path::PathBuf = [env::temp_dir().to_str().unwrap(), NAME, &format!("{}-{}", current_time, args.value_of("FILE").unwrap())].iter().collect(); //TODO error checking

    if fs::copy(&input_file, &tmp_location).is_err() {
        return println!("Error, failed when attempting to copy file to temporary location.\n    Copying from: {}\n  Copying to: {}", input_file.to_str().unwrap_or(""), tmp_location.to_str().unwrap_or(""));
    }

    //The server is running! Lets share the file.
    let body = server_io::RequestBody::default(tmp_location.to_str().unwrap()); //Shouldn't need any error handling here?
    let req = send_file(&format!("{}/share", SERVER_IP_ADDRESS), body).await;
    if req.is_err() {
        return println!("Error, failed to send request to server! Did it shutdown while we were waiting?");
    }

    let response = req.unwrap();

    println!("The file has been shared!\nThe link to your file is:  {}", &response);

    //TODO implement proper error handling here.
    let mut ctx: clipboard::ClipboardContext = clipboard::ClipboardProvider::new().expect("Error, failed to copy to clipboard! Please copy link manually.");
    ctx.set_contents(response).expect("Error, failed to copy to clipboard! Please copy link manually.");


    //TODO: Add an option to bypass this, and allow to share in-place (config?)
    //TODO: Add an option for debug logging
}