use semtech_udp::{
    server_runtime::{Event, UdpRuntime},
    Up as Packet, pull_resp, StringOrNum
};

use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = SocketAddr::from(([0, 0, 0, 0], 1691));
    let mut udp_runtime = UdpRuntime::new(addr).await?;

    loop {
        if let Ok(event) = udp_runtime.recv().await {
            match event {
                Event::NewClient((mac, addr)) => {
                    println!("New packet forwarder client: {}, {}", mac, addr);
                }
                Event::UpdateClient((mac, addr)) => {
                    println!("Mac existed, but IP updated: {}, {}", mac, addr);
                }
                Event::Packet(packet) => {
                    match packet {
                        Packet::PushData(packet) => {
                            if let Some(stat) = &packet.data.stat {
                                println!("Stat report: {:?}", stat);
                            }
                            if let Some(rxpk) = &packet.data.rxpk {
                                println!("Received packets:");
                                for recived_packet in rxpk {
                                    println!("\t{:?}", recived_packet);
                                    let buffer = [1, 2, 3, 4];
                                    let size = buffer.len() as u64;
                                    let data = base64::encode(buffer);
                                    let tmst = StringOrNum::N(recived_packet.tmst + 1_000_000);

                                    let txpk = pull_resp::TxPk {
                                        imme: false,
                                        tmst,
                                        freq: 902_800_000.0,
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
                                    udp_runtime
                                        .send(txpk, packet.gateway_mac)
                                        .await?;
                                }
                            }
                        }
                        // these are generally uninteresting but available for debug
                        _ => println!("{:?}", packet),

                    }
                }
            }
        }
    }
}
