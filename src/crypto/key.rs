use super::aead::{self, OpenError, SealError};
use crate::hex;
use crate::protocol::EncryptionPayload;
use data_encoding::DecodeError;
use serde::de::{self, Deserialize, Deserializer};
use serde::ser::{Serialize, Serializer};
use std::borrow::Cow;
use std::fmt::{self, Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::str::FromStr;
use thiserror::Error;
use zeroize::Zeroizing;

#[derive(Clone, Eq, PartialEq)]
pub struct Key(Zeroizing<[u8; 32]>);

impl Key {
    pub fn random() -> Self {
        Key::from_raw(rand::random())
    }

    pub fn from_raw(raw: [u8; 32]) -> Self {
        Key(raw.into())
    }

    pub fn display(&self) -> DisplayKey {
        DisplayKey(hex::encode(self.0.as_slice()))
    }

    pub fn seal(&self, data: impl AsRef<[u8]>) -> Result<EncryptionPayload, SealError> {
        aead::seal(self, data.as_ref())
    }

    pub fn open(&self, payload: &EncryptionPayload) -> Result<Vec<u8>, OpenError> {
        aead::open(self, payload)
    }
}

impl FromStr for Key {
    type Err = DecodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut bytes = [0u8; 32];
        hex::decode_mut(s, &mut bytes)?;
        Ok(Key::from_raw(bytes))
    }
}

impl Debug for Key {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("Key(********)")
    }
}

#[allow(clippy::derive_hash_xor_eq)]
impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl Deref for Key {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

#[derive(Clone, Copy, Debug, Default, Error, Eq, PartialEq)]
#[error("key must be exactly 32 bytes")]
pub struct KeyLengthError;

#[derive(Debug)]
pub struct DisplayKey(String);

impl DisplayKey {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for DisplayKey {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for Key {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.display().as_str())
    }
}

impl<'de> Deserialize<'de> for Key {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = Cow::<'de, str>::deserialize(deserializer)?;
        Key::from_str(&s).map_err(de::Error::custom)
    }
}
