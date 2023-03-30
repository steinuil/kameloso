use serde::de::DeserializeOwned;
use std::{io, path::Path};

use crate::kopipe::Kopipe;
use crate::mpv_error::IpcError;
use crate::mpv_reactor::Reactor;

// For pictures:
//   video-add <url>
//   get_property track-list
// To delete:
//   set_property vid <id>
//   video-remove <id>

// handle playlists

// Replies to commands are defined here, search for &cmd->result
// https://github.com/mpv-player/mpv/blob/master/player/command.c
pub mod reply {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    pub struct Loadfile {
        pub playlist_entry_id: i64,
    }
}

pub struct OverlayAddOptions {
    pub id: u8,
    pub x: i32,
    pub y: i32,
    pub file: String,
    pub offset: usize,
    pub w: u32,
    pub h: u32,
}

#[derive(Debug)]
pub struct MpvIpc {
    reactor: Reactor,
}

impl MpvIpc {
    pub async fn connect<P: AsRef<Path> + ?Sized>(path: &P) -> io::Result<Self> {
        Ok(MpvIpc {
            reactor: Reactor::start(Kopipe::open(path).await?).await,
        })
    }

    async fn command_reply<T: DeserializeOwned>(&mut self, cmd: &[&str]) -> Result<T, IpcError> {
        match self.reactor.send_command(cmd).await {
            Ok(v) => serde_json::from_value(v).map_err(IpcError::InvalidResponse),
            Err(e) => Err(e),
        }
    }

    async fn command_empty(&mut self, cmd: &[&str]) -> Result<(), IpcError> {
        let _ = self.reactor.send_command(cmd).await;
        Ok(())
    }

    pub async fn loadfile(&mut self, url: &str) -> Result<reply::Loadfile, IpcError> {
        self.command_reply(&["loadfile", url]).await
    }

    pub async fn loadfile_append_play(&mut self, url: &str) -> Result<(), IpcError> {
        self.command_empty(&["loadfile", url, "append-play"]).await
    }

    pub async fn get_playlist(&mut self) -> Result<serde_json::Value, IpcError> {
        self.command_reply(&["get_property", "playlist"]).await
    }

    pub async fn playlist_next(&mut self) -> Result<serde_json::Value, IpcError> {
        self.command_reply(&["playlist-next"]).await
    }

    pub async fn overlay_add(
        &mut self,
        opts: &OverlayAddOptions,
    ) -> Result<serde_json::Value, IpcError> {
        self.command_reply(&[
            "overlay-add",
            &opts.id.to_string(),
            &opts.x.to_string(),
            &opts.y.to_string(),
            &opts.file,
            &opts.offset.to_string(),
            "bgra",
            &opts.w.to_string(),
            &opts.h.to_string(),
            &(opts.w * 4).to_string(),
        ])
        .await
    }

    pub async fn overlay_remove(&mut self, id: u8) -> Result<serde_json::Value, IpcError> {
        self.command_reply(&["overlay-remove", &id.to_string()])
            .await
    }
}
