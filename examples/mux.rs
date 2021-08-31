use semtech_udp::{
    client_runtime, push_data,
    server_runtime::{self, Event, UdpRuntime},
    MacAddress,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;
use structopt::StructOpt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Opt::from_args();
    let addr = SocketAddr::from(([0, 0, 0, 0], cli.host));
    let (mut client_rx, client_tx) = UdpRuntime::new(addr).await?.split();

    println!("Starting server: {}", addr);

    let mut mux = HashMap::new();

    println!("Ready for clients");
    loop {
        match client_rx.recv().await {
            Event::UnableToParseUdpFrame(buf) => {
                println!("Semtech UDP Parsing Error");
                println!("UDP data: {:?}", buf);
            }
            Event::NewClient((mac, addr)) => {
                println!("New packet forwarder client: {}, {}", mac, addr);

                let mut clients = Vec::new();
                for port in &cli.client {
                    println!("Port {}", port);
                    clients
                        .push(client_instance(client_tx.clone(), mac.clone(), port.clone()).await?);
                }

                mux.insert(mac, clients);
            }
            Event::UpdateClient((mac, addr)) => {
                println!("Mac existed, but IP updated: {}, {}", mac, addr);
            }
            Event::PacketReceived(rxpk, gateway_mac) => {
                println!("Uplink Received {:?}", rxpk);
                if let Some(clients) = mux.get_mut(&gateway_mac) {
                    for sender in clients {
                        println!("Forwarding Uplink");
                        let mut packet = push_data::Packet::from_rxpk(rxpk.clone());
                        packet.gateway_mac = gateway_mac;
                        sender.send(packet.into()).await?;
                    }
                }
            }
            Event::NoClientWithMac(_packet, mac) => {
                println!("Tried to send to client with unknown MAC: {:?}", mac)
            }
        }
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "lora-mux", about = "Semtech GWMP over UDP Mux")]
pub struct Opt {
    /// port to host the service on
    #[structopt(long, default_value = "1681")]
    pub host: u16,
    /// addresses to be clients to (eg: 127.0.0.1:1680)
    /// WARNING: all addresses will receive all ACKs for transmits
    #[structopt(long, default_value = "1680")]
    pub client: Vec<String>,
}

async fn client_instance(
    mut client_tx: server_runtime::ClientTx,
    mac_address: MacAddress,
    host: String,
) -> Result<tokio::sync::mpsc::Sender<semtech_udp::Packet>, Box<dyn std::error::Error>> {
    let outbound = SocketAddr::from(([127, 0, 0, 1], 0));
    let host = SocketAddr::from_str(&host)?;
    println!(
        "Connecting to server {} from port {}",
        host,
        outbound.port()
    );
    let bytes = mac_address.bytes();
    let udp_runtime = client_runtime::UdpRuntime::new(
        [
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ],
        outbound,
        host,
    )
    .await?;

    let (mut receiver, sender) = (udp_runtime.subscribe(), udp_runtime.publish_to());

    tokio::spawn(async move {
        udp_runtime.run().await.unwrap();
    });

    let uplink_sender = sender.clone();
    tokio::spawn(async move {
        loop {
            let msg = receiver.recv().await.unwrap();
            println!("Received from miner {:?}", msg);
            match msg {
                semtech_udp::Packet::Down(down) => {
                    if let semtech_udp::Down::PullResp(packet) = down {
                        println!("Sending Downlink: {:?}", packet.data.txpk);
                        let txpk = packet.data.txpk.clone();
                        let prepared_send = client_tx.prepare_downlink(Some(txpk), mac_address);
                        tokio::spawn(async move {
                            if let Err(e) =
                                prepared_send.dispatch(Some(Duration::from_secs(5))).await
                            {
                                println!("Transmit Dispatch threw error: {:?}", e)
                            } else {
                                println!("Send complete");
                            }
                        });

                        let ack = (*packet).into_ack_for_gateway(mac_address);
                        sender.send(ack.into()).await.unwrap();
                    }
                }
                semtech_udp::Packet::Up(_up) => panic!("Should not receive Semtech up frames"),
            }
        }
    });

    Ok(uplink_sender)
}
