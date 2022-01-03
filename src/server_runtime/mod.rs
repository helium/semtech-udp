use super::{
    parser::Parser, pull_resp, pull_resp::TxPk, tx_ack::Packet as TxAck, MacAddress, Packet,
    SerializablePacket, Up,
};
pub use crate::push_data::RxPk;
use std::sync::Arc;
use std::{collections::HashMap, net::SocketAddr, time::Duration};
use tokio::{
    net::{ToSocketAddrs, UdpSocket},
    sync::{mpsc, oneshot},
    time::timeout,
};

mod error;
pub use error::Error;
pub type Result<T = ()> = std::result::Result<T, Error>;

#[derive(Debug)]
enum InternalEvent {
    Downlink((pull_resp::Packet, MacAddress, oneshot::Sender<TxAck>)),
    PacketBySocket((Packet, SocketAddr)),
    Client((MacAddress, SocketAddr)),
    PacketReceived(RxPk, MacAddress),
    UnableToParseUdpFrame(Vec<u8>),
    AckReceived(TxAck),
}

#[derive(Debug, Clone)]
pub enum Event {
    PacketReceived(RxPk, MacAddress),
    NewClient((MacAddress, SocketAddr)),
    UpdateClient((MacAddress, SocketAddr)),
    UnableToParseUdpFrame(Vec<u8>),
    NoClientWithMac(Box<pull_resp::Packet>, MacAddress),
}

// receives requests from clients
// dispatches them to UdpTx
#[derive(Debug, Clone)]
#[allow(dead_code)]
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

#[derive(Clone)]
pub struct Downlink {
    mac: MacAddress,
    packet: Option<pull_resp::Packet>,
    sender: mpsc::Sender<InternalEvent>,
}

impl Downlink {
    pub fn set_packet(&mut self, txpk: TxPk) {
        self.packet = Some(pull_resp::Packet {
            random_token: rand::thread_rng().gen(),
            data: pull_resp::Data::from_txpk(txpk),
        });
    }

    pub fn get_destination_mac(&mut self) -> MacAddress {
        self.mac
    }

    async fn just_dispatch(self) -> Result {
        if let Some(packet) = self.packet {
            let (sender, receiver) = oneshot::channel();

            self.sender
                .send(InternalEvent::Downlink((packet, self.mac, sender)))
                .await?;

            // wait for the ACK for the protocol layer
            receiver.await?.get_result().map_err(|e| e.into())
        } else {
            Err(Error::DispatchWithNoSendPacket)
        }
    }

    pub async fn dispatch(self, timeout_duration: Option<Duration>) -> Result {
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
    pub async fn send(&mut self, txpk: TxPk, mac: MacAddress, timeout: Option<Duration>) -> Result {
        let prepared_send = self.prepare_downlink(Some(txpk), mac);
        prepared_send.dispatch(timeout).await
    }

    pub fn prepare_downlink(&mut self, txpk: Option<TxPk>, mac: MacAddress) -> Downlink {
        let packet = txpk.map(|txpk| pull_resp::Packet {
            random_token: rand::thread_rng().gen(),
            data: pull_resp::Data::from_txpk(txpk),
        });

        Downlink {
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

    pub async fn send(&mut self, txpk: TxPk, mac: MacAddress, timeout: Option<Duration>) -> Result {
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

    pub async fn new<A: ToSocketAddrs>(addr: A) -> Result<UdpRuntime> {
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
    pub async fn run(self) -> Result {
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
    pub async fn run(mut self) -> Result {
        let mut buf = vec![0u8; 1024];
        loop {
            let msg = self.receiver.recv().await;
            if let Some(msg) = msg {
                match msg {
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
                                // Client not connected
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
                            sender.send(txack).map_err(|_| Error::AckSend)?;
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
