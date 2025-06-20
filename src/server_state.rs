use std::{path::PathBuf, sync::Arc};

use tokio::sync::Mutex;

use crate::{mpv::Client, qr::QrCodeParams};

#[derive(Debug, Clone)]
pub struct ServerState {
    pub ipc: Client,
    pub serve_dir: PathBuf,
    pub upload_dir: PathBuf,
    pub qr_code_params: Arc<Mutex<QrCodeParams>>,
}
