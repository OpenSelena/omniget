//! MLS (RFC 9420) engine for the OmniDisc client, per ADR 0014 and spike S-05.
//!
//! Everything here is deliberately transport-free: it turns bytes into bytes.
//! The Tauri layer owns the HTTP calls, the gateway and the persistence path;
//! this crate owns the cryptography and the group state machine so it can be
//! unit-tested without a server.

mod client;
mod file;
mod provider;
mod state;

pub use client::{
    fingerprint, ClaimedDevice, CommitOutput, DeviceRef, Incoming, MemberInfo, MlsClient,
    CIPHERSUITE_ID, VOICE_EXPORTER_INFO, VOICE_EXPORTER_LABEL,
};
pub use file::{
    chunk_count, decrypt_file, encrypt_file, encrypted_size, new_file_secret, FileSecret,
    CHUNK_SIZE, TAG_SIZE,
};
pub use state::{decrypt_state, encrypt_state, new_state_key, FORMAT_VERSION};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MlsError {
    #[error("openmls: {0}")]
    Mls(String),
    #[error("no MLS group {0}")]
    UnknownGroup(String),
    #[error("this device was removed from the group")]
    Evicted,
    #[error("malformed MLS payload: {0}")]
    Malformed(String),
    #[error("this device is already in MLS group {0}")]
    GroupExists(String),
    #[error("refused, the server cannot be trusted for this: {0}")]
    Untrusted(String),
    #[error("stored MLS state is unreadable: {0}")]
    State(String),
    #[error("file crypto: {0}")]
    File(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, MlsError>;
