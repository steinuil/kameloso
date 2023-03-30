use serde::Deserialize;
use serde_json::json;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, oneshot, Mutex, MutexGuard};

use crate::kopipe::Kopipe;
use crate::mpv_error::IpcError;

pub type MpvResult = std::result::Result<serde_json::Value, String>;

#[derive(Debug)]
struct HandlerJar {
    handlers: HashMap<i64, oneshot::Sender<MpvResult>>,
    next_request_id: i64,
}

impl HandlerJar {
    pub fn new() -> Self {
        HandlerJar {
            handlers: HashMap::new(),
            next_request_id: 0,
        }
    }

    pub fn handler(&mut self, request_id: i64) -> oneshot::Receiver<MpvResult> {
        let (sender, receiver) = oneshot::channel();
        self.handlers.insert(request_id, sender);
        receiver
    }

    pub fn pop_handler(&mut self, request_id: i64) -> Option<oneshot::Sender<MpvResult>> {
        self.handlers.remove(&request_id)
    }

    pub fn next_request_id(&mut self) -> i64 {
        let id = self.next_request_id;
        self.next_request_id += 1;
        id
    }
}

const LINE_SEPARATOR: u8 = b'\n';

async fn start_mpv_pipe(
    mut pipe: Kopipe,
    messages_tx: mpsc::UnboundedSender<Vec<u8>>,
    mut commands_rx: mpsc::UnboundedReceiver<Vec<u8>>,
) {
    let mut buf = [0; 256];
    let mut line = Vec::new();

    loop {
        tokio::select! {
            read = pipe.read(&mut buf) => match read {
                Ok(0)  => {
                    log::warn!("mpv pipe closed");
                    break
                }
                Err(e) => {
                    log::error!("Error from mpv pipe: {}", e);
                    todo!();
                }
                Ok(read) => {
                    let buf_slice = &buf[..read];

                    match buf_slice.iter().position(|c| c == &LINE_SEPARATOR) {
                        None => {
                            line.extend_from_slice(buf_slice);
                        }
                        Some(i) => {
                            line.extend_from_slice(&buf_slice[..i]);

                            for line in line.split(|b| *b ==  b'\n') {
                                if let Err(e) = messages_tx.send(line.to_vec()) {
                                    todo!();
                                }
                            }

                            line.clear();

                            if i != read - 1 {
                                line.extend_from_slice(&buf_slice[i + 1..])
                            }
                        }

                    }
                }
            },

            cmd = commands_rx.recv() => match cmd {
                None => todo!(),
                Some(msg) => {
                    if let Err(e) = pipe.write(msg.as_slice()).await {
                        todo!();
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum Message {
    CommandResponse {
        request_id: Option<i64>,
        data: Option<serde_json::Value>,
        error: String,
    },
    Event {
        event: String,
    },
}

async fn handle_mpv_messages(
    mut msg_rx: mpsc::UnboundedReceiver<Vec<u8>>,
    handler_jar: Arc<Mutex<HandlerJar>>,
) {
    loop {
        match msg_rx.recv().await {
            None => break,
            Some(msg) => match serde_json::from_slice(msg.as_slice()) {
                // The message:
                // - IS a reply to a command
                // - IS associated to a request_id
                // - IS NOT an error
                Ok(Message::CommandResponse {
                    request_id: Some(request_id),
                    data,
                    error,
                }) if error == "success" => {
                    match handler_jar.lock().await.pop_handler(request_id) {
                        Some(sender) => {
                            if let Err(e) = sender.send(Ok(data.unwrap_or(serde_json::Value::Null)))
                            {
                                todo!();
                            }
                        }
                        None => {
                            log::warn!("Received success reply with request_id={} but no matching sender found. data={:?}", request_id, data);
                        }
                    }
                }

                // The message:
                // - IS a reply to a command
                // - IS associated to a request_id
                // - IS an error
                Ok(Message::CommandResponse {
                    request_id: Some(request_id),
                    data: _,
                    error,
                }) => match handler_jar.lock().await.pop_handler(request_id) {
                    Some(sender) => {
                        if let Err(e) = sender.send(Err(error)) {
                            todo!();
                        }
                    }
                    None => {
                        log::warn!("Received error reply with request_id={} but no matching sender found. error={:?}", request_id, error);
                    }
                },

                // The message:
                // - IS a reply to a command
                // - IS NOT associated to a request_id
                // - IS NOT an error
                Ok(Message::CommandResponse {
                    request_id: None,
                    data,
                    error,
                }) if error == "success" => {
                    log::warn!(
                        "Received success reply without a request_id. data={:?}",
                        data
                    );
                }

                // The message:
                // - IS a reply to a command
                // - IS NOT associated to a request_id
                // - IS an error
                Ok(Message::CommandResponse {
                    request_id: None,
                    data: _,
                    error,
                }) => {
                    log::warn!("Received error reply without a request_id. error={}", error);
                }

                // The message is an event
                Ok(Message::Event { event }) => {
                    log::info!("Received event={}", event);
                }

                Err(e) => {
                    log::error!("Failed to decode mpv response={:?} : {}", msg, e);
                }
            },
        }
    }
}

#[derive(Debug)]
pub struct Reactor {
    handler_jar: Arc<Mutex<HandlerJar>>,
    commands_tx: mpsc::UnboundedSender<Vec<u8>>,
    // mpv_pipe_task: tokio::task::JoinHandle<()>,
    // handle_messages_task: tokio::task::JoinHandle<()>,
}

impl Reactor {
    pub async fn start(pipe: Kopipe) -> Self {
        let handler_jar = Arc::new(Mutex::new(HandlerJar::new()));
        let jar = handler_jar.clone();

        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();

        // let mpv_pipe_task =
        tokio::spawn(async move { start_mpv_pipe(pipe, msg_tx, cmd_rx).await });

        // let handle_messages_task =
        tokio::spawn(async move { handle_mpv_messages(msg_rx, handler_jar).await });

        Reactor {
            handler_jar: jar,
            commands_tx: cmd_tx,
            // mpv_pipe_task,
            // handle_messages_task,
        }
    }

    async fn handler_jar(&self) -> MutexGuard<HandlerJar> {
        self.handler_jar.lock().await
    }

    pub async fn send_command(&self, cmd: &[&str]) -> Result<serde_json::Value, IpcError> {
        let sender = {
            let mut handler_jar = self.handler_jar().await;

            let request_id = handler_jar.next_request_id();

            let mut msg = serde_json::to_vec(&json!({
                "command": cmd,
                "request_id": request_id
            }))
            .unwrap();

            msg.push(LINE_SEPARATOR);

            if let Err(e) = self.commands_tx.send(msg) {
                todo!();
            }

            handler_jar.handler(request_id)
        };

        sender.await?.map_err(IpcError::MpvError)
    }
}
