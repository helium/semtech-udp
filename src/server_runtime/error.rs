use super::{Event, InternalEvent};
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

#[derive(Error, Debug)]
pub enum Error {
    #[error("ACK receive channel unexpectedly closed due to dropped sender")]
    AckChannelRecv(#[from] mpsc::error::RecvError),
    #[error("Ack Error received from gateway")]
    AckError(#[from] crate::packet::tx_ack::Error),
    #[error("Send has timed out")]
    SendTimeout,
    #[error("Dispatch called with no packet")]
    DispatchWithNoSendPacket,
    #[error("Client requested to transmit to unknown MAC")]
    UnknownMac,
    #[error("Io Error from using UDP")]
    UdpError(#[from] std::io::Error),
    #[error("ClientEventQueue Full")]
    ClientEventQueueFull(#[from] Box<mpsc::error::SendError<Event>>),
    #[error("Internal queue closed or full")]
    InternalQueueClosedOrFull,
    #[error("Semtech UDP error")]
    SemtechUdp(#[from] crate::packet::Error),
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

impl From<oneshot::error::RecvError> for Error {
    fn from(_: oneshot::error::RecvError) -> Self {
        Error::AckRecvError
    }
}
