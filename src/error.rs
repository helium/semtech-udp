use std::error::Error as stdError;

use std::{fmt, str};

#[derive(Debug, Clone)]
pub enum Error {
    InvalidProtocolVersion,
    InvalidIdentifier,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::InvalidProtocolVersion => {
                write!(f, "Invalid protocol version (byte 0 in UDP frame)")
            }
            Error::InvalidIdentifier => {
                write!(f, "Invalid message identifier (byte 3 in UDP frame)")
            }
        }
    }
}

impl stdError for Error {
    fn description(&self) -> &str {
        match self {
            Error::InvalidProtocolVersion => "Invalid protocol version (byte 0 in UDP frame)",
            Error::InvalidIdentifier => "Invalid message identifier (byte 3 in UDP frame)",
        }
    }

    fn cause(&self) -> Option<&dyn stdError> {
        // Generic error, underlying cause isn't tracked.
        None
    }
}
