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

//TODO: Support download limiting
//TODO: support removing a file by partial id

#![warn(
    missing_docs,
    missing_debug_implementations,
    missing_copy_implementations,
    // clippy::missing_docs_in_private_items, //TODO
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications,
    deprecated
)]

mod cli;

use copypasta::{ClipboardContext, ClipboardProvider};
use human_panic::setup_panic;
use lazy_static::lazy_static;
use log::{error, info, trace};
use rand::Rng;
use riptide_config::Config;
use riptide_database::{establish_connection, insert_share, Share};
use std::error::Error;
use std::ffi::OsStr;
use std::fs::File;
use std::io::Error as IoError;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::tempfile;
use zip::write::FileOptions;

lazy_static! {
    /// The config file for riptide
    pub static ref CONFIG: Config = Config::load_config().unwrap_or_else(|e| {
        error!("Failed to load config: {}", e);
        panic!("Failed to load config: {}", e);
    });
}

/// Create a share from provided arguments and configuration.
fn create_share(
    path: &PathBuf,
    share_time: i64,
) -> Result<Share, Box<dyn Error + Send + Sync + 'static>> {
    trace!("getting file path");
    if !path.exists() {
        return Err(Box::new(IoError::new(
            ErrorKind::NotFound,
            "provided path does not exist",
        )));
    }

    // If the path is a directory, we need to create a temporary file to share
    // request user confirmation that they want to share a directory as a compressed (zipped) file
    let file_name;
    let mut file;
    if path.is_dir() {
        println!("You are attempting to share a directory. This will be compressed into a zip file. Is this ok? (y/n)");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        if input.trim() != "y" {
            return Err(Box::new(IoError::new(
                ErrorKind::InvalidInput,
                "user cancelled directory share",
            )));
        }

        // compress file into a zip, storing in tmp location
        let temp_file = tempfile()?;
        let mut zip = zip::ZipWriter::new(temp_file);
        zip.add_directory(path.to_string_lossy(), FileOptions::default())?;

        file_name = format!(
            "{}.zip",
            path.file_name()
                .unwrap_or_else(|| OsStr::new("unnamed_directory"))
                .to_string_lossy()
        );
        file = zip.finish()?;
    } else {
        file_name = path
            .file_name()
            .unwrap_or_else(|| OsStr::new("unnamed_file"))
            .to_string_lossy()
            .to_string();
        file = File::open(path)?;
    }

    trace!("getting file size");
    let size = file.metadata()?.len();

    let id: u32 = rand::thread_rng().gen(); //XXX: we should check that it's not already in use

    // Copying the file to a new location, so that it can be deleted after the share is complete
    trace!("copying file to new location");
    let mut output_file = std::fs::File::create(format!(
        "{}/{}",
        CONFIG.file_store_location().to_string_lossy(),
        id
    ))?;
    std::io::copy(&mut file, &mut output_file)?;

    trace!("setting file expiry");
    let crt = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_secs() as i64;
    let exp = crt + (share_time * 60 * 60);

    trace!("completing share creation");
    Ok(Share {
        file_id: (id as i64).abs(),
        crt,
        exp,
        file_size: size as i64,
        user_name: whoami::realname(),
        file_name,
    })
}

/// Attempts to save the share to the database, in the event of failure returns
/// an error which should be processed.
fn try_save_to_database(share: &Share) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    trace!("loading database location");
    let path = CONFIG.database_location();
    trace!(
        "database location found at `{}`... establishing database connection",
        path
    );
    let mut conn = establish_connection(path)?;

    trace!("inserting share to database");
    insert_share(&mut conn, share)?;
    Ok(())
}

/// Generate the url to the file, which may be shared to another user to allow
/// them to download your file.
fn generate_link_url(share: &Share) -> String {
    format!(
        "{}/agents/{}/files/{}",
        CONFIG.server_address(),
        CONFIG.public_id().unwrap(),
        share.file_id as u32
    )
}

fn save_to_clipboard(data: &str) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let mut ctx = ClipboardContext::new()?;
    ctx.set_contents(data.to_owned())?;
    // note: not sure why, but we need to get the contents of the clipboard to make it "stay" in the clipboard.
    assert_eq!(ctx.get_contents()?, data);
    Ok(())
}

fn handle_share(
    filename: &PathBuf,
    share_time: i64,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    trace!("creating share");
    let share: Share = create_share(filename, share_time)?;

    trace!("saving share to database");
    try_save_to_database(&share)?;

    trace!("generating link url");
    let link = generate_link_url(&share);

    trace!("saving to clipboard");
    if let Err(e) = save_to_clipboard(&link) {
        error!("Failed to save to clipboard: {}", e);
    }

    println!("The file has been shared!");
    println!("The link to your file is {}", &link);
    Ok(())
}

/// format a time to a human reable string, e.g. 10 seconds ago, 2 hours in the future
fn format_time_relative_to_now(seconds_past_epoch: i64) -> String {
    let now = SystemTime::now();
    let now = now
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_secs();

    let diff = seconds_past_epoch - now as i64;

    if diff < 0 {
        //format in terms of seconds, minutes, horus, or days ago
        let diff = diff.abs();
        if diff < 60 {
            format!("{} seconds ago", diff)
        } else if diff < 60 * 60 {
            format!("{} minutes ago", diff / 60)
        } else if diff < 60 * 60 * 24 {
            format!("{} hours ago", diff / (60 * 60))
        } else {
            format!("{} days ago", diff / (60 * 60 * 24))
        }
    } else {
        //format in terms of seconds, minutes, hours, or days in the future
        if diff < 60 {
            format!("{} seconds", diff)
        } else if diff < 60 * 60 {
            format!("{} minutes", diff / 60)
        } else if diff < 60 * 60 * 24 {
            format!("{} hours", diff / (60 * 60))
        } else {
            format!("{} days", diff / (60 * 60 * 24))
        }
    }
}

