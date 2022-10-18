use super::*;
use crate::tx_ack::Data;
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

impl Packet {
    pub fn parse_uplink(buffer: &[u8]) -> std::result::Result<Up, ParseError> {
        match Self::parse(buffer)? {
            Packet::Up(up) => Ok(up),
            Packet::Down(down) => Err(ParseError::UnexpectedDownlink(down)),
        }
    }
    pub fn parse_downlink(buffer: &[u8]) -> std::result::Result<Down, ParseError> {
        match Self::parse(buffer)? {
            Packet::Down(down) => Ok(down),
            Packet::Up(up) => Err(ParseError::UnexpectedUplink(up)),
        }
    }
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
                        let json_str =
                            std::str::from_utf8(&buffer[PACKET_PAYLOAD_START..terminate(buffer)])?;
                        let data = serde_json::from_str(json_str).map_err(|json_error| {
                            ParseError::InvalidJson {
                                identifier: id,
                                json_str: json_str.into(),
                                json_error,
                            }
                        })?;
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
                            // guard against some packet forwarders that put a 0 byte as the last byte
                            if buffer.len() == PACKET_PAYLOAD_START + 1
                                && buffer[PACKET_PAYLOAD_START] == 0
                            {
                                Data::default()
                            } else {
                                let json_str = std::str::from_utf8(
                                    &buffer[PACKET_PAYLOAD_START..terminate(buffer)],
                                )?;
                                serde_json::from_str(json_str).map_err(|json_error| {
                                    ParseError::InvalidJson {
                                        identifier: id,
                                        json_str: json_str.into(),
                                        json_error,
                                    }
                                })?
                            }
                        } else {
                            Data::default()
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
                        let json_str = std::str::from_utf8(&buffer[..terminate(buffer)])?;
                        let data = serde_json::from_str(json_str).map_err(|json_error| {
                            ParseError::InvalidJson {
                                identifier: id,
                                json_str: json_str.into(),
                                json_error,
                            }
                        })?;
                        pull_resp::Packet { random_token, data }.into()
                    }
                })
            }
        }
    }
}

// deals with null byte terminated json
fn terminate(buf: &[u8]) -> usize {
    if buf[buf.len() - 1] == 0 {
        buf.len() - 1
    } else {
        buf.len()
    }
}
