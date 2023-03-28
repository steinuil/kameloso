mod kopipe;
mod mpv_ipc;
mod server;

use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(version)]
struct CliOptions {
    #[arg(long, default_value = "mpv")]
    pub mpv_path: String,

    #[arg(long, default_value = "/tmp/kameloso-mpv-socket")]
    pub mpv_socket_path: String,

    #[arg(long, default_value = "public")]
    pub serve_dir: PathBuf,

    #[arg(long, default_value = "0.0.0.0:8080")]
    pub bind_address: String,
}

#[tokio::main]
async fn main() {
    let mut opts: CliOptions = CliOptions::parse();

    tide::log::start();

    opts.serve_dir = tokio::fs::canonicalize(opts.serve_dir)
        .await
        .expect("invalid serve dir");

    let mpv_socket_path = std::path::Path::new(&opts.mpv_socket_path);

    if mpv_socket_path.exists() {
        log::warn!("mpv socket already exists, trying to remove...");

        match tokio::fs::remove_file(mpv_socket_path).await {
            Ok(()) => log::info!("Cleaned up old socket"),
            Err(_) => {
                log::error!("Failed to clean up socket");
                std::process::exit(1);
            }
        }
    }

    let mut mpv_process = tokio::process::Command::new(opts.mpv_path)
        .arg(&format!("--input-ipc-server={}", opts.mpv_socket_path))
        .arg("--force-window")
        .arg("--idle")
        .arg("--keep-open")
        .arg("--keep-open-pause=no")
        .arg("--ytdl-format=best[height<=?480]")
        .spawn()
        .expect("Could not start mpv");

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let mpv_ipc = mpv_ipc::MpvIpc::connect(&opts.mpv_socket_path)
        .await
        .expect("Could not connect to mpv socket");

    {
        let qr_code =
            qrcode::QrCode::new(format!("http://{}", opts.bind_address).as_bytes()).unwrap();

        let qr_code_path = runtime_dir.join("qr-code.bgra");

        {
            let f = tokio::fs::File::create(&qr_code_path).await.unwrap();
            let mut out = tokio::io::BufWriter::new(f);

            qr::write_bgra(&qr_code, 4, &mut out).await.unwrap();
        }

        mpv_ipc
            .overlay_add(&mpv_ipc::OverlayAddOptions {
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

    let server = server::new(mpv_ipc, opts.serve_dir);

    let server_handle = tokio::spawn(server.listen(opts.bind_address));

    let _ = mpv_process.wait().await;

    server_handle.abort();
}
