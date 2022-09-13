use crate::crypto::{Key, OpenError, SealError};
use crate::protocol::{SocketMessage, SocketMessageKind, Topic};
use log::{trace, warn};
use std::error::Error;
use std::str::Utf8Error;
use std::thread::{self, JoinHandle};
use thiserror::Error;
use url::Url;
use parity_ws::{Handler, Message, Sender, WebSocket};

#[derive(Debug)]
pub struct Socket {
    key: Key,
    sender: Sender,
    event_loop: JoinHandle<Result<(), parity_ws::Error>>,
}

impl Socket {
    pub fn connect(
        url: Url,
        key: Key,
        message_handler: impl MessageHandler + Send + 'static,
    ) -> Result<Self, SocketError> {
        let mut socket = WebSocket::new({
            let mut params = Some((key.clone(), message_handler));
            move |sender| {
                let (key, message_handler) = params
                    .take()
                    .expect("more than one WebSocket connection established");
                SocketHandler {
                    key,
                    sender,
                    message_handler,
                }
            }
        })?;

        socket.connect(url)?;
        let sender = socket.broadcaster();
        let event_loop = thread::spawn(move || match socket.run() {
            Ok(_) => Ok(()),
            Err(err) => {
                warn!("socket runloop unexpectedly quit with error: {:?}", err);
                Err(err)
            }
        });

        Ok(Socket {
            key,
            sender,
            event_loop,
        })
    }

    fn handle(&self) -> SocketHandle {
        SocketHandle {
            key: &self.key,
            sender: &self.sender,
        }
    }

    pub fn subscribe(&self, topic: Topic) -> Result<(), SocketError> {
        self.handle().subscribe(topic)
    }

    pub fn publish(
        &self,
        topic: Topic,
        payload: impl AsRef<str>,
        silent: bool,
    ) -> Result<(), SocketError> {
        self.handle().publish(topic, payload, silent)
    }

    pub fn close(self) -> Result<(), SocketError> {
        self.sender.shutdown()?;
        self.event_loop
            .join()
            .expect("event loop should never panic")?;

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum SocketError {
    #[error("WebSocket error")]
    WebSocket(#[from] parity_ws::Error),
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("failed to seal AEAD payload: {0}")]
    Seal(#[from] SealError),
}

#[derive(Debug)]
pub struct SocketHandle<'a> {
    key: &'a Key,
    sender: &'a Sender,
}

impl SocketHandle<'_> {
    pub fn subscribe(&self, topic: Topic) -> Result<(), SocketError> {
        self.send(SocketMessage {
            topic,
            kind: SocketMessageKind::Sub,
            payload: None,
            silent: true,
        })?;

        Ok(())
    }

    pub fn publish(
        &self,
        topic: Topic,
        payload: impl AsRef<str>,
        silent: bool,
    ) -> Result<(), SocketError> {
        trace!("sending payload '{}'", payload.as_ref());

        let payload = self.key.seal(payload.as_ref())?;
        self.send(SocketMessage {
            topic,
            kind: SocketMessageKind::Pub,
            payload: Some(payload),
            silent,
        })?;

        Ok(())
    }

    fn send(&self, message: SocketMessage) -> Result<(), SocketError> {
        let json = serde_json::to_string(&message)?;
        trace!("sending message '{}'", json);

        self.sender.send(json)?;

        Ok(())
    }
}

pub trait MessageHandler {
    type Err: Error + Send + Sync + 'static;

    fn message(
        &mut self,
        socket: SocketHandle,
        topic: Topic,
        payload: String,
    ) -> Result<(), Self::Err>;
}

struct SocketHandler<M> {
    key: Key,
    sender: Sender,
    message_handler: M,
}

impl<M> SocketHandler<M>
where
    M: MessageHandler,
{
    fn decrypt_message(&self, message: &str) -> Result<(Topic, String), MessageError> {
        trace!("received message '{}'", message);

        let message: SocketMessage = serde_json::from_str(message)?;
        if let SocketMessageKind::Sub = message.kind {
            return Err(MessageError::Sub(message.topic));
        }

        let topic = message.topic;
        let payload = match message.payload {
            Some(payload) => payload,
            None => return Err(MessageError::MissingPayload),
        };

        let opened = self.key.open(&payload)?;
        let decrypted = String::from_utf8(opened).map_err(|err| err.utf8_error())?;

        trace!("received payload '{}'", decrypted);

        Ok((topic, decrypted))
    }
}

impl<M> Handler for SocketHandler<M>
where
    M: MessageHandler,
{
    fn on_message(&mut self, message: Message) -> parity_ws::Result<()> {
        let (topic, payload) = self.decrypt_message(message.as_text()?).map_err(Box::new)?;
        let handle = SocketHandle {
            key: &self.key,
            sender: &self.sender,
        };
        self.message_handler
            .message(handle, topic, payload)
            .map_err(Box::new)?;

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum MessageError {
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unexpected 'sub' message with topic '{0}'")]
    Sub(Topic),
    #[error("message payload missing")]
    MissingPayload,
    #[error("failed to open AEAD payload: {0}")]
    Aead(#[from] OpenError),
    #[error("invalid UTF-8 in decrypted payload: {0}")]
    Utf8(#[from] Utf8Error),
}
