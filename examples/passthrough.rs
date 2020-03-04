use semtech_udp;
extern crate arrayref;
use mio::net::UdpSocket;
use mio::{Events, Poll, PollOpt, Ready, Token};
use std::error::Error;
use std::time::Duration;

const SENDER: Token = Token(0);
const ECHOER: Token = Token(1);

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn main() -> Result<()> {
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
                    print!("[");
                    for i in 0..num_recv {
                        print!("0x{:X}, ", buffer[i])
                    }
                    println!("]");
                    let msg = semtech_udp::Packet::parse(&mut buffer, num_recv)?;
                    println!("{:?}", msg);
                    buffer = [0; 1024];
                }
                _ => unreachable!(),
            }
        }
    }
}
