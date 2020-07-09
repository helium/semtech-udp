/*
### 5.5. TX_ACK packet ###

That packet type is used by the gateway to send a feedback to the server
to inform if a downlink request has been accepted or rejected by the gateway.
The datagram may optionnaly contain a JSON string to give more details on
acknoledge. If no JSON is present (empty string), this means than no error
occured.

Bytes  | Function
:------:|---------------------------------------------------------------------
0      | protocol version = 2
1-2    | same token as the PULL_RESP packet to acknowledge
3      | TX_ACK identifier 0x05
4-11   | Gateway unique identifier (MAC address)
12-end | [optional] JSON object, starting with {, ending with }, see section 6

*/
use super::super::simple_up_packet;
use super::{write_preamble, Identifier, MacAddress, SerializablePacket};
use std::{
    error::Error,
    io::{Cursor, Write},
};

#[derive(Debug, Clone)]
pub struct Packet {
    pub random_token: u16,
    pub gateway_mac: MacAddress,
}

simple_up_packet!(Packet, Identifier::TxAck);

impl From<Packet> for super::Packet {
    fn from(packet: Packet) -> super::Packet {
        super::Packet::Up(super::Up::TxAck(packet))
    }
}
