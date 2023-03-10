use super::{
    pull_resp, pull_resp::TxPk, tx_ack::Packet as TxAck, MacAddress, Packet, ParseError,
    SerializablePacket, Up,
};
pub use crate::push_data::{RxPk, Stat};
use std::sync::Arc;
use std::time::SystemTime;
use std::{collections::HashMap, net::SocketAddr, time::Duration};
use tokio::{
    net::{ToSocketAddrs, UdpSocket},
    sync::{mpsc, oneshot},
    time::timeout,
};

mod error;
pub use error::Error;
pub type Result<T = ()> = std::result::Result<T, Error>;

const DEFAULT_DISCONNECT_THRESHOLD: u64 = 60;
const DEFAULT_CACHE_CHECK_FREQ: u64 = 60;
const MAX_MESSAGE_SIZE: usize = 65535;

#[derive(Debug)]
enum InternalEvent {
    Downlink((pull_resp::Packet, MacAddress, oneshot::Sender<TxAck>)),
    PacketBySocket((Packet, SocketAddr)),
    Client((MacAddress, SocketAddr)),
    PacketReceived(RxPk, MacAddress),
    StatReceived(Stat, MacAddress),
    UnableToParseUdpFrame(ParseError, Vec<u8>),
    AckReceived(TxAck),
    CheckCache,
    FailedSend((Box<pull_resp::Packet>, MacAddress)),
    SuccessSend((u16, oneshot::Sender<TxAck>)),
}

#[derive(Debug)]
pub enum Event {
    PacketReceived(RxPk, MacAddress),
    StatReceived(Stat, MacAddress),
    NewClient((MacAddress, SocketAddr)),
    UpdateClient((MacAddress, SocketAddr)),
    UnableToParseUdpFrame(ParseError, Vec<u8>),
    NoClientWithMac(Box<pull_resp::Packet>, MacAddress),
    ClientDisconnected((MacAddress, SocketAddr)),
}

// receives requests from clients
// dispatches them to UdpTx
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ClientTx {
    sender: mpsc::Sender<InternalEvent>,
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
    self_sender: mpsc::Sender<InternalEvent>,
    receiver: mpsc::Receiver<InternalEvent>,
    client_tx_sender: mpsc::Sender<Event>,
    clients: HashMap<MacAddress, Client>,
    downlink_senders: HashMap<u16, oneshot::Sender<TxAck>>,
    socket_sender: Arc<UdpSocket>,
    disconnect_threshold: Option<Duration>,
}

#[derive(Debug, Clone)]
struct Client {
    addr: SocketAddr,
    last_seen: SystemTime,
}

impl Client {
    fn new(addr: SocketAddr) -> Self {
        Client {
            addr,
            last_seen: SystemTime::now(),
        }
    }
    fn addr(&self) -> &SocketAddr {
        &self.addr
    }

    fn update_addr(&mut self, new_addr: SocketAddr) {
        self.addr = new_addr;
        self.seen();
    }

    fn seen(&mut self) {
        self.last_seen = SystemTime::now();
    }
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

