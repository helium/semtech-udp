use thiserror::Error;
use tokio::sync::mpsc;

#[derive(Error, Debug)]
pub enum Error {
    #[error("semtech udp error")]
    SemtechUdp(#[from] crate::packet::Error),
    #[error("tokio::mpsc send error")]
    SendError(#[from] mpsc::error::SendError<super::TxMessage>),
    #[error("std::io::Error")]
    IoError(#[from] std::io::Error),
}
