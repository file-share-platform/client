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

//TODO: Support encryption/passwords
//TODO: Support uploading files rather than streaming
//TODO: Support sending a directory by compressing into an archive
//TODO: Support download limiting
//TODO: Support uploading files with regular syncing (for x hours).

use chrono::{Duration, Utc};

use clap::Parser;

use cli_clipboard::{ClipboardContext, ClipboardProvider};
use config::Config;
use database::{establish_connection, insert_share, Share};
use human_panic::setup_panic;
use lazy_static::lazy_static;
use log::{warn, trace};
use rand::Rng;
use std::env;
use std::error::Error;
use std::fs::hard_link;
use std::io::Error as IoError;
use std::io::ErrorKind::{self};
use std::path::PathBuf;

lazy_static! {
    static ref CONFIG: Config = Config::load_config().expect("a valid config file"); //XXX: handle error gracefully?
    static ref ARGS: Args = Args::parse();
    static ref DEFAULT_SHARE_TIME: i64 = *CONFIG.default_share_time_hours() as i64;
}

/// Self host and share a file over the internet quickly and easily.
#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Name of the file to share
    //TODO: accept wild cards
    file: String,

    /// Remove the file share indicated by this id by index or id
    #[clap(short, long)]
    remove: bool,

    /// List all currently shared files
    #[clap(short, long)]
    list: bool,

    /// Set how many hours to share the file for
    #[clap(short, long, default_value_t=*DEFAULT_SHARE_TIME)]
    time: i64,
}

/// Collect the current path of where the share may be.
/// Returns a PathBuf to a file. If the file does not exist, or the path is invalid
/// returns an IoError.
fn get_file_path() -> Result<PathBuf, IoError> {
    let dir = env::current_dir()?;
    let path = dir.join(&ARGS.file);

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
fn create_share() -> Result<Share, IoError> {
    trace!("getting file path");
    let input_file = get_file_path()?;

    trace!("getting file name");
    let name = match input_file.file_name() {
        Some(n) => n,
        None => {
            return Err(IoError::new(
                ErrorKind::Other,
                "unable to extract name of file",
            ))
        }
    };

    trace!("getting file size");
    let size = input_file.metadata()?.len();

    let id: u32 = rand::thread_rng().gen();

    trace!("creating hard_link to file");
    //Create a hardlink to the file
    hard_link(
        &input_file,
        format!("{}/{}", CONFIG.file_store_location().to_string_lossy(), id),
    )?;

    trace!("setting file expiry");
    let exp = Utc::now() + Duration::hours(ARGS.time);

    trace!("completing share creation");
    Ok(Share {
        file_id: id as i32,
        exp: exp.timestamp(),
        crt: Utc::now().timestamp(),
        file_size: size as i64,
        user_name: whoami::realname(),
        file_name: name.to_string_lossy().to_string(),
    })
}

/// Generate warnings or conflicts that may exist with the given
/// configuration and sharing settings.
fn generate_warnings(share: &Share) -> Vec<&'static str> {
    let mut warnings = vec![];
    if share.file_size as u64 > *CONFIG.size_limit_bytes() {
        warnings.push("this file is greater than the recommended size limit.");
    }

    warnings
}

/// Attempts to save the share to the database, in the event of failure returns
/// an error which should be processed.
fn try_save_to_database(share: &Share) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    trace!("loading database location");
    let path = CONFIG.database_location();
    trace!("database location found at `{}`... establishing database connection", path);
    let mut conn = establish_connection(path)?;

    trace!("inserting share to database");
    insert_share(&mut conn, share)?;
    Ok(())
}

/// Generate the url to the file, which may be shared to another user to allow
/// them to download your file.
fn generate_link_url(share: &Share) -> String {
    format!(
        "{}/download/{}/{}",
        CONFIG.server_address(),
        CONFIG.public_id(),
        share.file_id as u32
    )
}

fn save_to_clipboard(data: &str) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let mut ctx = ClipboardContext::new().unwrap();
    ctx.set_contents(data.to_owned()).unwrap();
    // note: not sure why, but we need to get the contents of the clipboard to make it "stay"
    // in the clipboard.
    assert_eq!(ctx.get_contents().unwrap(), data);
    Ok(())
}

fn handle_share() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    trace!("creating share");
    let share: Share = create_share()?;

    trace!("saving share to database");
    try_save_to_database(&share)?;

    trace!("generating warnings");
    for warning in generate_warnings(&share) {
        warn!("{}", warning);
    }

    trace!("generating link url");
    let link = generate_link_url(&share);

    trace!("saving to clipboard");
    save_to_clipboard(&link)?;

    println!("The file has been shared!");
    println!("The link to your file is {}", &link);
    Ok(())
}

#[doc(hidden)]
fn main() {
    setup_panic!();
    pretty_env_logger::init();

    match handle_share() {
        Ok(_) => {}
        Err(e) => panic!("an error occured: {}", e),
    }
}
