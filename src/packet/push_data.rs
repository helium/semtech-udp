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
    pub fn from_rxpk(rxpk: RxPk) -> Packet {
        let rxpk = vec![rxpk];
        Packet {
            random_token: 0,
            gateway_mac: MacAddress { bytes: [0; 8] },
            data: Data {
                rxpk: Some(rxpk),
                stat: None,
            },
        }
    }

    pub fn random() -> Packet {
        let rxpk = vec![RxPk::V1(RxPkV1 {
            chan: 0,
            codr: CodingRate::_4_5,
            data: "AA=".to_string(),
            datr: DataRate::default(),
            freq: 902.800_000,
            lsnr: -15.0,
            modu: Modulation::LORA,
            rfch: 0,
            rssi: -80,
            size: 12,
            stat: CRC::OK,
            tmst: 12,
        })];

        Packet {
            random_token: rand::random(),
            gateway_mac: MacAddress { bytes: [0; 8] },
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
    pub data: String,
    pub datr: DataRate,
    pub freq: f64,
    pub lsnr: f32,
    pub modu: Modulation,
    pub rfch: u64,
    pub rssi: i32,
    pub size: u64,
    pub stat: CRC,
    pub tmst: u64,
}

#[derive(Debug, Serialize_repr, Deserialize_repr, Clone)]
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
    pub data: String,
    pub datr: DataRate,
    pub freq: f64,
    pub jver: usize,
    pub modu: String,
    pub rsig: Vec<RSig>,
    pub size: u64,
    pub stat: CRC,
    pub tmst: u64,
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
    pub rssis: Option<i32>,
    pub lsnr: f32,
    pub etime: Option<String>,
    pub foff: Option<i64>,
    pub ftstat: Option<u8>,
    pub ftver: Option<usize>,
    pub ftdelta: Option<isize>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum RxPk {
    V1(RxPkV1),
    V2(RxPkV2),
}

macro_rules! get_field {
    ($self:expr, $field:ident) => {
        match $self {
            RxPk::V1(pk) => &pk.$field,
            RxPk::V2(pk) => &pk.$field,
        }
    };
}
impl RxPk {
    pub fn get_snr(&self) -> f32 {
        match self {
            RxPk::V1(pk) => pk.lsnr,
            RxPk::V2(pk) => pk.rsig[0].lsnr,
        }
    }

    pub fn get_rssi(&self) -> i32 {
        match self {
            RxPk::V1(pk) => pk.rssi,
            RxPk::V2(pk) => {
                // erlang implementation spec packet_rssi(map()) -> number()
                // takes rssic so we will too
                pk.rsig[0].rssic
            }
        }
    }

    pub fn get_frequency(&self) -> &f64 {
        get_field!(self, freq)
    }

    pub fn get_data(&self) -> String {
        get_field!(self, data).clone()
    }

    pub fn get_timestamp(&self) -> &u64 {
        get_field!(self, tmst)
    }

    pub fn get_datarate(&self) -> DataRate {
        get_field!(self, datr).clone()
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
    alti: Option<u64>,
    rxnb: u64,
    rxok: u64,
    rxfw: u64,
    ackr: f64,
    dwnb: u64,
    txnb: u64,
}

impl SerializablePacket for Packet {
    fn serialize(&self, buffer: &mut [u8]) -> std::result::Result<u64, PktError> {
        let mut w = Cursor::new(buffer);
        write_preamble(&mut w, self.random_token)?;
        w.write_all(&[Identifier::PushData as u8])?;
        w.write_all(&self.gateway_mac.bytes())?;
        w.write_all(&serde_json::to_string(&self.data)?.as_bytes())?;
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
