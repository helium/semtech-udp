use super::{Event, InternalEvent};
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

#[derive(Error, Debug)]
pub enum Error {
    #[error("ACK receive channel unexpectedly closed due to dropped sender")]
    AckChannelRecv(mpsc::error::RecvError),
    #[error("Ack Error received from gateway")]
    AckError(crate::packet::tx_ack::Error),
    #[error("Send has timed out")]
    SendTimeout,
    #[error("Dispatch called with no packet")]
    DispatchWithNoSendPacket,
    #[error("Client requested to transmit to unknown MAC")]
    UnknownMac,
    #[error("Io Error from using UDP")]
    UdpError(std::io::Error),
    #[error("ClientEventQueue Full")]
    ClientEventQueueFull(Box<mpsc::error::SendError<Event>>),
    #[error("Internal queue closed or full")]
    InternalQueueClosedOrFull,
    #[error("Semtech UDP error")]
    SemtechUdp(crate::packet::Error),
    #[error("error receiving ACK")]
    AckRecvError,
    #[error("error sending ACK")]
    ErrorSendingAck,
}

impl From<tokio::time::error::Elapsed> for Error {
    fn from(_err: tokio::time::error::Elapsed) -> Error {
        Error::SendTimeout
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::UdpError(err)
    }
}

impl From<mpsc::error::SendError<Event>> for Error {
    fn from(err: mpsc::error::SendError<Event>) -> Error {
        Error::ClientEventQueueFull(err.into())
    }
}

impl From<mpsc::error::SendError<InternalEvent>> for Error {
    fn from(_err: mpsc::error::SendError<InternalEvent>) -> Error {
        Error::InternalQueueClosedOrFull
    }
}

impl From<crate::packet::Error> for Error {
    fn from(err: crate::packet::Error) -> Error {
        Error::SemtechUdp(err)
    }
}

impl From<mpsc::error::RecvError> for Error {
    fn from(e: mpsc::error::RecvError) -> Self {
        Error::AckChannelRecv(e)
    }
}

impl From<crate::packet::tx_ack::Error> for Error {
    fn from(e: crate::packet::tx_ack::Error) -> Self {
        Error::AckError(e)
    }
}

impl From<oneshot::error::RecvError> for Error {
    fn from(_: oneshot::error::RecvError) -> Self {
        Error::AckRecvError
    }
}
