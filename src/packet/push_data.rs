/*
### 3.2. PUSH_DATA packet ###

That packet type is used by the gateway mainly to forward the RF packets
received, and associated metadata, to the server.

Bytes  | Function
:------:|---------------------------------------------------------------------
0      | protocol version = 2
1-2    | random token
3      | PUSH_DATA identifier 0x00
4-11   | Gateway unique identifier (MAC address)
12-end | JSON object, starting with {, ending with }, see section 4
 */
use super::{
    push_ack, write_preamble, CodingRate, DataRate, Error as PktError, Identifier, MacAddress,
    Modulation, SerializablePacket,
};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::io::{Cursor, Write};

#[derive(Debug, Clone)]
pub struct Packet {
    pub random_token: u16,
    pub gateway_mac: MacAddress,
    pub data: Data,
}

impl Packet {
    pub fn from_rxpk(gateway_mac: MacAddress, rxpk: RxPk) -> Packet {
        let rxpk = vec![rxpk];
        Packet {
            random_token: 0,
            gateway_mac,
            data: Data {
                rxpk: Some(rxpk),
                stat: None,
            },
        }
    }

    pub fn from_stat(gateway_mac: MacAddress, stat: Stat) -> Packet {
        Packet {
            random_token: 0,
            gateway_mac,
            data: Data {
                rxpk: None,
                stat: Some(stat),
            },
        }
    }

