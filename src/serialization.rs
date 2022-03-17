use crate::hex;
use ethereum_types::Address;
use serde::de::{DeserializeOwned, Error as _};
use serde::ser::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;

pub mod jsonstring {
    use super::*;

    pub fn serialize<T, S>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Serialize,
        S: Serializer,
    {
        let json = match value {
            None => Cow::from(""),
            Some(value) => serde_json::to_string(value)
                .map_err(S::Error::custom)?
                .into(),
        };
        serializer.serialize_str(&json)
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
    where
        T: DeserializeOwned,
        D: Deserializer<'de>,
    {
        let json = Cow::<'de, str>::deserialize(deserializer)?;
        if !json.is_empty() {
            let value = serde_json::from_str(&json).map_err(D::Error::custom)?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }
}

pub mod prefixedhexstring {
    use super::*;

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("0x{}", hex::encode(bytes)))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string = Cow::<'de, str>::deserialize(deserializer)?;
        if !string.starts_with("0x") {
            return Err(D::Error::custom("hex string missing '0x' prefix"));
        }

        let bytes = hex::decode(&string[2..]).map_err(D::Error::custom)?;
        Ok(bytes)
    }
}

pub mod hexstring {
    use super::*;

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string = Cow::<'de, str>::deserialize(deserializer)?;
        let bytes = hex::decode(&*string).map_err(D::Error::custom)?;
        Ok(bytes)
    }
}

pub mod emptynoneaddress {
    use super::*;

    pub fn serialize<S>(value: &Option<Address>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(value) => value.serialize(serializer),
            None => serializer.serialize_str(""),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Address>, D::Error>
    where
        D: Deserializer<'de>,
    {
        match Cow::<'de, str>::deserialize(deserializer)?.as_ref() {
            "" => Ok(None),
            value => value.parse().map(Some).map_err(D::Error::custom),
        }
    }
}
