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
use super::{write_preamble, Error as PktError, Identifier, MacAddress, SerializablePacket};
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Write};

#[derive(Debug, Clone)]
pub struct Packet {
    pub random_token: u16,
    pub gateway_mac: MacAddress,
    pub data: Option<TxPkNack>,
}

impl Packet {
    pub fn has_error(&self) -> bool {
        if let Some(error) = &self.data {
            Error::NONE != error.txpk_ack.error
        } else {
            false
        }
    }

    pub fn get_error(&self) -> Option<Error> {
        self.data.as_ref().map(|txpk_ack| txpk_ack.txpk_ack.error)
    }
}

impl SerializablePacket for Packet {
    fn serialize(&self, buffer: &mut [u8]) -> Result<u64, PktError> {
        let mut w = Cursor::new(buffer);
        write_preamble(&mut w, self.random_token)?;
        w.write_all(&[Identifier::TxAck as u8])?;
        w.write_all(&self.gateway_mac.bytes())?;

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

use thiserror::Error;

#[derive(Error, Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
#[allow(non_camel_case_types)]
pub enum Error {
    #[error("TxAck::Error::NONE")]
    NONE,
    #[error("TxAck::Error::TOO_LATE")]
    TOO_LATE,
    #[error("TxAck::Error::TOO_EARLY")]
    TOO_EARLY,
    #[error("TxAck::Error::COLLISION_PACKET")]
    COLLISION_PACKET,
    #[error("TxAck::Error::COLLISION_BEACON")]
    COLLISION_BEACON,
    #[error("TxAck::Error::TX_FREQ")]
    TX_FREQ,
    #[error("TxAck::Error::TX_POWER")]
    TX_POWER,
    #[error("TxAck::Error::GPS_UNLOCKED")]
    GPS_UNLOCKED,
    #[error("TxAck::Error::SEND_LBT")]
    SEND_LBT,
    #[error("TxAck::Error::SEND_FAIL")]
    SEND_FAIL,
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
