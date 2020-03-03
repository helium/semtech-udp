/// A User Datagram Protocol socket.
///
/// This is an implementation of a bound UDP socket. This supports both IPv4 and
/// IPv6 addresses, and there is no corresponding notion of a server because UDP
/// is a datagram protocol.
///
/// # Examples
///
#[macro_use]
extern crate arrayref;
use mio::net::UdpSocket;
use mio::{Events, Poll, PollOpt, Ready, Token};
use std::convert::TryFrom;
use std::error::Error as stdError;
use std::time::Duration;
mod error;
pub use error::Error;
use serde_json::Value;
mod types;
use types::*;
mod util;

const SENDER: Token = Token(0);
const ECHOER: Token = Token(1);

const PROTOCOL_VERSION: u8 = 2;

type Result<T> = std::result::Result<T, Box<dyn stdError>>;

pub fn parse_gateway_rx(num_recv: usize, buffer: &mut [u8]) -> Result<()> {
    if buffer[0] != PROTOCOL_VERSION {
        return Err(Error::InvalidProtocolVersion.into());
    }

    let random_token: u16 = (buffer[1] as u16) << 8 | buffer[2] as u16;

    println!("Random token = {:x}", random_token);

    if let Ok(id) = Identifier::try_from(buffer[3]) {
        match id {
            Identifier::PullData => {
                let address = MacAddress::new(array_ref![buffer, 4, 6]);
                print!("PullData: ");
                println!("{:}", address);
            }
            Identifier::PushData => {
                print!("PushData: ");
                let address = MacAddress::new(array_ref![buffer, 4, 6]);
                println!("{:}", address);

                if let Ok(json_str) = std::str::from_utf8(&buffer[12..num_recv]) {
                    let v: Value = serde_json::from_str(json_str)?;
                    match &v["rxpk"] {
                        Value::Array(rxpk) => {
                            print!("rxpk: ");
                            for pkt in rxpk {
                                println!("\t{:?}", RxPk::from_value(pkt));
                            }
                        }
                        _ => (),
                    };

                    match &v["stat"] {
                        Value::Object(_) => {
                            let stat = Stat::from_value(&v["stat"]);
                            println!("{:?}", stat);
                            //
                        }
                        _ => (),
                    };
                }
            }
            Identifier::PullResp => {
                print!("PullResp: ");
                if let Ok(json_str) = std::str::from_utf8(&buffer[4..num_recv]) {
                    let v: Value = serde_json::from_str(json_str)?;
                    println!("{:?}", v);
                } else {
                    println!("PullResp:bad parsing!");
                }
            }
            Identifier::PullAck => {
                println!("PullAck");
                // assert not larger than 4
            }
            Identifier::PushAck => {
                println!("PushAck");
                // assert not larger than 4
            }
        }
    } else {
        return Err(Error::InvalidIdentifier.into());
    }
    Ok(())
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
                    parse_gateway_rx(num_recv, &mut buffer)?;
                    buffer = [0; 1024];
                }
                _ => unreachable!(),
            }
        }
    }
}
