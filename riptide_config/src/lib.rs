//! Abstraction for configuration in the riptide client.

#![warn(
    // missing_docs,
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications,
    deprecated
)]

mod error;

use error::{ConfigError, ErrorKind};
use getset::Getters;
use log::warn;
use serde_derive::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

/// Representation of the configuration file for the riptide client
#[derive(Debug, Clone, Getters, Serialize, Deserialize)]
#[getset(get = "pub")]
pub struct Config {
    public_id: Option<u64>,
    private_key: Option<Vec<u8>>,
    websocket_address: String,
    server_address: String,
    file_store_location: PathBuf,
    database_location: String,
    max_upload_attempts: u64,
    size_limit_bytes: u64,
    reconnect_delay_minutes: u64,
}

/// Information required to connect to central api
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Id {
    public_id: u64,
    passcode: String,
}

/// We call to this in the event that we are not registered yet.
fn register_server(ip: String, password: &str) -> Result<Id, ConfigError> {
    let response: Id = ureq::post(&ip)
        .set("Accept", "application/json")
        .set("Authorization", &format!("Basic {}", password))
        .call()
        .map_err(|e| {
            ConfigError::new(
                ErrorKind::NetworkError(e),
                "Unable to connect to server to register",
            )
        })?
        .into_json()
        .map_err(|e| {
            ConfigError::new(
                ErrorKind::IoError(e),
                "Unable to parse response from server to register",
            )
        })?;

    Ok(response)
}

fn get_config_dir() -> PathBuf {
    let dir =
        dirs::config_dir().unwrap_or_else(|| panic!("Unable to locate configuration directory"));
    dir.join("riptide")
}

impl Config {
    /// Reset the configuration file to the default values
    pub fn reset_config() -> Result<(), ConfigError> {
        let dir = get_config_dir();

        if !dir.exists() {
            warn!(
                "Config directory `{}` does not exist, creating it now.",
                dir.to_string_lossy()
            );
            std::fs::create_dir_all(&dir).map_err(|e| {
                ConfigError::new(
                    ErrorKind::IoError(e),
                    format!(
                        "Unable to create config directory `{}`",
                        dir.to_string_lossy()
                    ),
                )
            })?;
        }
        if !dir.is_dir() {
            return Err(ConfigError::new(ErrorKind::IsNotDirectory, format!("Config location `{}`, is not a directory. Please ensure that this provided location is a directory, then try again.", dir.to_string_lossy())));
        }

        //Generate configuration data
        let config_path = dir.join("riptide.conf");

        let default_config = include_str!("../default_config.toml")
            .replace("${CONFIG_DIR}", &get_config_dir().to_string_lossy());

        if config_path.is_dir() {
            return Err(ConfigError::new(ErrorKind::IsDirectory, format!("Configuration file `{}`, is a directory - not a file. Please ensure the provided path is a directory then try again.", config_path.to_string_lossy())));
        }

        // remove the old config file
        if config_path.exists() {
            std::fs::remove_file(&config_path).map_err(|e| {
                ConfigError::new(
                    ErrorKind::IoError(e),
                    format!(
                        "Unable to remove old configuration file `{}`",
                        config_path.to_string_lossy()
                    ),
                )
            })?;
        }

        //Write configuration data
        std::fs::write(config_path, default_config).map_err(|e| {
            ConfigError::new(
                ErrorKind::IoError(e),
                "Failed to write default configuration data to the disk.",
            )
        })?;

        // remove key file if present
        let key_path = get_config_dir().join("key");
        if key_path.exists() {
            std::fs::remove_file(key_path).map_err(|e| {
                ConfigError::new(
                    ErrorKind::IoError(e),
                    "Failed to remove old key file from disk.",
                )
            })?;
        }

        // remove all hard_links
        let hard_link_dir = get_config_dir().join("hard_links");
        if hard_link_dir.exists() {
            std::fs::remove_dir_all(&hard_link_dir).map_err(|e| {
                ConfigError::new(
                    ErrorKind::IoError(e),
                    "Failed to remove old hard links from disk.",
                )
            })?;
        }
        std::fs::create_dir(hard_link_dir).map_err(|e| {
            ConfigError::new(
                ErrorKind::IoError(e),
                "Failed to create new hard links directory.",
            )
        })?;

        // reset database
        let database_path = get_config_dir().join("riptide.db");
        if database_path.exists() {
            std::fs::remove_file(&database_path).map_err(|e| {
                ConfigError::new(
                    ErrorKind::IoError(e),
                    "Failed to remove old database from disk.",
                )
            })?;
        }
        std::fs::File::create(database_path).map_err(|e| {
            ConfigError::new(ErrorKind::IoError(e), "Failed to create new database file.")
        })?;

        Ok(())
    }

