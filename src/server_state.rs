use std::path::PathBuf;

use crate::mpv::Client;

#[derive(Debug, Clone)]
pub struct ServerState {
    pub ipc: Client,
    pub serve_dir: PathBuf,
    pub media_dir: PathBuf,
}
