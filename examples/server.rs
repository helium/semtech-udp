use std::net::SocketAddr;
use semtech_udp::server_runtime::UdpRuntime;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let addr = SocketAddr::from(([0, 0, 0, 0], 2132));

    let mut udp_runtime = UdpRuntime::new(addr).await?;

    loop {
        if let Ok(event) = udp_runtime.recv().await {
            println!("event {:?}", event);
        }
    }
}

