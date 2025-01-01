use std::{io, path::Path};

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
        loop {
            pipe.ready(Interest::READABLE | Interest::WRITABLE).await?;

            if pipe.is_readable() && pipe.is_readable() {
                break;
            }
        }

        Ok(pipe)
    };

    pipe
}
