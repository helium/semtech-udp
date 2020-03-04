use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum Identifier {
    PushData = 0,
    PushAck = 1,
    PullData = 2,
    PullResp = 3,
    PullAck = 4,
}

#[derive(Debug)]
pub enum PacketData {
    PushData(PushData),
    PushAck,
    PullData,
    PullResp,
    PullAck,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PushData {
    rxpk: Option<Vec<RxPk>>,
    stat: Option<Stat>,
}

#[derive(Debug)]
pub struct MacAddress {
    bytes: [u8; 8],
}

impl MacAddress {
    pub fn new(b: &[u8; 8]) -> MacAddress {
        MacAddress {
            bytes: [b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]],
        }
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl fmt::Display for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "MacAddress(")?;
        for i in 0..6 {
            write!(f, "{:02X}:", self.bytes[i])?;
        }
        write!(f, "{:02X}", self.bytes[7])?;
        write!(f, ")")
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RxPk {
    chan: u64,
    codr: String,
    data: String,
    datr: String,
    freq: f64,
    lsnr: f64,
    modu: String,
    rfch: u64,
    rssi: i64,
    size: u64,
    stat: u64,
    tmst: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Stat {
    ackr: f64,
    dwnb: u64,
    rxfw: u64,
    rxnb: u64,
    rxok: u64,
    time: String,
    txnb: u64,
}
