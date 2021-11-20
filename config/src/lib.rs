mod error;

use error::{ConfigError, ErrorKind};
use reqwest::blocking::Client;
use serde_derive::{Deserialize, Serialize};
use std::{convert::Infallible, num::ParseIntError, path::PathBuf, str::FromStr};

#[derive(Debug, Clone)]
pub struct Config {
    public_id: [u8; 6],
    private_key: [u8; 32],
    websocket_address: String,
    server_address: String,
    file_store_location: PathBuf,
    max_upload_attempts: usize,
    size_limit_bytes: usize,
    default_share_time_hours: usize,
    reconnect_delay_minutes: usize,
}

/// Information required to connect to central api
#[derive(Debug, Clone, Deserialize, Serialize)]
struct Id {
    public_id: String,
    private_key: String,
}

/// Opens a toml file, and attempts to load the toml::value as specified in the provided &str.
fn load_from_toml<'a>(name: &str, path: &PathBuf) -> Result<toml::Value, ConfigError> {
    let data = std::fs::read_to_string(&path).map_err(|e| {
        ConfigError::new(ErrorKind::IoError(e), "Failed to load configuration file")
    })?;

    let f = data.parse::<toml::Value>().map_err(|e| {
        ConfigError::new(
            ErrorKind::TomlParseError(e),
            "Unable to parse configuration file",
        )
    })?;

    if let Some(k) = f.get(name) {
        Ok(k.to_owned())
    } else {
        Err(ConfigError::new(
            ErrorKind::NotFound,
            format!("Key `{}` Not found in `{}`", name, path.to_string_lossy()),
        ))
    }
}

/// A function to load configuration from the environment.
///
/// Attempts to load from multiple sources falling back in this order:
/// 1. Load from environment
/// 2. Load from `~/.config/riptide`
///
/// Note that you must provide the expected conversion error as a generic. In the future this will be provided
/// internally via a trait.
///
/// **Example**
/// ```rust
///     # use config::load_env;
///     # use std::{num::ParseIntError, path::PathBuf};
///     # std::fs::write("./example_config.toml", "NUMBER_SHOES = 5");
///     # let path: PathBuf = PathBuf::from("./example_config.toml");
///     let num_shoes: usize = load_env::<usize, ParseIntError>("NUMBER_SHOES", &path).unwrap();
///     assert_eq!(num_shoes, 5);
///     println!("The number of shoes is {}", num_shoes);
///     # std::fs::remove_file("./example_config.toml").unwrap();
/// ```
/// A variety of types are supported for implicit conversion, look [here](https://docs.rs/toml/0.5.8/toml/value/enum.Value.html#impl-From%3C%26%27a%20str%3E) for a dedicated list of these types.
///
/// Internally this function relies on `toml::value::Value.try_into()` for type conversion.
///
pub fn load_env<'a, T, G>(name: &str, path: &PathBuf) -> Result<T, ConfigError>
where
    T: FromStr<Err = G> + serde::Deserialize<'a>,
    G: std::fmt::Display,
{
    use std::env::var;

    //1. Attempt to load from env
    if let Ok(d) = var(name.to_uppercase()) {
        let res = d
            .parse::<T>()
            .map_err(|e| ConfigError::new(ErrorKind::ParseError(e.to_string()), ""));
        return res;
    }

    //2. Attempt to load from config location
    let res = load_from_toml(name, path)?
        .try_into()
        .map_err(|e| {
            ConfigError::new(
                ErrorKind::ParseError(e.to_string()),
                format!("Able to find `{}` in configuration file `{}`, but it's type was invalid. Please fix this, then try again.", name, path.to_string_lossy())
            )
        })?;
    Ok(res)
}

/// We call to this in the event that we are not registered yet.
fn register_server<'a>(ip: String) -> Result<Id, ConfigError> {
    let response = Client::new()
        .post(&ip)
        .send()
            .map_err(|e| {
                ConfigError::new(ErrorKind::NetworkError(e), "Failed to contact server due to error")
            })?
        .json::<Id>()
            .map_err(|e| {
                ConfigError::new(ErrorKind::NetworkError(e), "Failed to parse network response to json")
            })?;

    Ok(response)
}

fn get_config_dir() -> PathBuf {
    let dir =
        dirs::config_dir().unwrap_or_else(|| panic!("Unable to locate configuration directory"));
    dir.join("riptide")
}