    pub fn register(password: &str) -> Result<Id, ConfigError> {
        let config = Config::__load_config()?;

        let key_path = get_config_dir().join("key");
        if key_path.exists() && !key_path.is_dir() {
            //Attempt to load key
            let data = std::fs::read(&key_path).map_err(|e| {
                ConfigError::new(
                    ErrorKind::IoError(e),
                    format!(
                        "Failed to read public/private key pair. Please remove `{}` and try again",
                        key_path.to_string_lossy()
                    ),
                )
            })?;
            let id: Id = bincode::deserialize(&data).map_err(|e| {
                ConfigError::new(
                    ErrorKind::BincodeError(*e),
                    "Failed to deserialize public/private key pair.",
                )
            })?;
            Ok(id)
        } else {
            //Generate new key
            println!("Api not registered. Attempting to register now....");
            let ip = format!("{}/api/v1/register", config.server_address());

            let id: Id = register_server(ip, password)?;
            let data = bincode::serialize(&id).map_err(|e| {
                ConfigError::new(
                    ErrorKind::BincodeError(*e),
                    "Failed to serialized public/private key pair to save to disk.",
                )
            })?;
            std::fs::write(key_path, data).map_err(|e| {
                ConfigError::new(
                    ErrorKind::IoError(e),
                    "Failed to write public/private key pair to disk.",
                )
            })?;

            println!("Registered websocket with id {}", id.public_id);

            Ok(id)
        }
    }

    pub fn set_hostname(hostname: &str, tls: bool) -> Result<(), ConfigError> {
        let config = Config::__load_config()?;
        let config = Config {
            server_address: format!("http{}://{}", if tls { "s" } else { "" }, hostname),
            websocket_address: format!("ws{}://{}", if tls { "s" } else { "" }, hostname),
            ..config
        };

        let config_path = get_config_dir().join("riptide.conf");

        let config_data = toml::to_string(&config).map_err(|e| {
            ConfigError::new(
                ErrorKind::ParseError(e.to_string()),
                "Failed to serialize configuration data to TOML.",
            )
        })?;

        std::fs::write(config_path, config_data).map_err(|e| {
            ConfigError::new(
                ErrorKind::IoError(e),
                "Failed to write configuration data to disk.",
            )
        })?;

        Ok(())
    }

    pub fn is_registered() -> bool {
        let key_path = get_config_dir().join("key");
        key_path.exists() && !key_path.is_dir()
    }

    /// creates a file in the config directory - the agent will reload this if present
    pub fn reload_agent() -> Result<(), ConfigError> {
        let filename = get_config_dir().join("reload_agent");
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        std::fs::write(filename, now.to_string()).map_err(|e| {
            ConfigError::new(
                ErrorKind::IoError(e),
                "Failed to write reload_agent file to the disk.",
            )
        })?;

        Ok(())
    }

