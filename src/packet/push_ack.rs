/*

### 3.3. PUSH_ACK packet ###

That packet type is used by the server to acknowledge immediately all the
PUSH_DATA packets received.

 Bytes  | Function
:------:|---------------------------------------------------------------------
 0      | protocol version = 2
 1-2    | same token as the PUSH_DATA packet to acknowledge
 3      | PUSH_ACK identifier 0x01

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

simple_down_packet!(Packet, Identifier::PushAck);

impl From<Packet> for super::Packet {
    fn from(packet: Packet) -> super::Packet {
        super::Packet::Down(super::Down::PushAck(packet))
    }
}
