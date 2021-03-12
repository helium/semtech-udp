use super::{
    parser::Parser, pull_resp, pull_resp::TxPk, tx_ack::Packet as TxAck, MacAddress, Packet,
    SerializablePacket, Up,
};
use std::sync::Arc;
use std::{collections::HashMap, net::SocketAddr, time::Duration};
use tokio::{
    net::UdpSocket,
    sync::{mpsc, oneshot},
    time::timeout,
};

pub use crate::push_data::RxPk;

#[derive(Debug)]
enum InternalEvent {
    Downlink((pull_resp::Packet, MacAddress, oneshot::Sender<TxAck>)),
    RawPacket(Up),
    PacketBySocket((Packet, SocketAddr)),
    Client((MacAddress, SocketAddr)),
    PacketReceived(RxPk, MacAddress),
    UnableToParseUdpFrame(Vec<u8>),
    AckReceived(TxAck),
}

#[derive(Debug, Clone)]
pub enum Event {
    PacketReceived(RxPk, MacAddress),
    RawPacket(Up),
    NewClient((MacAddress, SocketAddr)),
    UpdateClient((MacAddress, SocketAddr)),
    UnableToParseUdpFrame(Vec<u8>),
    NoClientWithMac(Box<pull_resp::Packet>, MacAddress),
}

// receives requests from clients
// dispatches them to UdpTx
#[derive(Debug, Clone)]
pub struct ClientTx {
    sender: mpsc::Sender<InternalEvent>,
    // you need to subscribe to the send channel
    receiver_copier: mpsc::Sender<Event>,
}

// sends packets to clients
#[derive(Debug)]
pub struct ClientRx {
    receiver: mpsc::Receiver<Event>,
}

// receives and parses UDP packets
struct UdpRx {
    socket_receiver: Arc<UdpSocket>,
    internal_sender: mpsc::Sender<InternalEvent>,
}

// processes Internal Events and Transmit over UDP
struct Internal {
    receiver: mpsc::Receiver<InternalEvent>,
    client_tx_sender: mpsc::Sender<Event>,
    clients: HashMap<MacAddress, SocketAddr>,
    downlink_senders: HashMap<u16, oneshot::Sender<TxAck>>,
    socket_sender: Arc<UdpSocket>,
}

#[derive(Debug)]
pub struct UdpRuntime {
    rx: ClientRx,
    tx: ClientTx,
}
use rand::Rng;

#[derive(Debug)]
pub enum Error {
    AckChannelRecv(mpsc::error::RecvError),
    AckError(super::packet::tx_ack::Error),
    SendTimeout,
    DispatchWithNoSendPacket,
    UnknownMac,
    UdpError(std::io::Error),
    ClientEventQueueFull(Box<mpsc::error::SendError<Event>>),
    InternalQueueClosedOrFull,
    SemtechUdpSerialization(super::packet::Error),
    AckRecvError,
    ErrorSendingAck,
}

#[derive(Clone)]
pub struct Downlink {
    random_token: u16,
    mac: MacAddress,
    packet: Option<pull_resp::Packet>,
    sender: mpsc::Sender<InternalEvent>,
}

impl Downlink {
    pub fn set_packet(&mut self, txpk: TxPk) {
        self.packet = Some(pull_resp::Packet {
            random_token: self.random_token,
            data: pull_resp::Data::from_txpk(txpk),
        });
    }

    async fn just_dispatch(self) -> Result<(), Error> {
        if let Some(packet) = self.packet {
            let (sender, receiver) = oneshot::channel();

            self.sender
                .send(InternalEvent::Downlink((packet, self.mac, sender)))
                .await?;

            // wait for the ACK for the protocol layer
            let ack = receiver.await?;
            if let Some(error) = ack.get_error() {
                Err(error.into())
            } else {
                Ok(())
            }
        } else {
            Err(Error::DispatchWithNoSendPacket)
        }
    }

    pub async fn dispatch(self, timeout_duration: Option<Duration>) -> Result<(), Error> {
        if let Some(duration) = timeout_duration {
            timeout(duration, self.just_dispatch()).await?
        } else {
            self.just_dispatch().await
        }
    }
}

