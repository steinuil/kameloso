use std::{path::PathBuf, sync::Arc};

use tide::StatusCode;
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

    async fn ipc(&self) -> MutexGuard<MpvIpc> {
        self.ipc.lock().await
    }
}

mod request {
    use serde::Deserialize;

    #[derive(Debug, Clone, Deserialize)]
    pub struct EnqueueUrl {
        pub url: String,
    }
}

async fn enqueue_url(mut req: tide::Request<ServerState>) -> tide::Result<tide::Response> {
    let body: request::EnqueueUrl = req
        .body_form()
        .await
        .map_err(|e| tide::Error::new(400, e.into_inner()))?;

    req.state()
        .ipc()
        .await
        .loadfile_append_play(&body.url)
        .await
        .map_err(|e| tide::Error::new(500, e))?;

    Ok(tide::Response::builder(StatusCode::SeeOther)
        .header("Location", "/")
        .build())
}

async fn playlist(req: tide::Request<ServerState>) -> tide::Result<tide::Response> {
    let playlist = req.state().ipc().await.get_playlist().await?;

    Ok(tide::Response::builder(StatusCode::Ok)
        .body(tide::Body::from_json(&playlist)?)
        .build())
}

async fn playlist_next(req: tide::Request<ServerState>) -> tide::Result<tide::Response> {
    let _ = req.state().ipc().await.playlist_next().await?;

    Ok(tide::Response::builder(StatusCode::SeeOther)
        .header("Location", "/")
        .build())
}

pub fn new(mpv_ipc: MpvIpc, serve_dir: PathBuf) -> tide::Server<ServerState> {
    let mut app = tide::with_state(ServerState::new(mpv_ipc, serve_dir));

    app.at("/api/enqueue").post(enqueue_url);
    app.at("/api/playlist").get(playlist);
    app.at("/api/playlist/next").post(playlist_next);

    app.at("/")
        .get(|req: tide::Request<ServerState>| async move {
            let mut path = req.state().serve_dir.clone();

            path.push("index.html");

            Ok(tide::Response::builder(StatusCode::Ok)
                .header("Content-Type", "text/html")
                .body(tide::Body::from_file(path).await?))
        });

    app.at("/static/*path")
        .get(|req: tide::Request<ServerState>| async move {
            let mut base_path = req.state().serve_dir.clone();
            base_path.push("static");
            let path = base_path.join(req.param("path")?);

            let abs_path = async_std::fs::canonicalize(path).await?;

            if !abs_path.starts_with(base_path) {
                return Ok(tide::Response::new(StatusCode::NotFound));
            }

            Ok(tide::Response::builder(StatusCode::Ok)
                .body(tide::Body::from_file(abs_path).await?)
                .build())
        });

    app
}
