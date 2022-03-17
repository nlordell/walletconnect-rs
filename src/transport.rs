use crate::client::{Client, ConnectorError, NotConnectedError, SessionError};
use crate::protocol::Transaction;
use ethereum_types::Address;
use futures::future::{BoxFuture, FutureExt};
use jsonrpc_core::{Call, MethodCall, Params};
use serde::Deserialize;
use serde_json::{json, Value};
use std::error::Error;
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
                return Err(web3::Error::Transport(
                    web3::error::TransportError::Message(format!(
                        "unknown chain ID '{}'",
                        chain_id
                    )),
                ))
            }
        };
        let url = format!("https://{}.infura.io/v3/{}", network, self.0);
        let http = Http::new(&url)?;

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
    pub fn new(client: Client, infura_id: impl Into<String>) -> Result<Self, TransportError> {
        WalletConnect::with_factory(client, InfuraTransportFactory(infura_id.into()))
    }
}

impl<T> WalletConnect<T>
where
    T: Transport,
{
    pub fn with_factory<F>(client: Client, mut factory: F) -> Result<Self, TransportError>
    where
        F: TransportFactory<Transport = T>,
        F::Error: 'static,
    {
        let (accounts, chain_id) = client.accounts()?;
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

    pub fn accounts(&self) -> (Vec<Address>, u64) {
        (self.0.accounts.clone(), self.0.chain_id)
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
    type Out = BoxFuture<'static, Result<Value, web3::Error>>;

    fn prepare(&self, method: &str, params: Vec<Value>) -> (RequestId, Call) {
        log::trace!("preparing call '{}' {:?}", method, params);
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
                    log::trace!(">>{}", params[0]);
                    let transaction = Transaction::deserialize(&params[0])?;
                    let tx = inner
                        .client
                        .send_transaction(transaction)
                        .await
                        .map_err(|err| {
                            web3::Error::Transport(web3::error::TransportError::Message(
                                err.to_string(),
                            ))
                        })?;
                    Ok(json!(tx))
                }
                request => inner.transport.send(id, request).await,
            }
        }
        .boxed()
    }
}
