use futures::{StreamExt, TryStreamExt};
use serde::Serialize;
use tokio::io::AsyncWriteExt;
use warp::http::StatusCode;
use warp::multipart::FormData;
use warp::{reply, Buf};

use crate::{
    mpv::{Error as IpcError, LoadFileOptions},
    server_state::ServerState,
};

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

impl From<IpcError> for ApiError {
    fn from(value: IpcError) -> Self {
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

impl warp::reject::Reject for IpcError {}

impl warp::Reply for IpcError {
    fn into_response(self) -> reply::Response {
        let err: ApiError = self.into();
        warp::reply::with_status(warp::reply::json(&err), err.status).into_response()
    }
}

pub async fn enqueue_url(
    enqueue_url: EnqueueUrl,
    state: ServerState,
) -> Result<impl warp::Reply, warp::Rejection> {
    if enqueue_url.url.trim().is_empty() {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            message: "attempted to enqueue empty url".to_string(),
        }
        .into());
    }

    state
        .ipc
        .load_file(&enqueue_url.url, &LoadFileOptions::AppendPlay)
        .await?;

    Ok(warp::reply::with_status(
        warp::reply::with_header(warp::reply(), "Location", "/"),
        StatusCode::SEE_OTHER,
    ))
}

// TODO add error messages
pub async fn upload_file(
    mut form: FormData,
    state: ServerState,
) -> Result<impl warp::Reply, warp::Rejection> {
    while let Some(Ok(p)) = form.next().await {
        if p.name() != "file" {
            return Err(warp::reject::reject());
        }

        let filename = p.filename().ok_or(warp::reject::reject())?;

        let filename = &state
            .upload_dir
            .join(format!("{}-{}", uuid::Uuid::new_v4(), filename));

        {
            let mut out = tokio::fs::File::create(&filename)
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
            .ipc
            .load_file(
                filename.to_str().expect("invalid path"),
                &LoadFileOptions::AppendPlay,
            )
            .await?;
    }

    Ok(warp::reply::with_status(
        warp::reply::with_header(warp::reply(), "Location", "/"),
        StatusCode::SEE_OTHER,
    ))
}

pub async fn get_playlist(state: ServerState) -> Result<impl warp::Reply, warp::Rejection> {
    let playlist = state.ipc.get_playlist().await?;

    Ok(warp::reply::json(&playlist))
}

pub async fn playlist_next(state: ServerState) -> Result<impl warp::Reply, warp::Rejection> {
    state.ipc.playlist_next().await?;

    Ok(warp::reply::with_status(
        warp::reply::with_header(warp::reply(), "Location", "/"),
        StatusCode::SEE_OTHER,
    ))
}

#[derive(Debug, Clone, Serialize)]
pub struct CurrentFileInfo {
    pub duration_ms: f64,
    pub position_ms: f64,
    pub is_paused: bool,
}

pub async fn current_file_info(state: ServerState) -> Result<impl warp::Reply, warp::Rejection> {
    let (duration_ms, position_ms, is_paused) = match futures::join!(
        state.ipc.get_duration_ms(),
        state.ipc.get_time_pos_ms(),
        state.ipc.get_paused()
    ) {
        (Ok(duration_ms), Ok(position_ms), Ok(is_paused)) => (duration_ms, position_ms, is_paused),
        _ => return Err(warp::reject()),
    };

    let info = CurrentFileInfo {
        duration_ms,
        position_ms,
        is_paused,
    };

    Ok(warp::reply::json(&info))
}
