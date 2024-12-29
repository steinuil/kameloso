use std::io;
use tokio::sync::oneshot;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("error response received: {0}")]
    Mpv(String),

    #[error(transparent)]
    Transport(#[from] io::Error),

    #[error(transparent)]
    Handler(#[from] oneshot::error::RecvError),

    #[error(transparent)]
    InvalidResponse(#[from] serde_json::Error),

    #[error("commands channel closed")]
    CommandsChannelClosed,
}
