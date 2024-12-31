use std::{io, path::Path};

#[cfg(windows)]
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient};
#[cfg(unix)]
use tokio::net::UnixStream;

#[cfg(windows)]
pub type Kopipe = NamedPipeClient;
#[cfg(unix)]
pub type Kopipe = UnixStream;

pub async fn open<P: AsRef<Path> + ?Sized>(path: &P) -> io::Result<Kopipe> {
    #[cfg(unix)]
    let pipe = UnixStream::connect(path).await;

    #[cfg(windows)]
    let pipe = ClientOptions::new().open(path.into());

    pipe
}
