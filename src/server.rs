use std::sync::Arc;

use tide::StatusCode;
use tokio::sync::{Mutex, MutexGuard};

use crate::mpv_ipc::MpvIpc;

#[derive(Debug, Clone)]
pub struct ServerState {
    pub ipc: Arc<Mutex<MpvIpc>>,
}

impl ServerState {
    pub fn new(mpv_ipc: MpvIpc) -> Self {
        ServerState {
            ipc: Arc::new(Mutex::new(mpv_ipc)),
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

pub fn new(mpv_ipc: MpvIpc) -> tide::Server<ServerState> {
    let mut app = tide::with_state(ServerState::new(mpv_ipc));

    app.at("/api/enqueue").post(enqueue_url);
    app.at("/api/playlist").get(playlist);
    app.at("/api/playlist/next").post(playlist_next);

    app.at("/").get(|_| async {
        Ok(tide::Response::builder(StatusCode::Ok)
            .header("Content-Type", "text/html")
            .body(tide::Body::from_file("public/index.html").await?))
    });

    app.at("/static/*path")
        .get(|req: tide::Request<ServerState>| async move {
            let path = req.param("path")?;

            // let abs_path = tokio::fs::canonicalize(path).await?;

            Ok(tide::Response::builder(StatusCode::Ok)
                .body(tide::Body::from_file(&format!("public/static/{}", path)).await?))
        });

    app
}