impl<'r> Config {
    fn __load_config() -> Result<Config, ConfigError> {
        //Validate critical paths exist
        //XXX Make this directory change with a provided flag on the cli
        let dir = get_config_dir();

        if !dir.exists() {
            return Err(ConfigError::new(ErrorKind::NotFound, format!("Config directory `{}` does not exist, please ensure this directory exists then try again.", dir.to_string_lossy())));
        }
        if !dir.is_dir() {
            return Err(ConfigError::new(ErrorKind::IsNotDirectory, format!("Config location `{}`, is not a directory. Please ensure that this provided location is a directory, then try again.", dir.to_string_lossy())));
        }

        let config_path = dir.join("riptide.conf");
        if !config_path.exists() {
            println!(
                "WARN: Configuration file `{}` doesn't seem to exist, creating file now...",
                config_path.to_string_lossy()
            );
            Config::reset_config()?;
        }
        if config_path.is_dir() {
            return Err(ConfigError::new(ErrorKind::IsDirectory, format!("Configuration file `{}`, is a directory - not a file. Please ensure the provided path is a directory then try again.", config_path.to_string_lossy())));
        }

        //Load information from disk
        let websocket_address = load_env::<String, Infallible>("websocket_address", &config_path)?;
        let server_address = load_env::<String, Infallible>("server_address", &config_path)?;
        let max_upload_attempts =
            load_env::<usize, ParseIntError>("max_upload_attempts", &config_path)?;
        let size_limit_bytes = load_env::<usize, ParseIntError>("size_limit_bytes", &config_path)?;
        let default_share_time_hours =
            load_env::<usize, ParseIntError>("default_share_time_hours", &config_path)?;
        let reconnect_delay_minutes =
            load_env::<usize, ParseIntError>("reconnect_delay_minutes", &config_path)?;

        //Acquire public/private key pair
        let agent_id = {
            let key_path = dir.join("key");
            if key_path.exists() && !key_path.is_dir() {
                //Attempt to load key
                let data = std::fs::read(&key_path)
                    .map_err(|e| {
                        ConfigError::new(ErrorKind::IoError(e), format!("Failed to read public/private key pair. Please remove `{}` and try again", key_path.to_string_lossy()))
                    })?;
                let id: Id = bincode::deserialize(&data).map_err(|e| {
                    ConfigError::new(
                        ErrorKind::BincodeError(*e),
                        "Failed to deserialize public/private key pair.",
                    )
                })?;
                id
            } else {
                //Generate new key
                println!("Api not registered. Attempting to register now....");
                let ip = format!("{}/client/ws-register", server_address);

                let id: Id = register_server(ip)?;
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

                id
            }
        };

        //Set and validate file_store_location
        let file_store_location = dir.join("hard_links");
        if !file_store_location.exists() {
            return Err(ConfigError::new(
                ErrorKind::NotFound,
                format!(
                    "Hardlinks directory does not exist at `{}`",
                    file_store_location.to_string_lossy()
                ),
            ));
        }
        if !file_store_location.is_dir() {
            return Err(ConfigError::new(ErrorKind::IsNotDirectory, format!("Hardlinks location is not a directory, please create a directory in this loaction. `{}`", file_store_location.to_string_lossy())));
        }

        let config: Config = Config {
            public_id: agent_id.public_id.as_bytes().try_into().unwrap(), //HACK
            private_key: agent_id.private_key.as_bytes().try_into().unwrap(), //HACK
            websocket_address,
            server_address,
            file_store_location,
            max_upload_attempts,
            size_limit_bytes,
            default_share_time_hours,
            reconnect_delay_minutes,
        };

        Ok(config)
    }

    pub fn reset_config() -> Result<(), ConfigError> {
        //Generate configuration data
        let default_config = include_str!("../default_config.toml")
            .replace("${CONFIG_DIR}", &get_config_dir().to_string_lossy());
        //Write configuration data
        std::fs::write(get_config_dir().join("riptide.conf"), default_config).map_err(|e| {
            ConfigError::new(
                ErrorKind::IoError(e),
                "Failed to write default configuration data to the disk.",
            )
        })?;
        Ok(())
    }

    //XXX generate these getters using a crate https://docs.rs/getset/0.1.1/getset/index.html
    pub fn public_id(&self) -> &[u8; 6] {
        &self.public_id
    }

    pub fn private_key(&self) -> &[u8; 32] {
        &self.private_key
    }

    pub fn websocket_address(&'r self) -> &'r str {
        &self.websocket_address
    }

    pub fn server_address(&'r self) -> &'r str {
        &self.server_address
    }

    pub fn file_store_location(&'r self) -> &'r PathBuf {
        &self.file_store_location
    }

    pub fn max_upload_attempts(&self) -> usize {
        self.max_upload_attempts
    }

    pub fn size_limit(&self) -> usize {
        self.size_limit_bytes
    }

    pub fn default_share_time_hours(&self) -> usize {
        self.default_share_time_hours
    }

    pub fn reconnect_delay_minutes(&self) -> usize {
        self.reconnect_delay_minutes
    }
}

#[cfg(feature = "sync")]
impl<'r> Config {
    pub fn load_config() -> Result<Config, ConfigError> {
        Config::__load_config()
    }
}

#[cfg(feature = "async")]
impl<'r> Config {
    pub async fn load_config_async() -> Result<Config, ConfigError> {
        tokio::task::spawn_blocking(|| {
            Config::__load_config()
        }).await.unwrap()
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::oneshot;
    use warp;
    use warp::Filter;

    use crate::register_server;

    /// Create a simple webserver which parses some basic http requests.
    fn create_http_server(ip: ([u8; 4], u16)) -> Result<oneshot::Sender<()>, ()> {
        let register = warp::post()
            .and(warp::path("test-register"))
            .and(warp::path::end())
            .map(|| {
                format!(
                    "
                    {{
                        \"public_id\": \"7N58aK\",
                        \"private_key\": \"oVZBbqJm5vXCmfTP8wQA0n13FeKd5Ego\"
                    }}"
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
            register_server("http://127.0.0.1:8001/test-register".into())
        }).await.unwrap().unwrap();

        assert_eq!(&res.public_id, "7N58aK");
        assert_eq!(&res.private_key, "oVZBbqJm5vXCmfTP8wQA0n13FeKd5Ego");

        let _ = close_server_tx.send(());
    }
}
