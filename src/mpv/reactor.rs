use std::collections::HashMap;

use tokio::{
    io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    select,
    sync::{
        mpsc::{self, error::SendError, UnboundedReceiver, UnboundedSender},
        oneshot,
    },
};

use crate::mpv::message::PropertyChange;

use super::{command::RawCommand, message::Message, message_buffer::MessageBuffer};

pub type Response = Result<serde_json::Value, String>;

pub type ResponseHandler = oneshot::Sender<Response>;

pub type PropertyChangeSender = UnboundedSender<serde_json::Value>;

pub enum Command {
    WithResponse {
        command: serde_json::Value,
        handler: ResponseHandler,
    },
    ObserveProperty {
        property: String,
        handler: ResponseHandler,
        data_tx: PropertyChangeSender,
    },
}

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
    commands_rx: UnboundedReceiver<Command>,
    command_handlers: HashMap<i64, ResponseHandler>,
    next_request_id: i64,
    observe_property_handlers: HashMap<String, PropertyChangeSender>,

    // This field is only here because Windows likes to return an empty buffer
    // the first time we poll for a read, so we check if this is the first time
    // we encountered an empty read and if we get a second one in a row, we terminate the task.
    // This is basically a noop on Linux so it's fine.
    // The correct way would probably be to await for .readable() before reading?
    is_maybe_eof: bool,
}

impl<NamedPipe: AsyncRead + AsyncWrite + Unpin> Reactor<NamedPipe> {
    fn new(mpv_pipe: NamedPipe, commands_rx: UnboundedReceiver<Command>) -> Self {
        Self {
            message_buffer: MessageBuffer::new(),
            mpv_pipe,
            commands_rx,
            command_handlers: HashMap::new(),
            next_request_id: 0,
            observe_property_handlers: HashMap::new(),
            is_maybe_eof: false,
        }
    }

    fn insert_command_handler(&mut self, handler: ResponseHandler) -> i64 {
        let id = self.next_request_id;
        self.command_handlers.insert(id, handler);
        self.next_request_id += 1;
        id
    }

    fn handle_input(&mut self, buf: &[u8]) {
        let messages = self.message_buffer.insert(buf);

        for message in messages {
            log::debug!("received message: {message:?}");

            match message {
                Ok(Message::Response { request_id, data }) => {
                    match self.command_handlers.remove(&request_id) {
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
                Ok(Message::ResponseWithoutId(data)) => {
                    log::warn!("received response without ID: {data:?}");
                }
                Ok(Message::PropertyChange(PropertyChange { name, data })) => {
                    log::debug!("received property-change event: {name}");

                    match self.observe_property_handlers.get(&name) {
                        Some(handler) => {
                            if handler.send(data).is_err() {
                                log::warn!("received property-change event but handler is closed. property: {name}");
                                // self.observe_property_handlers.remove(&id);
                            }
                        }
                        None => todo!(),
                    }
                }
                Ok(Message::Event { event, fields }) => {
                    log::debug!("received event: {event} {fields:?}");
                }
                Err(e) => {
                    log::error!("failed to decode mpv message: {e}");
                }
            }
        }
    }

    async fn send_command(
        &mut self,
        cmd: serde_json::Value,
        handler: ResponseHandler,
    ) -> Result<(), io::Error> {
        let request_id = self.insert_command_handler(handler);
        let cmd = RawCommand::new(request_id, cmd);

        log::debug!("sending command: {cmd:?}");

        let mut cmd = cmd.serialize_to_vec();
        cmd.push(Message::LINE_SEPARATOR);

        self.mpv_pipe.write_all(&cmd).await
    }

    async fn observe_property(
        &mut self,
        property: String,
        handler: ResponseHandler,
        data_tx: PropertyChangeSender,
    ) -> Result<(), io::Error> {
        self.observe_property_handlers
            .insert(property.clone(), data_tx);

        let cmd = serde_json::json!(["observe_property", 1, property]);

        self.send_command(cmd, handler).await
    }

    async fn step(&mut self) -> Result<(), Error> {
        let mut buf = [0; 256];

        select! {
            read = self.mpv_pipe.read(&mut buf) => match read? {
                0 if self.is_maybe_eof => {
                    log::info!("shutting down: mpv pipe closed");
                    return Err(Error::Pipe(PipeClosed::Mpv));
                },
                0 => {
                    log::debug!("read 0 bytes from named pipe, retrying");
                    self.is_maybe_eof = true;
                }
                read => {
                    self.handle_input(&buf[..read]);
                    self.is_maybe_eof = false;
                }
            },

            cmd = self.commands_rx.recv() => match cmd {
                None => {
                    log::info!("shutting down: commands channel closed");
                    return Err(Error::Pipe(PipeClosed::Commands));
                }
                Some(Command::WithResponse { command, handler }) => {
                    self.send_command(command, handler).await?;
                }
                Some(Command::ObserveProperty { property, handler, data_tx }) => {
                    self.observe_property(property, handler, data_tx).await?;
                }
            },
        }

        Ok(())
    }
}

pub async fn start<NamedPipe>(
    mpv_pipe: NamedPipe,
    commands_rx: UnboundedReceiver<Command>,
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
    cmd: serde_json::Value,
    commands_tx: &UnboundedSender<Command>,
) -> Result<oneshot::Receiver<Response>, SendError<Command>> {
    let (handler_tx, handler_rx) = oneshot::channel();
    commands_tx.send(Command::WithResponse {
        command: cmd,
        handler: handler_tx,
    })?;
    Ok(handler_rx)
}

pub async fn observe_property(
    property: String,
    commands_tx: &UnboundedSender<Command>,
) -> Result<
    (
        oneshot::Receiver<Response>,
        UnboundedReceiver<serde_json::Value>,
    ),
    SendError<Command>,
> {
    let (handler_tx, handler_rx) = oneshot::channel();
    let (data_tx, data_rx) = mpsc::unbounded_channel();
    commands_tx.send(Command::ObserveProperty {
        property,
        data_tx,
        handler: handler_tx,
    })?;
    Ok((handler_rx, data_rx))
}
