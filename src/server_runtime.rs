use super::{
    parser::Parser, pull_resp, pull_resp::TxPk, Down, MacAddress, Packet, SerializablePacket, Up,
};
use std::{collections::HashMap, net::SocketAddr, time::Duration};
use tokio::{
    net::{
        udp::{RecvHalf, SendHalf},
        UdpSocket,
    },
    sync::{
        broadcast,
        mpsc::{self, Receiver, Sender},
    },
    time::timeout,
};

#[derive(Debug)]
enum UdpMessage {
    PacketByMac((Packet, MacAddress)),
    PacketBySocket((Packet, SocketAddr)),
    Client((MacAddress, SocketAddr)),
}

#[derive(Debug, Clone)]
pub enum Event {
    Packet(Up),
    NewClient((MacAddress, SocketAddr)),
    UpdateClient((MacAddress, SocketAddr)),
    UnableToParseUdpFrame(Vec<u8>),
    NoClientWithMac(Box<pull_resp::Packet>, MacAddress),
}

// receives requests from clients
// dispatches them to UdpTx
#[derive(Debug)]
pub struct ClientTx {
    sender: Sender<UdpMessage>,
    // you need to subscribe to the send channel
    receiver_copier: broadcast::Sender<Event>,
}

// sends packets to clients
// broadcast enables many clients
pub type ClientRx = broadcast::Receiver<Event>;

// receives UDP packets
struct UdpRx {
    socket_receiver: RecvHalf,
    udp_tx_sender: Sender<UdpMessage>,
    client_tx_sender: broadcast::Sender<Event>,
}

// transmits UDP packets
struct UdpTx {
    receiver: Receiver<UdpMessage>,
    client_tx_sender: broadcast::Sender<Event>,
    clients: HashMap<MacAddress, SocketAddr>,
    socket_sender: SendHalf,
}

#[derive(Debug)]
pub struct UdpRuntime {
    rx: ClientRx,
    tx: ClientTx,
}
use rand::Rng;

#[derive(Debug)]
pub enum Error {
    QueueFull(mpsc::error::SendError<(Packet, MacAddress)>),
    AckChannelRecv(broadcast::RecvError),
    AckError(super::packet::tx_ack::Error),
    SendTimeout,
    DispatchWithNoSendPacket,
    UnknownMac,
    UdpError(std::io::Error),
    ClientEventQueueFull(broadcast::SendError<Event>),
    SocketEventQueueFull,
    SemtechUdpSerialization(super::packet::Error),
}

pub struct Downlink {
    random_token: u16,
    mac: MacAddress,
    packet: Option<pull_resp::Packet>,
    sender: Sender<UdpMessage>,
    receiver: broadcast::Receiver<Event>,
}

impl Downlink {
    pub fn set_packet(&mut self, txpk: TxPk) {
        self.packet = Some(pull_resp::Packet {
            random_token: self.random_token,
            data: pull_resp::Data::from_txpk(txpk),
        });
    }

