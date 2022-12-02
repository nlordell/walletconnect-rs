use super::options::{Connection, Options};
use super::session::Session;
use super::socket::{MessageHandler, Socket, SocketError, SocketHandle};
use super::storage::Storage;
use crate::protocol::{Topic, Transaction};
use crate::uri::Uri;
use ethers_core::types::{Address, Bytes, H256};
use futures::channel::oneshot;
use jsonrpc_core::{Id, MethodCall, Output, Params, Version};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use thiserror::Error;

#[derive(Debug)]
pub struct Connector {
    current_request: AtomicU64,
    context: SharedContext,
    socket: Socket,
}

impl Connector {
    pub fn new(options: Options) -> Result<Self, ConnectorError> {
        let handshake_topic = match &options.connection {
            Connection::Uri(uri) => Some(uri.handshake_topic().clone()),
            _ => None,
        };
        let session = Storage::for_session(options);
        let client_id = session.client_id.clone();

        // NOTE: WalletConnect bridge URLs are expected to be automatically
        // converted from a `http(s)` to `ws(s)` protocol for the WebSocket
        // connection.
        let mut url = session.bridge.clone();
        match url.scheme() {
            "http" => url.set_scheme("ws").unwrap(),
            "https" => url.set_scheme("wss").unwrap(),
            "ws" | "wss" => {}
            scheme => return Err(ConnectorError::BadScheme(scheme.into())),
        }

        let key = session.key.clone();
        let context = SharedContext::new(session);
        let handler = ConnectorHandler {
            context: context.clone(),
        };

        let socket = Socket::connect(url, key, handler)?;
        socket.subscribe(client_id)?;
        if let Some(handshake_topic) = handshake_topic {
            socket.subscribe(handshake_topic)?;
        }

        Ok(Connector {
            current_request: AtomicU64::default(),
            context,
            socket,
        })
    }

    pub fn accounts(&self) -> Result<(Vec<Address>, u64), NotConnectedError> {
        let session = &self.context.lock().session;
        if !session.connected {
            return Err(NotConnectedError);
        }

        Ok((
            session.accounts.clone(),
            session.chain_id.unwrap_or_default(),
        ))
    }

    async fn call<P, R>(&self, method: &str, params: P) -> Result<R, CallError>
    where
        P: Serialize,
        R: DeserializeOwned,
    {
        let id = self.current_request.fetch_add(1, Ordering::SeqCst);

        let topic = {
            let context = self.context.lock();
            context
                .session
                .peer_id
                .clone()
                .unwrap_or_else(|| context.session.handshake_topic.clone())
            //.ok_or(CallError::NotConnected)?
        };
        let payload = {
            let params = match json!(params) {
                Value::Array(params) => Params::Array(params),
                param => Params::Array(vec![param]),
            };
            let request = MethodCall {
                jsonrpc: Some(Version::V2),
                method: method.into(),
                params,
                id: Id::Num(id),
            };
            serde_json::to_string(&request)?
        };
        let silent = match method {
            "wc_sessionRequest" | "wc_sessionUpdate" => true,
            "eth_sendTransaction"
            | "eth_signTransaction"
            | "eth_sign"
            | "eth_signTypedData"
            | "eth_signTypedData_v1"
            | "eth_signTypedData_v3"
            | "personal_sign" => false,
            _ => true,
        };

        let (tx, rx) = oneshot::channel();
        let existing = {
            let mut context = self.context.lock();
            context.pending_requests.insert(Id::Num(id), tx)
        };

        // NOTE: Make sure panic is always outside the mutex guard's scope to
        // make sure we don't accidentially poison the mutex.
        debug_assert!(existing.is_none(), "request IDs should never collide",);

        if let Err(err) = self.socket.publish(topic, payload, silent) {
            // NOTE: Remove the request from the pending request map if we were
            // unable to send it as there will never be a response.
            let removed = {
                let mut context = self.context.lock();
                context.pending_requests.remove(&Id::Num(id))
            };

            // NOTE: Make sure panic is always outside the mutex guard's
            // scope to make sure we don't accidentially poison the mutex.
            debug_assert!(
                removed.is_some(),
                "immediately removed request should never be missing"
            );

            return Err(err.into());
        }

        let response = rx.await?;
        match response {
            Output::Success(response) => {
                let result = R::deserialize(&response.result)?;
                Ok(result)
            }
            Output::Failure(response) => Err(response.error.into()),
        }
    }

    pub async fn ensure_session<F>(&self, f: F) -> Result<(Vec<Address>, u64), SessionError>
    where
        F: FnOnce(Uri),
    {
        let uri = {
            let context = self.context.lock();
            if context.session.connected {
                return Ok((
                    context.session.accounts.clone(),
                    context.session.chain_id.unwrap_or_default(),
                ));
            }
            context.session.uri()
        };

        f(uri);
        let (accounts, chain_id) = self.create_session().await?;

        Ok((accounts, chain_id))
    }

