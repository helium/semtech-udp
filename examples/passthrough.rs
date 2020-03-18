use semtech_udp;
extern crate arrayref;
use mio::net::UdpSocket;
use mio::{Events, Poll, PollOpt, Ready, Token};
use std::error::Error;
use std::time::Duration;

const CLIENT: Token = Token(0);
const RADIO: Token = Token(1);

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn main() -> Result<()> {
    let client_server ="192.168.2.68:1680".parse()?;
    let mut client_socket = UdpSocket::bind(&"0.0.0.0:58058".parse()?)?;
    let radio_server = "0.0.0.0:1680".parse()?;
    let mut radio_socket = UdpSocket::bind(&radio_server)?;

    client_socket.connect(client_server)?;

    let poll = Poll::new()?;
    poll.register(
        &mut client_socket,
        CLIENT,
        Ready::readable(),
        PollOpt::level(),
    )?;
    poll.register(
        &mut radio_socket,
        RADIO,
        Ready::readable(),
        PollOpt::level(),
    )?;
    let mut buffer = [0; 1024];
    let mut events = Events::with_capacity(128);
    let mut radio_client = None;
    loop {
        poll.poll(&mut events, Some(Duration::from_millis(100)))?;
        for event in events.iter() {
            match event.token() {
                CLIENT => {
                    let num_recv = client_socket.recv(&mut buffer)?;
                    // forward the packet along
                    if let Some(radio_client) = &radio_client {
                        radio_socket.send_to(&buffer[0..num_recv], &radio_client)?;
                    }
                    let msg = semtech_udp::Packet::parse(&mut buffer, num_recv)?;
                    buffer = [0; 1024];
                    println!("From client {:?}", msg)
                }
                RADIO => {
                    let (num_recv, src) = radio_socket.recv_from(&mut buffer)?;
                    radio_client = Some(src);
                    client_socket.send(&buffer[0..num_recv])?;
                    let msg = semtech_udp::Packet::parse(&mut buffer, num_recv)?;
                    buffer = [0; 1024];
                    println!("From radio {:?}", msg)

                }
                _ => unreachable!(),
            }
        }
    }
}
