#[macro_use]
extern crate arrayref;
use mio::net::UdpSocket;
use mio::{Events, Poll, PollOpt, Ready, Token};
use std::convert::TryFrom;
use std::error::Error as stdError;
use std::time::Duration;
mod error;
pub use error::Error;
mod types;
use types::*;
mod util;

const SENDER: Token = Token(0);
const ECHOER: Token = Token(1);

const PROTOCOL_VERSION: u8 = 2;

type Result<T> = std::result::Result<T, Box<dyn stdError>>;

pub fn parse_gateway_rx(buffer: &mut [u8], num_recv: usize) -> Result<Packet> {
    if buffer[0] != PROTOCOL_VERSION {
        return Err(Error::InvalidProtocolVersion.into());
    }
    if let Ok(id) = Identifier::try_from(buffer[3]) {
        Ok(match id {
            Identifier::PullData => Packet::PullData(PullData::new(&buffer, num_recv)?),
            Identifier::PushData => Packet::PushData(PushData::new(&buffer, num_recv)?),
            Identifier::PullResp => Packet::PullResp(PullResp::new(&buffer, num_recv)?),
            Identifier::PullAck => Packet::PullAck(PullAck::new(&buffer, num_recv)?),
            Identifier::PushAck => Packet::PushAck(PushAck::new(&buffer, num_recv)?),
        })
    } else {
        Err(Error::InvalidIdentifier.into())
    }
}

pub fn run() -> Result<()> {
    //let sender_addr ="127.0.0.1:0".parse()?;
    //let mut sender_socket = UdpSocket::bind(&sender_addr)?;
    let echoer_addr = "0.0.0.0:1680".parse()?;
    let mut echoer_socket = UdpSocket::bind(&echoer_addr)?;
    // If we do not use connect here, SENDER and ECHOER would need to call send_to and recv_from
    // respectively.
    //sender_socket.connect(echoer_socket.local_addr()?)?;
    // We need a Poll to check if SENDER is ready to be written into, and if ECHOER is ready to be
    // read from.
    let poll = Poll::new()?;
    // We register our sockets here so that we can check if they are ready to be written/read.
    //poll.register(&mut sender_socket, SENDER, Ready::writable(), PollOpt::edge())?;
    poll.register(
        &mut echoer_socket,
        ECHOER,
        Ready::readable(),
        PollOpt::level(),
    )?;
    //let msg_to_send = [9; 9];
    let mut buffer = [0; 1024];
    let mut events = Events::with_capacity(128);
    loop {
        poll.poll(&mut events, Some(Duration::from_millis(100)))?;
        for event in events.iter() {
            match event.token() {
                // Our SENDER is ready to be written into.
                SENDER => {
                    // let bytes_sent = sender_socket.send(&msg_to_send)?;
                    // assert_eq!(bytes_sent, 9);
                    // println!("sent {:?} -> {:?} bytes", msg_to_send, bytes_sent);
                }
                // Our ECHOER is ready to be read from.
                ECHOER => {
                    let num_recv = echoer_socket.recv(&mut buffer)?;
                    parse_gateway_rx(&mut buffer, num_recv)?;
                    buffer = [0; 1024];
                }
                _ => unreachable!(),
            }
        }
    }
}
