mod kopipe;
pub mod mpv;
mod qr;
mod server_endpoints;
mod server_hyper;
mod server_state;

use clap::Parser;
use std::{
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
    sync::Arc,
};
use tokio::{
    fs,
    sync::{mpsc, Mutex, RwLock},
};

use crate::mpv::response::PlaylistEntry;

#[derive(Debug, Parser)]
#[command(version)]
struct CliOptions {
    /// Path to the mpv binary.
    #[arg(long, default_value = "mpv")]
    pub mpv_path: String,

    /// Bind the HTTP server to this address.
    #[arg(long, default_value = "0.0.0.0:8080")]
    pub bind_address: SocketAddr,

    /// Directory containing index.html and static that the HTTP server will serve.
    #[arg(long, default_value = "public")]
    pub serve_dir: PathBuf,

    /// Directory that the uploaded files will be saved to.
    #[arg(long, default_value = "uploads")]
    pub upload_dir: PathBuf,

    /// Extra arguments to pass to mpv after --
    #[arg()]
    pub mpv_args: Vec<String>,
}

fn get_runtime_dir_unix() -> PathBuf {
    if let Ok(path) = std::env::var("KAMELOSO_SOCKET_PATH") {
        return PathBuf::from(path);
    }

    if let Ok(path) = std::env::var("XDG_RUNTIME_DIR") {
        return PathBuf::from(path).join("kameloso");
    }

    PathBuf::from("/tmp/kameloso")
}

fn get_tmpdir_windows() -> PathBuf {
    if let Ok(path) = std::env::var("TMP") {
        return PathBuf::from(path);
    }

    if let Ok(path) = std::env::var("TEMP") {
        return PathBuf::from(path);
    }

    if let Ok(path) = std::env::var("USERPROFILE") {
        return PathBuf::from(path);
    }

    panic!("could not find temp dir on Windows");
}

fn get_socket_path_windows() -> PathBuf {
    if let Ok(path) = std::env::var("KAMELOSO_SOCKET_PATH") {
        return PathBuf::from(path);
    }

    // https://learn.microsoft.com/en-us/windows/win32/ipc/pipe-names
    PathBuf::from(r#"\\.\pipe\kameloso-mpv-socket"#)
}

#[tokio::main]
async fn main() {
    // Default to info log level
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let opts: CliOptions = CliOptions::parse();

    let _ = fs::create_dir(&opts.upload_dir).await;

    let runtime_dir = if cfg!(unix) {
        get_runtime_dir_unix()
    } else {
        get_tmpdir_windows().join("kameloso")
    };

    let _ = fs::create_dir(&runtime_dir).await;

    let mpv_socket_path = if cfg!(unix) {
        runtime_dir.join("mpv-socket")
    } else {
        get_socket_path_windows()
    };

    let serve_dir = fs::canonicalize(opts.serve_dir)
        .await
        .expect("serve dir doesn't exist or cannot be accessed");

    let mpv_socket_path = std::path::Path::new(&mpv_socket_path);

    if mpv_socket_path.exists() {
        log::warn!("mpv socket already exists, trying to remove...");

        match fs::remove_file(mpv_socket_path).await {
            Ok(()) => log::info!("Cleaned up old socket"),
            Err(_) => {
                log::error!("Failed to clean up socket");
                std::process::exit(1);
            }
        }
    }

    let mut mpv_cmd = tokio::process::Command::new(opts.mpv_path);

    mpv_cmd
        .arg(format!(
            "--input-ipc-server={}",
            mpv_socket_path.to_string_lossy()
        ))
        .arg("--force-window")
        .arg("--idle")
        .arg("--keep-open")
        .arg("--keep-open-pause=no")
        .arg("--no-pause")
        .args(opts.mpv_args);

    let mut mpv_process = mpv_cmd.spawn().expect("Could not start mpv");

    let (commands_tx, commands_rx) = mpsc::unbounded_channel();

    let mpv_ipc = mpv::Client::new(commands_tx);

    let mpv_pipe = kopipe::open_retry(mpv_socket_path, 10).await.unwrap();

    let reactor_handle = tokio::spawn(mpv::reactor::start(mpv_pipe, commands_rx));

    let local_ip =
        local_ip_address::local_ip().unwrap_or(std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    let qr_code_address = format!("http://{}:{}", local_ip, opts.bind_address.port());

    let qr_code_path = runtime_dir.join("qr-code.bgra");
    let magnification = 4;

    let qr_code_width = qr::generate_qr_code(&qr_code_address, &qr_code_path, magnification)
        .await
        .unwrap();

    let qr_code_params = qr::QrCodeParams {
        path: qr_code_path.to_string_lossy().to_string(),
        magnification,
        width: qr_code_width,
        active: true,
    };

    qr::add_qr_code_overlay(&mpv_ipc, &qr_code_params)
        .await
        .unwrap();

    let playlist: Arc<RwLock<Vec<PlaylistEntry>>> = Arc::new(RwLock::new(vec![]));
    let mut data_stream = mpv_ipc.observe_property("playlist").await.unwrap();

    tokio::spawn({
        let playlist = playlist.clone();
        async move {
            while let Some(p) = data_stream.recv().await {
                if let Ok(v) = serde_json::from_value::<Vec<PlaylistEntry>>(p) {
                    log::info!("playlist: {v:?}");
                    *playlist.write().await = v;
                } else {
                    log::error!("failed to decode playlist")
                }
            }
        }
    });

    let server_handle = tokio::spawn(server_hyper::start(
        opts.bind_address,
        server_state::ServerState {
            ipc: mpv_ipc,
            serve_dir,
            upload_dir: opts.upload_dir,
            qr_code_params: Arc::new(Mutex::new(qr_code_params)),
            playlist,
        },
    ));

    let _ = reactor_handle.await;
    server_handle.abort();
    let _ = mpv_process.wait().await;
    // let _ = mpv_process.kill().await;
}
