use semtech_udp::client_runtime::UdpRuntime;
use semtech_udp::{MacAddress, Up::PushData};
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;
use structopt::StructOpt;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mac_address = MacAddress::from([0, 0, 0, 0, 4, 3, 2, 1]);
    let cli = Opt::from_args();
    let outbound = SocketAddr::from(([0, 0, 0, 0], 0));
    let host = SocketAddr::from_str(cli.host.as_str())?;
    println!("Connecting to server {} from port {}", cli.host, outbound);
    let udp_runtime = UdpRuntime::new(mac_address.clone(), outbound, host).await?;

    let (mut receiver, sender) = (udp_runtime.subscribe(), udp_runtime.publish_to());

    tokio::spawn(async move {
        udp_runtime.run().await.unwrap();
    });

    let uplink_sender = sender.clone();
    tokio::spawn(async move {
        loop {
            println!("Sending a random uplink");
            uplink_sender
                .send(semtech_udp::Packet::Up(PushData(
                    semtech_udp::push_data::Packet::random(),
                )))
                .await
                .unwrap();
            sleep(Duration::from_secs(5)).await;
        }
    });

    loop {
        let msg = receiver.recv().await?;
        println!("msg: {:?}", msg);

        match msg {
            semtech_udp::Packet::Down(down) => {
                if let semtech_udp::Down::PullResp(packet) = down {
                    // it is the client's responsibility to ack the tx request
                    let ack =
                        (*packet).into_ack_for_gateway(semtech_udp::MacAddress::from(mac_address));
                    sender.send(ack.into()).await?;
                }
            }
            semtech_udp::Packet::Up(_up) => panic!("Should not receive Semtech up frames"),
        }
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "Semtech GWMP over UDP Client Example")]
pub struct Opt {
    #[structopt(short, long, default_value = "127.0.0.1:1680")]
    pub host: String,
}
