use crate::{Down, Up};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("io error")]
    Io(#[from] std::io::Error),
    #[error("json serialization error")]
    JsonSerialize(#[from] serde_json::error::Error),
    #[error("packet parse error")]
    Parse(#[from] ParseError),
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("invalid GWMP version")]
    InvalidProtocolVersion,
    #[error("invalid GWMP frame identifier")]
    InvalidIdentifier,
    #[error("utf8 error")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("invalid Json string for {identifier} frame: {json_str}. JsonError: {json_error}")]
    InvalidJson {
        identifier: crate::Identifier,
        json_str: String,
        json_error: serde_json::Error,
    },
    #[error("Received downlink when expecting uplinks only")]
    UnexpectedDownlink(Down),
    #[error("Received uplink when expecting downlinks only")]
    UnexpectedUplink(Box<Up>),
}
