use crate::client::{Client, ConnectorError, NotConnectedError, SessionError};
use crate::protocol::{Metadata, Transaction};
use crate::uri::Uri;
use ethereum_types::Address;
use futures::compat::{Compat, Future01CompatExt};
use futures::future::{BoxFuture, FutureExt, TryFutureExt};
use jsonrpc_core::{Call, MethodCall, Params};
use serde::Deserialize;
use serde_json::{json, Value};
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use web3::transports::Http;
use web3::types::U64;
use web3::{helpers, RequestId, Transport};

pub trait TransportFactory {
    type Transport: Transport;
    type Error: Error;

    fn new(&mut self, chain_id: u64) -> Result<Self::Transport, Self::Error>;
}

struct InfuraTransportFactory(String);

impl TransportFactory for InfuraTransportFactory {
    type Transport = Http;
    type Error = web3::Error;

    fn new(&mut self, chain_id: u64) -> Result<Self::Transport, Self::Error> {
        let network = match chain_id {
            1 => "mainnet",
            3 => "ropsten",
            4 => "rinkeby",
            5 => "goerli",
            42 => "kovan",
            _ => {
                return Err(web3::Error::Transport(format!(
                    "unknown chain ID '{}'",
                    chain_id
                )))
            }
        };
        let url = format!("https://{}.infura.io/v3/{}", network, self.0);
        let (event_loop, http) = Http::new(&url)?;

        event_loop.into_remote();
        Ok(http)
    }
}

#[derive(Clone, Debug)]
pub struct WalletConnect<T>(Arc<Inner<T>>);

#[derive(Debug)]
struct Inner<T> {
    client: Client,
    accounts: Vec<Address>,
    chain_id: u64,
    transport: T,
}

impl WalletConnect<Http> {
    pub async fn new(
        profile: impl Into<PathBuf>,
        meta: impl Into<Metadata>,
        infura_id: impl Into<String>,
        display: impl FnOnce(Uri),
    ) -> Result<Self, TransportError> {
        WalletConnect::with_factory(
            profile,
            meta,
            InfuraTransportFactory(infura_id.into()),
            display,
        )
        .await
    }
}

impl<T> WalletConnect<T>
where
    T: Transport,
{
    pub async fn with_factory<F>(
        profile: impl Into<PathBuf>,
        meta: impl Into<Metadata>,
        mut factory: F,
        display: impl FnOnce(Uri),
    ) -> Result<Self, TransportError>
    where
        F: TransportFactory<Transport = T>,
        F::Error: 'static,
    {
        let client = Client::new(profile, meta)?;
        let (accounts, chain_id) = client.ensure_session(display).await?;

        let transport = factory
            .new(chain_id)
            .map_err(|err| TransportError::Transport(Box::new(err)))?;

        Ok(WalletConnect(Arc::new(Inner {
            client,
            accounts,
            chain_id,
            transport,
        })))
    }
}

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("client connector error: {0}")]
    Connector(#[from] ConnectorError),
    #[error("error establising a WalletConnect session: {0}")]
    Session(#[from] SessionError),
    #[error("connection unexpectedly dropped: {0}")]
    ConnectionDropped(#[from] NotConnectedError),
    #[error("error creating transport: {0}")]
    Transport(Box<dyn Error>),
}

impl<T> Transport for WalletConnect<T>
where
    T: Transport + Send + Sync + 'static,
    T::Out: Send,
{
    type Out = Compat<BoxFuture<'static, Result<Value, web3::Error>>>;

    fn prepare(&self, method: &str, params: Vec<Value>) -> (RequestId, Call) {
        match method {
            "eth_accounts" | "eth_chainId" | "eth_sendTransaction" => {
                (0, helpers::build_request(0, method, params))
            }
            _ => self.0.transport.prepare(method, params),
        }
    }

    fn send(&self, id: RequestId, request: Call) -> Self::Out {
        let inner = self.0.clone();
        async move {
            match request {
                Call::MethodCall(MethodCall { method, .. }) if method == "eth_accounts" => {
                    Ok(json!(inner.accounts))
                }
                Call::MethodCall(MethodCall { method, .. }) if method == "eth_chainId" => {
                    Ok(json!(U64::from(inner.chain_id)))
                }
                Call::MethodCall(MethodCall {
                    method,
                    params: Params::Array(params),
                    ..
                }) if method == "eth_sendTransaction" && !params.is_empty() => {
                    let transaction = Transaction::deserialize(&params[0])?;
                    let tx = inner
                        .client
                        .send_transaction(transaction)
                        .await
                        .map_err(|err| web3::Error::Transport(err.to_string()))?;
                    Ok(json!(tx))
                }
                request => inner.transport.send(id, request).compat().await,
            }
        }
        .boxed()
        .compat()
    }
}
