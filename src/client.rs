mod core;
mod options;
mod session;
mod socket;
mod storage;

use self::core::Connector;
pub use self::core::{CallError, ConnectorError, NotConnectedError, SessionError};
pub use self::options::{Connection, Options, DEFAULT_BRIDGE_URL};
pub use self::socket::SocketError;
use crate::protocol::{Metadata, Transaction};
use crate::uri::Uri;
use ethers_core::types::{Address, Bytes, Signature, H256};
use std::path::PathBuf;

#[derive(Debug)]
pub struct Client {
    connection: Connector,
}

impl Client {
    pub fn new(
        profile: impl Into<PathBuf>,
        meta: impl Into<Metadata>,
    ) -> Result<Self, ConnectorError> {
        Client::with_options(Options::new(profile, meta.into()))
    }

    pub fn with_options(options: Options) -> Result<Self, ConnectorError> {
        Ok(Client {
            connection: Connector::new(options)?,
        })
    }

    pub fn accounts(&self) -> Result<(Vec<Address>, u64), NotConnectedError> {
        self.connection.accounts()
    }

    pub async fn ensure_session<F>(&self, f: F) -> Result<(Vec<Address>, u64), SessionError>
    where
        F: FnOnce(Uri),
    {
        self.connection.ensure_session(f).await
    }

    pub async fn send_transaction(&self, transaction: Transaction) -> Result<H256, CallError> {
        self.connection.send_transaction(transaction).await
    }

    pub async fn sign_transaction(&self, transaction: Transaction) -> Result<Bytes, CallError> {
        self.connection.sign_transaction(transaction).await
    }

    pub async fn personal_sign(&self, data: &[&str]) -> Result<Signature, CallError> {
        let sig = self.connection.personal_sign(data).await?;
        Ok(sig.as_ref().try_into().unwrap())
    }

    pub fn close(self) -> Result<(), SocketError> {
        self.connection.close()
    }
}