impl ClientRx {
    pub async fn recv(&mut self) -> Event {
        // we unwrap here because the send channel is dropped only iff ClientRx is dropped
        // ClientRx panics before it can get dropped (see UdpRuntime
        self.receiver.recv().await.unwrap()
    }
}

impl ClientTx {
    pub async fn send(
        &mut self,
        txpk: TxPk,
        mac: MacAddress,
        timeout: Option<Duration>,
    ) -> Result<(), Error> {
        let prepared_send = self.prepare_downlink(Some(txpk), mac);
        prepared_send.dispatch(timeout).await
    }

    pub fn prepare_downlink(&mut self, txpk: Option<TxPk>, mac: MacAddress) -> Downlink {
        // assign random token
        let random_token = rand::thread_rng().gen();

        let packet = if let Some(txpk) = txpk {
            // create pull_resp frame with the data
            Some(pull_resp::Packet {
                random_token,
                data: pull_resp::Data::from_txpk(txpk),
            })
        } else {
            None
        };

        Downlink {
            random_token,
            mac,
            packet,
            sender: self.get_sender(),
        }
    }

    fn get_sender(&mut self) -> mpsc::Sender<InternalEvent> {
        self.sender.clone()
    }
}

impl UdpRuntime {
    pub fn split(self) -> (ClientRx, ClientTx) {
        (self.rx, self.tx)
    }

    pub async fn send(
        &mut self,
        txpk: TxPk,
        mac: MacAddress,
        timeout: Option<Duration>,
    ) -> Result<(), Error> {
        self.tx.send(txpk, mac, timeout).await
    }

    pub fn prepare_empty_downlink(&mut self, mac: MacAddress) -> Downlink {
        self.tx.prepare_downlink(None, mac)
    }

    pub fn prepare_downlink(&mut self, txpk: TxPk, mac: MacAddress) -> Downlink {
        self.tx.prepare_downlink(Some(txpk), mac)
    }

    pub async fn recv(&mut self) -> Event {
        self.rx.recv().await
    }

    pub async fn new(addr: SocketAddr) -> Result<UdpRuntime, Error> {
        let socket = UdpSocket::bind(&addr).await?;
        let socket_receiver = Arc::new(socket);
        let socket_sender = socket_receiver.clone();

        let (udp_tx_sender, udp_tx_receiver) = mpsc::channel(100);
        let (client_tx_sender, client_tx_receiver) = mpsc::channel(100);

        let client_tx = ClientTx {
            sender: udp_tx_sender.clone(),
            receiver_copier: client_tx_sender.clone(),
        };

        let client_rx = ClientRx {
            receiver: client_tx_receiver,
        };

        let udp_rx = UdpRx {
            socket_receiver,
            internal_sender: udp_tx_sender,
        };

        let udp_tx = Internal {
            receiver: udp_tx_receiver,
            client_tx_sender,
            clients: HashMap::new(),
            downlink_senders: HashMap::new(),
            socket_sender,
        };

        // udp_rx reads from the UDP port
        // and sends packets to relevant parties
        tokio::spawn(async move {
            if let Err(e) = udp_rx.run().await {
                // we panic here because the ony error case here
                // if we lost the local socket somehow
                panic!("UdpRx threw error: {:?}", e)
            }
        });

        // udp_tx writes to the UDP port and maintains
        // gateway to IP map
        tokio::spawn(async move {
            if let Err(e) = udp_tx.run().await {
                // we panic here because the ony error case here
                // if we lost the local socket somehow
                panic!("UdpTx threw error: {:?}", e)
            }
        });

        Ok(UdpRuntime {
            rx: client_rx,
            tx: client_tx,
        })
    }
}

