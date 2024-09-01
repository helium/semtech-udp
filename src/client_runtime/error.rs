use thiserror::Error;
use tokio::sync::mpsc;

#[derive(Error, Debug)]
pub enum Error {
    #[error("semtech udp error: {0}")]
    SemtechUdp(#[from] crate::packet::Error),
    #[error("tokio::mpsc send error: {0}")]
    SendError(#[from] mpsc::error::SendError<super::TxMessage>),
    #[error("Error binding: {io_error}")]
    Binding { io_error: std::io::Error },
    #[error("Error connecting: {io_error}")]
    Connection { io_error: std::io::Error },
    #[error("Join error: {0}")]
    Join(#[from] tokio::task::JoinError),
    #[error("Error sending downlink request to client: {0}")]
    SendingClient(#[from] mpsc::error::SendError<super::Event>),
}
