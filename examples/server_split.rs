use semtech_udp::{
    pull_resp,
    server_runtime::{Event, UdpRuntime},
    MacAddress, StringOrNum,
};
use std::net::SocketAddr;
use structopt::StructOpt;
use tokio::sync::oneshot::{self, Receiver, Sender};
use tokio::time::{delay_for as sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Opt::from_args();
    let addr = SocketAddr::from(([0, 0, 0, 0], cli.port));
    println!("Starting server: {}", addr);

    // Splitting is optional and only useful if you are want to run concurrently
    // the client_rx & client_tx can both be held inside the UdpRuntime struct
    let (mut client_rx, mut client_tx) = UdpRuntime::new(addr).await?.split();

    // prepare a one-shot so that receive can unlocked sending
    let (tx, rx): (Sender<MacAddress>, Receiver<MacAddress>) = oneshot::channel();
    let mut tx = Some(tx);

    // spawn off tx thread for sending packets
    tokio::spawn(async move {
        let gateway_mac = rx.await.unwrap();
        let mut first_shot = true;
        while cli.delay != 0 || first_shot {
            first_shot = false;
            let buffer = vec![0; cli.length];
            let size = buffer.len() as u64;
            let data = base64::encode(buffer);
            let tmst = StringOrNum::N(0);

            let txpk = pull_resp::TxPk {
                imme: true,
                tmst,
                freq: cli.frequency,
                rfch: 0,
                powe: cli.power as u64,
                modu: "LORA".to_string(),
                datr: cli.data_rate.clone(),
                codr: "4/5".to_string(),
                ipol: cli.polarization_inversion,
                size,
                data,
                tmms: None,
                fdev: None,
                prea: None,
                ncrc: None,
            };

            println!("Sending: {:?}", txpk);

            let prepared_send = client_tx.prepare_downlink(Some(txpk), gateway_mac);

            tokio::spawn(async move {
                if let Err(e) = prepared_send.dispatch(Some(Duration::from_secs(5))).await {
                    panic!("Transmit Dispatch threw error: {:?}", e)
                } else {
                    println!("Send complete");
                }
            });

            sleep(Duration::from_secs(cli.delay)).await;
        }
    });

    println!("Ready for clients");
    loop {
        if let Some(event) = client_rx.recv().await {
            match event {
                Event::UnableToParseUdpFrame(buf) => {
                    println!("Semtech UDP Parsing Error");
                    println!("UDP data: {:?}", buf);
                }
                Event::NewClient((mac, addr)) => {
                    println!("New packet forwarder client: {}, {}", mac, addr);

                    // unlock the tx thread by sending it the gateway mac of the
                    // the first client (connection via PULL_DATA frame)
                    if let Some(tx) = tx.take() {
                        tx.send(mac).unwrap();
                    }
                }
                Event::UpdateClient((mac, addr)) => {
                    println!("Mac existed, but IP updated: {}, {}", mac, addr);
                }
                Event::PacketReceived(rxpk, addr) => {
                    println!("Packet Receveived from {}:", addr);
                    println!("{:?}", rxpk);
                }
                Event::NoClientWithMac(_packet, mac) => {
                    println!("Tried to send to client with unknown MAC: {:?}", mac)
                }
                Event::RawPacket(_) => (),
            }
        }
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "semtech-server", about = "LoRa test device utility")]
pub struct Opt {
    /// Port to run service on
    #[structopt(long, default_value = "1680")]
    port: u16,

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

    /// Data rate (Spreading Factor / Bandwidth)
    #[structopt(long, default_value = "SF12BW125")]
    data_rate: String,

    /// Polarization inversion (set true when sending to devices)
    #[structopt(long)]
    polarization_inversion: bool,
}