impl UdpRx {
    pub async fn run(self) -> Result<(), Error> {
        let mut buf = vec![0u8; 1024];
        loop {
            match self.socket_receiver.recv_from(&mut buf).await {
                Err(e) => return Err(e.into()),
                Ok((n, src)) => {
                    let packet = if let Ok(packet) = Packet::parse(&buf[0..n]) {
                        Some(packet)
                    } else {
                        let mut vec = Vec::new();
                        vec.extend_from_slice(&buf[0..n]);
                        self.internal_sender
                            .send(InternalEvent::UnableToParseUdpFrame(vec))
                            .await?;
                        None
                    };

                    if let Some(packet) = packet {
                        match packet {
                            Packet::Up(packet) => {
                                // echo all packets to client
                                self.internal_sender
                                    .send(InternalEvent::RawPacket(packet.clone()))
                                    .await?;

                                match packet {
                                    Up::PullData(pull_data) => {
                                        let mac = pull_data.gateway_mac;
                                        // first send (mac, addr) to update map owned by UdpRuntimeTx
                                        let client = (mac, src);
                                        self.internal_sender
                                            .send(InternalEvent::Client(client))
                                            .await?;

                                        // send the ack_packet
                                        let ack_packet = pull_data.into_ack();
                                        self.internal_sender
                                            .send(InternalEvent::PacketBySocket((
                                                ack_packet.into(),
                                                src,
                                            )))
                                            .await?
                                    }
                                    Up::TxAck(txack) => {
                                        self.internal_sender
                                            .send(InternalEvent::AckReceived(txack))
                                            .await?;
                                    }
                                    Up::PushData(push_data) => {
                                        // Send all received packets as RxPk Events
                                        if let Some(rxpk) = &push_data.data.rxpk {
                                            for packet in rxpk {
                                                self.internal_sender
                                                    .send(InternalEvent::PacketReceived(
                                                        packet.clone(),
                                                        push_data.gateway_mac,
                                                    ))
                                                    .await?;
                                            }
                                        }

                                        let socket_addr = src;
                                        // send the ack_packet
                                        let ack_packet = push_data.into_ack();
                                        self.internal_sender
                                            .send(InternalEvent::PacketBySocket((
                                                ack_packet.into(),
                                                socket_addr,
                                            )))
                                            .await?;
                                    }
                                }
                            }
                            Packet::Down(_) => {
                                panic!("Should not receive this frame from forwarder")
                            }
                        };
                    }
                }
            }
        }
    }
}

impl Internal {
    pub async fn run(mut self) -> Result<(), Error> {
        let mut buf = vec![0u8; 1024];
        loop {
            let msg = self.receiver.recv().await;
            if let Some(msg) = msg {
                match msg {
                    InternalEvent::RawPacket(up) => {
                        self.client_tx_sender.send(Event::RawPacket(up)).await?;
                    }
                    InternalEvent::UnableToParseUdpFrame(frame) => {
                        self.client_tx_sender
                            .send(Event::UnableToParseUdpFrame(frame))
                            .await?;
                    }
                    InternalEvent::PacketReceived(rxpk, mac) => {
                        self.client_tx_sender
                            .send(Event::PacketReceived(rxpk, mac))
                            .await?;
                    }
                    InternalEvent::Downlink((packet, mac, ack_sender)) => {
                        let mut no_client = true;

                        if let Some(addr) = self.clients.get(&mac) {
                            let n = packet.serialize(&mut buf)? as usize;
                            // We receive an error here if we are trying to send the packet to a
                            // client that is no longer connected to us. Delete the client from map
                            if self.socket_sender.send_to(&buf[..n], addr).await.is_err() {
                                self.clients.remove(&mac);
                            } else {
                                // store token and one-shot channel
                                self.downlink_senders
                                    .insert(packet.random_token, ack_sender);
                                no_client = false;
                            }
                        }
                        if no_client {
                            self.client_tx_sender
                                .send(Event::NoClientWithMac(packet.into(), mac))
                                .await?;
                        }
                    }
                    InternalEvent::AckReceived(txack) => {
                        if let Some(sender) = self.downlink_senders.remove(&txack.random_token) {
                            sender.send(txack).map_err(|_| Error::ErrorSendingAck)?;
                        } else {
                            eprintln!("ACK received for unknown random_token")
                        }
                    }
                    InternalEvent::PacketBySocket((packet, addr)) => {
                        let n = packet.serialize(&mut buf)? as usize;
                        // only ACKs are sent via PacketBySocket
                        // so this will be an error only if we have somehow lost UDP connection
                        // between receiving a packet and sending the ACK
                        let _ = self.socket_sender.send_to(&buf[..n], &addr).await;
                    }
                    InternalEvent::Client((mac, addr)) => {
                        // tell user if same MAC has new IP
                        if let Some(existing_addr) = self.clients.get(&mac) {
                            if *existing_addr != addr {
                                self.clients.insert(mac, addr);
                                self.client_tx_sender
                                    .send(Event::UpdateClient((mac, addr)))
                                    .await?;
                            }
                        }
                        // simply insert if no entry exists
                        else {
                            self.clients.insert(mac, addr);
                            self.client_tx_sender
                                .send(Event::NewClient((mac, addr)))
                                .await?;
                        }
                    }
                }
            }
        }
    }
}

