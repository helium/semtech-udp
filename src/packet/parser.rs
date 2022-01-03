use super::*;
use crate::tx_ack::TxPkNack;
use std::convert::TryFrom;

const PROTOCOL_VERSION_INDEX: usize = 0;
const IDENTIFIER_INDEX: usize = 3;
const PACKET_PAYLOAD_START: usize = 8;

fn random_token(buffer: &[u8]) -> u16 {
    (buffer[1] as u16) << 8 | buffer[2] as u16
}

pub fn gateway_mac(buffer: &[u8]) -> MacAddress {
    MacAddress::new(
        buffer[0], buffer[1], buffer[2], buffer[3], buffer[4], buffer[5], buffer[6], buffer[7],
    )
}

pub trait Parser {
    fn parse(buffer: &[u8]) -> std::result::Result<Packet, ParseError>;
}

impl Parser for Packet {
    fn parse(buffer: &[u8]) -> std::result::Result<Packet, ParseError> {
        if buffer[PROTOCOL_VERSION_INDEX] != PROTOCOL_VERSION {
            return Err(ParseError::InvalidProtocolVersion);
        };

        match Identifier::try_from(buffer[IDENTIFIER_INDEX]) {
            Err(_) => Err(ParseError::InvalidIdentifier),
            Ok(id) => {
                let random_token = random_token(buffer);
                let buffer = &buffer[4..];
                Ok(match id {
                    // up packets
                    Identifier::PullData => {
                        let gateway_mac = gateway_mac(&buffer[..PACKET_PAYLOAD_START]);
                        pull_data::Packet {
                            random_token,
                            gateway_mac,
                        }
                        .into()
                    }
                    Identifier::PushData => {
                        let gateway_mac = gateway_mac(&buffer[..PACKET_PAYLOAD_START]);
                        let json_str = std::str::from_utf8(&buffer[PACKET_PAYLOAD_START..])?;
                        println!("{}", json_str);
                        let data = serde_json::from_str(json_str)?;

                        push_data::Packet {
                            random_token,
                            gateway_mac,
                            data,
                        }
                        .into()
                    }
                    Identifier::TxAck => {
                        let gateway_mac = gateway_mac(&buffer[..PACKET_PAYLOAD_START]);
                        let data = if buffer.len() > PACKET_PAYLOAD_START {
                            let json_str = std::str::from_utf8(&buffer[PACKET_PAYLOAD_START..])?;
                            serde_json::from_str(json_str)?
                        } else {
                            TxPkNack::default()
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
                        let json_str = std::str::from_utf8(buffer)?;
                        let data = serde_json::from_str(json_str)?;
                        pull_resp::Packet { random_token, data }.into()
                    }
                })
            }
        }
    }
}
