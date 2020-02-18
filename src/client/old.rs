/*
use super::rpc::{Request, Response};
use super::storage::Storage;
use crate::session::{Metadata, Session};
use futures::channel::{mpsc, oneshot};
use futures::future::{FutureExt, Shared};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use thiserror::Error;
use url::{ParseError, Url};
use ws::{Factory, Handler, Handshake, Message, Sender, WebSocket};

#[derive(Debug)]
pub struct Client {
    sender: Sender,
    event_loop: JoinHandle<Option<ws::Error>>,
    connected: Shared<oneshot::Receiver<()>>,
    context: Arc<Mutex<Context>>,
}

#[derive(Debug)]
struct Context {
    session: Storage<Session>,
    requests: HashMap<i32, oneshot::Sender<Response>>,
}

pub const DEFAULT_BRIDGE_URL: &str = "https://bridge.walletconnect.org";

impl Client {
    pub fn new(profile: impl AsRef<Path>, meta: Metadata) -> Result<Self, CreationError> {
        Client::with_bridge(profile, DEFAULT_BRIDGE_URL, meta)
    }

    pub fn with_bridge(
        profile: impl AsRef<Path>,
        bridge: impl AsRef<str>,
        meta: Metadata,
    ) -> Result<Self, CreationError> {
        let bridge = Url::parse(bridge.as_ref())?;

        // NOTE: WalletConnect bridge URLs are expected to be automatically
        // converted from a `http(s)` to `ws(s)` protocol for the WebSocket
        // connection.
        let rpc_url = {
            let mut rpc_url = bridge.clone();
            match rpc_url.scheme() {
                "http" => rpc_url.set_scheme("ws").unwrap(),
                "https" => rpc_url.set_scheme("wss").unwrap(),
                "ws" | "wss" => {}
                scheme => return Err(CreationError::BadScheme(scheme.into())),
            }
            rpc_url
        };

        let context = {
            let session = Storage::for_session(profile.as_ref(), bridge, meta);
            Arc::new(Mutex::new(Context {
                session,
                requests: HashMap::new(),
            }))
        };
        let (connected_tx, connected_rx) = oneshot::channel();
        let handler = ClientHandler {
            context: context.clone(),
            connected: Some(connected_tx),
        };

        let factory = ClientFactory(Some(handler));
        let mut socket = WebSocket::new(factory)?;
        socket.connect(rpc_url)?;
        let sender = socket.broadcaster();
        let event_loop = thread::spawn(move || socket.run().err());

        Ok(Client {
            sender,
            event_loop,
            context,
            connected: connected_rx.shared(),
        })
    }

    pub fn session(&self) -> Session {
        self.context.lock().unwrap().session.clone()
    }

    pub fn close(self) -> Result<(), CloseError> {
        self.sender.shutdown()?;
        // NOTE: Intentionally propagate the event loop panic, as it is not
        // intended to do so.
        let err = self.event_loop.join().unwrap();

        if let Some(err) = err {
            Err(err.into())
        } else {
            Ok(())
        }
    }
}

struct ClientFactory(Option<ClientHandler>);

impl Factory for ClientFactory {
    type Handler = ClientHandler;

    fn connection_made(&mut self, _: Sender) -> Self::Handler {
        self.0
            .take()
            .expect("more than one connection made for a single client")
    }
}

struct ClientHandler {
    context: Arc<Mutex<Context>>,
    connected: Option<oneshot::Sender<()>>,
}

impl Handler for ClientHandler {
    fn on_open(&mut self, _: Handshake) -> ws::Result<()> {
        self.connected
            .take()
            .expect("connection opened more than once")
            .send(())
            .expect("client should not be dropped");

        Ok(())
    }

    fn on_message(&mut self, msg: Message) -> ws::Result<()> {
        let response = Response::parse(msg.as_text()?);
        let mut context = self.context.lock().unwrap();
        if let Some(sender) = context.requests.remove(&response.id) {
            // NOTE: ignore errors where the receiver is dropped.
            let _ = sender.send(response);
        }

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum CreationError {
    #[error("failed to parse URL: {0}")]
    Parse(#[from] ParseError),
    #[error("invalid URL scheme '{0}', must be 'http(s)' or 'ws(s)'")]
    BadScheme(String),
    #[error("IO error when creating the WebSocket: {0}")]
    Io(#[from] ws::Error),
}

#[derive(Debug, Error)]
pub enum CloseError {
    #[error("failed to shutdown WebSocket: {0}")]
    Shutdown(#[from] ws::Error),
}
*/
