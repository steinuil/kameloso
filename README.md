# kamelåså

`kameloso` is a LAMP (Local Area Media Party) application. Think a Chromecast or Spotify jam, but better.

It starts a web app connected to [mpv](https://mpv.io/) and lets you queue video links or upload videos.

## Features

- Runs locally, no cloud dependencies.
- Queue up any video/audio link supported by `yt-dlp`.
- Skip videos you don't like.
- Probably more secure than ed's [very-bad-idea.py](https://github.com/9001/copyparty/blob/master/bin/mtag/very-bad-idea.py).

## Installation

Install these dependencies:

- [mpv](https://mpv.io/)
- [yt-dlp](https://github.com/yt-dlp/yt-dlp) (optional, but recommended)
- [ffmpeg](https://ffmpeg.org/) (yt-dlp may use it)

Download the latest release from the Releases page, or build it yourself. Go to the Building section for that.

You should now have a binary called `kameloso` or `kameloso.exe` and a directory called `public`.

## Usage

Generally you only need to run `kameloso` in the directory that contains the `public` directory. If mpv is not in your `$PATH` (which likely means you're on Windows) you may also need to use the `--mpv-path` flag. So:

1. Open a terminal and run `kameloso` in the directory containing the `public` directory.
    - If you're on Windows, you probably need to specify the mpv path like this: `kameloso.exe --mpv-path path\to\mpv.exe`. Replace with the actual path to the executable.
    - You might also want to create a `start.bat` file in the same directory so you can just double-click it next time you want to open it:
      ```batch
      kameloso.exe --mpv-path path\to\mpv.exe
      ```
2. `kameloso` will open an mpv window and start an HTTP server, by default at `0.0.0.0:8080`. See below for how to change it.
3. After a second you will see a QR code in the top left of the mpv window. This should contain a local URL pointing to the web UI.
4. Go to the URL and start queueing up videos!
    - If that URL does not work, run `ip addr` on Linux or `ipconfig` on Windows and check the machine's local IP address, which should look something like `192.168.1.<some number>`. The web UI will be at `http://<the machine's local IP address>:8080/`.
    - If that also doesn't work, make sure that your firewall isn't blocking incoming traffic on the `8080` port.
  
The web UI looks like this:

![Screenshot of the kameloso web UI](https://github.com/steinuil/kameloso/blob/master/extras/pictures/ui.png)

And [this](https://github.com/steinuil/kameloso/blob/master/extras/pictures/intended-usage.jpg) is how it looks in the wild.

### Useful tips

- You can get around youtube's age restriction by [configuring](https://github.com/yt-dlp/yt-dlp?tab=readme-ov-file#configuration) `yt-dlp` to use [cookies from your browser](https://github.com/yt-dlp/yt-dlp?tab=readme-ov-file#changes-from-youtube-dl).

### Other options

Run `kameloso --help` to see a full list of options. You might need to set:

- `--mpv-path <path/to/mpv.exe>`: Set this if you don't have `mpv` in your `$PATH`.
- `--bind-address <ip>:<port>`: Change the bind address of the HTTP server. If you just want to change the port, set it to `0.0.0.0:<your port>`
- `--serve-dir <path>`: Set this to the path of the `public` directory. By default it looks for `public`. This directory will be created if it doesn't already exist.
- `--upload-dir <path>`: This is the path of the directory to which the uploaded files will be saved. By default `kameloso` will create a directory called `uploads` in the directory it's run from.

If you want to pass arguments to mpv you'll need to pass them after `--`, for example if you want to set audio normalization:

```
kameloso -- --af=dynaudnorm=f=100
```

If you're running it directly from `cargo run`, you'll need to add two double dashes (one for `cargo run`, the other one for `kameloso`)

```
cargo run -- -- --af=dynaudnorm=f=100
```

### Customizing the web UI

If you want to customize the web UI, just change the `index.html` file in the `public/` directory. Note that if you want to serve other files as well, they have to be in the `public/static/` directory.

## Building

Install [Rust](https://www.rust-lang.org/), clone this repository and run `cargo build --release`. You will find the binary in `target/release/kameloso`.
