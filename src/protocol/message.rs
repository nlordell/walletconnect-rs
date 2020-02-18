use super::Topic;
use crate::serialization;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SocketMessage {
    pub topic: Topic,
    #[serde(rename = "type")]
    pub kind: SocketMessageKind,
    #[serde(with = "serialization::jsonstring")]
    pub payload: Option<EncryptionPayload>,
    #[serde(default)]
    pub silent: bool,
}

#[derive(Copy, Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SocketMessageKind {
    Pub,
    Sub,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EncryptionPayload {
    #[serde(with = "serialization::hexstring")]
    pub data: Vec<u8>,
    #[serde(with = "serialization::hexstring")]
    pub hmac: Vec<u8>,
    #[serde(with = "serialization::hexstring")]
    pub iv: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn message_serialization() {
        let message = SocketMessage {
            topic: "de5682be-2a03-4b8e-866e-1e89dbca422b".parse().unwrap(),
            kind: SocketMessageKind::Pub,
            payload: Some(EncryptionPayload {
                data: vec![0x04, 0x2],
                hmac: vec![0x13, 0x37],
                iv: vec![0x00],
            }),
            silent: false,
        };
        let json = json!({
            "topic": "de5682be-2a03-4b8e-866e-1e89dbca422b",
            "type": "pub",
            "payload": "{\"data\":\"0402\",\"hmac\":\"1337\",\"iv\":\"00\"}",
            "silent": false,
        });

        assert_eq!(serde_json::to_value(&message).unwrap(), json);
        assert_eq!(
            serde_json::from_value::<SocketMessage>(json).unwrap(),
            message
        );
    }
}
