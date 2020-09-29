use semtech_udp::{
    pull_resp,
    server_runtime::{Event, UdpRuntime},
    StringOrNum, Up as Packet,
};
use std::net::SocketAddr;
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
        if let Ok(event) = udp_runtime.recv().await {
            match event {
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
                Event::Packet(packet) => {
                    match packet {
                        Packet::PushData(packet) => {
                            if let Some(rxpk) = &packet.data.rxpk {
                                println!("Received packets:");
                                for received_packet in rxpk {
                                    println!("\t{:?}", received_packet);

                                    let buffer = [1, 2, 3, 4];
                                    let size = buffer.len() as u64;
                                    let data = base64::encode(buffer);
                                    let tmst = StringOrNum::N(received_packet.tmst + 1_000_000);

                                    let txpk = pull_resp::TxPk {
                                        imme: false,
                                        tmst,
                                        freq: 902.800_000,
                                        rfch: 0,
                                        powe: 27,
                                        modu: "LORA".to_string(),
                                        datr: "SF8BW500".to_string(),
                                        codr: "4/5".to_string(),
                                        ipol: true,
                                        size,
                                        data,
                                        tmms: None,
                                        fdev: None,
                                        prea: None,
                                        ncrc: None,
                                    };

                                    // this async call returns when TxAck is received
                                    if let Err(e) = udp_runtime.send(txpk, packet.gateway_mac).await
                                    {
                                        println!("Warning: error on send {:?}", e);
                                    }
                                    println!("Send complete");
                                }
                            }
                        }
                        _ => println!("{:?}", packet),
                    }
                }
                Event::NoClientWithMac(_packet, mac) => {
                    println!("Tried to send to client with unknown MAC: {:?}", mac)
                }
            }
        }
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "virtual-lorawan-device", about = "LoRaWAN test device utility")]
pub struct Opt {
    /// port to run service on
    #[structopt(short, long, default_value = "1680")]
    pub port: u16,
}
