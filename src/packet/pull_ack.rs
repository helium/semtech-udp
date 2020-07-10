/*

### 5.3. PULL_ACK packet ###

That packet type is used by the server to confirm that the network route is
open and that the server can send PULL_RESP packets at any time.

 Bytes  | Function
:------:|---------------------------------------------------------------------
 0      | protocol version = 2
 1-2    | same token as the PULL_DATA packet to acknowledge
 3      | PULL_ACK identifier 0x04

 */

use super::super::simple_down_packet;
use super::{write_preamble, Identifier, SerializablePacket};
use std::{
    error::Error,
    io::{Cursor, Write},
};

#[derive(Debug, Clone)]
pub struct Packet {
    pub random_token: u16,
}

simple_down_packet!(Packet, Identifier::PullAck);

impl From<Packet> for super::Packet {
    fn from(packet: Packet) -> super::Packet {
        super::Packet::Down(super::Down::PullAck(packet))
    }
}
