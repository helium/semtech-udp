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
    pub data: TxPkNack,
}

impl Packet {
    pub fn get_result(&self) -> Result<(), Error> {
        self.data.txpk_ack.error
    }
}

impl SerializablePacket for Packet {
    fn serialize(&self, buffer: &mut [u8]) -> Result<u64, PktError> {
        let mut w = Cursor::new(buffer);
        write_preamble(&mut w, self.random_token)?;
        w.write_all(&[Identifier::TxAck as u8])?;
        w.write_all(&self.gateway_mac.bytes())?;
        w.write_all(&serde_json::to_string(&self.data)?.as_bytes())?;
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
/// We take all of the errors from the GWMP protocol
/// except for the NONE response. We write a custom
/// serializer and deserializer to accommodate that
#[derive(Error, Debug, Clone, Copy, PartialEq)]
pub enum Error {
    #[error("TxAck::Error::TOO_LATE")]
    TooLate,
    #[error("TxAck::Error::TOO_EARLY")]
    TooEarly,
    #[error("TxAck::Error::COLLISION_PACKET")]
    CollisionPacket,
    #[error("TxAck::Error::COLLISION_BEACON")]
    CollisionBeacon,
    #[error("TxAck::Error::TX_FREQ")]
    InvalidTransmitFrequency,
    #[error("TxAck::Error::TX_POWER")]
    InvalidTransmitPower,
    #[error("TxAck::Error::GPS_UNLOCKED")]
    GpsUnlocked,
    #[error("TxAck::Error::SEND_LBT")]
    SendLBT,
    #[error("TxAck::Error::SEND_FAIL")]
    SendFail,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TxPkNack {
    txpk_ack: SubTxPkAck,
}

impl Default for TxPkNack {
    fn default() -> Self {
        TxPkNack {
            txpk_ack: SubTxPkAck { error: Ok(()) },
        }
    }
}

impl TxPkNack {
    pub fn new_with_error(error: Error) -> TxPkNack {
        TxPkNack {
            txpk_ack: SubTxPkAck { error: Err(error) },
        }
    }
}
#[derive(Debug, Serialize, Clone, Deserialize)]
struct SubTxPkAck {
    #[serde(deserialize_with = "deserialize", serialize_with = "serialize")]
    pub error: Result<(), Error>,
}

/// Because `error: NONE` is possible, we write a custom serializer
/// and deserializer that will provide Ok(()) the NONE case but the
/// tx_ack::Error type in other cases
use serde::{
    de::{self, Deserializer},
    Serializer,
};
pub fn deserialize<'de, D>(d: D) -> std::result::Result<Result<(), Error>, D::Error>
where
    D: Deserializer<'de>,
{
    match String::deserialize(d)?.as_str() {
        "None" => Ok(Ok(())),
        "TOO_LATE" => Ok(Err(Error::TooLate)),
        "TOO_EARLY" => Ok(Err(Error::TooEarly)),
        "COLLISION_PACKET" => Ok(Err(Error::CollisionPacket)),
        "COLLISION_BEACON" => Ok(Err(Error::CollisionBeacon)),
        "TX_FREQ" => Ok(Err(Error::InvalidTransmitFrequency)),
        "TX_POWER" => Ok(Err(Error::InvalidTransmitPower)),
        "GPS_UNLOCKED" => Ok(Err(Error::GpsUnlocked)),
        "SEND_LBT" => Ok(Err(Error::SendLBT)),
        "SEND_FAIL" => Ok(Err(Error::SendFail)),
        _ => Err(de::Error::custom("path contains invalid UTF-8 characters")),
    }
}

fn serialize<S>(res: &Result<(), Error>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match res {
        Ok(()) => s.serialize_str("NONE"),
        Err(Error::TooLate) => s.serialize_str("TOO_LATE"),
        Err(Error::TooEarly) => s.serialize_str("TOO_EARLY"),
        Err(Error::CollisionPacket) => s.serialize_str("COLLISION_PACKET"),
        Err(Error::CollisionBeacon) => s.serialize_str("COLLISION_BEACON"),
        Err(Error::InvalidTransmitFrequency) => s.serialize_str("TX_FREQ"),
        Err(Error::InvalidTransmitPower) => s.serialize_str("TX_POWER"),
        Err(Error::GpsUnlocked) => s.serialize_str("GPS_UNLOCKED"),
        Err(Error::SendLBT) => s.serialize_str("SEND_LBT"),
        Err(Error::SendFail) => s.serialize_str("SEND_FAIL"),
    }
}
