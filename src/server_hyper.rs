use std::net::SocketAddr;
use warp::Filter;

use crate::server_state::ServerState;

fn with_arg<T: std::marker::Send + std::clone::Clone>(
    t: T,
) -> impl Filter<Extract = (T,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || t.clone())
}

pub async fn start(addr: SocketAddr, state: ServerState) {
    let enqueue = warp::path("enqueue")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::form())
        .and(with_arg(state.clone()))
        .and_then(crate::server_endpoints::enqueue_url);

    let upload_file = warp::path("upload")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::multipart::form())
        .and(with_arg(state.clone()))
        .and_then(crate::server_endpoints::upload_file);

    let get_playlist = warp::get()
        .and(with_arg(state.clone()))
        .and_then(crate::server_endpoints::get_playlist);

    let playlist_next = warp::post()
        .and(with_arg(state.clone()))
        .and_then(crate::server_endpoints::playlist_next);

    let playlist = warp::path("playlist").and(
        warp::path::end()
            .and(get_playlist)
            .or(warp::path("next").and(warp::path::end()).and(playlist_next)),
    );

    let api_routes = warp::path("api").and(enqueue.or(upload_file).or(playlist));

    let static_files = warp::path("static").and(warp::fs::dir(state.serve_dir.join("static")));
    let index_html = warp::path::end().and(warp::fs::file(state.serve_dir.join("index.html")));

    let routes = api_routes.or(index_html).or(static_files);

    warp::serve(routes).run(addr).await
}
