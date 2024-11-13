use crate::packet::types::{deserialize_codr, serialize_codr};
use crate::push_data::CRC;
use crate::{DataRate, Modulation};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum RxPk {
    V1(RxPkV1),
    V2(RxPkV2),
}

/*
Name |  Type  | Function
:----:|:------:|--------------------------------------------------------------
time | string | UTC time of pkt RX, us precision, ISO 8601 'compact' format
tmms | number | GPS time of pkt RX, number of milliseconds since 06.Jan.1980
tmst | number | Internal timestamp of "RX finished" event (32b unsigned)
freq | number | RX central frequency in MHz (unsigned float, Hz precision)
chan | number | Concentrator "IF" channel used for RX (unsigned integer)
rfch | number | Concentrator "RF chain" used for RX (unsigned integer)
stat | number | CRC status: 1 = OK, -1 = fail, 0 = no CRC
modu | string | Modulation identifier "LORA" or "FSK"
datr | string | LoRa datarate identifier (eg. SF12BW500)
datr | number | FSK datarate (unsigned, in bits per second)
codr | string | LoRa ECC coding rate identifier
rssi | number | RSSI in dBm (signed integer, 1 dB precision)
lsnr | number | Lora SNR ratio in dB (signed float, 0.1 dB precision)
size | number | RF packet payload size in bytes (unsigned integer)
data | string | Base64 encoded RF packet payload, padded
 */
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RxPkV1 {
    pub chan: u64,
    #[serde(
        serialize_with = "serialize_codr",
        deserialize_with = "deserialize_codr"
    )]
    pub codr: Option<lora_modulation::CodingRate>,
    #[serde(with = "crate::packet::types::base64")]
    pub data: Vec<u8>,
    pub datr: DataRate,
    pub freq: f64,
    pub lsnr: f32,
    pub modu: Modulation,
    pub rfch: u64,
    pub rssi: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rssis: Option<i32>,
    pub size: u64,
    pub stat: CRC,
    pub tmst: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
}

/*
Name   |  Type  | Function
:--------:|:------:|--------------------------------------------------------------
jver    | string | Version of the JSON rxpk frame format (always 2)
brd     | number | (unsigned integer) Radio ID (default 0)
aesk    | number | concentrator used for RX
delayed | bool   | true if the messsage has been delayed due to buffering
rsig    | object | array of object Received signal information, per antenna
time    | string | UTC time of pkt RX, us precision, ISO 8601 'compact' format
tmms    | number | GPS time of pkt RX, number of milliseconds since 06.Jan.1980
tmst    | number | Internal timestamp of "RX finished" event (32b unsigned)
freq    | number | RX central frequency in MHz (unsigned float, Hz precision)
chan    | number | Concentrator "IF" channel used for RX (unsigned integer)
rfch    | number | Concentrator "RF chain" used for RX (unsigned integer)
stat    | number | CRC status: 1 = OK, -1 = fail, 0 = no CRC
modu    | string | Modulation identifier "LORA" or "FSK"
datr    | string | LoRa datarate identifier (eg. SF12BW500)
datr    | number | FSK datarate (unsigned, in bits per second)
codr    | string | LoRa ECC coding rate identifier
size    | number | RF packet payload size in bytes (unsigned integer)
data    | string | Base64 encoded RF packet payload, padded
 */
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RxPkV2 {
    pub aesk: usize,
    pub brd: usize,
    #[serde(
        serialize_with = "serialize_codr",
        deserialize_with = "deserialize_codr"
    )]
    pub codr: Option<lora_modulation::CodingRate>,
    #[serde(with = "crate::packet::types::base64")]
    pub data: Vec<u8>,
    pub datr: DataRate,
    pub freq: f64,
    pub jver: usize,
    pub modu: String,
    pub rsig: Vec<RSig>,
    pub size: u64,
    pub stat: CRC,
    pub tmst: u32,
    pub delayed: Option<bool>,
    pub tmms: Option<u64>,
    pub time: Option<String>,
}

/*
   Name |  Ty
   pe  | Function
:------:|:------:|--------------------------------------------------------------
ant     | number | Antenna number on which signal has been received
chan    | number | (unsigned integer) Concentrator "IF" channel used for RX
rssic   | number | (signed integer) RSSI in dBm of the channel (1 dB precision)
rssis   | number | (signed integer) RSSI in dBm of the signal (1 dB precision)
rssisd  | number | (unsigned integer) Standard deviation of RSSI during preamble
lsnr    | number | (signed float) Lora SNR ratio in dB (0.1 dB precision)
etime   | string | Encrypted 'main' fine timestamp, ns precision [0..999999999]
foff    | number | Frequency offset in Hz [-125 kHz..+125 khz]
ftstat  | number | (8 bits unsigned integer) Fine timestamp status
ftver   | number | Version of the 'main' fine timestamp
ftdelta | number | Number of nanoseconds between the 'main' fts and the 'alternative' one
 */
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RSig {
    pub ant: usize,
    pub chan: u64,
    pub rssic: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rssis: Option<i32>,
    pub lsnr: f32,
    pub etime: Option<String>,
    pub foff: Option<i64>,
    pub ftstat: Option<u8>,
    pub ftver: Option<usize>,
    pub ftdelta: Option<isize>,
}
