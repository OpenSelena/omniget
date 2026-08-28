//! MLS state at rest: XChaCha20-Poly1305 under a key the caller keeps in the OS
//! keyring, behind a version header. openmls 0.9 changes the storage encoding
//! (S-05 condition 1), so the version is written now to make that migration an
//! import rather than a data loss.

use crate::{MlsError, Result};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use rand::Rng;

pub const FORMAT_VERSION: u32 = 1;
const MAGIC: &[u8; 6] = b"ODMLS\0";
const NONCE_LEN: usize = 24;

pub fn new_state_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    rand::rng().fill_bytes(&mut key);
    key
}

fn header(version: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(MAGIC.len() + 4);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&version.to_be_bytes());
    out
}

pub fn encrypt_state(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>> {
    let aad = header(FORMAT_VERSION);
    let mut nonce = [0u8; NONCE_LEN];
    rand::rng().fill_bytes(&mut nonce);
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key));
    let ciphertext = cipher
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: plaintext,
                aad: &aad,
            },
        )
        .map_err(|_| MlsError::State("could not encrypt the MLS state".into()))?;
    let mut out = aad;
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

pub fn decrypt_state(key: &[u8; 32], blob: &[u8]) -> Result<Vec<u8>> {
    let head = MAGIC.len() + 4;
    if blob.len() < head + NONCE_LEN {
        return Err(MlsError::State("the stored MLS state is truncated".into()));
    }
    if &blob[..MAGIC.len()] != MAGIC {
        return Err(MlsError::State("this file is not an MLS state blob".into()));
    }
    let version = u32::from_be_bytes([blob[6], blob[7], blob[8], blob[9]]);
    if version != FORMAT_VERSION {
        return Err(MlsError::State(format!(
            "the stored MLS state is version {version}, this build reads {FORMAT_VERSION}"
        )));
    }
    let nonce = &blob[head..head + NONCE_LEN];
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key));
    cipher
        .decrypt(
            XNonce::from_slice(nonce),
            Payload {
                msg: &blob[head + NONCE_LEN..],
                aad: &blob[..head],
            },
        )
        .map_err(|_| MlsError::State("the stored MLS state failed its integrity check".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_refuses_tampering() {
        let key = new_state_key();
        let blob = encrypt_state(&key, b"group state").expect("encrypt");
        assert_eq!(&blob[..6], MAGIC);
        assert_eq!(decrypt_state(&key, &blob).expect("decrypt"), b"group state");

        let mut bad = blob.clone();
        let last = bad.len() - 1;
        bad[last] ^= 0xff;
        assert!(decrypt_state(&key, &bad).is_err());

        assert!(decrypt_state(&new_state_key(), &blob).is_err());
        assert!(decrypt_state(&key, &blob[..8]).is_err());
    }

    #[test]
    fn a_future_version_is_named_not_guessed() {
        let key = new_state_key();
        let mut blob = encrypt_state(&key, b"x").expect("encrypt");
        blob[9] = 2;
        let err = decrypt_state(&key, &blob).expect_err("version mismatch");
        assert!(err.to_string().contains("version 2"), "{err}");
    }
}
