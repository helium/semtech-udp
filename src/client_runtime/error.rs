use thiserror::Error;
use tokio::sync::mpsc;

#[derive(Error, Debug)]
pub enum Error {
    #[error("semtech udp error")]
    SemtechUdp(#[from] crate::packet::Error),
    #[error("tokio::mpsc send error")]
    SendError(#[from] mpsc::error::SendError<super::TxMessage>),
    #[error("Error binding: {io_error}")]
    Binding { io_error: std::io::Error },
    #[error("Error connecting: {io_error}")]
    Connection { io_error: std::io::Error },
}
