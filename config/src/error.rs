#[derive(Debug)]
pub struct ConfigError {
    pub kind: ErrorKind,
    pub message: String,
}

impl<'k> ConfigError {
    pub fn new<S>(kind: ErrorKind, message: S) -> ConfigError
    where
        S: AsRef<str> + 'k,
    {
        ConfigError {
            kind,
            message: message.as_ref().to_owned(),
        }
    }

    pub fn error_code(&self) -> u8 {
        //TODO, return error code based on kind
        1
    }

    /// Get a baisc message to be displayed to the user
    pub fn message(&self) -> String {
        todo!()
    }

    /// Get a detailed message to be displayed to the user.
    /// Will automatically re-print any internal types. This may be verbose,
    /// and show more information to the user than we would really like in most
    /// cases. Ideally this should be hidden behind an environmental variable.
    pub fn detailed_message(&self) -> String {
        todo!()
    }
}

//XXX consider creating conversions from wrapped types into our own ErrorKind.
#[derive(Debug)]
pub enum ErrorKind {
    IoError(std::io::Error),
    TomlParseError(toml::de::Error),
    BincodeError(bincode::ErrorKind),
    NetworkError(reqwest::Error),
    ParseError(String),
    NotFound,
    IsNotDirectory,
    IsDirectory,
    SaveError,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //TODO implement std::fmt::display for error type
        write!(f, "A configuration error has occured")
    }
}

//TODO, implement source, description, and cause for this.
impl std::error::Error for ConfigError {}
