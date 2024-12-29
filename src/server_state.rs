use std::path::PathBuf;

use crate::mpv::Client;

#[derive(Debug, Clone)]
pub struct ServerState {
    pub ipc: Client,
    pub serve_dir: PathBuf,
}

impl ServerState {
    pub fn new(mpv_ipc: Client, serve_dir: PathBuf) -> Self {
        ServerState {
            ipc: mpv_ipc,
            serve_dir,
        }
    }
}
