/*
### 5.5. TX_ACK packet ###

That packet type is used by the gateway to send a feedback to the server
to inform if a downlink request has been accepted or rejected by the gateway.
The datagram may optionnaly contain a JSON string to give more details on
acknoledge. If no JSON is present (empty string), this means than no error
occured.

Bytes  | Function
:------:|---------------------------------------------------------------------
0      | protocol version = 2
1-2    | same token as the PULL_RESP packet to acknowledge
3      | TX_ACK identifier 0x05
4-11   | Gateway unique identifier (MAC address)
12-end | [optional] JSON object, starting with {, ending with }, see section 6

*/
use super::{write_preamble, Identifier, MacAddress, SerializablePacket};
use serde::{Deserialize, Serialize};
use std::{
    error::Error as stdError,
    fmt,
    io::{Cursor, Write},
};

#[derive(Debug, Clone)]
pub struct Packet {
    pub random_token: u16,
    pub gateway_mac: MacAddress,
    pub data: Option<TxPkNack>,
}

impl Packet {
    pub fn has_error(&self) -> bool {
        self.data.is_some()
    }

    pub fn get_error(&self) -> Option<Error> {
        if let Some(txpk_ack) = &self.data {
            Some(txpk_ack.txpk_ack.error)
        } else {
            None
        }
    }
}

impl SerializablePacket for Packet {
    fn serialize(&self, buffer: &mut [u8]) -> std::result::Result<u64, Box<dyn stdError>> {
        let mut w = Cursor::new(buffer);
        write_preamble(&mut w, self.random_token)?;
        w.write_all(&[Identifier::TxAck as u8])?;
        if let Some(data) = &self.data {
            w.write_all(&serde_json::to_string(&data)?.as_bytes())?;
        }

        Ok(w.position())
    }
}

impl From<Packet> for super::Packet {
    fn from(packet: Packet) -> super::Packet {
        super::Packet::Up(super::Up::TxAck(packet))
    }
}

// Value             | Definition
// :-----------------:|---------------------------------------------------------------------
// NONE              | Packet has been programmed for downlink
// TOO_LATE          | Rejected because it was already too late to program this packet for downlink
// TOO_EARLY         | Rejected because downlink packet timestamp is too much in advance
// COLLISION_PACKET  | Rejected because there was already a packet programmed in requested timeframe
// COLLISION_BEACON  | Rejected because there was already a beacon planned in requested timeframe
// TX_FREQ           | Rejected because requested frequency is not supported by TX RF chain
// TX_POWER          | Rejected because requested power is not supported by gateway
// GPS_UNLOCKED      | Rejected because GPS is unlocked, so GPS timestamp cannot be used
//
// Examples (white-spaces, indentation and newlines added for readability):
//
// ``` json
// {"txpk_ack":{
// "error":"COLLISION_PACKET"
// }}
// ```

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum Error {
    NONE,
    TOO_LATE,
    TOO_EARLY,
    COLLISION_PACKET,
    COLLISION_BEACON,
    TX_FREQ,
    TX_POWER,
    GPS_UNLOCKED,
    UNKNOWN_MAC, // this is an added error type in case a client tries
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NONE => write!(f, "TxAck::Error::NONE"),
            Error::TOO_LATE => write!(f, "TxAck::Error::TOO_LATE"),
            Error::TOO_EARLY => write!(f, "TxAck::Error::TOO_EARLY"),
            Error::COLLISION_PACKET => write!(f, "TxAck::Error::COLLISION_PACKET"),
            Error::COLLISION_BEACON => write!(f, "TxAck::Error::COLLISION_BEACON"),
            Error::TX_FREQ => write!(f, "TxAck::Error::TX_FREQ, Transmit frequency is rejected"),
            Error::TX_POWER => write!(f, "TxAck::Error::TX_POWER"),
            Error::GPS_UNLOCKED => write!(f, "TxAck::Error::GPS_UNLOCKED"),
            Error::UNKNOWN_MAC => write!(
                f,
                "TxAck::Error::UNKNOWN_MAC, Server does not have IP for MAC address requested"
            ),
        }
    }
}

impl stdError for Error {
    fn source(&self) -> Option<&(dyn stdError + 'static)> {
        Some(self)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TxPkNack {
    txpk_ack: SubTxPkAck,
}

impl TxPkNack {
    pub fn new(error: Error) -> TxPkNack {
        TxPkNack {
            txpk_ack: SubTxPkAck { error },
        }
    }
}
#[derive(Debug, Serialize, Deserialize, Clone)]
struct SubTxPkAck {
    pub error: Error,
}
