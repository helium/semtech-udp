use super::*;
use std::convert::TryFrom;

fn random_token(buffer: &[u8]) -> u16 {
    (buffer[1] as u16) << 8 | buffer[2] as u16
}

pub fn gateway_mac(buffer: &[u8]) -> MacAddress {
    MacAddress::new(array_ref![buffer, 0, 8])
}

pub trait Parser {
    fn parse(buffer: &[u8], num_recv: usize) -> std::result::Result<Packet, Error>;
}

impl Parser for Packet {
    fn parse(buffer: &[u8], num_recv: usize) -> std::result::Result<Packet, Error> {
        if buffer[0] != PROTOCOL_VERSION {
            Err(Error::InvalidProtocolVersion)
        } else if let Ok(id) = Identifier::try_from(buffer[3]) {
            // all packets have random_token
            let random_token = random_token(buffer);
            Ok(match id {
                // up packets
                Identifier::PullData => {
                    let gateway_mac = gateway_mac(&buffer[4..12]);
                    pull_data::Packet {
                        random_token,
                        gateway_mac,
                    }
                    .into()
                }
                Identifier::PushData => {
                    let gateway_mac = gateway_mac(&buffer[4..12]);
                    let json_str = std::str::from_utf8(&buffer[12..num_recv])?;
                    let data = serde_json::from_str(json_str).unwrap();

                    push_data::Packet {
                        random_token,
                        gateway_mac,
                        data,
                    }
                    .into()
                }
                Identifier::TxAck => {
                    let gateway_mac = gateway_mac(&buffer[4..12]);
                    let data = if num_recv > 12 {
                        let json_str = std::str::from_utf8(&buffer[12..num_recv])?;
                        Some(serde_json::from_str(json_str).unwrap())
                    } else {
                        None
                    };
                    tx_ack::Packet {
                        random_token,
                        gateway_mac,
                        data,
                    }
                    .into()
                }
                // down packets
                Identifier::PushAck => push_ack::Packet { random_token }.into(),
                Identifier::PullAck => pull_ack::Packet { random_token }.into(),
                Identifier::PullResp => {
                    let json_str = std::str::from_utf8(&buffer[4..num_recv])?;
                    let data = serde_json::from_str(json_str)?;
                    pull_resp::Packet { random_token, data }.into()
                }
            })
        } else {
            Err(Error::InvalidIdentifier)
        }
    }
}

use std::{fmt, str};

#[derive(Debug, Clone)]
pub enum Error {
    InvalidProtocolVersion,
    InvalidIdentifier,
    Utf8(std::str::Utf8Error),
    Json,
}

impl From<std::str::Utf8Error> for Error {
    fn from(err: std::str::Utf8Error) -> Error {
        Error::Utf8(err)
    }
}

impl From<serde_json::error::Error> for Error {
    fn from(_: serde_json::error::Error) -> Error {
        Error::Json
    }
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
            Error::Utf8(err) => write!(f, "UTF-8 from bytes parsing error: {}", err),
            Error::Json => write!(f, "Json Deserialization Error"),
        }
    }
}

impl stdError for Error {
    fn description(&self) -> &str {
        match self {
            Error::InvalidProtocolVersion => "Invalid protocol version (byte 0 in UDP frame)",
            Error::InvalidIdentifier => "Invalid message identifier (byte 3 in UDP frame)",
            Error::Utf8(_err) => "UTF-8 from bytes parsing error",
            Error::Json => "Json Deserialization Error",
        }
    }

    fn cause(&self) -> Option<&dyn stdError> {
        // Generic error, underlying cause isn't tracked.
        None
    }
}
