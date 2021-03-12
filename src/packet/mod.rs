use super::Result;
use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};
use std::fmt;

mod types;
pub use types::*;

mod error;
pub use error::{Error, ParseError};

const PROTOCOL_VERSION: u8 = 2;

#[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum Identifier {
    PushData = 0,
    PushAck = 1,
    PullData = 2,
    PullResp = 3,
    PullAck = 4,
    TxAck = 5,
}

pub mod pull_ack;
pub mod pull_data;
pub mod pull_resp;
pub mod push_ack;
pub mod push_data;
pub mod tx_ack;

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

#[derive(Debug, Copy, Clone, Hash, PartialEq)]
pub struct MacAddress {
    bytes: [u8; 8],
}

pub mod parser;

impl Eq for MacAddress {}

impl MacAddress {
    pub fn new(b: &[u8; 8]) -> MacAddress {
        MacAddress {
            bytes: [b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]],
        }
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl fmt::Display for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "MacAddress(")?;
        for i in 0..6 {
            write!(f, "{:02X}:", self.bytes[i])?;
        }
        write!(f, "{:02X}", self.bytes[7])?;
        write!(f, ")")
    }
}

use std::io::{Cursor, Write};

fn write_preamble(w: &mut Cursor<&mut [u8]>, token: u16) -> Result<()> {
    Ok(w.write_all(&[PROTOCOL_VERSION, (token >> 8) as u8, token as u8])?)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum StringOrNum {
    S(String),
    N(u64),
}

pub trait SerializablePacket {
    fn serialize(&self, buffer: &mut [u8]) -> std::result::Result<u64, Error>;
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
                w.write_all(&self.gateway_mac.bytes())?;
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
