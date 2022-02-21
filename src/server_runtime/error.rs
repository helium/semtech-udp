use super::{Event, InternalEvent, SystemTime};
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

#[derive(Error, Debug)]
pub enum Error {
    #[error("Ack Error received from gateway: {0}")]
    Ack(#[from] crate::packet::tx_ack::Error),
    #[error("Send has timed out")]
    SendTimeout,
    #[error("Dispatch called with no packet")]
    DispatchWithNoSendPacket,
    #[error("Client requested to transmit to unknown MAC")]
    UnknownMac,
    #[error("Io Error from using UDP: {0}")]
    UdpError(#[from] std::io::Error),
    #[error("ClientEventQueue Full: {0}")]
    ClientEventQueueFull(#[from] Box<mpsc::error::SendError<Event>>),
    #[error("Internal queue closed or full")]
    InternalQueueClosedOrFull,
    #[error("Semtech UDP error: {0}")]
    SemtechUdp(#[from] crate::packet::Error),
    #[error("error receiving ACK")]
    AckRecv,
    #[error("error sending ACK")]
    AckSend,
    #[error("Join error: {0}")]
    Join(#[from] tokio::task::JoinError),
    #[error("Client was last seen in the future ({last_seen:?}) compared to now ({now:?})")]
    LastSeen {
        last_seen: SystemTime,
        now: SystemTime,
    },
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
        Error::AckRecv
    }
}
