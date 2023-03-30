use std::io;
use thiserror::Error;
use tokio::sync::oneshot;

#[derive(Debug, Error)]
pub enum IpcError {
    #[error("error response received: {0}")]
    MpvError(String),

    #[error(transparent)]
    Transport(#[from] io::Error),

    #[error(transparent)]
    Handler(#[from] oneshot::error::RecvError),

    #[error(transparent)]
    InvalidResponse(#[from] serde_json::Error),
}
