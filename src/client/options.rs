use super::session::Session;
use crate::crypto::Key;
use crate::protocol::{Metadata, Topic};
use crate::uri::Uri;
use lazy_static::lazy_static;
use std::path::PathBuf;
use url::Url;

lazy_static! {
    pub static ref DEFAULT_BRIDGE_URL: Url =
        Url::parse("https://bridge.walletconnect.org").unwrap();
}

#[derive(Clone, Debug)]
pub enum Connection {
    Bridge(Url),
    Uri(Uri),
}

impl Default for Connection {
    fn default() -> Self {
        Connection::Bridge(DEFAULT_BRIDGE_URL.clone())
    }
}

#[derive(Clone, Debug)]
pub struct Options {
    pub profile: PathBuf,
    pub meta: Metadata,
    pub connection: Connection,
    pub chain_id: Option<u64>,
}

impl Options {
    pub fn new(profile: impl Into<PathBuf>, meta: Metadata) -> Self {
        Options {
            profile: profile.into(),
            meta,
            connection: Connection::default(),
            chain_id: None,
        }
    }

    pub fn with_uri(profile: impl Into<PathBuf>, meta: Metadata, uri: Uri) -> Self {
        Options {
            profile: profile.into(),
            meta,
            connection: Connection::Uri(uri),
            chain_id: None,
        }
    }

    pub fn create_session(self) -> Session {
        let client_meta = self.meta;
        let (handshake_topic, bridge, key) = match self.connection {
            Connection::Bridge(bridge) => (Topic::new(), bridge, Key::random()),
            Connection::Uri(uri) => uri.into_parts(),
        };
        let chain_id = self.chain_id;

        Session {
            connected: false,
            accounts: Vec::new(),
            chain_id,
            bridge,
            key,
            client_id: Topic::new(),
            client_meta,
            peer_id: None,
            peer_meta: None,
            handshake_id: 0,
            handshake_topic,
        }
    }

    pub fn matches(&self, session: &Session) -> bool {
        self.meta == session.client_meta
            && match &self.connection {
                Connection::Bridge(bridge) => *bridge == session.bridge,
                Connection::Uri(uri) => *uri == session.uri(),
            }
    }
}
