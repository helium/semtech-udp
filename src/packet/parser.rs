use super::*;
use crate::tx_ack::Data;
use std::{convert::TryFrom, result::Result};

const PROTOCOL_VERSION_INDEX: usize = 0;
const IDENTIFIER_INDEX: usize = 3;
const PREFIX_LEN: usize = IDENTIFIER_INDEX + 1;
const PACKET_PAYLOAD_START: usize = 8;
const GATEWAY_MAC_LEN: usize = 8;

fn random_token(buffer: &[u8]) -> u16 {
    (buffer[1] as u16) << 8 | buffer[2] as u16
}

pub fn gateway_mac(buffer: &[u8]) -> Result<MacAddress, ParseError> {
    if buffer.len() < GATEWAY_MAC_LEN {
        Err(ParseError::InvalidPacketLength(buffer.len(), 8))
    } else {
        Ok(MacAddress::new(
            buffer[0], buffer[1], buffer[2], buffer[3], buffer[4], buffer[5], buffer[6], buffer[7],
        ))
    }
}

impl Packet {
    pub fn parse_uplink(buffer: &[u8]) -> Result<Up, ParseError> {
        match Self::parse(buffer)? {
            Packet::Up(up) => Ok(up),
            Packet::Down(down) => Err(ParseError::UnexpectedDownlink(down)),
        }
    }
    pub fn parse_downlink(buffer: &[u8]) -> Result<Down, ParseError> {
        match Self::parse(buffer)? {
            Packet::Down(down) => Ok(down),
            Packet::Up(up) => Err(ParseError::UnexpectedUplink(Box::new(up))),
        }
    }
}

impl Packet {
    fn parse(buffer: &[u8]) -> Result<Packet, ParseError> {
        if buffer.len() < PREFIX_LEN {
            return Err(ParseError::InvalidPacketLength(buffer.len(), PREFIX_LEN));
        }

        let protocol_version = buffer[PROTOCOL_VERSION_INDEX];
        if protocol_version != PROTOCOL_VERSION {
            return Err(ParseError::InvalidProtocolVersion(protocol_version));
        };

        let frame_identifier = buffer[IDENTIFIER_INDEX];
        match Identifier::try_from(frame_identifier) {
            Err(_) => Err(ParseError::InvalidIdentifier(frame_identifier)),
            Ok(id) => {
                // the token is before the identifier which we've already done a length check for
                let random_token = random_token(buffer);
                let buffer = &buffer[PREFIX_LEN..];

                Ok(match id {
                    // up packets
                    Identifier::PullData => {
                        let gateway_mac = gateway_mac(buffer)?;
                        pull_data::Packet {
                            random_token,
                            gateway_mac,
                        }
                        .into()
                    }
                    Identifier::PushData => {
                        let gateway_mac = gateway_mac(buffer)?;
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
                        let gateway_mac = gateway_mac(buffer)?;
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

// deals with null byte terminated json and empty
fn terminate(buf: &[u8]) -> usize {
    if buf.is_empty() {
        0
    } else if buf[buf.len() - 1] == 0 {
        buf.len() - 1
    } else {
        buf.len()
    }
}
