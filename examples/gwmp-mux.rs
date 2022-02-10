use semtech_udp::client_runtime::RxMessage;
use semtech_udp::{
    client_runtime, push_data,
    server_runtime::{self, Error, Event, UdpRuntime},
    tx_ack, MacAddress,
};
use slog::{self, debug, error, info, o, warn, Drain, Logger};
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::str::FromStr;
use structopt::StructOpt;
use tokio::{io::AsyncReadExt, signal, time::Duration};

fn main() {
    let cli = Opt::from_args();
    let logger = mk_logger(cli.log_level, cli.disable_timestamp);
    let scope_guard = slog_scope::set_global_logger(logger);
    let run_logger = slog_scope::logger().new(o!());
    let logger = slog_scope::logger().new(o!());

    let _log_guard = slog_stdlog::init().unwrap();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async move {
        let (shutdown_trigger, shutdown_signal) = triggered::trigger();

        let logger = slog_scope::logger().new(o!());
        if let Err(e) = host_and_mux_sup(cli, run_logger, shutdown_signal).await {
            error!(&logger, "Error with host: {e}");
        }
        watch_for_shutdown().await;
        shutdown_trigger.trigger();
    });

    runtime.shutdown_timeout(Duration::from_secs(0));
    info!(&logger, "Shutting down");
    drop(scope_guard);
}

async fn watch_for_shutdown() {
    let mut in_buf = [0u8; 64];
    let mut stdin = tokio::io::stdin();
    loop {
        tokio::select!(
                 _ = signal::ctrl_c() => return,
                    read = stdin.read(&mut in_buf) => if let Ok(0) = read { return },

        )
    }
}

async fn host_and_mux_sup(
    cli: Opt,
    logger: Logger,
    shutdown_signal: triggered::Listener,
) -> Result<(), Box<dyn std::error::Error>> {
    let logger_copy = logger.clone();
    let shutdown_signal_copy = shutdown_signal.clone();
    tokio::select!(
             _ = shutdown_signal => {
            info!(&logger, "Shutting down host_and_mux");
            Ok(())},
                res = host_and_mux(cli, logger_copy, shutdown_signal_copy) => res
    )
}

async fn host_and_mux(
    cli: Opt,
    logger: Logger,
    shutdown_signal: triggered::Listener,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = SocketAddr::from(([0, 0, 0, 0], cli.host));
    info!(&logger, "Starting server: {addr}");
    let (mut client_rx, client_tx) = UdpRuntime::new(addr).await?.split();
    let mut mux = HashMap::new();
    info!(&logger, "Ready for clients");
    let client_shutdown_signal = shutdown_signal.clone();

    loop {
        match client_rx.recv().await {
            Event::UnableToParseUdpFrame(error, buf) => {
                error!(&logger, "Semtech UDP Parsing Error: {error}");
                error!(&logger, "UDP data: {buf:?}");
            }
            Event::NewClient((mac, addr)) => {
                info!(&logger, "New packet forwarder client: {mac}, {addr}");
                let mut clients = Vec::new();
                for address in &cli.client {
                    let client_instance_logger = slog_scope::logger().new(o!());
                    match spawn_client_instance(
                        client_shutdown_signal.clone(),
                        client_tx.clone(),
                        client_instance_logger,
                        mac,
                        address.clone(),
                    )
                    .await
                    {
                        Ok(client) => {
                            info!(&logger, "Connected to client {address}");
                            clients.push(client)
                        }
                        Err(e) => error!(&logger, "Error creating client: {}", e),
                    }
                }
                mux.insert(mac, clients);
            }
            Event::UpdateClient((mac, addr)) => {
                info!(&logger, "Mac existed, but IP updated: {mac}, {addr}");
            }
            Event::PacketReceived(rxpk, gateway_mac) => {
                info!(&logger, "Uplink Received {rxpk:?}");
                if let Some(clients) = mux.get_mut(&gateway_mac) {
                    for sender in clients {
                        debug!(&logger, "Forwarding Uplink");
                        let mut packet = push_data::Packet::from_rxpk(rxpk.clone());
                        packet.gateway_mac = gateway_mac;
                        sender.send(packet.into()).await?;
                    }
                }
            }
            Event::NoClientWithMac(_packet, mac) => {
                warn!(&logger, "Downlink sent but unknown mac: {mac:?}");
            }
        }
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "gwmp-mux", about = "Multiplexer for Semtech GWMP over UDP")]
pub struct Opt {
    /// port to host the service on
    #[structopt(long, default_value = "1681")]
    pub host: u16,
    /// addresses to be clients to (eg: 127.0.0.1:1680)
    /// WARNING: all addresses will receive all ACKs for transmits
    #[structopt(long, default_value = "127.0.0.1:1680")]
    pub client: Vec<String>,