/// format a bytes to a human readable string
fn format_bytes_to_readable_string(bytes: i64) -> String {
    let mut bytes = bytes as f64;
    let mut suffix = "B";
    if bytes > 1024.0 {
        bytes /= 1024.0;
        suffix = "KB";
    }
    if bytes > 1024.0 {
        bytes /= 1024.0;
        suffix = "MB";
    }
    if bytes > 1024.0 {
        bytes /= 1024.0;
        suffix = "GB";
    }
    if bytes > 1024.0 {
        bytes /= 1024.0;
        suffix = "TB";
    }
    if bytes > 1024.0 {
        bytes /= 1024.0;
        suffix = "PB";
    }
    format!("{:.2} {}", bytes, suffix)
}

fn list_shares() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let path = CONFIG.database_location();
    let mut conn = establish_connection(path)?;

    let shares = riptide_database::get_shares(&mut conn, &whoami::realname())?;

    println!(
        "{0: <10} | {1: <20} | {2: <10} | {3: <20} | {4: <20}",
        "ID", "Name", "Size", "Created", "Expires"
    );
    println!(
        "{:-<10}-+-{:-<20}-+-{:-<10}-+-{:-<20}-+-{:-<20}",
        "", "", "", "", ""
    );

    for share in shares {
        println!(
            "{0: <10} | {1: <20} | {2: <10} | {3: <20} | {4: <20}",
            share.file_id,
            &share.file_name[..(20.min(share.file_name.len()))],
            format_bytes_to_readable_string(share.file_size),
            format_time_relative_to_now(share.crt),
            format_time_relative_to_now(share.exp),
        );
    }

    Ok(())
}

fn remove_share(id: u32) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let path = CONFIG.database_location();
    let mut conn = establish_connection(path)?;

    // check if share exists
    let share = riptide_database::get_share(&mut conn, &id, &whoami::realname())?;

    if share.is_none() {
        println!("Share with id {} does not exist", id);
        return Ok(());
    }

    riptide_database::remove_share(&mut conn, id)?;

    Ok(())
}

#[doc(hidden)]
fn main() {
    setup_panic!();
    pretty_env_logger::init();

    trace!("loading cli arguments");
    let matches = cli::build_cli().get_matches();

    if !Config::exists() || matches.value_of("rest-config").is_some() {
        info!("Starting first time setup, would you like to configure your installation [y/N]");

        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        if input.trim().to_lowercase() != "y" {
            error!("Exiting");
            std::process::exit(1);
        }

        // ask user for hostname
        info!("Please enter the hostname of the server you want to connect to:");

        let mut hostname = String::new();
        loop {
            std::io::stdin()
                .read_line(&mut hostname)
                .expect("Failed to read line");

            // check if hostname is valid
            // should not contain http or ws
            if hostname.contains("http") || hostname.contains("ws") {
                error!("Hostname should not contain http or ws");
                hostname.clear();
                continue;
            }

            // hostname should not contain slashes
            if hostname.contains('/') {
                error!("Hostname should not contain slashes");
                hostname.clear();
                continue;
            }

            // hostname should not contain spaces
            if hostname.contains(' ') {
                error!("Hostname should not contain spaces");
                hostname.clear();
                continue;
            }

            break;
        }

        // ask the user if this host is using TLS or not
        info!("Is the server using TLS? [y/n]");
        let mut tls = false;
        loop {
            let mut input = String::new();
            std::io::stdin()
                .read_line(&mut input)
                .expect("Failed to read line");

            if input.trim().to_lowercase() == "y" {
                tls = true;
                break;
            } else if input.trim().to_lowercase() == "n" {
                break;
            } else {
                error!("Please enter y or n");
            }
        }

        // ask user for host password
        info!("Please enter the password of the server you want to connect to (empty for none):");
        let mut password = String::new();
        std::io::stdin()
            .read_line(&mut password)
            .expect("Failed to read line");

        // reset config
        if let Err(e) = Config::reset_config() {
            error!("Failed to reset config: {}", e);
            std::process::exit(1);
        }

        // set host details
        if let Err(e) = Config::set_hostname(hostname.trim(), tls) {
            error!("Failed to set hostname: {}", e);
            std::process::exit(1);
        }

        // register with the new host, password is not saved
        if let Err(e) = Config::register(password.trim()) {
            error!("Failed to register: {}", e);
            std::process::exit(1);
        }

        // set the config flag to reload the riptide_agent with new details
        if let Err(e) = Config::reload_agent() {
            error!("Failed to set reload flag: {}", e);
            std::process::exit(1);
        }
    }

    if let Some(file) = matches.get_one::<PathBuf>("file") {
        let time = *matches.get_one::<i64>("time").unwrap_or(&48);

        trace!("file argument found: {:?}", file);
        trace!("time argument found: {}", time);

        handle_share(file, time).unwrap();
    } else if matches.is_present("list") {
        trace!("list argument found");

        list_shares().unwrap();
    } else if let Some(id) = matches.get_one::<u64>("remove") {
        trace!("version argument found");
        remove_share(*id as u32).unwrap();
    }
}
