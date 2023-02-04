use semtech_udp::pull_resp::Time;
use semtech_udp::{
    pull_resp::{self, PhyData},
    server_runtime::{Error, Event, UdpRuntime},
    tx_ack, Bandwidth, CodingRate, DataRate, MacAddress, Modulation, SpreadingFactor,
};
use std::net::SocketAddr;
use structopt::StructOpt;
use tokio::sync::oneshot::{self, Receiver, Sender};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Opt::from_args();
    let addr = SocketAddr::from(([0, 0, 0, 0], cli.port));
    println!("Starting server: {addr}");

    // Splitting is optional and only useful if you are want to run concurrently
    // the client_rx & client_tx can both be held inside the UdpRuntime struct
    let (mut client_rx, mut client_tx) = UdpRuntime::new(addr).await?.split();

    // prepare a one-shot so that receive can unlocked sending
    let (tx, rx): (Sender<MacAddress>, Receiver<MacAddress>) = oneshot::channel();
    let mut tx = Some(tx);

    if cli.test {
        // spawn off tx thread for sending packets
        tokio::spawn(async move {
            let gateway_mac = rx.await.unwrap();
            let mut first_shot = true;

            while cli.delay != 0 || first_shot {
                first_shot = false;
                let data = vec![0; cli.length];

                let txpk = pull_resp::TxPk {
                    time: Time::immediate(),
                    freq: cli.frequency,
                    rfch: 0,
                    powe: cli.power as u64,
                    modu: Modulation::LORA,
                    datr: DataRate::new(cli.spreading_factor.clone(), cli.bandwidth.clone()),
                    codr: CodingRate::_4_5,
                    ipol: cli.polarization_inversion,
                    data: PhyData::new(data),
                    fdev: None,
                    prea: None,
                    ncrc: None,
                };

                println!("Sending: {txpk}");

                let prepared_send = client_tx.prepare_downlink(Some(txpk), gateway_mac);

                tokio::spawn(async move {
                    if let Err(e) = prepared_send.dispatch(Some(Duration::from_secs(5))).await {
                        if let Error::Ack(tx_ack::Error::AdjustedTransmitPower(
                            adjusted_power,
                            _tmst,
                        )) = e
                        {
                            // Generally, all packet forwarders will reduce output power to appropriate levels.
                            // Packet forwarder may optionally indicate the actual power used.
                            println!("Packet sent at adjusted power: {adjusted_power:?}")
                        } else {
                            println!("Transmit Dispatch threw error: {e:?}")
                        }
                    } else {
                        println!("Send complete");
                    }
                });

                sleep(Duration::from_secs(cli.delay)).await;
            }
        });
    }

    println!("Ready for clients");
    loop {
        match client_rx.recv().await {
            Event::UnableToParseUdpFrame(error, buf) => {
                println!("Semtech UDP Parsing Error: {error}");
                println!("UDP data: {buf:?}");
            }
            Event::NewClient((mac, addr)) => {
                println!("New packet forwarder client: {mac}, {addr}");

                // unlock the tx thread by sending it the gateway mac of the
                // the first client (connection via PULL_DATA frame)
                if let Some(tx) = tx.take() {
                    tx.send(mac).unwrap();
                }
            }
            Event::UpdateClient((mac, addr)) => {
                println!("Mac existed, but IP updated: {mac}, {addr}");
            }
            Event::PacketReceived(rxpk, addr) => {
                println!("Packet Receveived from {addr}: {rxpk:?}");
            }
            Event::StatReceived(stat, addr) => {
                println!("Stat Receveived from {addr}: {stat:?}");
            }
            Event::NoClientWithMac(_packet, mac) => {
                println!("Tried to send to client with unknown MAC: {mac:?}")
            }
            Event::ClientDisconnected((mac, addr)) => {
                println!("Client disconnected: {mac}, {addr}");
            }
        }
    }
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "Semtech GWMP over UDP Host Example",
    about = "Example application for semtech-udp library"
)]
pub struct Opt {
    /// Port to run service on
    #[structopt(long, default_value = "1680")]
    port: u16,

    /// whether to provide all raw packets
    #[structopt(long)]
    test: bool,

    /// Power output
    #[structopt(long, default_value = "27")]
    power: u8,

    /// Length
    #[structopt(long, default_value = "52")]
    length: usize,

    /// Seconds of delay between transmits. Set to 0 for one-shot
    #[structopt(long, default_value = "0")]
    delay: u64,

    /// Transmit frequency in MHz
    #[structopt(long, default_value = "868.1")]
    frequency: f64,

    /// Spreading Factor (eg: SF12)
    #[structopt(long, default_value = "SF12")]
    spreading_factor: SpreadingFactor,

    /// Bandwdith (eg: BW125)
    #[structopt(long, default_value = "BW125")]
    bandwidth: Bandwidth,

    /// Polarization inversion (set true when sending to devices)
    #[structopt(long)]
    polarization_inversion: bool,
}
