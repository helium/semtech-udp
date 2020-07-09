/*
### 5.2. PULL_DATA packet ###

That packet type is used by the gateway to poll data from the server.

This data exchange is initialized by the gateway because it might be
impossible for the server to send packets to the gateway if the gateway is
behind a NAT.

When the gateway initialize the exchange, the network route towards the
server will open and will allow for packets to flow both directions.
The gateway must periodically send PULL_DATA packets to be sure the network
route stays open for the server to be used at any time.

 Bytes  | Function
:------:|---------------------------------------------------------------------
 0      | protocol version = 2
 1-2    | random token
 3      | PULL_DATA identifier 0x02
 4-11   | Gateway unique identifier (MAC address)
 */

use super::super::simple_up_packet;
use super::{pull_ack, write_preamble, Identifier, MacAddress, SerializablePacket};
use std::{
    error::Error,
    io::{Cursor, Write},
};
#[derive(Debug, Clone)]
pub struct Packet {
    pub random_token: u16,
    pub gateway_mac: MacAddress,
}

simple_up_packet!(Packet, Identifier::PullData);

impl From<Packet> for super::Packet {
    fn from(packet: Packet) -> super::Packet {
        super::Packet::Up(super::Up::PullData(packet))
    }
}

impl Packet {
    pub fn into_ack(self) -> pull_ack::Packet {
        pull_ack::Packet {
            random_token: self.random_token,
        }
    }
}
