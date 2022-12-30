mod kopipe;
mod mpv_ipc;
mod server;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(version)]
struct CliOptions {
    #[arg(long, default_value = "mpv")]
    pub mpv_path: String,

    #[arg(long, default_value = "/tmp/kameloso-mpv-socket")]
    pub mpv_socket_path: String,

    #[arg(long, default_value = "0.0.0.0:8080")]
    pub bind_address: String,
}

#[tokio::main]
async fn main() {
    let opts: CliOptions = CliOptions::parse();

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

    let server = server::new(mpv_ipc);

    tide::log::start();

    tokio::join!(server.listen(opts.bind_address), mpv_process.wait());
}
