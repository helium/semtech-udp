use semtech_udp::client_runtime::{Event, UdpRuntime};
use semtech_udp::MacAddress;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;
use structopt::StructOpt;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (shutdown_trigger, shutdown_signal) = triggered::trigger();

    let mac_address = MacAddress::from([0, 0, 0, 0, 4, 3, 2, 1]);
    let cli = Opt::from_args();
    let host = SocketAddr::from_str(cli.host.as_str())?;
    println!("Connecting to server {}", cli.host);
    let (uplink_sender, mut downlink_request_receiver, udp_runtime) =
        UdpRuntime::new(mac_address, host).await?;

    let udp_runtime_task = tokio::spawn(udp_runtime.run(shutdown_signal));

    tokio::spawn(async move {
        loop {
            println!("Sending a random uplink");

            uplink_sender
                .send(semtech_udp::push_data::Packet::random())
                .await
                .unwrap();
            sleep(Duration::from_secs(5)).await;
        }
    });

    while let Some(event) = downlink_request_receiver.recv().await {
        match event {
            Event::LostConnection => {
                println!("Lost connection to GWMP client");
            }
            Event::Reconnected => {
                println!("Reconnected to GWMP client");
            }
            Event::DownlinkRequest(downlink_request) => downlink_request.ack().await?,
            Event::UnableToParseUdpFrame(parse_error, _buffer) => {
                println!("Error parsing UDP frame {parse_error}")
            }
        }
    }
    shutdown_trigger.trigger();
    if let Err(e) = udp_runtime_task.await? {
        println!("UdpRunTime return error {e}");
    }
    Ok(())
}

#[derive(Debug, StructOpt)]
#[structopt(name = "Semtech GWMP over UDP Client Example")]
pub struct Opt {
    #[structopt(short, long, default_value = "127.0.0.1:1680")]
    pub host: String,
}
