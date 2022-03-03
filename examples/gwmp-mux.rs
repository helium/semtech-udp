use semtech_udp::{
    client_runtime::{self, Event as ClientEvent},
    push_data,
    server_runtime::{self, Event as ServerEvent, UdpRuntime},
    tx_ack, MacAddress,
};
use slog::{self, debug, error, info, o, warn, Drain, Logger};
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::str::FromStr;
use structopt::StructOpt;
use tokio::{io::AsyncReadExt, signal, time::Duration};

pub type Result<T = ()> = std::result::Result<T, Error>;

fn main() {
    let cli = Opt::from_args();
    let logger = mk_logger(cli.log_level, cli.disable_timestamp);
    let scope_guard = slog_scope::set_global_logger(logger);
    let logger = slog_scope::logger().new(o!());

    let _log_guard = slog_stdlog::init().unwrap();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let res = runtime.block_on(async move {
        let (shutdown_trigger, shutdown_signal) = triggered::trigger();

        let handle = tokio::spawn(async move {
            let logger = slog_scope::logger().new(o!());
            if let Err(e) = host_and_mux(cli, shutdown_signal).await {
                error!(&logger, "host_and_mux error: {e}")
            }
        });

        watch_for_shutdown().await;
        info!(&logger, "Triggering gwmp-mux shutdown");
        shutdown_trigger.trigger();
        let _ = handle
            .await
            .expect("Error awaiting host_and_mux_sup shutdown");
        info!(&logger, "Shutdown complete");
    });
    runtime.shutdown_timeout(Duration::from_secs(0));
    drop(scope_guard);
    res
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

struct Client {
    shutdown_trigger: triggered::Trigger,
    clients: Vec<(
        client_runtime::ClientTx,
        tokio::task::JoinHandle<std::result::Result<(), Error>>,
    )>,
}

impl Client {
    async fn create(
        mac: MacAddress,
        client_tx: &server_runtime::ClientTx,
        client_list: &[String],
    ) -> Result<Client> {
        let logger = slog_scope::logger().new(o!());
        let mut clients = Vec::new();
        let (shutdown_trigger, shutdown_signal) = triggered::trigger();
        for address in client_list {
            let socket = SocketAddr::from_str(address)?;
            let (sender, receiver, udp_runtime) =
                client_runtime::UdpRuntime::new(mac, socket).await?;
            info!(logger, "Connecting to server {socket} on behalf of {mac}",);
            let handle = tokio::spawn(run_client_instance(
                shutdown_signal.clone(),
                udp_runtime,
                client_tx.clone(),
                receiver,
                mac,
            ));
            clients.push((sender, handle));
        }
        Ok(Client {
            shutdown_trigger,
            clients,
        })
    }

    async fn shutdown(self) -> Result {
        let logger = slog_scope::logger().new(o!());
        self.shutdown_trigger.trigger();
        for (_client, handle) in self.clients {
            if let Err(e) = handle.await {
                error!(logger, "Error awaiting client instance shutdown: {e}")
            }
        }
        Ok(())
    }
}

async fn host_and_mux(cli: Opt, shutdown_signal: triggered::Listener) -> Result {
    let logger = slog_scope::logger().new(o!());
    let addr = SocketAddr::from(([0, 0, 0, 0], cli.host));
    info!(&logger, "Starting server: {addr}");
    let (mut client_rx, client_tx) = UdpRuntime::new(addr).await?.split();

    let mut mux: HashMap<MacAddress, Client> = HashMap::new();
    info!(&logger, "Ready for clients");

    loop {
        let shutdown_signal = shutdown_signal.clone();
        tokio::select!(
             _ = shutdown_signal => {
                info!(&logger, "Awaiting mux-client instances shutdown");
                for (_, client) in mux.into_iter() {
                    client.shutdown().await?;
                }
                info!(&logger, "host_and_mux shutdown complete");
                return Ok(());
            },
            server_event = client_rx.recv() => {
                let mut to_send = None;
                match server_event {
                    ServerEvent::UnableToParseUdpFrame(error, buf) => {
                        error!(logger, "Semtech UDP Parsing Error: {error}");
                        error!(logger, "UDP data: {buf:?}");
                    }
                    ServerEvent::NewClient((mac, addr)) => {
                        info!(logger, "New packet forwarder client: {mac}, {addr}");
                        let client = Client::create(mac, &client_tx, &cli.client).await?;
                        mux.insert(mac, client);
                    }
                    ServerEvent::UpdateClient((mac, addr)) => {
                        info!(logger, "Mac existed, but IP updated: {mac}, {addr}");
                    }
                    ServerEvent::PacketReceived(rxpk, mac) => {
                        info!(logger, "From {mac} received uplink: {rxpk}");
                        to_send = Some(push_data::Packet::from_rxpk(mac, rxpk));
                    }
                    ServerEvent::StatReceived(stat, mac) => {
                        info!(logger, "From {mac} received stat: {stat:?}");
                        to_send = Some(push_data::Packet::from_stat(mac, stat));
                    }
                    ServerEvent::NoClientWithMac(_packet, mac) => {
                        warn!(logger, "Downlink sent but unknown mac: {mac}");
                    }
                    ServerEvent::ClientDisconnected((mac, addr)) => {
                        info!(logger, "{mac} disconnected from {addr}");
                        if let Some(client) = mux.remove(&mac) {
                            client.shutdown().await?;
                        }
                    }
                }
                if let Some(packet) = to_send {
                    if let Some(client) = mux.get_mut(&packet.gateway_mac) {
                        for (sender, _handle) in &client.clients {
                            debug!(logger, "Forwarding Uplink");
                            let logger = logger.clone();
                            let sender = sender.clone();
                            let packet = packet.clone();
                            tokio::spawn ( async move {
                                if let Err(e) = sender.send(packet).await {
                                    error!(logger, "Error sending uplink: {e}")
                                }
                            });
                        }
                    }
                }
            }
        );
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

async fn run_client_instance(
    shutdown_signal: triggered::Listener,
    udp_runtime: client_runtime::UdpRuntime,
    client_tx: server_runtime::ClientTx,
    receiver: client_runtime::ClientRx,
    mac: MacAddress,
) -> Result {
    let logger = slog_scope::logger().new(o!());

    let runtime = tokio::spawn(udp_runtime.run(shutdown_signal.clone()));
    let receive = tokio::spawn(run_client_instance_handle_downlink(
        mac, receiver, client_tx,
    ));
    tokio::select!(
        _ = shutdown_signal =>
            info!(&logger, "Shutting down client instance {mac}"),
        resp = runtime => if let Err(e) = resp {
            error!(&logger, "Error in client instance {mac} udp_runtime: {e}")
        },
        resp = receive => if let Err(e) = resp {
            error!(&logger, "Error in client instance {mac} receiver: {e}")
        }
    );

    Ok(())
}

async fn run_client_instance_handle_downlink(
    mac: semtech_udp::MacAddress,
    mut receiver: client_runtime::ClientRx,
    mut client_tx: server_runtime::ClientTx,
) -> Result {
    let logger = slog_scope::logger().new(o!());

    while let Some(client_event) = receiver.recv().await {
        match client_event {
            ClientEvent::DownlinkRequest(downlink_request) => {
                let prepared_send =
                    client_tx.prepare_downlink(Some(downlink_request.txpk().clone()), mac);
                let logger = logger.clone();
                tokio::spawn(async move {
                    if let Err(e) =
                        match prepared_send.dispatch(Some(Duration::from_secs(15))).await {
                            Err(server_runtime::Error::Ack(e)) => {
                                error!(&logger, "Error Downlinking to {mac}: {:?}", e);
                                downlink_request.nack(e).await
                            }
                            Err(server_runtime::Error::SendTimeout) => {
                                warn!(
                        &logger,
                        "Gateway {mac} did not ACK or NACK. Packet forward may not be connected?"
                    );
                                downlink_request.nack(tx_ack::Error::SendFail).await
                            }
                            Ok(()) => {
                                debug!(&logger, "Downlink to {mac} successful");
                                downlink_request.ack().await
                            }
                            Err(e) => {
                                error!(&logger, "Unhandled downlink error: {:?}", e);
                                Ok(())
                            }
                        }
                    {
                        debug!(&logger, "Error sending downlink to {mac}: {e}");
                    }
                });
            }
            ClientEvent::UnableToParseUdpFrame(parse_error, buffer) => {
                error!(
                    &logger,
                    "Error parsing frame from {mac}: {parse_error}, {buffer:?}"
                );
            }
            ClientEvent::LostConnection => {
                warn!(
                    &logger,
                    "Lost connection to GWMP client {mac}. Dropping frames."
                )
            }
            ClientEvent::Reconnected => {
                warn!(&logger, "Reconnected to GWMP client {mac}")
            }
        }
    }
    Ok(())
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

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("semtech udp server_runtime error: {0}")]
    ServerRuntime(#[from] semtech_udp::server_runtime::Error),
    #[error("semtech udp client_runtime error: {0}")]
    ClientRuntime(#[from] semtech_udp::client_runtime::Error),
    #[error("error parsing socket address: {0}")]
    AddrParse(#[from] std::net::AddrParseError),
    #[error("join error: {0}")]
    Join(#[from] tokio::task::JoinError),
}
