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
//! - `--time`, sets the amount of time (in hours) that the file should remain shared.

use chrono::{Duration, Utc};
use clap::ArgMatches;
use clap::{App, Arg};
use config::Config;
use database::{establish_connection, insert_share};
use std::env;
use std::error::Error;
use std::fs::hard_link;
use std::io::Error as IoError;
use std::io::ErrorKind::{self};
use std::path::PathBuf;
use ws_com_framework::File as Share;

/// Load all provided arguments from the user.
fn get_args() -> ArgMatches<'static> {
    App::new("RipTide")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Josiah Bull <Josiah.Bull7@gmail.com>")
        .arg(
            Arg::with_name("file")
                .short("f")
                .long("file")
                .takes_value(true)
                .help("Sets the file to share."),
        )
        .arg(
            Arg::with_name("list")
                .short("l")
                .long("list")
                .takes_value(false)
                .help("lists all currently shared files"),
        )
        .arg(
            Arg::with_name("time")
                .short("t")
                .long("time")
                .takes_value(true)
                .help("sets the amount of time (in hours) that the file should remain shared."),
        )
        .arg(
            Arg::with_name("remove")
                .short("r")
                .long("remove")
                .takes_value(true)
                .help("removes the share of the provided file"),
        )
        .get_matches()
}

/// Collect the current path of where the share may be.
/// Returns a PathBuf to a file. If the file does not exist, or the path is invalid
/// returns an IoError.
fn get_file_path(args: &ArgMatches<'_>) -> Result<PathBuf, IoError> {
    let dir = env::current_dir()?;
    let name = args.value_of("file").unwrap(); //SAFETY: This is required in get_args(), therefore we know it's there and valid.
    let path = dir.join(name);

    if !path.exists() {
        return Err(IoError::new(
            ErrorKind::NotFound,
            "provided path does not exist",
        ));
    }

    if path.is_dir() {
        return Err(IoError::new(ErrorKind::Other, "provided path is directory"));
    }

    Ok(path)
}

/// Create a share from provided arguments and configuration.
fn create_share(args: &ArgMatches<'_>, config: &Config) -> Result<Share, IoError> {
    let input_file = get_file_path(args)?;

    //XXX Note that we could allow files without extensions
    let ext = match input_file.extension() {
        Some(f) => f,
        None => {
            return Err(IoError::new(
                ErrorKind::Other,
                "file does not have extension",
            ))
        }
    };

    let name = match input_file.file_stem() {
        Some(n) => n,
        None => {
            return Err(IoError::new(
                ErrorKind::Other,
                "unable to extract name of file",
            ))
        }
    };

    let size = input_file.metadata()?.len();

    //XXX update this to get_random_base_62();
    let id = utils::hex::get_random_hex(6);

    let exp = Utc::now() + Duration::seconds((config.default_share_time_hours() * 60 * 60) as i64);

    //Create a hardlink to the file
    //TODO do this only *after* the file has been succesfully added to the database
    hard_link(
        &input_file,
        format!("{}/hard_links/{}", config.file_store_location(), id),
    )?;

    //TODO
    // if let Some(share_time) = args.value_of("time") {
    //     let time = share_time
    //         .parse::<u64>()
    //         .expect("Please enter a valid share time!");
    //     share.exp = Utc::now() + Duration::seconds((time * 60 * 60) as i64);
    // }

    Ok(Share {
        id: id.as_bytes().try_into().unwrap(), //SAFETY: We know this will always be 6 bytes, as the length is specified above when we declare id.
        user: whoami::realname(),
        exp,
        crt: Utc::now(),
        name: name.to_string_lossy().to_string(),
        size: size as usize,
        ext: ext.to_string_lossy().to_string(),

        //Note: this type isn't required at this stage.
        //It is calculated dynamically by the server agent upon request for metadata.
        //Therefore we can set this to a nonsense value.
        hash: vec![0; 32].try_into().unwrap(),
    })
}

/// Generate warnings or conflicts that may exist with the given
/// configuration and sharing settings.
fn generate_warnings(share: &Share, config: &Config) -> Vec<&'static str> {
    let mut warnings = vec![];
    if share.size > config.size_limit() {
        warnings.push("this file is greater than the recommended size limit.");
    }

    warnings
}

/// Attempts to save the share to the database, in the event of failure returns
/// an error which should be processed.
fn try_save_to_database(share: &Share) -> Result<(), Box<dyn Error>> {
    let conn = establish_connection()?;
    insert_share(&conn, share)?;
    Ok(())
}

/// Generate the url to the file, which may be shared to another user to allow
/// them to download your file.
fn generate_link_url(share: &Share, config: &Config) -> String {
    let public_id = config.public_id().unwrap(); //TODO
    format!("{}/{}/{}", config.server_address(), public_id, String::from_utf8_lossy(&share.id))
}

// fn save_to_clipboard(data: &str) -> Result<(), Box<dyn Error>> {
//     todo!();
// }

#[doc(hidden)]
fn main() -> Result<(), Box<dyn Error>> {
    let args = get_args();
    let config = Config::load_config()?;

    let share: Share = create_share(&args, &config)?;

    try_save_to_database(&share)?;

    let link = generate_link_url(&share, &config);

    // save_to_clipboard(&link)?;

    for warning in generate_warnings(&share, &config) {
        println!("WARN: {}", warning);
    }

    println!("The file has been shared!");
    println!("The link to your file is {}", &link);
    Ok(())
}
