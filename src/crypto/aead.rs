use crate::protocol::EncryptionPayload;
use openssl::error::ErrorStack;
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::sign::Signer;
use openssl::symm::{self, Cipher};
use rand::Rng;
use thiserror::Error;

fn hmac_sha256(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, ErrorStack> {
    let key = PKey::hmac(key)?;
    let mut signer = Signer::new(MessageDigest::sha256(), &key)?;
    signer.update(data)?;
    signer.update(iv)?;
    let hmac = signer.sign_to_vec()?;

    Ok(hmac)
}

fn generate_iv() -> Vec<u8> {
    let mut iv = vec![0; 16];
    rand::thread_rng().fill(&mut iv[..]);
    iv
}

pub fn seal(key: &[u8], plaintext: &[u8]) -> Result<EncryptionPayload, SealError> {
    let cipher = Cipher::aes_256_cbc();
    let iv = generate_iv();
    let data = symm::encrypt(cipher, key, Some(&iv), plaintext)?;
    let hmac = hmac_sha256(key, &iv, &data)?;

    Ok(EncryptionPayload { data, iv, hmac })
}

pub fn open(key: &[u8], payload: &EncryptionPayload) -> Result<Vec<u8>, OpenError> {
    let hmac = hmac_sha256(key, &payload.iv, &payload.data)?;
    if hmac != payload.hmac {
        return Err(OpenError::Verify);
    }

    let cipher = Cipher::aes_256_cbc();
    let plaintext = symm::decrypt(cipher, key, Some(&payload.iv), &payload.data)?;

    Ok(plaintext)
}

#[derive(Debug, Error)]
pub enum SealError {
    #[error("internal OpenSSL error: {0}")]
    OpenSsl(#[from] ErrorStack),
}

#[derive(Debug, Error)]
pub enum OpenError {
    #[error("internal OpenSSL error: {0}")]
    OpenSsl(#[from] ErrorStack),
    #[error("unable to verify integrity of payload")]
    Verify,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::Key;
    use crate::hex;
    use std::str;

    #[test]
    fn roundtrip() {
        let message = "walletconnect-rs";
        let key = Key::random();

        let payload = seal(&key, message.as_bytes()).unwrap();
        let plaintext = open(&key, &payload).unwrap();

        assert_eq!(str::from_utf8(&plaintext).unwrap(), message);
    }

    #[test]
    fn open_payload() {
        // Test vector retrieved by inspecting a WalletConnect session with
        // https://example.walletconnect.org

        let key = hex::decode("26075c07b19284e193101d7f27d7f96aa1802645663110a47c5c3bd3da580cae")
            .unwrap();
        let payload = EncryptionPayload {
            data: hex::decode(
                "61e66ba15a7cd452fe14a47ab47a0b49b5deb8bffb9b24c736539600a808a107\
                 98b573ca1c8353e585d95866cd1f2756fef5b0ea334fca5a8f877322712e0b97\
                 33b75400c199212c741bf973c11d3b797f5fb0f413db8a939cfddc4bf8dc96dd\
                 62c01237c8e7038c93f8dbd7d14d22ea82b568cc45fadb3face32350847985cb\
                 57a3e70cb520fe987544084ae125d7913de81c3e7e6e88039ef40cc4b19be1a7\
                 90b6c5509d0822acb7f2bc6d83de528c8f787e29906c5f7ec50d7a8f7b36796f\
                 a3b44edc3538ca6ac039cd17714c50f63b6b9788d3860195e094e571a2a5dba9\
                 b74c8065c04aad11bce2545eb19bd94ad0ee261195b8fa0a738442983d6415a8\
                 81d5d8cd69c07088eb4d979082762c429a3a7ac7d84a4eec84a5144a8675a0e4\
                 094dc1fbc243def3edb2fd15196aa19bce82bedd955126992ff7d952a735a889",
            )
            .unwrap(),
            hmac: hex::decode("1ff024bb7234f3b514b0e0ee130d81f1a367ec09fc2cf191ab52ed07e1f8bbe9")
                .unwrap(),
            iv: hex::decode("019dc30e6463c2c1acd165310d686553").unwrap(),
        };
        let message = r#"{"id":1580823313241457,"jsonrpc":"2.0","method":"wc_sessionRequest","params":[{"peerId":"e8526892-8e47-42e4-9ea3-20c0b164bb83","peerMeta":{"description":"","url":"https://example.walletconnect.org","icons":["https://example.walletconnect.org/favicon.ico"],"name":"WalletConnect Example"},"chainId":null}]}"#;

        let plaintext = open(&key, &payload).unwrap();
        assert_eq!(str::from_utf8(&plaintext).unwrap(), message);
    }
}
