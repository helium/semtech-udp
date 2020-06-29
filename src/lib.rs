#[macro_use]
extern crate arrayref;
pub use error::Error;
use std::convert::TryFrom;
use std::error::Error as stdError;
use std::io::{Cursor, Write};

mod error;
mod types;
pub use types::*;
#[cfg(test)]
mod tests;

const PROTOCOL_VERSION: u8 = 2;

fn random_token(buffer: &[u8]) -> u16 {
    (buffer[1] as u16) << 8 | buffer[2] as u16
}

pub fn gateway_mac(buffer: &[u8]) -> MacAddress {
    MacAddress::new(array_ref![buffer, 0, 8])
}

#[derive(Debug, Clone)]
pub struct Packet {
    random_token: u16,
    gateway_mac: Option<MacAddress>,
    data: PacketData,
}

impl Packet {
    pub fn from_data(data: PacketData) -> Packet {
        Packet {
            random_token: 0,
            gateway_mac: None,
            data,
        }
    }

    pub fn data(&self) -> &PacketData {
        &self.data
    }

    pub fn set_gateway_mac(&mut self, mac: &[u8]) {
        self.gateway_mac = Some(gateway_mac(&mac));
    }

    pub fn set_token(&mut self, token: u16) {
        self.random_token = token;
    }

    pub fn parse(buffer: &[u8], num_recv: usize) -> std::result::Result<Packet, Box<dyn stdError>> {
        if buffer[0] != PROTOCOL_VERSION {
            Err(Error::InvalidProtocolVersion.into())
        } else if let Ok(id) = Identifier::try_from(buffer[3]) {
            Ok(Packet {
                // all packets have random_token
                random_token: random_token(buffer),
                // only PULL_DATA nad PUSH_DATA have MAC_IDs
                gateway_mac: match id {
                    Identifier::PullData | Identifier::PushData | Identifier::TxAck => {
                        Some(gateway_mac(buffer))
                    }
                    _ => None,
                },
                data: match id {
                    Identifier::PullData => PacketData::PullData,
                    Identifier::PushData => {
                        let json_str = std::str::from_utf8(&buffer[12..num_recv])?;
                        PacketData::PushData(serde_json::from_str(json_str)?)
                    }
                    Identifier::PullResp => {
                        let json_str = std::str::from_utf8(&buffer[4..num_recv])?;
                        PacketData::PullResp(serde_json::from_str(json_str)?)
                    }
                    Identifier::PullAck => PacketData::PullAck,
                    Identifier::PushAck => PacketData::PushAck,
                    Identifier::TxAck => PacketData::TxAck,
                },
            })
        }
        else {
            Err(Error::InvalidIdentifier.into())
        }
    }

    pub fn serialize(self, buffer: &mut [u8]) -> std::result::Result<u64, Box<dyn stdError>> {
        let mut w = Cursor::new(buffer);
        w.write_all(&[
            PROTOCOL_VERSION,
            (self.random_token >> 8) as u8,
            self.random_token as u8,
        ])?;

        w.write_all(&[match &self.data {
            PacketData::PushData(_) => Identifier::PushData,
            PacketData::PushAck => Identifier::PushAck,
            PacketData::PullData => Identifier::PullData,
            PacketData::PullResp(_) => Identifier::PullResp,
            PacketData::PullAck => Identifier::PullAck,
            PacketData::TxAck => Identifier::TxAck,
        } as u8])?;

        if let Some(mac) = self.gateway_mac {
            w.write_all(mac.bytes())?;
        };

        if let PacketData::PushData(data)  = self.data {
            w.write_all(&serde_json::to_string(&data)?.as_bytes())?;
        }
        Ok(w.position())
    }
}
