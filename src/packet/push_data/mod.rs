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
mod rxpk;
pub use rxpk::*;

use super::{
    push_ack, types, write_preamble, Error as PktError, Identifier, MacAddress, SerializablePacket,
};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::io::{Cursor, Write};
use types::{DataRate, Modulation};

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
            codr: Some(lora_modulation::CodingRate::_4_5),
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

#[derive(Debug, Serialize_repr, Deserialize_repr, Copy, Clone, PartialEq, Eq)]
#[repr(i8)]
pub enum CRC {
    Disabled = 0,
    OK = 1,
    Fail = -1,
}

use std::fmt;
impl fmt::Display for RxPk {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "@{} us, {:.2} MHz, {:?}, {}, snr: {}, len: {}",
            self.timestamp(),
            self.frequency(),
            self.datarate(),
            if let Some(rssis) = self.signal_rssi() {
                format!("rssis: {rssis}")
            } else {
                format!("rssic: {}", self.channel_rssi())
            },
            self.snr(),
            self.data().len()
        )
    }
}

macro_rules! get_field_ref {
    ($self:expr, $field:ident) => {
        match $self {
            RxPk::V1(pk) => &pk.$field,
            RxPk::V2(pk) => &pk.$field,
        }
    };
}
macro_rules! get_field {
    ($self:expr, $field:ident) => {
        match $self {
            RxPk::V1(pk) => pk.$field,
            RxPk::V2(pk) => pk.$field,
        }
    };
}
use std::cmp;

impl RxPk {
    pub fn snr(&self) -> f32 {
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

    pub fn channel_rssi(&self) -> i32 {
        match self {
            RxPk::V1(pk) => pk.rssi,
            RxPk::V2(pk) => pk.rsig.iter().fold(-150, |max, x| cmp::max(max, x.rssic)),
        }
    }

    pub fn signal_rssi(&self) -> Option<i32> {
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

    pub fn frequency(&self) -> f64 {
        get_field!(self, freq)
    }

    pub fn data(&self) -> &Vec<u8> {
        get_field_ref!(self, data)
    }

    pub fn timestamp(&self) -> u32 {
        get_field!(self, tmst)
    }

    pub fn time(&self) -> &Option<String> {
        get_field_ref!(self, time)
    }

    pub fn datarate(&self) -> DataRate {
        get_field!(self, datr)
    }

    pub fn crc_status(&self) -> CRC {
        get_field!(self, stat)
    }

    pub fn coding_rate(&self) -> Option<lora_modulation::CodingRate> {
        get_field!(self, codr)
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
temp | number | Current temperature in degree celcius (float)
*/

// the order of this is important as it makes us identical to Semtech
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Stat {
    pub time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lati: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alti: Option<i64>,
    pub rxnb: u64,
    pub rxok: u64,
    pub rxfw: u64,
    // if there were no upstream datagrams, this field can be null
    pub ackr: Option<f64>,
    pub dwnb: u64,
    pub txnb: u64,
    pub temp: Option<f64>,
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
                assert_eq!(rxpk.snr(), expected_snr)
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

    #[test]
    fn snr_roundtrip() {
        let json = "{\"rxpk\":[{\"jver\":1,\"tmst\":682631918,\"chan\":0,\"rfch\":0,\"freq\":865.062500,\"mid\": 0,\"stat\":1,\"modu\":\"LORA\",\"datr\":\"SF12BW125\",\"codr\":\"4/5\",\"rssis\":-95,\"lsnr\":6.8,\"foff\":-1300,\"rssi\":-94,\"size\":20,\"data\":\"QNbPNwABAQANyqD8ngiq26Hk4gs=\"}]}";
        let parsed: Data = serde_json::from_str(json).expect("Error parsing push_data::Data");
        check_given_snr(parsed.clone(), 6.8);
        let serialized = serde_json::to_string(&parsed).expect("Error serializing push_data::Data");
        let reparsed: Data =
            serde_json::from_str(&serialized).expect("Error parsing push_data::Data");
        check_given_snr(reparsed, 6.8);
    }
}
