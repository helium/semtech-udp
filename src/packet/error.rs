use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("io error")]
    Io(std::io::Error),
    #[error("json serialization error")]
    JsonSerialize(serde_json::error::Error),
    #[error("packet parse error")]
    ParseError(ParseError),
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("invalid GWMP version")]
    InvalidProtocolVersion,
    #[error("invalid GWMP frame identifier")]
    InvalidIdentifier,
    #[error("utf8 error")]
    Utf8(std::str::Utf8Error),
    #[error("unable to parse GWMP JSON")]
    Json,
}

impl From<ParseError> for Error {
    fn from(err: ParseError) -> Error {
        Error::ParseError(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<serde_json::error::Error> for Error {
    fn from(err: serde_json::error::Error) -> Error {
        Error::JsonSerialize(err)
    }
}

impl From<std::str::Utf8Error> for ParseError {
    fn from(err: std::str::Utf8Error) -> ParseError {
        ParseError::Utf8(err)
    }
}

impl From<serde_json::error::Error> for ParseError {
    fn from(_: serde_json::error::Error) -> ParseError {
        ParseError::Json
    }
}