    /// Log level to show (default info)
    #[structopt(parse(from_str = parse_log), default_value = "info")]
    pub log_level: slog::Level,

    #[structopt(long)]
    pub disable_timestamp: bool,
}

async fn spawn_client_instance(
    shutdown_signal: triggered::Listener,
    client_tx: server_runtime::ClientTx,
    logger: slog::Logger,
    mac_address: MacAddress,
    host: String,
) -> Result<tokio::sync::mpsc::Sender<semtech_udp::Packet>, Box<dyn std::error::Error>> {
    let outbound = SocketAddr::from(([0, 0, 0, 0], 0));
    let socket = SocketAddr::from_str(&host)?;
    info!(
        &logger,
        "Connecting to server {socket} from port {} on behalf of {mac_address}",
        outbound.port()
    );
    let udp_runtime = client_runtime::UdpRuntime::new(mac_address, outbound, socket).await?;

    let (receiver, sender) = (udp_runtime.subscribe(), udp_runtime.publish_to());

    tokio::spawn(async move {
        udp_runtime.run().await.unwrap();
    });
    let local_logger = logger.clone();
    let return_sender = sender.clone();
    tokio::spawn(async move {
        tokio::select!(
             _ = shutdown_signal => info!(&local_logger, "Shutting down client instance"),
                res = client_instance(receiver, sender, client_tx, logger, mac_address, host) => {
                if let Err(e) = res {
                    error!(local_logger, "Error with client instance: {e:?}");
                }
        }

        )
    });
    Ok(return_sender)
}

async fn client_instance(
    mut receiver: tokio::sync::broadcast::Receiver<RxMessage>,
    sender: tokio::sync::mpsc::Sender<RxMessage>,
    mut client_tx: server_runtime::ClientTx,
    logger: slog::Logger,
    mac_address: MacAddress,
    host: String,
) -> Result<tokio::sync::mpsc::Sender<semtech_udp::Packet>, Box<dyn std::error::Error>> {
    let uplink_sender = sender.clone();
    tokio::spawn(async move {
        loop {
            let msg = receiver.recv().await.unwrap();
            match msg {
                semtech_udp::Packet::Down(down) => {
                    if let semtech_udp::Down::PullResp(packet) = down {
                        info!(
                            &logger,
                            "Sending Downlink from {host} to {mac_address}: {:?}", packet.data.txpk
                        );
                        let txpk = packet.data.txpk.clone();
                        let prepared_send = client_tx.prepare_downlink(Some(txpk), mac_address);
                        let sender = sender.clone();
                        let logger = slog_scope::logger().new(o!());
                        tokio::spawn(async move {
                            let packet = match prepared_send
                                .dispatch(Some(Duration::from_secs(5)))
                                .await
                            {
                                Err(Error::Ack(e)) => {
                                    error!(&logger, "Error Downlinking to {mac_address}: {:?}", e);
                                    Some((*packet).into_nack_with_error_for_gateway(e, mac_address))
                                }
                                Err(Error::SendTimeout) => {
                                    warn!(&logger, "Gateway {mac_address} did not ACK or NACK. Packet forward may not be connected?");
                                    Some((*packet).into_nack_with_error_for_gateway(
                                        tx_ack::Error::SendFail,
                                        mac_address,
                                    ))
                                }
                                Ok(()) => {
                                    debug!(&logger, "Downlink to {mac_address} successful");
                                    Some((*packet).into_ack_for_gateway(mac_address))
                                }
                                Err(e) => {
                                    error!(&logger, "Unhandled downlink error: {:?}", e);
                                    None
                                }
                            };
                            if let Some(packet) = packet {
                                sender.send(packet.into()).await.unwrap();
                            }
                        });
                    }
                }
                semtech_udp::Packet::Up(_up) => panic!("Should not receive Semtech up frames"),
            }
        }
    });

    Ok(uplink_sender)
}

/// An empty timestamp function for when timestamp should not be included in
/// the output.
fn timestamp_none(_io: &mut dyn io::Write) -> io::Result<()> {
    Ok(())
}

fn mk_logger(log_level: slog::Level, disable_timestamp: bool) -> Logger {
    let decorator = slog_term::PlainDecorator::new(io::stdout());
    let timestamp = if !disable_timestamp {
        slog_term::timestamp_local
    } else {
        timestamp_none
    };
    let drain = slog_term::FullFormat::new(decorator)
        .use_custom_timestamp(timestamp)
        .build()
        .fuse();
    let async_drain = slog_async::Async::new(drain)
        .build()
        .filter_level(log_level)
        .fuse();
    slog::Logger::root(async_drain, o!())
}

fn parse_log(src: &str) -> slog::Level {
    src.parse().unwrap()
}
