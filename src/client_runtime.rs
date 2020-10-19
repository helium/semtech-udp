/*
   This module wraps the UdpSocket objects such that a user can
   run sending and receiving concurrently as tasks,
   receive downlink packets and send uplink packets easily
*/
use super::{parser::Parser, pull_data, Down, MacAddress, Packet, SerializablePacket, Up};
use std::{net::SocketAddr, sync::Arc};
use tokio::net::UdpSocket;
use tokio::sync::{
    broadcast,
    mpsc::{self, Receiver, Sender},
};

pub type RxMessage = Packet;
pub type TxMessage = Packet;

pub struct UdpRuntimeRx {
    sender: broadcast::Sender<RxMessage>,
    udp_sender: Sender<TxMessage>,
    socket_recv: Arc<UdpSocket>,
}

#[derive(Debug)]
pub enum Error {
    SemtechUdpSerialize(super::Error),
    SemtechUdpDeserialize(super::parser::Error),
    SendError(tokio::sync::mpsc::error::SendError<TxMessage>),
}

impl From<super::parser::Error> for Error {
    fn from(err: super::parser::Error) -> Error {
        Error::SemtechUdpDeserialize(err)
    }
}

impl From<tokio::sync::mpsc::error::SendError<TxMessage>> for Error {
    fn from(err: tokio::sync::mpsc::error::SendError<TxMessage>) -> Error {
        Error::SendError(err)
    }
}

impl From<super::Error> for Error {
    fn from(err: super::Error) -> Error {
        Error::SemtechUdpSerialize(err)
    }
}

pub struct UdpRuntimeTx {
    gateway_id: [u8; 8],
    receiver: Receiver<TxMessage>,
    sender: Sender<TxMessage>,
    socket_send: Arc<UdpSocket>,
}

pub struct UdpRuntime {
    rx: UdpRuntimeRx,
    tx: UdpRuntimeTx,
    poll_sender: Sender<TxMessage>,
}

impl UdpRuntime {
    pub fn split(self) -> (UdpRuntimeRx, UdpRuntimeTx, Sender<TxMessage>) {
        (self.rx, self.tx, self.poll_sender)
    }

    pub fn publish_to(&self) -> Sender<TxMessage> {
        self.tx.sender.clone()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<RxMessage> {
        self.rx.sender.subscribe()
    }

    pub async fn run(self) -> Result<(), Error> {
        let (rx, tx, poll_sender) = self.split();

        // udp_runtime_rx reads from the UDP port
        // and sends packets to the receiver channel
        tokio::spawn(async move {
            if let Err(e) = rx.run().await {
                panic!("UdpRuntimeRx threw error: {:?}", e)
            }
        });

        // udp_runtime_tx writes to the UDP port
        // by receiving packets from the sender channel
        tokio::spawn(async move {
            if let Err(e) = tx.run().await {
                panic!("UdpRuntimeTx threw error: {:?}", e)
            }
        });

        // spawn a timer for telling tx to send a PullReq frame
        tokio::spawn(async move {
            loop {
                let packet = pull_data::Packet::new(rand::random());
                if let Err(e) = poll_sender.send(packet.into()).await {
                    panic!("UdpRuntime error from sending PullData {}", e)
                }
                sleep(Duration::from_millis(10000)).await;
            }
        });

        Ok(())
    }

    pub async fn new(
        mac: [u8; 8],
        local: SocketAddr,
        host: SocketAddr,
    ) -> Result<UdpRuntime, Box<dyn std::error::Error>> {
        let socket = UdpSocket::bind(&local).await?;
        // "connecting" filters for only frames from the server
        socket.connect(host).await?;

        let socket_recv = Arc::new(socket);
        let socket_send= socket_recv.clone();


        let (rx_sender, _) = broadcast::channel(100);
        let (tx_sender, tx_receiver) = mpsc::channel(100);

        Ok(UdpRuntime {
            rx: UdpRuntimeRx {
                sender: rx_sender,
                udp_sender: tx_sender.clone(),
                socket_recv,
            },
            poll_sender: tx_sender.clone(),
            tx: UdpRuntimeTx {
                gateway_id: mac,
                receiver: tx_receiver,
                sender: tx_sender,
                socket_send,
            },
        })
    }
}

use std::time::Duration;
use tokio::time::sleep;

impl UdpRuntimeRx {
    pub async fn run(self) -> Result<(), Error> {
        let mut buf = vec![0u8; 1024];
        loop {
            match self.socket_recv.recv(&mut buf).await {
                Ok(n) => {
                    let packet = Packet::parse(&buf[0..n], n)?;
                    match packet {
                        Packet::Up(_) => panic!("Should not be receiving any up packets"),
                        Packet::Down(down) => match down.clone() {
                            Down::PullResp(pull_resp) => {
                                // send downlinks to LoRaWAN layer
                                self.sender.send(pull_resp.clone().into()).unwrap();
                                // provide ACK
                                self.udp_sender.send(pull_resp.into_ack().into()).await?;
                            }
                            Down::PullAck(_) | Down::PushAck(_) => {
                                // send downlinks to LoRaWAN layer
                                self.sender.send(Packet::Down(down.clone())).unwrap();
                            }
                        },
                    }
                }
                Err(e) => {
                    println!("Socket receive error: {}", e);
                }
            }
        }
    }
}

impl UdpRuntimeTx {
    pub async fn run(mut self) -> Result<(), Error> {
        let mut buf = vec![0u8; 1024];
        loop {
            let tx = self.receiver.recv().await;
            if let Some(mut data) = tx {
                match &mut data {
                    Packet::Up(ref mut up) => {
                        up.set_gateway_mac(MacAddress::new(&self.gateway_id));

                        match up {
                            Up::PushData(ref mut push_data) => {
                                push_data.random_token = rand::random()
                            }
                            Up::PullData(ref mut pull_data) => {
                                pull_data.random_token = rand::random()
                            }
                            Up::TxAck(_) => (),
                        }
                    }
                    Packet::Down(_) => panic!("Should not be sending any down packets"),
                }

                let n = data.serialize(&mut buf)? as usize;

                if let Err(e) = self.socket_send.send(&buf[..n]).await {
                    println!("Socket error: {}", e);
                }
            }
        }
    }
}
