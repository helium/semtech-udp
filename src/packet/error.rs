use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("io error")]
    Io(#[from] std::io::Error),
    #[error("json serialization error")]
    JsonSerialize(#[from] serde_json::error::Error),
    #[error("packet parse error")]
    ParseError(#[from] ParseError),
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("invalid GWMP version")]
    InvalidProtocolVersion,
    #[error("invalid GWMP frame identifier")]
    InvalidIdentifier,
    #[error("utf8 error")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("unable to parse GWMP JSON")]
    Json,
}

impl From<serde_json::error::Error> for ParseError {
    fn from(_: serde_json::error::Error) -> ParseError {
        ParseError::Json
    }
}
