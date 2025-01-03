use std::{io, path::Path, time::Duration};

#[cfg(unix)]
use tokio::net::UnixStream as Kopipe;
#[cfg(windows)]
use tokio::{
    io::Interest,
    net::windows::named_pipe::{ClientOptions, NamedPipeClient as Kopipe},
};

pub async fn open<P: AsRef<Path> + ?Sized>(path: &P) -> io::Result<Kopipe> {
    #[cfg(unix)]
    let pipe = Kopipe::connect(path).await;

    #[cfg(windows)]
    let pipe = {
        let pipe = ClientOptions::new().open(path.as_ref())?;

        Ok(pipe)
    };

    pipe
}

pub async fn open_retry<P: AsRef<Path> + ?Sized>(path: &P, times: usize) -> io::Result<Kopipe> {
    let mut i = 1;

    loop {
        match open(&path).await {
            Ok(pipe) => return Ok(pipe),
            Err(e) if i >= times => {
                log::error!("failed to connect to mpv ipc");
                return Err(e);
            }
            Err(_) => {}
        }

        log::info!("mpv socket not available yet, retrying in 100ms");
        tokio::time::sleep(Duration::from_millis(100)).await;

        i += 1;
    }
}
