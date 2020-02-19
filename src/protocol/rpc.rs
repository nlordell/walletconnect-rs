use crate::protocol::Topic;
use crate::serialization;
use ethereum_types::{Address, U256};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub description: String,
    pub url: Url,
    #[serde(default)]
    pub icons: Vec<Url>,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PeerMetadata {
    Strict(Metadata),
    Malformed(Value),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionRequest {
    pub chain_id: Option<u64>,
    pub peer_id: Topic,
    pub peer_meta: Metadata,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionParams {
    pub approved: bool,
    pub accounts: Vec<Address>,
    pub chain_id: u64,
    pub peer_id: Topic,
    pub peer_meta: PeerMetadata,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionUpdate {
    pub approved: bool,
    pub accounts: Vec<Address>,
    pub chain_id: u64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    pub from: Address,
    #[serde(default, with = "serialization::emptynone")]
    pub to: Option<Address>,
    #[serde(default)]
    pub gas_limit: Option<U256>,
    #[serde(default)]
    pub gas_price: Option<U256>,
    #[serde(default)]
    pub value: U256,
    #[serde(default, with = "serialization::prefixedhexstring")]
    pub data: Vec<u8>,
    #[serde(default)]
    pub nonce: Option<U256>,
}
