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

mod error;
mod share;
mod database;

use clap::{App, Arg};
use std::env;
use std::path;
use ws_com_framework::File;
use chrono::{Duration, prelude::*};

extern crate clipboard;

use clipboard::ClipboardProvider;

const NAME: &str = "file-share";
const VERSION: &str = "0.0.1";
const SIZE_LIMIT: usize = 2147483648; //Set the file transfer limit default to 2 GB. Should be enough for most people.
const SERVER_IP_ADDRESS: &str = "http://127.0.0.1:8000";
const DEFAULT_SHARE_TIME_HOURS: u128 = 2;
const SERVER_FILE_LOCATION: &str = "/opt/file-share";

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
        // .arg(Arg::with_name("list")
        //     .short("l")
        //     .long("list")
        //     .takes_value(false)
        //     .help("lists all currently shared files"))
        .arg(Arg::with_name("time")
            .short("t")
            .long("time")
            .takes_value(true)
            .help("sets the amount of time (in hours) that the file should remain shared."))
        // .arg(Arg::with_name("remove")
        //     .short("r")
        //     .long("remove")
        //     .takes_value(true)
        //     .help("removes the specified file"))
        // .arg(Arg::with_name("remove-all")
        //     .long("remove-all")
        //     .takes_value(false)
        //     .help("removes all current file shares"))
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
        args.value_of("FILE").unwrap(),
    ]
    .iter()
    .collect();

    //Create a new share
    let mut share: File = match share::create_new_file(input_file.to_str().unwrap()) {
        Ok(file) => file,
        Err(e) => return println!("An error occured: {}", e),
    };

    if args.is_present("restrict-wget") {
        share.wget = true;
    }

    if args.is_present("restrict-website") {
        share.website = true;
    }

    if let Some(share_time) = args.value_of("time") {
        let time = share_time
            .parse::<u64>()
            .expect("Please enter a valid share time!");
        share.exp = Utc::now() + Duration::seconds((time * 60 * 60) as i64);
    }

    match share::validate_file(&share) {
        Ok(_) => (),
        Err(e) => return println!("An error occurred: {}", e),
    }

    //Save the share
    let db = database::create_pool().expect("Unable to create database!");
    database::add_share(&db, share).await.expect("Failed to add share to server!");

    //Load 

    // println!(
    //     "The file has been shared!\nThe link to your file is:  {}",
    //     &response
    // );

    // //TODO implement proper error handling here.
    // let mut ctx: clipboard::ClipboardContext = clipboard::ClipboardProvider::new()
    //     .expect("Error, failed to copy to clipboard! Please copy link manually.");
    // ctx.set_contents(response)
    //     .expect("Error, failed to copy to clipboard! Please copy link manually.");

    //TODO: Add an option for debug logging
}
