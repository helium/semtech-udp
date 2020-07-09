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
use super::{write_preamble, Identifier, SerializablePacket, StringOrNum};
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    io::{Cursor, Write},
};

#[derive(Debug, Clone)]
pub struct Packet {
    pub random_token: u16,
    pub data: Data,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Data {
    pub txpk: TxPk,
}

impl Data {
    pub fn from_txpk(txpk: TxPk) -> Data {
        Data {
            txpk,
        }
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
    pub imme: bool,        // Send packet immediately (will ignore tmst & time)
    pub tmst: StringOrNum, // Send packet on a certain timestamp value (will ignore time)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tmms: Option<StringOrNum>, // Send packet at a certain GPS time (GPS synchronization required)
    pub freq: f64,    // TX central frequency in MHz (unsigned float, Hz precision)
    pub rfch: u64,    // Concentrator "RF chain" used for TX (unsigned integer)
    pub powe: u64,    // TX output power in dBm (unsigned integer, dBm precision)
    pub modu: String, // Modulation identifier "LORA" or "FSK"
    pub datr: String, // LoRa datarate identifier (eg. SF12BW500)
    pub codr: String, // LoRa ECC coding rate identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fdev: Option<u64>, //FSK frequency deviation (unsigned integer, in Hz)
    pub ipol: bool,   // Lora modulation polarization inversion
    pub prea: Option<u64>, // RF preamble size (unsigned integer)
    pub size: u64,    // RF packet payload size in bytes (unsigned integer)
    pub data: String, // Base64 encoded RF packet payload, padding optional
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ncrc: Option<bool>, // If true, disable the CRC of the physical layer (optional)
}

impl SerializablePacket for Packet {
    fn serialize(&self, buffer: &mut [u8]) -> std::result::Result<u64, Box<dyn Error>> {
        let mut w = Cursor::new(buffer);
        write_preamble(&mut w, self.random_token)?;
        w.write_all(&[Identifier::PullResp as u8])?;
        w.write_all(&serde_json::to_string(&self.data)?.as_bytes())?;
        Ok(w.position())
    }
}

impl From<Packet> for super::Packet {
    fn from(packet: Packet) -> super::Packet {
        super::Packet::Down(super::Down::PullResp(Box::new(packet)))
    }
}
