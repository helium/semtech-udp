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

fn gateway_mac(buffer: &[u8]) -> MacAddress {
    MacAddress::new(array_ref![buffer, 4, 8])
}

#[derive(Debug)]
pub struct Packet {
    random_token: u16,
    gateway_mac: Option<MacAddress>,
    data: PacketData,
}

impl Packet {
    pub fn data(&self) -> &PacketData {
        &self.data
    }

    pub fn parse(buffer: &[u8], num_recv: usize) -> std::result::Result<Packet, Box<dyn stdError>> {
        if buffer[0] != PROTOCOL_VERSION {
            Err(Error::InvalidProtocolVersion.into())
        } else {
            if let Ok(id) = Identifier::try_from(buffer[3]) {
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
                            println!("{:}", json_str);
                            PacketData::PushData(serde_json::from_str(json_str)?)
                        }
                        Identifier::PullResp => {
                            let json_str = std::str::from_utf8(&buffer[4..num_recv])?;
                            println!("{:}", json_str);
                            PacketData::PullResp(serde_json::from_str(json_str)?)
                        }
                        Identifier::PullAck => PacketData::PullAck,
                        Identifier::PushAck => PacketData::PushAck,
                        Identifier::TxAck => PacketData::TxAck,
                    },
                })
            } else {
                Err(Error::InvalidIdentifier.into())
            }
        }
    }

    pub fn serialize(self, buffer: &mut [u8]) -> std::result::Result<u64, Box<dyn stdError>> {
        let mut w = Cursor::new(buffer);
        w.write(&[
            PROTOCOL_VERSION,
            (self.random_token >> 8) as u8,
            self.random_token as u8,
        ])?;

        w.write(&[match &self.data {
            PacketData::PushData(_) => Identifier::PushData,
            PacketData::PushAck => Identifier::PushAck,
            PacketData::PullData => Identifier::PullData,
            PacketData::PullResp(_) => Identifier::PullResp,
            PacketData::PullAck => Identifier::PullAck,
            PacketData::TxAck => Identifier::TxAck,
        } as u8])?;

        if let Some(mac) = self.gateway_mac {
            w.write(mac.bytes())?;
        };

        match self.data {
            PacketData::PushData(data) => {
                w.write(&serde_json::to_string(&data)?.as_bytes())?;
            }
            _ => (),
        };
        Ok(w.position())
    }
}
