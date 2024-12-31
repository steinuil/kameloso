mod kopipe;
mod mpv;
mod qr;
mod server_endpoints;
mod server_hyper;
mod server_state;

use clap::Parser;
use std::{net::SocketAddr, path::PathBuf};
use tokio::{fs, sync::mpsc};

#[derive(Debug, Parser)]
#[command(version)]
struct CliOptions {
    #[arg(long, default_value = "mpv")]
    pub mpv_path: String,

    #[arg(long, default_value = "public")]
    pub serve_dir: PathBuf,

    #[arg(long, default_value = "0.0.0.0:8080")]
    pub bind_address: String,

    #[arg(long, default_value = "kameloso-interactions.log")]
    pub interactions_log: PathBuf,

    #[arg(long, default_value = "uploads")]
    pub upload_dir: PathBuf,
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
    env_logger::init();

    let mut opts: CliOptions = CliOptions::parse();

    let bind_address: SocketAddr = opts.bind_address.parse().unwrap();
    let local_ip = local_ip_address::local_ip().unwrap();

    let _ = fs::create_dir(&opts.media_dir).await;

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

    opts.serve_dir = fs::canonicalize(opts.serve_dir)
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

    let mut mpv_process = tokio::process::Command::new(opts.mpv_path)
        .arg(&format!(
            "--input-ipc-server={}",
            mpv_socket_path.to_string_lossy()
        ))
        .arg("--force-window")
        .arg("--idle")
        .arg("--keep-open")
        .arg("--keep-open-pause=no")
        .arg("--ytdl-format=best*")
        .spawn()
        .expect("Could not start mpv");

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let (commands_tx, commands_rx) = mpsc::unbounded_channel();

    let mpv_pipe = kopipe::open(mpv_socket_path).await.unwrap();

    let mpv_ipc = mpv::Client::new(commands_tx);

    let reactor_handle = tokio::spawn(mpv::reactor::start(mpv_pipe, commands_rx));

    {
        let qr_code_address = format!("http://{}:{}", local_ip, bind_address.port());

        let qr_code = qrcode::QrCode::new(qr_code_address.as_bytes()).unwrap();

        let qr_code_path = runtime_dir.join("qr-code.bgra");

        {
            let f = fs::File::create(&qr_code_path).await.unwrap();
            let mut out = tokio::io::BufWriter::new(f);

            qr::write_bgra(&qr_code, 4, &mut out).await.unwrap();
        }

        mpv_ipc
            .overlay_add(&mpv::OverlayAddOptions {
                id: 3,
                x: 20,
                y: 20,
                file: qr_code_path.to_string_lossy().to_string(),
                w: (qr_code.width() as u32 + 2) * 4,
                h: (qr_code.width() as u32 + 2) * 4,
                offset: 0,
            })
            .await
            .unwrap();
    }

    let server_handle = tokio::spawn(server_hyper::start(
        bind_address,
        server_state::ServerState {
            ipc: mpv_ipc,
            serve_dir: opts.serve_dir,
            media_dir: opts.media_dir,
        },
    ));

    let _ = mpv_process.wait().await;

    server_handle.abort();
    reactor_handle.abort();
}
