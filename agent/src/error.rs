#[derive(Debug)]
pub enum Error {
    ReadFile(std::io::Error),
    // Closed(String),
    Http(reqwest::Error),
    Conversion(String),
}

impl std::convert::From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::ReadFile(e)
    }
}

impl std::convert::From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Error {
        Error::Http(e)
    }
}

impl std::convert::From<std::num::ParseIntError> for Error {
    fn from(e: std::num::ParseIntError) -> Error {
        Error::Conversion(e.to_string())
    }
}