    pub fn reload_requested() -> Result<bool, ConfigError> {
        let filename = get_config_dir().join("reload_agent");
        let result = filename.exists();

        if result {
            std::fs::remove_file(filename).map_err(|e| {
                ConfigError::new(
                    ErrorKind::IoError(e),
                    "Failed to remove reload_agent file from the disk.",
                )
            })?;
        }

        Ok(result)
    }

    pub fn exists() -> bool {
        let config_path = get_config_dir().join("riptide.conf");
        config_path.exists()
    }

    /// load the configuration from the disk
    pub fn __load_config() -> Result<Config, ConfigError> {
        let dir = get_config_dir();
        let config_path = dir.join("riptide.conf");

        // if not exist, throw error
        if !config_path.exists() {
            return Err(ConfigError::new(
                    ErrorKind::NotFound,
                    format!(
                        "Configuration file `{}` does not exist. Please run `riptide init` to create a new configuration file.",
                        config_path.to_string_lossy()
                    ),
                ));
        }

        // if not file, throw error
        if !config_path.is_file() {
            return Err(ConfigError::new(
                    ErrorKind::IsDirectory,
                    format!(
                        "Configuration file `{}` is not a file. Please ensure that this provided location is a file, then try again.",
                        config_path.to_string_lossy()
                    ),
                ));
        }

        // try to load from disk
        let config_data = std::fs::read_to_string(config_path).map_err(|e| {
            ConfigError::new(
                ErrorKind::IoError(e),
                "Failed to read configuration file from disk.",
            )
        })?;

        // try to parse config
        let config: Config = toml::from_str(&config_data).map_err(|e| {
            ConfigError::new(
                ErrorKind::ParseError(e.to_string()),
                "Failed to parse configuration file.",
            )
        })?;

        // if registered, set the public and private key
        if Config::is_registered() {
            let key_path = dir.join("key");
            let data = std::fs::read(&key_path).map_err(|e| {
                ConfigError::new(
                    ErrorKind::IoError(e),
                    "Failed to read public/private key pair from disk.",
                )
            })?;
            let id: Id = bincode::deserialize(&data).map_err(|e| {
                ConfigError::new(
                    ErrorKind::BincodeError(*e),
                    "Failed to deserialize public/private key pair.",
                )
            })?;

            let config = Config {
                public_id: Some(id.public_id),
                private_key: Some(id.passcode.into_bytes()),
                ..config
            };

            Ok(config)
        } else {
            Ok(config)
        }
    }
}

impl Config {
    /// Attempt to load the configuration from the disk, synchronously. Wrap in spawn_blocking if in an async context.
    pub fn load_config() -> Result<Config, ConfigError> {
        Config::__load_config()
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::oneshot;
    use warp::Filter;

    use crate::register_server;

    /// Create a simple webserver which parses some basic http requests.
    fn create_http_server(ip: ([u8; 4], u16)) -> Result<oneshot::Sender<()>, ()> {
        let register = warp::post()
            .and(warp::path("test-register"))
            .and(warp::path::end())
            .map(|| {
                String::from(
                    "{
                        \"public_id\": 16024170730851851829,
                        \"passcode\": \"tHQDrCd3XLcJt9LsomWIwry8uMcuUJtV\"
                    }",
                )
            });

        let routes = register;

        let (tx, rx) = oneshot::channel();
        let (_, server) = warp::serve(routes).bind_with_graceful_shutdown(ip, async {
            rx.await.ok();
        });

        tokio::task::spawn(server);

        Ok(tx)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_register() {
        let close_server_tx = create_http_server(([127, 0, 0, 1], 8001)).unwrap();

        let res = tokio::task::spawn_blocking(|| {
            register_server("http://127.0.0.1:8001/test-register".into(), "--")
        })
        .await
        .unwrap()
        .unwrap();

        assert_eq!(res.public_id, 16024170730851851829);
        assert_eq!(res.passcode, "tHQDrCd3XLcJt9LsomWIwry8uMcuUJtV");

        let _ = close_server_tx.send(());
    }
}
