/*
   This module wraps the UdpSocket objects such that a user can
   run sending and receiving concurrently as tasks,
   receive downlink packets and send uplink packets easily
*/
use crate::{
    pull_data, pull_resp, push_data, Down, MacAddress, Packet, ParseError, SerializablePacket, Up,
};
use std::sync::Arc;
use tokio::{
    net::{ToSocketAddrs, UdpSocket},
    sync::mpsc::{self, Receiver, Sender},
};

mod error;
pub use error::Error;
pub type Result<T = ()> = std::result::Result<T, Error>;

pub type RxMessage = Packet;
pub type TxMessage = Packet;

struct Rx {
    mac: MacAddress,
    udp_sender: mpsc::Sender<TxMessage>,
    client_sender: mpsc::Sender<Event>,
    socket_recv: Arc<UdpSocket>,
}

struct Tx {
    mac: MacAddress,
    receiver: Receiver<TxMessage>,
    client_sender: mpsc::Sender<Event>,
    socket_send: Arc<UdpSocket>,
}

pub struct UdpRuntime {
    rx: Rx,
    tx: Tx,
    poll_sender: Sender<TxMessage>,
}

pub type ClientRx = mpsc::Receiver<Event>;

#[derive(Debug)]
pub enum Event {
    Reconnected,
    LostConnection,
    DownlinkRequest(DownlinkRequest),
    UnableToParseUdpFrame(ParseError, Vec<u8>),
}

// A downlink request is sent to the client and contains the necessary
// information and channels to create the NACK or ACK
#[derive(Debug, Clone)]
pub struct DownlinkRequest {
    mac: MacAddress,
    pub pull_resp: pull_resp::Packet,
    udp_sender: Sender<TxMessage>,
}

impl DownlinkRequest {
    pub fn txpk(&self) -> &pull_resp::TxPk {
        &self.pull_resp.data.txpk
    }

    pub async fn ack(self) -> Result {
        let ack = self.pull_resp.into_ack_for_gateway(self.mac);
        Ok(self.udp_sender.send(ack.into()).await?)
    }
    pub async fn nack(self, error: super::tx_ack::Error) -> Result {
        let nack = self
            .pull_resp
            .into_nack_with_error_for_gateway(error, self.mac);
        Ok(self.udp_sender.send(nack.into()).await?)
    }
}

#[derive(Debug, Clone)]
pub struct ClientTx {
    udp_sender: mpsc::Sender<TxMessage>,
}

impl ClientTx {
    pub async fn send(&self, push_data: push_data::Packet) -> Result {
        Ok(self
            .udp_sender
            .send(Packet::Up(Up::PushData(push_data)))
            .await?)
    }
}

impl UdpRuntime {
    pub async fn new<H: ToSocketAddrs>(
        mac: MacAddress,
        host: H,
    ) -> Result<(ClientTx, ClientRx, UdpRuntime)> {
        let outbound_socket = std::net::SocketAddr::from(([0, 0, 0, 0], 0));
        Self::new_with_outbound_socket(outbound_socket, mac, host).await
    }

    pub async fn new_with_outbound_socket<L: ToSocketAddrs, H: ToSocketAddrs>(
        outbound_socket: L,
        mac: MacAddress,
        host: H,
    ) -> Result<(ClientTx, ClientRx, UdpRuntime)> {
        let socket = UdpSocket::bind(&outbound_socket)
            .await
            .map_err(|io_error| Error::Binding { io_error })?;
        // "connecting" filters for only frames from the server
        socket
            .connect(host)
            .await
            .map_err(|io_error| Error::Binding { io_error })?;

        let socket_recv = Arc::new(socket);
        let socket_send = socket_recv.clone();

        let (tx_sender, tx_receiver) = mpsc::channel(100);
        let (downlink_request_tx, downlink_request_rx) = mpsc::channel(100);

        let client_sender = ClientTx {
            udp_sender: tx_sender.clone(),
        };

        Ok((
            client_sender,
            downlink_request_rx,
            UdpRuntime {
                rx: Rx {
                    mac,
                    client_sender: downlink_request_tx.clone(),
                    udp_sender: tx_sender.clone(),
                    socket_recv,
                },
                poll_sender: tx_sender,
                tx: Tx {
                    mac,
                    client_sender: downlink_request_tx,
                    receiver: tx_receiver,
                    socket_send,
                },
            },
        ))
    }

    pub async fn run(self, shutdown_signal: triggered::Listener) -> Result {
        let (rx, tx, poll_sender) = (self.rx, self.tx, self.poll_sender);
        // udp_runtime_rx reads from the UDP port
        let udp_listener = tokio::spawn(rx.run());

        // udp_runtime_tx writes to the UDP port
        // by receiving packets from the sender channel
        let udp_writer = tokio::spawn(tx.run());

        let pull_req_sender = tokio::spawn(async move {
            loop {
                let packet = pull_data::Packet::new(rand::random());
                poll_sender.send(packet.into()).await?;
                sleep(Duration::from_millis(10000)).await;
            }
        });

        tokio::select!(
            _ = shutdown_signal => Ok(()),
            resp = udp_listener => resp?,
            resp = udp_writer => resp?,
            resp = pull_req_sender => resp?,
        )
    }
}

use std::time::Duration;
use tokio::time::sleep;

impl Rx {
    fn new_downlink_request(&self, pull_resp: pull_resp::Packet) -> DownlinkRequest {
        DownlinkRequest {
            pull_resp,
            mac: self.mac,
            udp_sender: self.udp_sender.clone(),
        }
    }

    pub async fn run(self) -> Result {
        let mut buf = vec![0u8; 1024];
        loop {
            match self.socket_recv.recv(&mut buf).await {
                Ok(n) => {
                    match Packet::parse_downlink(&buf[0..n]) {
                        Ok(down) => match down {
                            // pull_resp is a request to sent an RF packet
                            // we hand this off to the runtime client
                            Down::PullResp(pull_resp) => {
                                let downlink_request = self.new_downlink_request(*pull_resp);
                                self.client_sender
                                    .send(Event::DownlinkRequest(downlink_request))
                                    .await?;
                            }
                            // pull_ack just lets us know that the "connection is open"
                            // could potentially have a timer that waits for these on every
                            // pull_data frame
                            Down::PullAck(_) => (),
                            // push_ack is sent immediately after push_data (uplink, ie: RF packet received)
                            Down::PushAck(_) => (),
                        },
                        Err(e) => {
                            let mut vec = Vec::new();
                            vec.extend_from_slice(&buf[0..n]);

                            self.client_sender
                                .send(Event::UnableToParseUdpFrame(e, vec))
                                .await?;
                        }
                    }
                }
                Err(_) => {
                    // back off of CPU
                    sleep(Duration::from_millis(100)).await;
                }
            }
        }
    }
}

impl Tx {
    pub async fn run(mut self) -> Result {
        let mut buf = vec![0u8; 1024];
        let mut connected = true;
        loop {
            let tx = self.receiver.recv().await;
            if let Some(mut data) = tx {
                match &mut data {
                    Packet::Up(ref mut up) => {
                        up.set_gateway_mac(self.mac);
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

                match self.socket_send.send(&buf[..n]).await {
                    Ok(_) => {
                        if !connected {
                            connected = true;
                            self.client_sender.send(Event::Reconnected).await?;
                        }
                    }
                    Err(_) => {
                        if connected {
                            connected = false;
                            self.client_sender.send(Event::LostConnection).await?;
                        }
                    }
                }
            }
        }
    }
}
