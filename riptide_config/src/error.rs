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

#[derive(Debug)]
pub enum ErrorKind {
    IoError(std::io::Error),
    TomlParseError(toml::de::Error),
    BincodeError(bincode::ErrorKind),
    NetworkError(ureq::Error),
    ParseError(String),
    NotFound,
    IsNotDirectory,
    IsDirectory,
    SaveError,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            ErrorKind::IoError(e) => write!(f, "IO Error: {}", e),
            ErrorKind::TomlParseError(e) => write!(f, "Toml Parse Error: {}", e),
            ErrorKind::BincodeError(e) => write!(f, "Bincode Error: {}", e),
            ErrorKind::NetworkError(e) => write!(f, "Network Error: {}", e),
            ErrorKind::ParseError(e) => write!(f, "Parse Error: {}", e),
            ErrorKind::NotFound => write!(f, "Not Found"),
            ErrorKind::IsNotDirectory => write!(f, "Is Not Directory"),
            ErrorKind::IsDirectory => write!(f, "Is Directory"),
            ErrorKind::SaveError => write!(f, "Save Error"),
        }
    }
}

//TODO, implement source, description, and cause for this.
impl std::error::Error for ConfigError {}
