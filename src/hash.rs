//Author Josiah Bull, Copyright 2021
//This module hashes values on the current computer, getting a semi-unique identifier of the current PC.

use whoami;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};

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
    pub fn get_hash(&mut self, time: Option<u128>) -> u64 {
        if time.is_some() {
            self.time = time.unwrap();
        }
        let mut s = DefaultHasher::new();
        self.hash(&mut s);
        s.finish()
    }
}