    async fn just_dispatch(self) -> Result<Option<u32>> {
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

    pub async fn dispatch(self, timeout_duration: Option<Duration>) -> Result<Option<u32>> {
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
        // ClientRx panics before it can get dropped (see UdpRuntime)
        self.receiver.recv().await.unwrap()
    }
}

impl ClientTx {
    pub async fn send(
        &mut self,
        txpk: TxPk,
        mac: MacAddress,
        timeout: Option<Duration>,
    ) -> Result<Option<u32>> {
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

    pub async fn send(
        &mut self,
        txpk: TxPk,
        mac: MacAddress,
        timeout: Option<Duration>,
    ) -> Result<Option<u32>> {
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
        };

        let client_rx = ClientRx {
            receiver: client_tx_receiver,
        };

        let udp_rx = UdpRx {
            socket_receiver,
            internal_sender: udp_tx_sender.clone(),
        };

        let udp_tx = Internal {
            self_sender: udp_tx_sender,
            receiver: udp_tx_receiver,
            client_tx_sender,
            clients: HashMap::new(),
            downlink_senders: HashMap::new(),
            socket_sender,
            disconnect_threshold: Some(Duration::from_secs(DEFAULT_DISCONNECT_THRESHOLD)),
        };

        // udp_rx reads from the UDP port
        // and sends packets to relevant parties
        tokio::spawn(async move {
            if let Err(e) = udp_rx.run().await {
                // we panic here because the ony error case here
                // if we lost the local socket somehow
                panic!("UdpRx threw error: {e:?}")
            }
        });

        // udp_tx writes to the UDP port and maintains
        // gateway to IP map
        tokio::spawn(async move {
            if let Err(e) = udp_tx.run().await {
                // we panic here because the ony error case here
                // if we lost the local socket somehow
                panic!("UdpTx threw error: {e:?}")
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
        let cache_sender = self.internal_sender.clone();
        let cache_sender = tokio::spawn(async move {
            loop {
                cache_sender.send(InternalEvent::CheckCache).await?;
                tokio::time::sleep(Duration::from_secs(DEFAULT_CACHE_CHECK_FREQ)).await;
            }
        });

        let socket_handler = tokio::spawn(async move {
            let mut buf = vec![0u8; MAX_MESSAGE_SIZE];
            loop {
                match self.socket_receiver.recv_from(&mut buf).await {
                    Err(e) => return Err(e.into()),
                    Ok((n, src)) => {
                        let packet = match Packet::parse_uplink(&buf[0..n]) {
                            Ok(packet) => Some(packet),
                            Err(e) => {
                                let mut vec = Vec::new();
                                vec.extend_from_slice(&buf[0..n]);
                                self.internal_sender
                                    .send(InternalEvent::UnableToParseUdpFrame(e, vec))
                                    .await?;
                                None
                            }
                        };
                        if let Some(packet) = packet {
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

                                    if let Some(stat) = &push_data.data.stat {
                                        self.internal_sender
                                            .send(InternalEvent::StatReceived(
                                                stat.clone(),
                                                push_data.gateway_mac,
                                            ))
                                            .await?;
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
                    }
                }
            }
        });

        tokio::select!(
            resp = cache_sender => resp?,
            resp = socket_handler => resp?,
        )
    }
}

impl Internal {
    pub async fn run(mut self) -> Result {
        let mut buf = vec![0u8; MAX_MESSAGE_SIZE];
        loop {
            let msg = self.receiver.recv().await;
            if let Some(msg) = msg {
                match msg {
                    InternalEvent::CheckCache => {
                        let now = SystemTime::now();
                        if let Some(disconnect_threshold) = self.disconnect_threshold {
                            for (mac, client) in self.clients.clone().into_iter() {
                                let time_since_last_seen = now
                                    .duration_since(client.last_seen)
                                    .map_err(|_| Error::LastSeen {
                                        last_seen: client.last_seen,
                                        now,
                                    })?;

                                if time_since_last_seen > disconnect_threshold {
                                    // Client not connected
                                    self.client_tx_sender
                                        .send(Event::ClientDisconnected((mac, *client.addr())))
                                        .await?;
                                    self.clients.remove(&mac);
                                }
                            }
                        }
                    }
                    InternalEvent::UnableToParseUdpFrame(error, frame) => {
                        self.client_tx_sender
                            .send(Event::UnableToParseUdpFrame(error, frame))
                            .await?;
                    }
                    InternalEvent::PacketReceived(rxpk, mac) => {
                        self.client_tx_sender
                            .send(Event::PacketReceived(rxpk, mac))
                            .await?;
                    }
                    InternalEvent::StatReceived(stat, mac) => {
                        self.client_tx_sender
                            .send(Event::StatReceived(stat, mac))
                            .await?;
                    }
                    InternalEvent::Downlink((packet, mac, ack_sender)) => {
                        if let Some(client) = self.clients.get(&mac) {
                            // we spawn off here because one slow client can slow down all of the
                            // event processing
                            let n = packet.serialize(&mut buf)? as usize;
                            let buf = Vec::from(&buf[..n]);
                            let socket_sender = self.socket_sender.clone();
                            let client_addr = *client.addr();
                            let self_sender = self.self_sender.clone();
                            tokio::spawn(async move {
                                match socket_sender.send_to(&buf, client_addr).await {
                                    Err(_) => {
                                        self_sender
                                            .send(InternalEvent::FailedSend((packet.into(), mac)))
                                            .await
                                            .unwrap();
                                    }
                                    Ok(_) => {
                                        self_sender
                                            .send(InternalEvent::SuccessSend((
                                                packet.random_token,
                                                ack_sender,
                                            )))
                                            .await
                                            .unwrap();
                                    }
                                }
                            });
                        } else {
                            self.client_tx_sender
                                .send(Event::NoClientWithMac(packet.into(), mac))
                                .await?;
                        }
                    }
                    InternalEvent::AckReceived(txack) => {
                        if let Some(sender) = self.downlink_senders.remove(&txack.random_token) {
                            // we may have received an ACK on a transmit that timed out already
                            // therefore, this send may fail.
                            let _ = sender.send(txack);
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
                        if let Some(client) = self.clients.get_mut(&mac) {
                            if *client.addr() != addr {
                                client.update_addr(addr);
                                self.client_tx_sender
                                    .send(Event::UpdateClient((mac, addr)))
                                    .await?;
                            } else {
                                // refresh the seen
                                client.seen();
                            }
                        }
                        // simply insert if no entry exists
                        else {
                            self.clients.insert(mac, Client::new(addr));
                            self.client_tx_sender
                                .send(Event::NewClient((mac, addr)))
                                .await?;
                        }
                    }
                    InternalEvent::SuccessSend((random_token, ack_sender)) => {
                        self.downlink_senders.insert(random_token, ack_sender);
                    }
                    InternalEvent::FailedSend((packet, mac)) => {
                        self.clients.remove(&mac);
                        self.client_tx_sender
                            .send(Event::NoClientWithMac(packet, mac))
                            .await?;
                    }
                }
            }
        }
    }
}
