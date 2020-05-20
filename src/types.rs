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
    TxAck = 5,
}

#[derive(Debug)]
pub enum PacketData {
    PushData(PushData),
    PushAck,
    PullData,
    PullResp(PullResp),
    PullAck,
    TxAck,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PushData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rxpk: Option<Vec<RxPk>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stat: Option<Stat>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PullResp {
    pub txpk: TxPk,
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
    pub data: String,
    pub datr: String,
    pub freq: f64,
    pub lsnr: f64,
    modu: String,
    rfch: u64,
    pub rssi: i64,
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StringOrNum {
    S(String),
    N(u64),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TxPk {
    imme: bool,            // Send packet immediately (will ignore tmst & time)
    pub tmst: StringOrNum, // Send packet on a certain timestamp value (will ignore time)
    #[serde(skip_serializing_if = "Option::is_none")]
    tmms: Option<StringOrNum>, // Send packet at a certain GPS time (GPS synchronization required)
    pub freq: f64,         // TX central frequency in MHz (unsigned float, Hz precision)
    rfch: u64,             // Concentrator "RF chain" used for TX (unsigned integer)
    powe: u64,             // TX output power in dBm (unsigned integer, dBm precision)
    modu: String,          // Modulation identifier "LORA" or "FSK"
    pub datr: String,      // LoRa datarate identifier (eg. SF12BW500)
    codr: String,          // LoRa ECC coding rate identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    fdev: Option<u64>, //FSK frequency deviation (unsigned integer, in Hz)
    ipol: bool,            // Lora modulation polarization inversion
    prea: Option<u64>,     // RF preamble size (unsigned integer)
    size: u64,             // RF packet payload size in bytes (unsigned integer)
    pub data: String,      // Base64 encoded RF packet payload, padding optional
    #[serde(skip_serializing_if = "Option::is_none")]
    ncrc: Option<bool>, // If true, disable the CRC of the physical layer (optional)
}
