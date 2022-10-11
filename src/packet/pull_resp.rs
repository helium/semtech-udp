/*
### 5.4. PULL_RESP packet ###
That packet type is used by the server to send RF packets and associated
metadata that will have to be emitted by the gateway.

Bytes  | Function
:------:|---------------------------------------------------------------------
0      | protocol version = 2
1-2    | random token
3      | PULL_RESP identifier 0x03
4-end  | JSON object, starting with {, ending with }, see section 6
 */
use super::{
    tx_ack, write_preamble, CodingRate, DataRate, Error as PktError, Identifier, MacAddress,
    Modulation, SerializablePacket, StringOrNum,
};
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Write};

#[derive(Debug, Clone)]
pub struct Packet {
    pub random_token: u16,
    pub data: Data,
}

impl Packet {
    pub fn into_ack_for_gateway(self, gateway_mac: MacAddress) -> tx_ack::Packet {
        tx_ack::Packet {
            gateway_mac,
            random_token: self.random_token,
            data: tx_ack::Data::default(),
        }
    }

    pub fn into_nack_with_error_for_gateway(
        self,
        error: super::tx_ack::Error,
        gateway_mac: MacAddress,
    ) -> tx_ack::Packet {
        tx_ack::Packet {
            gateway_mac,
            random_token: self.random_token,
            data: super::tx_ack::Data::new_with_error(error),
        }
    }

    // sets a default Gateway value
    pub fn into_nack_with_error(self, e: super::tx_ack::Error) -> tx_ack::Packet {
        self.into_nack_with_error_for_gateway(e, MacAddress::from([0; 8]))
    }

    // sets a default Gateway value
    pub fn into_ack(self) -> tx_ack::Packet {
        self.into_ack_for_gateway(MacAddress::from([0; 8]))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Data {
    pub txpk: TxPk,
}

impl Data {
    pub fn from_txpk(txpk: TxPk) -> Data {
        Data { txpk }
    }
}

/*
 Name |  Type  | Function
:----:|:------:|--------------------------------------------------------------
 imme | bool   | Send packet immediately (will ignore tmst & time)
 tmst | number | Send packet on a certain timestamp value (will ignore time)
 tmms | number | Send packet at a certain GPS time (GPS synchronization required)
 freq | number | TX central frequency in MHz (unsigned float, Hz precision)
 rfch | number | Concentrator "RF chain" used for TX (unsigned integer)
 powe | number | TX output power in dBm (unsigned integer, dBm precision)
 modu | string | Modulation identifier "LORA" or "FSK"
 datr | string | LoRa datarate identifier (eg. SF12BW500)
 datr | number | FSK datarate (unsigned, in bits per second)
 codr | string | LoRa ECC coding rate identifier
 fdev | number | FSK frequency deviation (unsigned integer, in Hz)
 ipol | bool   | Lora modulation polarization inversion
 prea | number | RF preamble size (unsigned integer)
 size | number | RF packet payload size in bytes (unsigned integer)
 data | string | Base64 encoded RF packet payload, padding optional
 ncrc | bool   | If true, disable the CRC of the physical layer (optional)
 */

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TxPk {
    pub imme: bool, // Send packet immediately (will ignore tmst & time)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tmst: Option<StringOrNum>, // Send packet on a certain timestamp value (will ignore time)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tmms: Option<StringOrNum>, // Send packet at a certain GPS time (GPS synchronization required)
    pub freq: f64,        // TX central frequency in MHz (unsigned float, Hz precision)
    pub rfch: u64,        // Concentrator "RF chain" used for TX (unsigned integer)
    pub powe: u64,        // TX output power in dBm (unsigned integer, dBm precision)
    pub modu: Modulation, // Modulation identifier "LORA" or "FSK"
    pub datr: DataRate,   // LoRa datarate identifier (eg. SF12BW500)
    pub codr: CodingRate, // LoRa ECC coding rate identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fdev: Option<u64>, //FSK frequency deviation (unsigned integer, in Hz)
    pub ipol: bool,       // Lora modulation polarization inversion
    pub prea: Option<u64>, // RF preamble size (unsigned integer)
    #[serde(flatten)]
    pub data: PhyData,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ncrc: Option<bool>, // If true, disable the CRC of the physical layer (optional)
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PhyData {
    #[serde(with = "crate::packet::types::base64")]
    data: Vec<u8>,
    size: usize,
}

impl AsRef<[u8]> for PhyData {
    fn as_ref(&self) -> &[u8] {
        self.data.as_ref()
    }
}

impl PhyData {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            size: data.len(),
            data,
        }
    }
    pub fn set(&mut self, data: Vec<u8>) {
        self.size = data.len();
        self.data = data;
    }
}

impl TxPk {
    pub fn is_immediate(&self) -> bool {
        self.imme
    }

    pub fn get_tmst(&self) -> Option<u32> {
        if let Some(StringOrNum::N(tmst)) = self.tmst {
            Some(tmst)
        } else {
            None
        }
    }
}

use std::fmt;
impl fmt::Display for TxPk {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}, {:.2} MHz, {:?}, len: {}",
            if let Some(StringOrNum::N(time)) = self.tmst {
                format!("@{} us", time)
            } else {
                "immediately".into()
            },
            self.freq,
            self.datr,
            self.data.size
        )
    }
}

impl SerializablePacket for Packet {
    fn serialize(&self, buffer: &mut [u8]) -> std::result::Result<u64, PktError> {
        let mut w = Cursor::new(buffer);
        write_preamble(&mut w, self.random_token)?;
        w.write_all(&[Identifier::PullResp as u8])?;
        w.write_all(serde_json::to_string(&self.data)?.as_bytes())?;
        Ok(w.position())
    }
}

impl From<Packet> for super::Packet {
    fn from(packet: Packet) -> super::Packet {
        super::Packet::Down(super::Down::PullResp(Box::new(packet)))
    }
}

impl From<Box<Packet>> for super::Packet {
    fn from(packet: Box<Packet>) -> super::Packet {
        super::Packet::Down(super::Down::PullResp(packet))
    }
}