    pub fn random() -> Packet {
        let rxpk = vec![RxPk::V1(RxPkV1 {
            chan: 0,
            codr: CodingRate::_4_5,
            data: vec![0, 0],
            datr: DataRate::default(),
            freq: 902.800_000,
            lsnr: -15.0,
            modu: Modulation::LORA,
            rfch: 0,
            rssi: -80,
            rssis: Some(-80),
            size: 12,
            stat: CRC::OK,
            tmst: 12,
            time: None,
        })];

        Packet {
            random_token: rand::random(),
            gateway_mac: MacAddress::from([0; 8]),
            data: Data {
                rxpk: Some(rxpk),
                stat: None,
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Data {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rxpk: Option<Vec<RxPk>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stat: Option<Stat>,
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
    pub codr: CodingRate,
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

#[derive(Debug, Serialize_repr, Deserialize_repr, Clone, PartialEq, Eq)]
#[repr(i8)]
pub enum CRC {
    Disabled = 0,
    OK = 1,
    Fail = -1,
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
    pub codr: CodingRate,
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

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum RxPk {
    V1(RxPkV1),
    V2(RxPkV2),
}

use std::fmt;
impl fmt::Display for RxPk {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "@{} us, {:.2} MHz, {:?}, {}, snr: {}, len: {}",
            self.get_timestamp(),
            self.get_frequency(),
            self.get_datarate(),
            if let Some(rssis) = self.get_signal_rssi() {
                format!("rssis: {rssis}")
            } else {
                format!("rssic: {}", self.get_channel_rssi())
            },
            self.get_snr(),
            self.get_data().len()
        )
    }
}

macro_rules! get_field {
    ($self:expr, $field:ident) => {
        match $self {
            RxPk::V1(pk) => &pk.$field,
            RxPk::V2(pk) => &pk.$field,
        }
    };
}
use std::cmp;

impl RxPk {
    pub fn get_snr(&self) -> f32 {
        match self {
            RxPk::V1(pk) => pk.lsnr,
            RxPk::V2(pk) => pk
                .rsig
                .iter()
                // truncate the decimal when choosing best LSNR value
                .fold(-150.0, |max, x| {
                    if (max as i32) < (x.lsnr as i32) {
                        x.lsnr
                    } else {
                        max
                    }
                }),
        }
    }

    pub fn get_channel_rssi(&self) -> i32 {
        match self {
            RxPk::V1(pk) => pk.rssi,
            RxPk::V2(pk) => pk.rsig.iter().fold(-150, |max, x| cmp::max(max, x.rssic)),
        }
    }

    pub fn get_signal_rssi(&self) -> Option<i32> {
        match self {
            RxPk::V1(pk) => pk.rssis,
            RxPk::V2(pk) => pk.rsig.iter().fold(None, |max, x| {
                if let Some(rssis) = x.rssis {
                    Some(if let Some(current_max) = max {
                        cmp::max(current_max, rssis)
                    } else {
                        rssis
                    })
                } else {
                    max
                }
            }),
        }
    }

    pub fn get_frequency(&self) -> &f64 {
        get_field!(self, freq)
    }

    pub fn get_data(&self) -> &Vec<u8> {
        get_field!(self, data)
    }

    pub fn get_timestamp(&self) -> &u32 {
        get_field!(self, tmst)
    }

    pub fn get_time(&self) -> &Option<String> {
        get_field!(self, time)
    }

    pub fn get_datarate(&self) -> DataRate {
        get_field!(self, datr).clone()
    }

    pub fn get_crc_status(&self) -> &CRC {
        get_field!(self, stat)
    }
}

/*
Name |  Type  | Function
:----:|:------:|--------------------------------------------------------------
time | string | UTC 'system' time of the gateway, ISO 8601 'expanded' format
lati | number | GPS latitude of the gateway in degree (float, N is +)
long | number | GPS latitude of the gateway in degree (float, E is +)
alti | number | GPS altitude of the gateway in meter RX (integer)
rxnb | number | Number of radio packets received (unsigned integer)
rxok | number | Number of radio packets received with a valid PHY CRC
rxfw | number | Number of radio packets forwarded (unsigned integer)
ackr | number | Percentage of upstream datagrams that were acknowledged
dwnb | number | Number of downlink datagrams received (unsigned integer)
txnb | number | Number of packets emitted (unsigned integer)
*/

// the order of this is important as it makes us identical to Semtech
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Stat {
    time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    lati: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    long: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    alti: Option<i64>,
    rxnb: u64,
    rxok: u64,
    rxfw: u64,
    // if there were no upstream datagrams, this field can be null
    ackr: Option<f64>,
    dwnb: u64,
    txnb: u64,
}

impl SerializablePacket for Packet {
    fn serialize(&self, buffer: &mut [u8]) -> std::result::Result<u64, PktError> {
        let mut w = Cursor::new(buffer);
        write_preamble(&mut w, self.random_token)?;
        w.write_all(&[Identifier::PushData as u8])?;
        w.write_all(self.gateway_mac.as_bytes())?;
        w.write_all(serde_json::to_string(&self.data)?.as_bytes())?;
        Ok(w.position())
    }
}

impl From<Packet> for super::Packet {
    fn from(packet: Packet) -> super::Packet {
        super::Packet::Up(super::Up::PushData(packet))
    }
}

impl Packet {
    pub fn into_ack(self) -> push_ack::Packet {
        push_ack::Packet {
            random_token: self.random_token,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn check_given_snr(data: Data, expected_snr: f32) {
        if let Some(mut rxpk) = data.rxpk {
            assert_eq!(rxpk.len(), 1);
            if let Some(rxpk) = rxpk.pop() {
                assert_eq!(rxpk.get_snr(), expected_snr)
            } else {
                // rxpk is empty vector
                assert!(false)
            }
        } else {
            // rxpk is None
            assert!(false)
        }
    }

    #[test]
    fn rxpk_positive_lsnr() {
        let json = "{\"rxpk\":[{\"aesk\":0,\"brd\":263,\"codr\":\"4/5\",\"data\":\"QC65rwEA4w8CaH7LyGf/3+dxzrXkkfEsRCcXbFM=\",\"datr\":\"SF12BW125\",\"freq\":868.5,\"jver\":2,\"modu\":\"LORA\",\"rsig\":[{\"ant\":0,\"chan\":7,\"lsnr\":7.8,\"rssic\":-103}],\"size\":29,\"stat\":1,\"time\":\"2022-03-31T07:51:15.709338Z\",\"tmst\":445296860}]}";
        let parsed: Data = serde_json::from_str(json).expect("Error parsing push_data::Data");
        check_given_snr(parsed, 7.8);
    }

    #[test]
    fn rxpk_negative_lsnr() {
        let json = "{\"rxpk\":[{\"aesk\":0,\"brd\":261,\"codr\":\"4/5\",\"data\":\"QI8cACQA6iAD3TTei0kPKKyxBA==\",\"datr\":\"SF11BW125\",\"freq\":868.1,\"jver\":2,\"modu\":\"LORA\",\"rsig\":[{\"ant\":0,\"chan\":5,\"lsnr\":-3.5,\"rssic\":-120}],\"size\":19,\"stat\":1,\"time\":\"2022-03-31T07:51:12.631018Z\",\"tmst\":442218540}]}";
        let parsed: Data = serde_json::from_str(json).expect("Error parsing push_data::Data");
        check_given_snr(parsed, -3.5);
    }
}
