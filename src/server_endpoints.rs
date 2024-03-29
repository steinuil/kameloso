use std::ffi::OsStr;
use std::path::Path;

use futures::TryStreamExt;
use serde::Serialize;
use tokio::io::AsyncWriteExt;
use warp::http::StatusCode;
use warp::multipart::FormData;
use warp::{reply, Buf};

use crate::mpv_error;
use crate::server_state::ServerState;

use self::request::EnqueueUrl;

mod request {
    use serde::Deserialize;

    #[derive(Debug, Clone, Deserialize)]
    pub struct EnqueueUrl {
        pub url: String,
    }
}

fn serialize_status_code<S: serde::Serializer>(
    code: &StatusCode,
    ser: S,
) -> Result<S::Ok, S::Error> {
    ser.serialize_u16(code.as_u16())
}

#[derive(Clone, Debug, Serialize)]
pub struct ApiError {
    #[serde(serialize_with = "serialize_status_code")]
    pub status: StatusCode,
    pub message: String,
}

impl warp::reject::Reject for ApiError {}

impl warp::Reply for ApiError {
    fn into_response(self) -> reply::Response {
        warp::reply::with_status(warp::reply::json(&self), self.status).into_response()
    }
}

impl From<mpv_error::IpcError> for ApiError {
    fn from(value: mpv_error::IpcError) -> Self {
        // match value {
        //     mpv_ipc::IpcError::MpvError(_) => todo!(),
        //     mpv_ipc::IpcError::Transport(_) => todo!(),
        //     mpv_ipc::IpcError::Handler(_) => todo!(),
        //     mpv_ipc::IpcError::InvalidResponse(_) => todo!(),
        // }

        ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: value.to_string(),
        }
    }
}

pub async fn enqueue_url(
    enqueue_url: EnqueueUrl,
    state: ServerState,
) -> Result<impl warp::Reply, warp::Rejection> {
    state
        .ipc()
        .await
        .loadfile_append_play(&enqueue_url.url)
        .await
        .map_err(ApiError::from)?;

    Ok(warp::reply::with_status(
        warp::reply::with_header(warp::reply(), "Location", "/"),
        StatusCode::SEE_OTHER,
    ))
}

pub async fn upload_file(
    form: FormData,
    state: ServerState,
) -> Result<impl warp::Reply, warp::Rejection> {
    let parts: Vec<_> = form
        .try_collect()
        .await
        .map_err(|_| warp::reject::reject())?;

    for p in parts {
        if p.name() == "file" {
            let extension = match p.filename() {
                None => return Err(warp::reject::reject()),
                Some(fname) => Path::new(fname)
                    .extension()
                    .and_then(OsStr::to_str)
                    .ok_or_else(warp::reject::reject)?,
            };

            let out_filename = format!("/tmp/kameloso/{}.{}", uuid::Uuid::new_v4(), extension);

            {
                let mut out = tokio::fs::File::create(&out_filename)
                    .await
                    .map_err(|_| warp::reject::reject())?;

                let mut stream = p.stream();
                loop {
                    match stream
                        .try_next()
                        .await
                        .map_err(|_| warp::reject::reject())?
                    {
                        None => break,
                        Some(chunk) => {
                            out.write_all(chunk.chunk())
                                .await
                                .map_err(|_| warp::reject::reject())?;
                        }
                    }
                }
            }

            state
                .ipc()
                .await
                .loadfile_append_play(&out_filename)
                .await
                .map_err(|_| warp::reject::reject())?;
        } else {
            return Err(warp::reject::reject());
        }
    }

    Ok(warp::reply::with_status(
        warp::reply::with_header(warp::reply(), "Location", "/"),
        StatusCode::SEE_OTHER,
    ))
}

pub async fn get_playlist(state: ServerState) -> Result<impl warp::Reply, warp::Rejection> {
    let playlist = state
        .ipc()
        .await
        .get_playlist()
        .await
        .map_err(ApiError::from)?;

    Ok(warp::reply::json(&playlist))
}

pub async fn playlist_next(state: ServerState) -> Result<impl warp::Reply, warp::Rejection> {
    state
        .ipc()
        .await
        .playlist_next()
        .await
        .map_err(ApiError::from)?;

    Ok(warp::reply::with_status(
        warp::reply::with_header(warp::reply(), "Location", "/"),
        StatusCode::SEE_OTHER,
    ))
}
