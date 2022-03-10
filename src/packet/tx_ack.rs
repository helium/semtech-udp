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
        self.data.get_result()
    }
}

impl SerializablePacket for Packet {
    fn serialize(&self, buffer: &mut [u8]) -> Result<u64, PktError> {
        let mut w = Cursor::new(buffer);
        write_preamble(&mut w, self.random_token)?;
        w.write_all(&[Identifier::TxAck as u8])?;
        w.write_all(self.gateway_mac.as_bytes())?;
        w.write_all(serde_json::to_string(&self.data)?.as_bytes())?;
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

/// We take all of the errors from the GWMP protocol.
/// These are tolerated in both "warn" or "error" fields
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum ErrorField {
    None,
    TooLate,
    TooEarly,
    CollisionPacket,
    CollisionBeacon,
    TxFreq,
    TxPower,
    GpsUnlocked,
    SendLBT,
    SendFail,
}

use std::convert::Into;

impl From<Result<(), Error>> for ErrorField {
    fn from(other: Result<(), Error>) -> ErrorField {
        match other {
            Err(Error::TooLate) => ErrorField::TooLate,
            Err(Error::TooEarly) => ErrorField::TooEarly,
            Err(Error::CollisionPacket) => ErrorField::CollisionPacket,
            Err(Error::CollisionBeacon) => ErrorField::CollisionBeacon,
            Err(Error::InvalidTransmitFrequency) => ErrorField::TxFreq,
            Err(Error::InvalidTransmitPower(_)) => ErrorField::TxPower,
            Err(Error::GpsUnlocked) => ErrorField::GpsUnlocked,
            Err(Error::SendLBT) => ErrorField::SendLBT,
            Err(Error::SendFail) => ErrorField::SendFail,
            _ => ErrorField::None,
        }
    }
}

impl From<ErrorField> for Result<(), Error> {
    fn from(other: ErrorField) -> Self {
        match other {
            ErrorField::TooEarly => Err(Error::TooEarly),
            ErrorField::CollisionPacket => Err(Error::CollisionPacket),
            ErrorField::CollisionBeacon => Err(Error::CollisionBeacon),
            ErrorField::TooLate => Err(Error::TooLate),
            ErrorField::TxFreq => Err(Error::InvalidTransmitFrequency),
            ErrorField::TxPower => Err(Error::InvalidTransmitPower(None)),
            ErrorField::GpsUnlocked => Err(Error::GpsUnlocked),
            ErrorField::SendLBT => Err(Error::SendLBT),
            ErrorField::SendFail => Err(Error::SendFail),
            ErrorField::None => Ok(()),
        }
    }
}

use thiserror::Error;
#[derive(Debug, Error, Clone, Copy, PartialEq, Serialize, Deserialize)]
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
    #[error("TxAck::Error::TX_POWER({0:?})")]
    InvalidTransmitPower(Option<i32>),
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
            txpk_ack: SubTxPkAck::Error {
                error: ErrorField::None,
            },
        }
    }
}

impl TxPkNack {
    pub fn new_with_error(error: Error) -> TxPkNack {
        let txpk_ack = if let Error::InvalidTransmitPower(Some(v)) = error {
            SubTxPkAck::Warn {
                warn: ErrorField::from(Err(error)),
                value: Some(v),
            }
        } else {
            SubTxPkAck::Error {
                error: ErrorField::from(Err(error)),
            }
        };
        TxPkNack { txpk_ack }
    }

    pub fn get_result(&self) -> Result<(), Error> {
        match &self.txpk_ack {
            SubTxPkAck::Error { error } => {
                let res: Result<(), Error> = (*error).into();
                res
            }
            SubTxPkAck::Warn { warn, value } => {
                if let ErrorField::TxPower = warn {
                    Err(Error::InvalidTransmitPower(*value))
                } else {
                    (*warn).into()
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum SubTxPkAck {
    Error {
        error: ErrorField,
    },
    Warn {
        warn: ErrorField,
        value: Option<i32>,
    },
}

#[test]
fn tx_nack_too_late() {
    let json = "{\"txpk_ack\": { \"error\": \"TOO_LATE\"}}";
    let parsed: TxPkNack = serde_json::from_str(json).expect("Error parsing tx_ack");
    if let Err(Error::TooLate) = parsed.get_result() {
    } else {
        assert!(false);
    }
}

#[test]
fn tx_ack_deser() {
    let json = "{\"txpk_ack\":{\"error\":\"NONE\"}}";
    let parsed: TxPkNack = serde_json::from_str(json).expect("Error parsing tx_ack");
    if let Err(_) = parsed.get_result() {
        assert!(false);
    }
}

#[test]
fn tx_nack_tx_power_legacy() {
    let json = "{ \"txpk_ack\": { \"error\" : \"TX_POWER\"}}";
    let parsed: TxPkNack = serde_json::from_str(json).expect("Error parsing tx_ack");
    if let Err(Error::InvalidTransmitPower(v)) = parsed.get_result() {
        assert!(v.is_none());
    } else {
        assert!(false);
    }
}

#[test]
fn tx_nack_tx_power_sx1302deser() {
    let json = "{ \"txpk_ack\": { \"warn\" : \"TX_POWER\", \"value\" : 27 }}";
    let parsed: TxPkNack = serde_json::from_str(json).expect("Error parsing tx_ack");
    if let Err(Error::InvalidTransmitPower(v)) = parsed.get_result() {
        if let Some(v) = v {
            assert_eq!(v, 27)
        } else {
            assert!(false)
        }
    } else {
        assert!(false)
    }
}

#[test]
fn tx_nack_tx_power_sx1302_ser() {
    let invalid_transmit_power = TxPkNack::new_with_error(Error::InvalidTransmitPower(Some(27)));
    let str = serde_json::to_string(&invalid_transmit_power).expect("serialization error");
    assert_eq!("{\"txpk_ack\":{\"warn\":\"TX_POWER\",\"value\":27}}", str)
}
