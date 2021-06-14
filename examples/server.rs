use semtech_udp::{
    pull_resp,
    server_runtime::{Event, UdpRuntime},
    CodingRate, DataRate, Modulation, StringOrNum,
};
use std::net::SocketAddr;
use std::time::Duration;
use structopt::StructOpt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Opt::from_args();
    let addr = SocketAddr::from(([0, 0, 0, 0], cli.port));
    println!("Starting server: {}", addr);
    let mut udp_runtime = UdpRuntime::new(addr).await?;
    println!("Ready for clients");
    loop {
        println!("Waiting for event");
        match udp_runtime.recv().await {
            Event::UnableToParseUdpFrame(buf) => {
                println!("Semtech UDP Parsing Error");
                println!("UDP data: {:?}", buf);
            }
            Event::NewClient((mac, addr)) => {
                println!("New packet forwarder client: {}, {}", mac, addr);
            }
            Event::UpdateClient((mac, addr)) => {
                println!("Mac existed, but IP updated: {}, {}", mac, addr);
            }
            Event::PacketReceived(rxpk, gateway_mac) => {
                println!("{:?}", rxpk);

                let data = vec![1, 2, 3, 4];
                let size = data.len() as u64;
                let tmst = StringOrNum::N(rxpk.get_timestamp() + 1_000_000);

                let txpk = pull_resp::TxPk {
                    imme: false,
                    tmst,
                    freq: 902.800_000,
                    rfch: 0,
                    powe: 27,
                    modu: Modulation::LORA,
                    datr: DataRate::default(),
                    codr: CodingRate::_4_5,
                    ipol: true,
                    size,
                    data,
                    tmms: None,
                    fdev: None,
                    prea: None,
                    ncrc: None,
                };

                let prepared_send = udp_runtime.prepare_downlink(txpk, gateway_mac);

                tokio::spawn(async move {
                    if let Err(e) = prepared_send.dispatch(Some(Duration::from_secs(5))).await {
                        panic!("Transmit Dispatch threw error: {:?}", e)
                    } else {
                        println!("Send complete");
                    }
                });
            }
            Event::NoClientWithMac(_packet, mac) => {
                println!("Tried to send to client with unknown MAC: {:?}", mac)
            }
            Event::RawPacket(_) => (),
        }
    }
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "Semtech GWMP over UDP Host Example",
    about = "Example application for semtech-udp library"
)]
pub struct Opt {
    /// port to run service on
    #[structopt(short, long, default_value = "1680")]
    pub port: u16,
}
