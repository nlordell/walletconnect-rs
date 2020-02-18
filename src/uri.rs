use crate::crypto::Key;
use crate::protocol::Topic;
use std::ops::Deref;
use std::str::FromStr;
use thiserror::Error;
use url::Url;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Uri {
    handshake_topic: Topic,
    version: u64,
    bridge: Url,
    key: Key,
    url: Url,
}

const VERSION: u64 = 1;

impl Uri {
    pub fn parse(uri: impl AsRef<str>) -> Result<Self, InvalidSessionUri> {
        let url = Url::parse(uri.as_ref())?;
        if url.scheme() != "wc" {
            return Err(InvalidSessionUri);
        }

        let mut path = url.path().splitn(2, '@');
        let handshake_topic = path.next().ok_or(InvalidSessionUri)?.parse()?;
        let version = path.next().ok_or(InvalidSessionUri)?.parse()?;
        if version != VERSION {
            return Err(InvalidSessionUri);
        }

        let mut bridge: Option<Url> = None;
        let mut key: Option<Key> = None;
        for (name, value) in url.query_pairs() {
            match &*name {
                "bridge" => bridge = Some(value.parse()?),
                "key" => key = Some(value.parse()?),
                _ => return Err(InvalidSessionUri),
            }
        }

        Ok(Uri {
            handshake_topic,
            version,
            bridge: bridge.ok_or(InvalidSessionUri)?,
            key: key.ok_or(InvalidSessionUri)?,
            url,
        })
    }

    pub fn handshake_topic(&self) -> &Topic {
        &self.handshake_topic
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn bridge(&self) -> &Url {
        &self.bridge
    }

    pub fn key(&self) -> &Key {
        &self.key
    }

    pub fn into_parts(self) -> (Topic, Url, Key) {
        (self.handshake_topic, self.bridge, self.key)
    }

    pub fn as_url(&self) -> &Url {
        &self.url
    }
}

impl Deref for Uri {
    type Target = Url;

    fn deref(&self) -> &Self::Target {
        self.as_url()
    }
}

impl FromStr for Uri {
    type Err = InvalidSessionUri;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uri::parse(s)
    }
}

impl AsRef<str> for Uri {
    fn as_ref(&self) -> &str {
        self.as_url().as_str()
    }
}

impl AsRef<[u8]> for Uri {
    fn as_ref(&self) -> &[u8] {
        self.as_url().as_str().as_bytes()
    }
}

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error("session URI is invalid")]
pub struct InvalidSessionUri;

macro_rules! impl_invalid_session_uri_from {
    ($err:ty) => {
        impl From<$err> for InvalidSessionUri {
            fn from(_: $err) -> Self {
                InvalidSessionUri
            }
        }
    };
}

impl_invalid_session_uri_from!(data_encoding::DecodeError);
impl_invalid_session_uri_from!(std::num::ParseIntError);
impl_invalid_session_uri_from!(url::ParseError);
impl_invalid_session_uri_from!(uuid::Error);
