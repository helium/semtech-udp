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

use std::error::Error;
impl PushData {
    // Write our own serializer so that we can match Semtech format exactly
    // There are two things that we need slightly different behavior to match
    // byte for byte the Semtech output
    // 
    // First off, rxpk and stat may both be in PushData payload, but when they
    // are omitted, there is no null value which is what serde was doing by default
    //
    // Secondly, they represent frequency with 6 points of precision
    // serde by default drops the useless 0s
    pub fn as_bytes(&self) -> std::result::Result<Vec<u8>, Box<dyn Error>> {
        let mut v = Vec::new();
        v.push(b'{');
        if let Some(rxpk) = &self.rxpk {
            //w.write("\"rxpk\":[".as_bytes())?;
            v.extend_from_slice("\"rxpk\":[".as_bytes());

            for (index, pk) in rxpk.iter().enumerate() {
                v.extend_from_slice("{\"tmst\":".as_bytes());
                v.extend_from_slice(serde_json::to_string(&pk.tmst)?.as_bytes());

                v.extend_from_slice(",\"chan\":".as_bytes());
                v.extend_from_slice(serde_json::to_string(&pk.chan)?.as_bytes());

                v.extend_from_slice(",\"rfch\":".as_bytes());
                v.extend_from_slice(serde_json::to_string(&pk.rfch)?.as_bytes());

                v.extend_from_slice(",\"freq\":".as_bytes());
                v.extend_from_slice(format!("{:.6}", pk.freq).as_bytes());

                v.extend_from_slice(",\"stat\":".as_bytes());
                v.extend_from_slice(serde_json::to_string(&pk.stat)?.as_bytes());

                v.extend_from_slice(",\"modu\":".as_bytes());
                v.extend_from_slice(serde_json::to_string(&pk.modu)?.as_bytes());

                v.extend_from_slice(",\"datr\":".as_bytes());
                v.extend_from_slice(serde_json::to_string(&pk.datr)?.as_bytes());

                v.extend_from_slice(",\"codr\":".as_bytes());
                v.extend_from_slice(serde_json::to_string(&pk.codr)?.as_bytes());

                v.extend_from_slice(",\"lsnr\":".as_bytes());
                v.extend_from_slice(serde_json::to_string(&pk.lsnr)?.as_bytes());

                v.extend_from_slice(",\"rssi\":".as_bytes());
                v.extend_from_slice(serde_json::to_string(&pk.rssi)?.as_bytes());

                v.extend_from_slice(",\"size\":".as_bytes());
                v.extend_from_slice(serde_json::to_string(&pk.size)?.as_bytes());

                v.extend_from_slice(",\"data\":".as_bytes());
                v.extend_from_slice(serde_json::to_string(&pk.data)?.as_bytes());

                v.push(b'}');

                if index != rxpk.len()-1 {
                    v.push(b',');
                }
            }
            v.push(b']');
            // append with comma if stat exists
            if let Some(_) = &self.stat {
                v.push(b',');
            }
        }

        if let Some(stat) = &self.stat {
            v.extend_from_slice("\"stat\":".as_bytes());
            v.extend_from_slice(serde_json::to_string(&stat)?.as_bytes());
        }
        v.push(b'}');
        Ok(v)
    }
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

// the order of this is important as it makes us identical to Semtech
#[derive(Debug, Serialize, Deserialize)]
pub struct Stat {
    time: String,
    rxnb: u64,
    rxok: u64,
    rxfw: u64,
    ackr: f64,
    dwnb: u64,
    txnb: u64,
}