impl From<tokio::time::error::Elapsed> for Error {
    fn from(_err: tokio::time::error::Elapsed) -> Error {
        Error::SendTimeout
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::UdpError(err)
    }
}

impl From<mpsc::error::SendError<Event>> for Error {
    fn from(err: mpsc::error::SendError<Event>) -> Error {
        Error::ClientEventQueueFull(err.into())
    }
}

impl From<mpsc::error::SendError<InternalEvent>> for Error {
    fn from(_err: mpsc::error::SendError<InternalEvent>) -> Error {
        Error::InternalQueueClosedOrFull
    }
}

impl From<super::packet::Error> for Error {
    fn from(err: super::packet::Error) -> Error {
        Error::SemtechUdpSerialization(err)
    }
}

impl From<mpsc::error::RecvError> for Error {
    fn from(e: mpsc::error::RecvError) -> Self {
        Error::AckChannelRecv(e)
    }
}

impl From<super::packet::tx_ack::Error> for Error {
    fn from(e: super::packet::tx_ack::Error) -> Self {
        Error::AckError(e)
    }
}

impl From<oneshot::error::RecvError> for Error {
    fn from(_: oneshot::error::RecvError) -> Self {
        Error::AckRecvError
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let msg = match self {
            Error::AckChannelRecv(_) => "AckChannelRecv".to_string(),
            Error::AckError(error) => format!("AckError on transmit {}", error),
            Error::UnknownMac => "UnknownMac on on transmit".to_string(),
            Error::UdpError(error) => format!("UdpError: {}", error),
            Error::ClientEventQueueFull(event) => {
                format!("Client Event Queue Full. Dropping event: {:?}", event)
            }
            Error::InternalQueueClosedOrFull => "Internal Queue Full or Closed".to_string(),
            Error::SemtechUdpSerialization(err) => {
                format!("Semtech Udp Serialization Error: {:?}", err)
            }
            Error::SendTimeout => "Sending RF Packet Timed Out".to_string(),
            Error::DispatchWithNoSendPacket => "Dispatched PreparedSend with no Packet".to_string(),
            Error::AckRecvError => "Error waiting for ACK at sending process".to_string(),
            Error::ErrorSendingAck => "Error sending ACK to sending process".to_string(),
        };
        write!(f, "{}", msg)
    }
}

use std::error::Error as StdError;

impl StdError for Error {
    fn description(&self) -> &str {
        match self {
            Error::AckChannelRecv(_) => "AckChannelRecv",
            Error::AckError(_) => "AckError on transmit",
            Error::UnknownMac => "UnknownMac on on transmit",
            Error::UdpError(_) => "UdpError",
            Error::ClientEventQueueFull(_) => "Client Event Queue Full. Dropping event",
            Error::InternalQueueClosedOrFull => "Internal Queue Full or Closed",
            Error::SemtechUdpSerialization(_) => "Semtech Udp Serialization Error",
            Error::SendTimeout => "Sending RF Packet Timed Out",
            Error::DispatchWithNoSendPacket => "Dispatched PreparedSend with no Packet",
            Error::AckRecvError => "Error waiting for ACK at sending process",
            Error::ErrorSendingAck => "Error sending ACK to sending process",
        }
    }
}
