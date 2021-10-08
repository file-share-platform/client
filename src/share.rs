//Author Josiah Bull, Copyright 2021

//!Handles communication with the file server.
use crate::error::Error;
use crate::{DEFAULT_SHARE_TIME_HOURS, SERVER_FILE_LOCATION, SIZE_LIMIT};
use std::ffi::OsStr;
use std::fs::hard_link;
use std::path::{Path, PathBuf};
use ws_com_framework::File;
use chrono::{Duration, prelude::*};


///Returns a result which contains `File` if successful, otherwise retursn a `Error`. Populates all values with reasonable defaults derived from the provided path.
pub fn create_new_file(path_raw: &str) -> Result<File, Error> {
    //Check file exists

    let path = Path::new(path_raw);
    if !path.exists() {
        return Err(Error::FileExistError("File doesn't exist!".into()));
    }

    if path.is_dir() {
        return Err(Error::FileExistError("File is a directory!".into()));
    }

    //Create the file extension.
    let ext: String = match path.extension().and_then(OsStr::to_str) {
        Some(file) => file.to_owned(),
        None => return Err(Error::FileExtensionError),
    };

    //Extract filename
    let name: String = match path.file_stem().and_then(OsStr::to_str) {
        Some(name) => name.to_owned(),
        None => return Err(Error::FileNameError),
    };

    //Collect size of file
    let size: usize = PathBuf::from(path).metadata()?.len() as usize;
    if size > SIZE_LIMIT {
        return Err(Error::FileSizeError("Too Large!".into()));
    }

    //TODO update this to get_random_base_62();
    let id: String = utils::hex::get_random_hex(6);

    //Create a hard link to the relevant file
    if let Err(e) = hard_link(
        path_raw,
        format!("{}/hard_links/{}", SERVER_FILE_LOCATION, id),
    ) {
        return Err(Error::HardLinkError(e.to_string()));
    }

    Ok(File {
        id,
        user: whoami::realname(),
        exp: Utc::now() + Duration::seconds((DEFAULT_SHARE_TIME_HOURS * 60 * 60) as i64),
        crt: Utc::now(),
        wget: false,
        website: false,
        name,
        size,
        ext,
    })
}

///Used to validate all values in the struct. Will test the paths, check exp dates are valid, and do a whole bunch of other checks to prevent problems.
pub fn validate_file(f: &File) -> Result<(), Error> {
    if f.website && f.wget {
        return Err(Error::RestrictionError);
    }
    if f.exp < Utc::now() {
        return Err(Error::TimeError);
    }

    Ok(())
}