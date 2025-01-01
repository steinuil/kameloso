use std::collections::HashMap;

use tokio::{
    io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    select,
    sync::{
        mpsc::{error::SendError, UnboundedReceiver, UnboundedSender},
        oneshot,
    },
};

use super::{command::RawCommand, message::Message, message_buffer::MessageBuffer};

pub type Response = Result<serde_json::Value, String>;

pub type ResponseHandler = oneshot::Sender<Response>;

pub type CommandWithHandler = (Vec<String>, ResponseHandler);

#[derive(Debug)]
pub enum PipeClosed {
    Mpv,
    Commands,
}

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("channel closed: {0:?}")]
    Pipe(PipeClosed),
}

struct Reactor<NamedPipe> {
    message_buffer: MessageBuffer,
    mpv_pipe: NamedPipe,
    commands_rx: UnboundedReceiver<CommandWithHandler>,
    handlers: HashMap<i64, ResponseHandler>,
    next_request_id: i64,

    // This field is only here because Windows likes to return an empty buffer
    // the first time we poll for a read, so we check if this is the first time
    // we encountered an empty read and if we get a second one, we terminate the task.
    has_received_empty_read: bool,
}

impl<NamedPipe: AsyncRead + AsyncWrite + Unpin> Reactor<NamedPipe> {
    fn new(mpv_pipe: NamedPipe, commands_rx: UnboundedReceiver<CommandWithHandler>) -> Self {
        Self {
            message_buffer: MessageBuffer::new(),
            mpv_pipe,
            commands_rx,
            handlers: HashMap::new(),
            next_request_id: 0,
            has_received_empty_read: false,
        }
    }

    fn insert_handler(&mut self, handler: ResponseHandler) -> i64 {
        let id = self.next_request_id;
        self.handlers.insert(id, handler);
        self.next_request_id += 1;
        id
    }

    fn handle_input(&mut self, buf: &[u8]) {
        let messages = self.message_buffer.insert(buf);

        for message in messages {
            log::debug!("received message: {message:?}");

            match message {
                Ok(Message::Response { request_id, data }) => {
                    match self.handlers.remove(&request_id) {
                        Some(handler) => {
                            if handler.send(data).is_err() {
                                log::warn!(
                                    "received reply but handler is closed. request_id: {request_id}"
                                );
                            }
                        }
                        None => todo!(),
                    }
                }
                Ok(Message::ResponseWithoutId(_data)) => {
                    log::warn!("");
                }
                Ok(Message::Event(ev)) => {
                    log::info!("received event: {ev}");
                }
                Err(e) => {
                    log::error!("failed to decode mpv message: {e}");
                }
            }
        }
    }

    async fn send_command(
        &mut self,
        cmd: Vec<String>,
        handler: ResponseHandler,
    ) -> Result<(), io::Error> {
        let request_id = self.insert_handler(handler);
        let cmd = RawCommand::new(request_id, cmd);

        log::debug!("sending command: {cmd:?}");

        let mut cmd = cmd.serialize_to_vec();
        cmd.push(Message::LINE_SEPARATOR);

        self.mpv_pipe.write_all(&cmd).await
    }

    async fn step(&mut self) -> Result<(), Error> {
        let mut buf = [0; 256];

        select! {
            read = self.mpv_pipe.read(&mut buf) => match read? {
                0 if self.has_received_empty_read => {
                    log::info!("shutting down: mpv pipe closed");
                    return Err(Error::Pipe(PipeClosed::Mpv));
                },
                0 => {
                    log::debug!("read 0 bytes from named pipe, retrying");
                    self.has_received_empty_read = true;
                }
                read => {
                    self.handle_input(&buf[..read]);
                }
            },

            cmd = self.commands_rx.recv() => match cmd {
                None => {
                    log::info!("shutting down: commands channel closed");
                    return Err(Error::Pipe(PipeClosed::Commands));
                }
                Some((cmd, handler)) => {
                    self.send_command(cmd, handler).await?;
                }
            },
        }

        Ok(())
    }
}

pub async fn start<NamedPipe>(
    mpv_pipe: NamedPipe,
    commands_rx: UnboundedReceiver<CommandWithHandler>,
) -> Result<PipeClosed, io::Error>
where
    NamedPipe: AsyncRead + AsyncWrite + Unpin,
{
    let mut pipe = Reactor::new(mpv_pipe, commands_rx);

    loop {
        let result = pipe.step().await;

        match result {
            Ok(()) => {}
            Err(Error::Pipe(c)) => return Ok(c),
            Err(Error::Io(e)) => return Err(e),
        }
    }
}

pub async fn send_command(
    cmd: Vec<String>,
    commands_tx: &UnboundedSender<CommandWithHandler>,
) -> Result<oneshot::Receiver<Response>, SendError<CommandWithHandler>> {
    let (handler_tx, handler_rx) = oneshot::channel();
    commands_tx.send((cmd, handler_tx))?;
    Ok(handler_rx)
}
