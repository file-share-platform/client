//Author Josiah Bull, Copyright 2021
//! Fast and easy file sharing over the internet, through a simple cli.
//!
//! Provides a simple cli to share files over the internet.
//! 
//! Expected Syntax: `share ./myfiles/data/file.txt`
//! 
//! Supported Options:
//! - `--remove :file_id`, removes a given file share.
//! - `--list`, lists all currently shared files
//! - `--time`, sets the amount of time (in hours) that the file should remain shared. Default is 48 hours.
//! - `--restrict-wget`, disables users downloading the file with wget, will force them to use web interface.
//! - `--restrict-website`, users will only be able to collect the file using curl or wget.
//! - `--help`, displays this interface
//! - `--remove-all`, removes all shares.

//TODO check that xorg-dev is installed! It's needed for clipboard interaction on linux.

mod errors;
mod server_io;
mod common;

use std::env;
use std::path;
use clap::{Arg, App};
use server_io::{notify_new_share, check_heartbeat};
use common::*;
use std::io::prelude::*;
use std::fs::File;

extern crate clipboard;

use clipboard::ClipboardProvider;


const NAME: &str = "fileshare";
const VERSION: &str = "0.0.1";
const SIZE_LIMIT: u64 = 2147483648; //Set the file transfer limit default to 2 GB. Should be enough for most people.
const SERVER_IP_ADDRESS: &str = "http://127.0.0.1:8000";
const DEFAULT_SHARE_TIME_HOURS: u128 = 2;
const SERVER_FILE_LOCATION: &str ="/opt/fileShare";

/// Entry Point
#[tokio::main]
async fn main() {
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
            .help("sets the amount of time (in hours) that the file should remain shared. Default is 2 hours."))
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
            .help("disables users downloading the file with wget, will force them to use web interface."))
        .arg(Arg::with_name("restrict-website")
            .long("restrict-website")
            .takes_value(false)
            .help("users will only be able to collect the file using curl or wget."))
        .arg(Arg::with_name("force")
            .long("force")
            .takes_value(false)
            .help("disables boundary checks set in the config file."))
        .get_matches();

    let input_file: path::PathBuf = [
        env::current_dir()
            .expect("Failed to get current directory of program.")
            .to_str()
            .expect("Failed string conversion!"), 
        args.value_of("FILE")
            .unwrap()]
    .iter()
    .collect();

    //Check if server is running
    if !check_heartbeat(&format!("{}/heartbeat", SERVER_IP_ADDRESS)).await.is_ok() {
        return println!("Error: File server is not up!");            
    }

    //Create a new share
    let mut share_file: server_io::ShareFile = match server_io::ShareFile::new(input_file.to_str().unwrap()) {
        Ok(file) => file,
        Err(e) => return println!("An error occured: {}", e),
    };

    if args.is_present("restrict-wget") {
        share_file = share_file.set_restrict_wget(true);
    }

    if args.is_present("restrict-website") {
        share_file = share_file.set_restrict_website(true);
    }

    if let Some(share_time) = args.value_of("time") {
        let time = share_time.parse::<u64>().expect("Please enter a valid share time!");
        share_file = share_file.set_exp(&(get_time() + time * 60 * 60 * 1000));
    }

    match share_file.validate() {
        Ok(_) => (),
        Err(e) => return println!("An error occurred: {}", e),
    }

    //Save the share
    let share_file_name = format!("{}/{}.share", SERVER_FILE_LOCATION, share_file.get_id());
    let mut file = File::create(&share_file_name).expect("Failed to create share file!");
    file.write_all(share_file.to_json_string().expect("Error: Failed to create share file!").as_bytes()).expect("Failed to create share file!");
    
    let req = notify_new_share(&format!("{}/share", SERVER_IP_ADDRESS), share_file_name).await;
    if req.is_err() {
        return println!("Error, failed to send request to server! Error: {}", req.unwrap_err());
    }

    let response = req.unwrap();

    println!("The file has been shared!\nThe link to your file is:  {}", &response);

    //TODO implement proper error handling here.
    let mut ctx: clipboard::ClipboardContext = clipboard::ClipboardProvider::new().expect("Error, failed to copy to clipboard! Please copy link manually.");
    ctx.set_contents(response).expect("Error, failed to copy to clipboard! Please copy link manually.");


    //TODO: Add an option for debug logging
}