use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};
use std::fmt;

use super::Result;

#[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum Identifier {
    PushData = 0,
    PushAck = 1,
    PullData = 2,
    PullResp = 3,
    PullAck = 4,
}

fn random_token(buffer: &[u8]) -> u16 {
    (buffer[1] as u16) << 8 | buffer[2] as u16
}

fn gateway_mac(buffer: &[u8]) -> MacAddress {
    MacAddress::new(array_ref![buffer, 4, 6])
}

#[derive(Debug)]
pub enum Packet {
    PushData(PushData),
    PushAck(PushAck),
    PullData(PullData),
    PullResp(PullResp),
    PullAck(PullAck),
}

#[derive(Debug)]
pub struct PushData {
    random_token: u16,
    gateway_mac: MacAddress,
    data: PushDataJson,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PushDataJson {
    rxpk: Option<Vec<RxPk>>,
    stat: Option<Stat>,
}

impl PushData {
    pub fn new(buffer: &[u8], num_recv: usize) -> Result<PushData> {
        Ok(PushData {
            random_token: random_token(buffer),
            gateway_mac: gateway_mac(buffer),
            data: serde_json::from_str(std::str::from_utf8(&buffer[12..num_recv])?)?,
        })
    }
}

#[derive(Debug)]
pub struct PushAck {
    random_token: u16,
}

impl PushAck {
    pub fn new(buffer: &[u8], _num_recv: usize) -> Result<PushAck> {
        Ok(PushAck {
            random_token: random_token(buffer),
        })
    }
}

#[derive(Debug)]
pub struct PullData {
    random_token: u16,
    gateway_mac: MacAddress,
}

impl PullData {
    pub fn new(buffer: &[u8], _num_recv: usize) -> Result<PullData> {
        Ok(PullData {
            random_token: random_token(buffer),
            gateway_mac: gateway_mac(buffer),
        })
    }
}

#[derive(Debug)]
pub struct PullResp {
    random_token: u16, // need json objs
}

impl PullResp {
    pub fn new(buffer: &[u8], _num_recv: usize) -> Result<PullResp> {
        Ok(PullResp {
            random_token: random_token(buffer),
        })
    }
}

#[derive(Debug)]
pub struct PullAck {
    random_token: u16,
}

impl PullAck {
    pub fn new(buffer: &[u8], _num_recv: usize) -> Result<PullAck> {
        Ok(PullAck {
            random_token: random_token(buffer),
        })
    }
}

#[derive(Debug)]
pub struct MacAddress {
    bytes: [u8; 6],
}

impl MacAddress {
    pub fn new(b: &[u8; 6]) -> MacAddress {
        MacAddress {
            bytes: [b[0], b[1], b[2], b[3], b[4], b[5]],
        }
    }
}

impl fmt::Display for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "MacAddress(")?;
        for i in 0..5 {
            write!(f, "{:02X}:", self.bytes[i])?;
        }
        write!(f, "{:02X}", self.bytes[5])?;
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
