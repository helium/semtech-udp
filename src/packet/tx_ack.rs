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
    pub data: Data,
}

impl Packet {
    pub fn get_result(&self) -> Result<Option<u32>, Error> {
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

// ERRORS
//
// Value             | Definition
// :-----------------:|---------------------------------------------------------------------
// NONE              | Packet has been programmed for downlink
// TOO_LATE          | Rejected because it was already too late to program this packet for downlink
// TOO_EARLY         | Rejected because downlink packet timestamp is too much in advance
// COLLISION_PACKET  | Rejected because there was already a packet programmed in requested timeframe
// COLLISION_BEACON  | Rejected because there was already a beacon planned in requested timeframe
// TX_FREQ           | Rejected because requested frequency is not supported by TX RF chain
// GPS_UNLOCKED      | Rejected because GPS is unlocked, so GPS timestamp cannot be used
//
// WARNINGS
//
// Value             | Definition
// :-----------------:|---------------------------------------------------------------------
// TX_POWER          | Requested transmit power is not supported by gateway and was reduced. Adjusted power follows in "value", dBm.
//
// Examples (white-spaces, indentation and newlines added for readability):
//
// ``` json
// {"txpk_ack":{
// "error":"COLLISION_PACKET"
// }}
// ```
//
// ``` json
// {"txpk_ack":{
// "warn":"TX_POWER", "value": 27
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

impl From<Result<(), Error>> for ErrorField {
    fn from(other: Result<(), Error>) -> ErrorField {
        match other {
            Err(Error::TooLate) => ErrorField::TooLate,
            Err(Error::TooEarly) => ErrorField::TooEarly,
            Err(Error::CollisionPacket) => ErrorField::CollisionPacket,
            Err(Error::CollisionBeacon) => ErrorField::CollisionBeacon,
            Err(Error::InvalidTransmitFrequency) => ErrorField::TxFreq,
            Err(Error::InvalidTransmitPower(_)) => ErrorField::TxPower,
            Err(Error::AdjustedTransmitPower(_, _)) => ErrorField::TxPower,
            Err(Error::GpsUnlocked) => ErrorField::GpsUnlocked,
            Err(Error::SendLBT) => ErrorField::SendLBT,
            Err(Error::SendFail) => ErrorField::SendFail,
            _ => ErrorField::None,
        }
    }
}

impl ErrorField {
    fn to_result(&self, tmst: Option<u32>) -> Result<Option<u32>, Error> {
        match self {
            ErrorField::TooEarly => Err(Error::TooEarly),
            ErrorField::CollisionPacket => Err(Error::CollisionPacket),
            ErrorField::CollisionBeacon => Err(Error::CollisionBeacon),
            ErrorField::TooLate => Err(Error::TooLate),
            ErrorField::TxFreq => Err(Error::InvalidTransmitFrequency),
            ErrorField::TxPower => Err(Error::InvalidTransmitPower(None)),
            ErrorField::GpsUnlocked => Err(Error::GpsUnlocked),
            ErrorField::SendLBT => Err(Error::SendLBT),
            ErrorField::SendFail => Err(Error::SendFail),
            ErrorField::None => Ok(tmst),
        }
    }
}

use thiserror::Error;
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    #[error("TxAck::Error::ADJUSTED_TX_POWER({0:?})")]
    AdjustedTransmitPower(Option<i32>, Option<u32>),
    #[error("TxAck::Error::GPS_UNLOCKED")]
    GpsUnlocked,
    #[error("TxAck::Error::SEND_LBT")]
    SendLBT,
    #[error("TxAck::Error::SEND_FAIL")]
    SendFail,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Data {
    txpk_ack: TxPkAck,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TxPkAck {
    #[serde(skip_serializing_if = "Option::is_none")]
    tmst: Option<u32>,
    #[serde(flatten)]
    result: TxPkAckResult,
}

impl Default for Data {
    fn default() -> Self {
        Data {
            txpk_ack: TxPkAck {
                tmst: None,
                result: TxPkAckResult::Error {
                    error: ErrorField::None,
                },
            },
        }
    }
}

impl Data {
    pub fn new_with_error(error: Error) -> Data {
        let (tmst, result) = if let Error::AdjustedTransmitPower(value, tmst) = error {
            (
                tmst,
                TxPkAckResult::Warn {
                    warn: ErrorField::from(Err(error)),
                    value,
                },
            )
        } else {
            (
                None,
                TxPkAckResult::Error {
                    error: ErrorField::from(Err(error)),
                },
            )
        };
        Data {
            txpk_ack: TxPkAck { tmst, result },
        }
    }

    pub fn get_result(&self) -> Result<Option<u32>, Error> {
        match &self.txpk_ack.result {
            TxPkAckResult::Error { error } => (*error).to_result(self.txpk_ack.tmst),
            TxPkAckResult::Warn { warn, value } => {
                // We need special handling of the ErrorField when warning
                // otherwise, the into will specify it as InvalidTransmitPower
                if let ErrorField::TxPower = warn {
                    Err(Error::AdjustedTransmitPower(*value, self.txpk_ack.tmst))
                } else {
                    (*warn).to_result(self.txpk_ack.tmst)
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum TxPkAckResult {
    Error {
        error: ErrorField,
    },
    Warn {
        warn: ErrorField,
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<i32>,
    },
}

#[test]
fn tx_nack_too_late() {
    let json = "{\"txpk_ack\": { \"error\": \"TOO_LATE\"}}";
    let parsed: Data = serde_json::from_str(json).expect("Error parsing tx_ack");
    if let Err(Error::TooLate) = parsed.get_result() {
    } else {
        assert!(false);
    }
}

#[test]
fn tx_ack_deser() {
    let json = "{\"txpk_ack\":{\"error\":\"NONE\"}}";
    let parsed: Data = serde_json::from_str(json).expect("Error parsing tx_ack");
    if let Err(_) = parsed.get_result() {
        assert!(false);
    }
}

#[test]
fn tx_ack_deser_with_tmst() {
    let json = "{\"txpk_ack\":{\"error\":\"NONE\", \"tmst\": 1234}}";
    let parsed: Data = serde_json::from_str(json).expect("Error parsing tx_ack");
    match parsed.get_result() {
        Ok(Some(tmst)) => assert_eq!(1234, tmst),
        _ => assert!(false),
    }
}

#[test]
fn tx_nack_tx_power_legacy() {
    let json = "{ \"txpk_ack\": { \"error\" : \"TX_POWER\"}}";
    let parsed: Data = serde_json::from_str(json).expect("Error parsing tx_ack");
    if let Err(Error::InvalidTransmitPower(v)) = parsed.get_result() {
        assert!(v.is_none());
    } else {
        assert!(false);
    }
}

#[test]
fn tx_nack_tx_power_sx1302deser() {
    let json = "{ \"txpk_ack\": { \"warn\" : \"TX_POWER\", \"value\" : 27 }}";
    let parsed: Data = serde_json::from_str(json).expect("Error parsing tx_ack");
    if let Err(Error::AdjustedTransmitPower(power_used, tmst)) = parsed.get_result() {
        if let (Some(power_used), None) = (power_used, tmst) {
            assert_eq!(power_used, 27)
        } else {
            assert!(false)
        }
    } else {
        assert!(false)
    }
}

#[test]
fn tx_nack_tx_power_sx1302deser_with_tmst() {
    let json = "{ \"txpk_ack\": { \"warn\" : \"TX_POWER\", \"value\" : 27, \"tmst\": 1234 }}";
    let parsed: Data = serde_json::from_str(json).expect("Error parsing tx_ack");
    if let Err(Error::AdjustedTransmitPower(power_used, tmst)) = parsed.get_result() {
        if let (Some(power_used), Some(tmst)) = (power_used, tmst) {
            assert_eq!(power_used, 27);
            assert_eq!(tmst, 1234)
        } else {
            assert!(false)
        }
    } else {
        assert!(false)
    }
}

#[test]
fn tx_nack_tx_power_sx1302_ser() {
    let invalid_transmit_power = Data::new_with_error(Error::AdjustedTransmitPower(Some(27), None));
    let str = serde_json::to_string(&invalid_transmit_power).expect("serialization error");
    assert_eq!("{\"txpk_ack\":{\"warn\":\"TX_POWER\",\"value\":27}}", str)
}