    pub async fn create_session(&self) -> Result<(Vec<Address>, u64), SessionError> {
        let params = {
            let mut context = self.context.lock();
            if context.session.connected {
                return Err(SessionError::Connected);
            }
            if context.session_pending {
                return Err(SessionError::Pending);
            }

            context.session_pending = true;
            context.session.request()
        };

        let result = self.call("wc_sessionRequest", params).await;

        let (accounts, chain_id) = {
            let mut context = self.context.lock();
            context.session_pending = false;

            // NOTE: Propagate the error only after updating signaling that the
            // session is no longer pending.
            let session_params = result?;
            context
                .session
                .update(move |session| session.apply(session_params));

            (
                context.session.accounts.clone(),
                context.session.chain_id.unwrap_or_default(),
            )
        };

        Ok((accounts, chain_id))
    }

    // pub fn update_session() {}
    // pub fn kill_session() {}

    pub async fn send_transaction(&self, transaction: Transaction) -> Result<H256, CallError> {
        self.call("eth_sendTransaction", transaction).await
    }

    pub async fn sign_transaction(&self, transaction: Transaction) -> Result<Bytes, CallError> {
        self.call("eth_signTransaction", transaction).await
    }

    pub async fn personal_sign(&self, data: &[&str]) -> Result<Bytes, CallError> {
        self.call("personal_sign", data).await
    }

    // pub fn sign_message() {}
    // pub fn signed_typed_data() {}
    // pub fn send_custom_request() {}

    // pub fn approve_session() {}
    // pub fn reject_session() {}
    // pub fn approve_request() {}
    // pub fn reject_request() {}

    pub fn close(self) -> Result<(), SocketError> {
        self.socket.close()
    }
}

#[derive(Debug, Error)]
#[error("not connected to pear")]
pub struct NotConnectedError;

#[derive(Debug, Error)]
pub enum ConnectorError {
    #[error("invalid URL scheme '{0}', must be 'http(s)' or 'ws(s)'")]
    BadScheme(String),
    #[error("socket error: {0}")]
    SocketError(#[from] SocketError),
}

#[derive(Debug, Error)]
pub enum CallError {
    #[error("not connected to peer")]
    NotConnected,
    #[error("socket error: {0}")]
    Socket(#[from] SocketError),
    #[error("request was canceled")]
    Canceled(#[from] oneshot::Canceled),
    #[error("JSON RPC error: {0}")]
    Rpc(#[from] jsonrpc_core::Error),
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("session already connected")]
    Connected,
    #[error("session already pending")]
    Pending,
    #[error("error performing JSON RPC request")]
    Call(#[from] CallError),
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Clone, Debug)]
struct SharedContext(Arc<Mutex<Context>>);

#[derive(Debug)]
struct Context {
    session: Storage<Session>,
    pending_requests: HashMap<Id, oneshot::Sender<Output>>,
    session_pending: bool,
}

impl SharedContext {
    fn new(session: Storage<Session>) -> Self {
        SharedContext(Arc::new(Mutex::new(Context {
            session,
            pending_requests: HashMap::new(),
            session_pending: false,
        })))
    }

    fn lock(&self) -> MutexGuard<Context> {
        self.0.lock().expect("mutex guard should never be poisoned")
    }
}

struct ConnectorHandler {
    context: SharedContext,
}

impl MessageHandler for ConnectorHandler {
    type Err = MessageError;

    fn message(&mut self, _: SocketHandle, _: Topic, payload: String) -> Result<(), MessageError> {
        if let Ok(request) = serde_json::from_str::<MethodCall>(&payload) {
            match request.method.as_str() {
                "wc_sessionUpdate" => {
                    let session_update = request.params.parse()?;
                    let mut context = self.context.lock();
                    context
                        .session
                        .update(|session| session.update(session_update));
                }
                _ => return Err(MessageError::UnsupportedRequest(payload)),
            }
        } else {
            let response = serde_json::from_str::<Output>(&payload)?;

            let mut context = self.context.lock();
            let sender = context
                .pending_requests
                .remove(response.id())
                .ok_or_else(|| {
                    let id = response.id().clone();
                    MessageError::UnregisteredId(id)
                })?;

            // NOTE: We ignore send errors as they are "normal" in the sense
            // that it is not considered an error to drop the future that is
            // waiting for the response before it arrives.
            let _ = sender.send(response);
        }

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum MessageError {
    #[error("received response for unregistered request ID '{0:?}'")]
    UnregisteredId(Id),
    #[error("received unknown notification '{0}'")]
    UnsupportedRequest(String),
    #[error("JSON deserialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("JSON RPC error: {0}")]
    Rpc(#[from] jsonrpc_core::Error),
}
