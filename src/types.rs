use super::util::{parse_f64_field, parse_i64_field, parse_string_field, parse_u64_field};
use std::fmt;
use num_enum::TryFromPrimitive;

#[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum Identifier {
    PushData = 0,
    PushAck = 1,
    PullData = 2,
    PullResp = 3,
    PullAck = 4,
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