    async fn just_dispatch(self) -> Result<(), Error> {
        let (mut sender, mut receiver) = (self.sender, self.receiver);

        if let Some(packet) = self.packet {
            // send it to UdpTx channel
            sender
                .send(UdpMessage::PacketByMac((packet.into(), self.mac)))
                .await?;

            // loop over responses until the TxAck is received
            loop {
                match receiver.recv().await? {
                    Event::Packet(packet) => {
                        if let Up::TxAck(ack) = packet {
                            if ack.random_token == self.random_token {
                                return if let Some(error) = ack.get_error() {
                                    Err(error.into())
                                } else {
                                    Ok(())
                                };
                            }
                        }
                    }
                    Event::NoClientWithMac(_packet, _mac) => {
                        return Err(Error::UnknownMac);
                    }
                    _ => (),
                }
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
            receiver: self.receiver_copier.subscribe(),
        }
    }

    fn get_sender(&mut self) -> Sender<UdpMessage> {
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

    pub async fn recv(&mut self) -> Result<Event, broadcast::RecvError> {
        self.rx.recv().await
    }

    pub async fn new(addr: SocketAddr) -> Result<UdpRuntime, Error> {
        let socket = UdpSocket::bind(&addr).await?;
        let (socket_receiver, socket_sender) = socket.split();

        let (udp_tx_sender, udp_tx_receiver) = mpsc::channel(100);

        // broadcasts to client
        let (client_tx_sender, client_tx_receiver) = broadcast::channel(100);

        let client_rx = ClientTx {
            sender: udp_tx_sender.clone(),
            receiver_copier: client_tx_sender.clone(),
        };

        let client_tx = client_tx_receiver;

        let udp_rx = UdpRx {
            socket_receiver,
            udp_tx_sender,
            client_tx_sender: client_tx_sender.clone(),
        };

        let udp_tx = UdpTx {
            receiver: udp_tx_receiver,
            client_tx_sender,
            clients: HashMap::new(),
            socket_sender,
        };

        // udp_rx reads from the UDP port
        // and sends packets to relevant parties
        tokio::spawn(async move {
            if let Err(e) = udp_rx.run().await {
                panic!("UdpRx threw error: {:?}", e)
            }
        });

        // udp_tx writes to the UDP port and maintains
        // gateway to IP map
        tokio::spawn(async move {
            if let Err(e) = udp_tx.run().await {
                panic!("UdpTx threw error: {:?}", e)
            }
        });

        Ok(UdpRuntime {
            rx: client_tx,
            tx: client_rx,
        })
    }
}

impl UdpRx {
    pub async fn run(mut self) -> Result<(), Error> {
        let mut buf = vec![0u8; 1024];
        loop {
            match self.socket_receiver.recv_from(&mut buf).await {
                Err(e) => return Err(e.into()),
                Ok((n, src)) => {
                    let packet = if let Ok(packet) = Packet::parse(&buf[0..n], n) {
                        Some(packet)
                    } else {
                        let mut vec = Vec::new();
                        vec.extend_from_slice(&buf[0..n]);
                        self.client_tx_sender
                            .send(Event::UnableToParseUdpFrame(vec))?;
                        None
                    };

                    if let Some(packet) = packet {
                        match packet {
                            Packet::Up(packet) => {
                                // echo all packets to client
                                self.client_tx_sender.send(Event::Packet(packet.clone()))?;
                                match packet {
                                    Up::PullData(pull_data) => {
                                        let mac = pull_data.gateway_mac;
                                        // first send (mac, addr) to update map owned by UdpRuntimeTx
                                        let client = (mac, src);
                                        self.udp_tx_sender.send(UdpMessage::Client(client)).await?;

                                        // send the ack_packet
                                        let ack_packet = pull_data.into_ack();
                                        let mut udp_tx = self.udp_tx_sender.clone();
                                        udp_tx
                                            .send(UdpMessage::PacketByMac((ack_packet.into(), mac)))
                                            .await?
                                    }
                                    Up::PushData(push_data) => {
                                        let socket_addr = src;
                                        // send the ack_packet
                                        let ack_packet = push_data.into_ack();
                                        self.udp_tx_sender
                                            .send(UdpMessage::PacketBySocket((
                                                ack_packet.into(),
                                                socket_addr,
                                            )))
                                            .await?;
                                    }
                                    _ => (),
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

impl UdpTx {
    pub async fn run(mut self) -> Result<(), Error> {
        let mut buf = vec![0u8; 1024];
        loop {
            let msg = self.receiver.recv().await;
            if let Some(msg) = msg {
                match msg {
                    UdpMessage::PacketByMac((packet, mac)) => {
                        if let Some(addr) = self.clients.get(&mac) {
                            let n = packet.serialize(&mut buf)? as usize;
                            let _sent = self.socket_sender.send_to(&buf[..n], addr).await?;
                        } else if let Packet::Down(Down::PullResp(pull_resp)) = packet {
                            self.client_tx_sender
                                .send(Event::NoClientWithMac(pull_resp, mac))
                                .unwrap();
                        }
                    }
                    UdpMessage::PacketBySocket((packet, addr)) => {
                        let n = packet.serialize(&mut buf)? as usize;
                        let _sent = self.socket_sender.send_to(&buf[..n], &addr).await?;
                    }
                    UdpMessage::Client((mac, addr)) => {
                        // tell user if same MAC has new IP
                        if let Some(existing_addr) = self.clients.get(&mac) {
                            if *existing_addr != addr {
                                self.clients.insert(mac, addr);
                                self.client_tx_sender
                                    .send(Event::UpdateClient((mac, addr)))?;
                            }
                        }
                        // simply insert if no entry exists
                        else {
                            self.clients.insert(mac, addr);
                            self.client_tx_sender.send(Event::NewClient((mac, addr)))?;
                        }
                    }
                }
            }
        }
    }
}


impl From<tokio::time::Elapsed> for Error {
    fn from(_err: tokio::time::Elapsed) -> Error {
        Error::SendTimeout
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::UdpError(err)
    }
}

impl From<broadcast::SendError<Event>> for Error {
    fn from(err: broadcast::SendError<Event>) -> Error {
        Error::ClientEventQueueFull(err)
    }
}

impl From<mpsc::error::SendError<UdpMessage>> for Error {
    fn from(_err: mpsc::error::SendError<UdpMessage>) -> Error {
        Error::SocketEventQueueFull
    }
}

impl From<super::packet::Error> for Error {
    fn from(err: super::packet::Error) -> Error {
        Error::SemtechUdpSerialization(err)
    }
}

impl From<mpsc::error::SendError<(Packet, MacAddress)>> for Error {
    fn from(e: mpsc::error::SendError<(Packet, MacAddress)>) -> Self {
        Error::QueueFull(e)
    }
}

impl From<broadcast::RecvError> for Error {
    fn from(e: broadcast::RecvError) -> Self {
        Error::AckChannelRecv(e)
    }
}

impl From<super::packet::tx_ack::Error> for Error {
    fn from(e: super::packet::tx_ack::Error) -> Self {
        Error::AckError(e)
    }
}


impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let msg = match self {
            Error::QueueFull(event) => format!("QueueFull. Droppping event: {}", event),
            Error::AckChannelRecv(_) => "AckChannelRecv".to_string(),
            Error::AckError(error) => format!("AckError on trasmit {}", error),
            Error::UnknownMac => "UnknownMac on on transmit".to_string(),
            Error::UdpError(error) => format!("UdpError: {}", error),
            Error::ClientEventQueueFull(event) => {
                format!("ClientEventQueueFull. Droppping event: {:?}", event)
            }
            Error::SocketEventQueueFull => "Internal UDP buffer full".to_string(),
            Error::SemtechUdpSerialization(err) => {
                format!("SemtechUdpSerilaization Error: {:?}", err)
            }
            Error::SendTimeout => format!("Sending RF Packet Timed Out"),
            Error::DispatchWithNoSendPacket => format!("Dispatched PreparedSend with no Packet"),
        };
        write!(f, "{}", msg)
    }
}

use std::error::Error as StdError;

impl StdError for Error {
    fn description(&self) -> &str {
        match self {
            Error::QueueFull(_) => "QueueFull",
            Error::AckChannelRecv(_) => "AckChannelRecv",
            Error::AckError(_) => "AckError on trasmit",
            Error::UnknownMac => "UnknownMac on on transmit",
            Error::UdpError(_) => "UdpError",
            Error::ClientEventQueueFull(_) => "ClientEventQueueFull. Droppping event",
            Error::SocketEventQueueFull => "Internal UDP buffer full",
            Error::SemtechUdpSerialization(_) => "SemtechUdpSerilaization Error",
            Error::SendTimeout => "Sending RF Packet Timed Out",
            Error::DispatchWithNoSendPacket => "Dispatched PreparedSend with no Packet",
        }
    }
}
