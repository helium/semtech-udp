#![allow(clippy::upper_case_acronyms)]
use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};
use std::fmt;

mod types;
pub use types::*;

mod error;
pub use error::{Error, ParseError};
pub type Result<T = ()> = std::result::Result<T, Error>;

pub use macaddr::MacAddr8 as MacAddress;

const PROTOCOL_VERSION: u8 = 2;

#[derive(Debug, Eq, PartialEq, TryFromPrimitive, Clone)]
#[repr(u8)]
pub enum Identifier {
    PushData = 0,
    PushAck = 1,
    PullData = 2,
    PullResp = 3,
    PullAck = 4,
    TxAck = 5,
}

impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

pub mod pull_ack;
pub mod pull_data;
pub mod pull_resp;
pub mod push_ack;
pub mod push_data;
pub mod tx_ack;

pub mod parser;

#[derive(Debug, Clone)]
pub enum Packet {
    Up(Up),
    Down(Down),
}

impl SerializablePacket for Packet {
    fn serialize(&self, buffer: &mut [u8]) -> Result<u64> {
        match self {
            Packet::Up(up) => match up {
                Up::PushData(pkt) => pkt.serialize(buffer),
                Up::PullData(pkt) => pkt.serialize(buffer),
                Up::TxAck(pkt) => pkt.serialize(buffer),
            },
            Packet::Down(down) => match down {
                Down::PushAck(pkt) => pkt.serialize(buffer),
                Down::PullAck(pkt) => pkt.serialize(buffer),
                Down::PullResp(pkt) => pkt.serialize(buffer),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub enum Up {
    PushData(push_data::Packet),
    PullData(pull_data::Packet),
    TxAck(tx_ack::Packet),
}

impl Up {
    pub fn set_gateway_mac(&mut self, mac: MacAddress) {
        match self {
            Up::PushData(push_data) => push_data.gateway_mac = mac,
            Up::PullData(pull_data) => pull_data.gateway_mac = mac,
            Up::TxAck(tx_ack) => tx_ack.gateway_mac = mac,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Down {
    PushAck(push_ack::Packet),
    PullAck(pull_ack::Packet),
    PullResp(Box<pull_resp::Packet>),
}

use std::io::{Cursor, Write};

fn write_preamble(w: &mut Cursor<&mut [u8]>, token: u16) -> Result {
    Ok(w.write_all(&[PROTOCOL_VERSION, (token >> 8) as u8, token as u8])?)
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum Tmst {
    Immediate,
    Tmst(u32),
}
use serde::Deserializer;

impl<'de> Deserialize<'de> for Tmst {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Tmst, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        use serde_json::Value;
        // use the JSON deserialize so as to accept strings or nums
        let value = Value::deserialize(deserializer)?;
        if let Value::String(str) = value {
            if str == "immediate" {
                Ok(Tmst::Immediate)
            } else {
                Err(Error::custom("invalid string for tmst field"))
            }
        } else if let Value::Number(num) = value {
            match num.as_u64() {
                Some(value) => {
                    if value < 2_u64.pow(32) {
                        Ok(Tmst::Tmst(value as u32))
                    } else {
                        Err(Error::custom(
                            "tmst field must be a 32-bit number. it appears to be larger",
                        ))
                    }
                }
                None => Err(Error::custom(
                    "when tmst field is a number, it must be an integer",
                )),
            }
        } else {
            Err(Error::custom("tmst field must be string or number"))
        }
    }
}

pub trait SerializablePacket {
    fn serialize(&self, buffer: &mut [u8]) -> Result<u64>;
}

#[macro_export]
// Up Packets feature Gateway Mac
macro_rules! simple_up_packet {
    ($packet:ident,$name:expr) => {
        impl SerializablePacket for $packet {
            fn serialize(&self, buffer: &mut [u8]) -> Result<u64> {
                let mut w = Cursor::new(buffer);
                write_preamble(&mut w, self.random_token)?;
                w.write_all(&[$name as u8])?;
                w.write_all(&self.gateway_mac.as_bytes())?;
                Ok(w.position())
            }
        }
    };
}

#[macro_export]
// Down packets only have random token and identifier
macro_rules! simple_down_packet {
    ($packet:ident,$name:expr) => {
        impl SerializablePacket for $packet {
            fn serialize(&self, buffer: &mut [u8]) -> std::result::Result<u64, PktError> {
                let mut w = Cursor::new(buffer);
                write_preamble(&mut w, self.random_token)?;
                w.write_all(&[$name as u8])?;
                Ok(w.position())
            }
        }
    };
}
