//Author Josiah Bull, Copyright 2021
//!This module hashes values on the current computer, getting a semi-unique identifier of the current PC.
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};

///Contains some basic values which aim to identify this computer.
#[derive(Hash)]
pub struct ComputerIdentifier {
    language: String,
    device_name: String,
    platform: String,
    operating_system: String,
    name: String,
    time: u128,
}

impl Default for ComputerIdentifier {
    fn default() -> Self {
        ComputerIdentifier {
            language: whoami::lang().collect(),
            device_name: whoami::devicename(),
            platform: whoami::platform().to_string(),
            operating_system: whoami::distro(),
            name: whoami::username(),
            time: SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis() as u128,
        }
    }
}

impl ComputerIdentifier {
    ///Generates a hash that semi-uniquely identifies this computer. Takes an optional paramter of the current time, which may be required depending on how the recipent is validating the request.
    pub fn get_hash(&mut self, time: Option<u128>) -> u64 {
        if let Some(new_time) = time {
            self.time = new_time;
        }
        let mut s = DefaultHasher::new();
        self.hash(&mut s);
        s.finish()
    }
}