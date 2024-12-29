use std::{io, path::Path};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[cfg(windows)]
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient};
#[cfg(unix)]
use tokio::net::UnixStream;

#[cfg(windows)]
pub struct Kopipe(NamedPipeClient);
#[cfg(unix)]
pub struct Kopipe(UnixStream);

impl Kopipe {
    #[cfg(windows)]
    pub async fn open<P: AsRef<Path> + ?Sized>(path: &P) -> io::Result<Self> {
        Ok(Kopipe(ClientOptions::new().open(path)?))
    }

    #[cfg(unix)]
    pub async fn open<P: AsRef<Path> + ?Sized>(path: &P) -> io::Result<Self> {
        Ok(Kopipe(UnixStream::connect(path).await?))
    }

    pub async fn write(&mut self, buf: &[u8]) -> io::Result<()> {
        self.0.write_all(buf).await
    }

    pub async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf).await
    }
}
