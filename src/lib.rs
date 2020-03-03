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
use std::error::Error as stdError;
use mio::net::UdpSocket;
use mio::{Events, Poll, PollOpt, Ready, Token};
use std::time::Duration;
use num_enum::TryFromPrimitive;
use std::convert::TryFrom;
use std::fmt;
mod error;
pub use error::Error as Error;
use serde_json::Value;

mod util;
use util::{parse_u64_field, parse_f64_field,parse_string_field, parse_i64_field};
const SENDER: Token = Token(0);
const ECHOER: Token = Token(1);

const PROTOCOL_VERSION: u8 = 2;

type Result<T> = std::result::Result<T, Box<stdError>>;

 // 0      | protocol version = 2
 // 1-2    | random token
 // 3      | PULL_DATA identifier 0x02
 // 4-11   | Gateway unique identifier (MAC address)
#[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
#[repr(u8)]
enum Identifier {
    PushData = 0,
    PushAck = 1,
    PullData = 2,
    PullResp = 3,
    PullAck = 4,
}

struct MacAddress {
    bytes: [u8; 6],
}

impl MacAddress {
    pub fn new(b: &[u8; 6]) -> MacAddress{
        MacAddress {
            bytes: [
                b[0], b[1], b[2], b[3], b[4], b[5]
            ]
        }
    }
}

impl fmt::Display for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "MacAddress(");
        for i in 0..5 {
            write!(f, "{:02X}:", self.bytes[i]);
        }
        write!(f, "{:02X}", self.bytes[5]);
        write!(f, ")")

    }
}

#[derive(Debug)]
struct Packet {
    chan: u64,
    codr: String,
    data: String,
    datr: String,
    freq: f64,
    lsnr: f64,
    modu: String,
    rfch: u64,
    rssi: i64,
    size: u64,
    stat: u64,
    tmst: u64,
}


impl Packet {
    pub fn from_el(el: &serde_json::value::Value) -> Packet {
        Packet{
            chan: parse_u64_field(&el, "chan"),
            codr: parse_string_field(&el, "codr"),
            data: parse_string_field(&el, "data"),
            datr: parse_string_field(&el, "datr"),
            freq: parse_f64_field(&el, "freq"),
            lsnr: parse_f64_field(&el, "lsnr"),
            modu: parse_string_field(&el, "modu"), 
            rfch: parse_u64_field(&el, "rfch"),
            rssi: parse_i64_field(&el, "rssi"),
            size: parse_u64_field(&el, "size"),
            stat: parse_u64_field(&el, "stat"),
            tmst: parse_u64_field(&el, "tmst"),
        }
    }
}

#[derive(Debug)]
struct Stat {
    ackr: f64,
    dwnb: u64,
    rxfw: u64,
    rxnb: u64,
    rxok: u64,
    time: String,
    txnb: u64
}

impl Stat {
    pub fn from_map(map: &serde_json::value::Value) -> Stat {
        Stat{
            ackr: parse_f64_field(map, "ackr"),
            dwnb: parse_u64_field(map, "dwnb"),
            rxfw: parse_u64_field(map, "rxfw"),
            rxnb: parse_u64_field(map, "rxnb"),
            rxok: parse_u64_field(map, "rxok"),
            time: parse_string_field(map, "time"),
            txnb: parse_u64_field(map, "txnb"),
        }
    }
}

pub fn parse_gateway_rx(num_recv: usize, buffer: &mut [u8]) -> Result<()> {
    if(buffer[0] != PROTOCOL_VERSION) {
        return Err(Error::InvalidProtocolVersion.into());
    }

    let random_token: u16 = (buffer[1] as u16) << 8 | buffer[2] as u16;

    println!("Random token = {:x}", random_token);

    if let Ok(id) = Identifier::try_from(buffer[3]) {
        match id {
            Identifier::PullData => {
                let address = MacAddress::new(array_ref![buffer,4,6]);
                print!("PullData: ");
                println!("{:}", address);
            },
            Identifier::PushData => {
                print!("PushData: ");
                let address = MacAddress::new(array_ref![buffer,4,6]);
                if let Ok(json_str) = std::str::from_utf8(&buffer[12..num_recv]) {
                    let v: Value = serde_json::from_str(json_str)?;
                    println!("v = {:?}", v);
                    match &v["rxpk"] {
                        Value::Array(rxpk) => {
                            print!("rxpk: ");
                            for pkt in rxpk {
                                println!("{:?}", Packet::from_el(pkt));
                            }
                        },
                        _ => (),
                    };

                    match &v["stat"] {
                        Value::Object(stat) => {
                            let stat = Stat::from_map(&v["stat"]);
                            println!("{:?}", stat);
                            //
                        },
                        _ => (),
                    };
                }
            },
            Identifier::PullResp => {
                print!("PullResp: ");
                if let Ok(json_str) = std::str::from_utf8(&buffer[4..num_recv]) {
                    let v: Value = serde_json::from_str(json_str)?;
                    println!("{:?}", v);
                } else {
                    println!("PullResp:bad parsing!");
                }
            },
            Identifier::PullAck => {
                println!("PullAck");
                // assert not larger than 4
            },
            Identifier::PushAck => {
                println!("PushAck");
                // assert not larger than 4
            },
        }
   
    } else {
        return Err(Error::InvalidIdentifier.into());
    }
    Ok(())
}

pub fn run() -> Result<()> {


    //let sender_addr ="127.0.0.1:0".parse()?;
    //let mut sender_socket = UdpSocket::bind(&sender_addr)?;
    let echoer_addr ="0.0.0.0:1680".parse()?;
    let mut echoer_socket = UdpSocket::bind(&echoer_addr)?;
    // If we do not use connect here, SENDER and ECHOER would need to call send_to and recv_from
    // respectively.
    //sender_socket.connect(echoer_socket.local_addr()?)?;
    // We need a Poll to check if SENDER is ready to be written into, and if ECHOER is ready to be
    // read from.
    let mut poll = Poll::new()?;
    // We register our sockets here so that we can check if they are ready to be written/read.
    //poll.register(&mut sender_socket, SENDER, Ready::writable(), PollOpt::edge())?;
    poll.register(&mut echoer_socket, ECHOER, Ready::readable(), PollOpt::level())?;
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
                },
                // Our ECHOER is ready to be read from.
                ECHOER => {
                    let num_recv = echoer_socket.recv(&mut buffer)?;
                    parse_gateway_rx(num_recv, &mut buffer);
                    buffer = [0; 1024];
                }
                _ => unreachable!()
            }
        }
    }
    Ok(())
}
