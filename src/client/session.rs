use crate::crypto::Key;
use crate::protocol::{
    Metadata, PeerMetadata, SessionParams, SessionRequest, SessionUpdate, Topic,
};
use crate::uri::Uri;
use ethereum_types::Address;
use serde::{Deserialize, Serialize};
use url::form_urlencoded::Serializer;
use url::Url;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub connected: bool,
    pub accounts: Vec<Address>,
    pub chain_id: Option<u64>,
    pub bridge: Url,
    pub key: Key,
    pub client_id: Topic,
    pub client_meta: Metadata,
    pub peer_id: Option<Topic>,
    pub peer_meta: Option<PeerMetadata>,
    pub handshake_id: u64,
    pub handshake_topic: Topic,
}

impl Session {
    pub fn uri(&self) -> Uri {
        Uri::parse(&format!(
            "wc:{}@1?{}",
            self.handshake_topic,
            Serializer::new(String::new())
                .append_pair("bridge", self.bridge.as_str())
                .append_pair("key", self.key.display().as_str())
                .finish()
        ))
        .expect("WalletConnect URIs from sessions are always valid")
    }

    pub fn request(&self) -> SessionRequest {
        SessionRequest {
            peer_id: self.client_id.clone(),
            peer_meta: self.client_meta.clone(),
            chain_id: self.chain_id,
        }
    }

    pub fn apply(&mut self, params: SessionParams) {
        self.connected = params.approved;
        self.accounts = params.accounts;
        self.chain_id = Some(params.chain_id);
        self.peer_id = Some(params.peer_id);
        self.peer_meta = Some(params.peer_meta);
    }

    pub fn update(&mut self, update: SessionUpdate) {
        self.connected = update.approved;
        self.accounts = update.accounts;
        self.chain_id = Some(update.chain_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn new_topic_is_random() {
        assert_ne!(Topic::new(), Topic::new());
    }

    #[test]
    fn zero_topic() {
        assert_eq!(
            json!(Topic::zero()),
            json!("00000000-0000-0000-0000-000000000000")
        );
    }

    #[test]
    fn topic_serialization() {
        let topic = Topic::new();
        let serialized = serde_json::to_string(&topic).unwrap();
        let deserialized = serde_json::from_str(&serialized).unwrap();
        assert_eq!(topic, deserialized);
    }
}
