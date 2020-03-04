use super::util::{parse_f64_field, parse_i64_field, parse_string_field, parse_u64_field};
use num_enum::TryFromPrimitive;
use serde_json::Value;
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

pub enum Packet {
    PushData(PushData),
    PushAck(PushAck),
    PullData(PullData),
    PullResp(PullResp),
    PullAck(PullAck),
}

pub struct PushData {
    random_token: u16,
    gateway_mac: MacAddress,
    rxpk: Option<Vec<RxPk>>,
    stat: Option<Stat>,
}

impl PushData {
    pub fn new(buffer: &[u8], num_recv: usize) -> Result<PushData> {
        let mut rxpk: Option<Vec<RxPk>> = None;
        let mut stat = None;
        if let Ok(json_str) = std::str::from_utf8(&buffer[12..num_recv]) {
            let v: Value = serde_json::from_str(json_str)?;
            match &v["rxpk"] {
                Value::Array(rxpk_arr) => {
                    let mut temp = Vec::new();
                    for pkt in rxpk_arr {
                        temp.push(RxPk::from_value(pkt));
                    }
                    rxpk = Some(temp);
                }
                _ => (),
            };

            match &v["stat"] {
                Value::Object(_) => {
                    stat = Some(Stat::from_value(&v["stat"]));
                }
                _ => (),
            };
        }

        Ok(PushData {
            random_token: random_token(buffer),
            gateway_mac: gateway_mac(buffer),
            rxpk,
            stat,
        })
    }
}

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

#[derive(Debug)]
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

impl RxPk {
    pub fn from_value(el: &serde_json::value::Value) -> RxPk {
        RxPk {
            chan: parse_u64_field(&el, "chan"),
            codr: parse_string_field(&el, "codr"),
            data: parse_string_field(&el, "data"),
            datr: parse_string_field(&el, "datr"),
            freq: parse_f64_field(&el, "freq"),
            lsnr: parse_f64_field(&el, "lsnr"),
            modu: parse_string_field(&el, "modu"),
            rfch: parse_u64_field(&el, "rfch"),
            rssi: parse_i64_field(&el, "rssi"),
            size: parse_u64_field(&el, "size"),
            stat: parse_u64_field(&el, "stat"),
            tmst: parse_u64_field(&el, "tmst"),
        }
    }
}

#[derive(Debug)]
pub struct Stat {
    ackr: f64,
    dwnb: u64,
    rxfw: u64,
    rxnb: u64,
    rxok: u64,
    time: String,
    txnb: u64,
}

impl Stat {
    pub fn from_value(map: &serde_json::value::Value) -> Stat {
        Stat {
            ackr: parse_f64_field(map, "ackr"),
            dwnb: parse_u64_field(map, "dwnb"),
            rxfw: parse_u64_field(map, "rxfw"),
            rxnb: parse_u64_field(map, "rxnb"),
            rxok: parse_u64_field(map, "rxok"),
            time: parse_string_field(map, "time"),
            txnb: parse_u64_field(map, "txnb"),
        }
    }
}
