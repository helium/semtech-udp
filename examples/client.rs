use semtech_udp::client_runtime::UdpRuntime;
use std::net::SocketAddr;
use std::str::FromStr;
use structopt::StructOpt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Opt::from_args();
    let outbound = SocketAddr::from(([0, 0, 0, 0], cli.port));
    let host = SocketAddr::from_str(cli.host.as_str())?;
    println!("Connecting to server {} from port {}", cli.host, cli.port);
    let udp_runtime = UdpRuntime::new([0, 0, 0, 0, 1, 2, 3, 4], outbound, host).await?;

    let (mut receiver, _sender) = (udp_runtime.subscribe(), udp_runtime.publish_to());

    tokio::spawn(async move {
        udp_runtime.run().await.unwrap();
    });

    loop {
        let msg = receiver.recv().await?;
        println!("msg: {:?}", msg);
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "virtual-lorawan-device", about = "LoRaWAN test device utility")]
pub struct Opt {
    /// dial out port
    #[structopt(short, long, default_value = "1600")]
    pub port: u16,
    #[structopt(short, long, default_value = "127.0.0.1:1680")]
    pub host: String,
}
