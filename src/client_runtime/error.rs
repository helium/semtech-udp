use thiserror::Error;
use tokio::sync::mpsc;

#[derive(Error, Debug)]
pub enum Error {
    #[error("semtech udp error")]
    SemtechUdpError(crate::packet::Error),
    #[error("tokio::mpsc send error")]
    SendError(mpsc::error::SendError<super::TxMessage>),
    #[error("std::io::Error")]
    IoError(std::io::Error),
}

impl From<crate::packet::Error> for Error {
    fn from(err: crate::packet::Error) -> Error {
        Error::SemtechUdpError(err)
    }
}

impl From<mpsc::error::SendError<super::TxMessage>> for Error {
    fn from(err: mpsc::error::SendError<super::TxMessage>) -> Error {
        Error::SendError(err)
    }
}
impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::IoError(err)
    }
}
