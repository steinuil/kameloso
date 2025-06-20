use serde::de::DeserializeOwned;
use tokio::sync::mpsc::UnboundedSender;

use super::{
    error::Error,
    reactor::{self, CommandWithHandler},
};

use self::response::*;

// For pictures:
//   video-add <url>
//   get_property track-list
// To delete:
//   set_property vid <id>
//   video-remove <id>

// Replies to commands are defined here, search for &cmd->result
// https://github.com/mpv-player/mpv/blob/master/player/command.c
pub mod response {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Deserialize)]
    pub struct LoadFile {
        pub playlist_entry_id: i64,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct PlaylistEntry {
        pub id: u64,
        pub filename: String,
        pub title: Option<String>,
        #[serde(default)]
        pub current: bool,
        #[serde(default)]
        pub playing: bool,
    }
}

#[derive(Debug, Clone)]
pub struct OverlayAddOptions {
    pub id: u8,
    pub x: i32,
    pub y: i32,
    pub file: String,
    pub offset: usize,
    pub w: u32,
    pub h: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadFileOptions {
    Replace,
    Append,
    AppendPlay,
    InsertNext,
    InsertNextPlay,
    InsertAt(u64),
    InsertAtPlay(u64),
}

#[derive(Debug, Clone)]
pub struct Client {
    commands_tx: UnboundedSender<CommandWithHandler>,
}

impl Client {
    pub fn new(commands_tx: UnboundedSender<CommandWithHandler>) -> Self {
        Client { commands_tx }
    }

    async fn command_reply_json<T: DeserializeOwned>(
        &self,
        cmd: serde_json::Value,
    ) -> Result<T, Error> {
        let future = reactor::send_command(cmd, &self.commands_tx)
            .await
            .map_err(|_| Error::CommandsChannelClosed)?;

        let response = future.await?.map_err(Error::Mpv)?;

        serde_json::from_value(response).map_err(Error::InvalidResponse)
    }

    async fn command_reply<T: DeserializeOwned>(&self, cmd: &[&str]) -> Result<T, Error> {
        let cmd = cmd.iter().map(|s| s.to_string()).collect();

        self.command_reply_json(cmd).await
    }

    pub async fn load_file(&self, url: &str, options: &LoadFileOptions) -> Result<LoadFile, Error> {
        match options {
            LoadFileOptions::Replace => self.command_reply(&["loadfile", url, "replace"]).await,
            LoadFileOptions::Append => self.command_reply(&["loadfile", url, "append"]).await,
            LoadFileOptions::AppendPlay => {
                self.command_reply(&["loadfile", url, "append-play"]).await
            }
            LoadFileOptions::InsertNext => {
                self.command_reply(&["loadfile", url, "insert-next"]).await
            }
            LoadFileOptions::InsertNextPlay => {
                self.command_reply(&["loadfile", url, "insert-next-play"])
                    .await
            }
            LoadFileOptions::InsertAt(index) => {
                self.command_reply(&["loadfile", url, "insert-at", &index.to_string()])
                    .await
            }
            LoadFileOptions::InsertAtPlay(index) => {
                self.command_reply(&["loadfile", url, "insert-at-play", &index.to_string()])
                    .await
            }
        }
    }

    pub async fn get_playlist(&self) -> Result<Vec<PlaylistEntry>, Error> {
        self.command_reply(&["get_property", "playlist"]).await
    }

    pub async fn playlist_next(&self) -> Result<(), Error> {
        self.command_reply(&["playlist-next"]).await
    }

    pub async fn overlay_add(&self, opts: &OverlayAddOptions) -> Result<(), Error> {
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

    pub async fn overlay_remove(&self, id: u8) -> Result<serde_json::Value, Error> {
        self.command_reply(&["overlay-remove", &id.to_string()])
            .await
    }

    pub async fn get_duration_ms(&self) -> Result<f64, Error> {
        self.command_reply(&["get_property", "duration/full"]).await
    }

    pub async fn get_time_pos_ms(&self) -> Result<f64, Error> {
        self.command_reply(&["get_property", "time-pos/full"]).await
    }

    pub async fn get_paused(&self) -> Result<bool, Error> {
        self.command_reply(&["get_property", "pause"]).await
    }

    pub async fn observe_property(&self, property: &str) -> Result<(), Error> {
        self.command_reply_json(serde_json::json!(["observe_property", 1, property]))
            .await
    }
}
