# kamelåså

`kameloso` is a LAMJP (Local Area Media Jam Party) application. Think a Chromecast or Spotify jam, but better.

It starts an HTTP server connected to [mpv](https://mpv.io/) and lets you queue video links or upload videos.

## Usage

Install these dependencies:

- [mpv](https://mpv.io/)
- [yt-dlp](https://github.com/yt-dlp/yt-dlp) (optional, but recommended)

For now you have to compile kamelåså yourself, so head to the Building section.

## Building

Install [Rust](https://www.rust-lang.org/), clone this repository and run `cargo build --release`.
