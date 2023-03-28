use std::{path::PathBuf, sync::Arc};
use tokio::sync::{Mutex, MutexGuard};

use crate::mpv_ipc::MpvIpc;

#[derive(Debug, Clone)]
pub struct ServerState {
    pub ipc: Arc<Mutex<MpvIpc>>,
    pub serve_dir: PathBuf,
}

impl ServerState {
    pub fn new(mpv_ipc: MpvIpc, serve_dir: PathBuf) -> Self {
        ServerState {
            ipc: Arc::new(Mutex::new(mpv_ipc)),
            serve_dir,
        }
    }

    pub async fn ipc(&self) -> MutexGuard<MpvIpc> {
        self.ipc.lock().await
    }
}
