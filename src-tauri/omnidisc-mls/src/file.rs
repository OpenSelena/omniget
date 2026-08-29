//! Per-file XChaCha20-Poly1305, one AEAD per 1 MiB chunk (ADR 0014 §3).
//!
//! Nonce = `file_nonce(16) ‖ chunk_index u64 BE`, AAD =
//! `"omnidisc-file-v1|" ‖ file_id ‖ index ‖ total_chunks`. Putting the index and
//! the total in both nonce and AAD is what makes a reordered, duplicated or
//! truncated file fail loudly instead of decoding into garbage.

use crate::{MlsError, Result};
use chacha20poly1305::aead::{AeadInPlace, KeyInit};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

pub const CHUNK_SIZE: usize = 1024 * 1024;
pub const TAG_SIZE: usize = 16;

/// The key material that travels inside the MLS message, never near the server.
/// `Debug` is deliberately not derived so a key cannot be logged by accident.
#[derive(Clone, Serialize, Deserialize)]
pub struct FileSecret {
    pub key: [u8; 32],
    pub nonce: [u8; 16],
}

pub fn new_file_secret() -> FileSecret {
    let mut rng = rand::rng();
    let mut key = [0u8; 32];
    let mut nonce = [0u8; 16];
    rng.fill_bytes(&mut key);
    rng.fill_bytes(&mut nonce);
    FileSecret { key, nonce }
}

pub fn chunk_count(size: u64) -> u64 {
    if size == 0 {
        1
    } else {
        size.div_ceil(CHUNK_SIZE as u64)
    }
}

pub fn encrypted_size(size: u64) -> u64 {
    size + chunk_count(size) * TAG_SIZE as u64
}

fn chunk_nonce(file_nonce: &[u8; 16], index: u64) -> XNonce {
    let mut n = [0u8; 24];
    n[..16].copy_from_slice(file_nonce);
    n[16..].copy_from_slice(&index.to_be_bytes());
    *XNonce::from_slice(&n)
}

fn chunk_aad(file_id: &str, index: u64, total: u64) -> Vec<u8> {
    let mut aad = Vec::with_capacity(64);
    aad.extend_from_slice(b"omnidisc-file-v1|");
    aad.extend_from_slice(file_id.as_bytes());
    aad.extend_from_slice(&index.to_be_bytes());
    aad.extend_from_slice(&total.to_be_bytes());
    aad
}

/// Streams `src` into `dst`, one chunk at a time. Memory stays at one chunk
/// regardless of file size, which is the whole point (a 2 GB upload must not
/// mean 2 GB of RAM). Returns the plaintext size and its SHA-256, both of which
/// go in the manifest so the receiver can verify what it got.
pub fn encrypt_file(
    src: &Path,
    dst: &Path,
    secret: &FileSecret,
    file_id: &str,
) -> Result<(u64, String)> {
    let size = std::fs::metadata(src)?.len();
    let total = chunk_count(size);
    let cipher = XChaCha20Poly1305::new(Key::from_slice(&secret.key));
    let mut reader = BufReader::with_capacity(CHUNK_SIZE, File::open(src)?);
    let mut writer = BufWriter::with_capacity(CHUNK_SIZE, File::create(dst)?);
    let mut hasher = Sha256::new();
    let mut buf: Vec<u8> = Vec::with_capacity(CHUNK_SIZE + TAG_SIZE);
    let mut index = 0u64;
    loop {
        buf.clear();
        buf.resize(CHUNK_SIZE, 0);
        let mut filled = 0usize;
        while filled < CHUNK_SIZE {
            let n = reader.read(&mut buf[filled..])?;
            if n == 0 {
                break;
            }
            filled += n;
        }
        if filled == 0 && index > 0 {
            break;
        }
        buf.truncate(filled);
        hasher.update(&buf);
        cipher
            .encrypt_in_place(
                &chunk_nonce(&secret.nonce, index),
                &chunk_aad(file_id, index, total),
                &mut buf,
            )
            .map_err(|_| MlsError::File(format!("chunk {index}: could not be encrypted")))?;
        writer.write_all(&buf)?;
        index += 1;
        if filled < CHUNK_SIZE {
            break;
        }
    }
    writer.flush()?;
    Ok((size, hex(&hasher.finalize())))
}

/// Reverse of [`encrypt_file`]; returns the SHA-256 of the plaintext it wrote so
/// the caller can compare it with the manifest.
pub fn decrypt_file(
    src: &Path,
    dst: &Path,
    secret: &FileSecret,
    file_id: &str,
    size: u64,
) -> Result<String> {
    let total = chunk_count(size);
    let cipher = XChaCha20Poly1305::new(Key::from_slice(&secret.key));
    let mut reader = BufReader::with_capacity(CHUNK_SIZE, File::open(src)?);
    let mut writer = BufWriter::with_capacity(CHUNK_SIZE, File::create(dst)?);
    let mut hasher = Sha256::new();
    let mut buf: Vec<u8> = Vec::with_capacity(CHUNK_SIZE + TAG_SIZE);
    let mut remaining = size;
    for index in 0..total {
        let plain_len = remaining.min(CHUNK_SIZE as u64) as usize;
        buf.clear();
        buf.resize(plain_len + TAG_SIZE, 0);
        reader
            .read_exact(&mut buf)
            .map_err(|_| MlsError::File(format!("chunk {index}: the file ends too early")))?;
        cipher
            .decrypt_in_place(
                &chunk_nonce(&secret.nonce, index),
                &chunk_aad(file_id, index, total),
                &mut buf,
            )
            .map_err(|_| MlsError::File(format!("chunk {index}: authentication failed")))?;
        hasher.update(&buf);
        writer.write_all(&buf)?;
        remaining -= plain_len as u64;
    }
    writer.flush()?;
    Ok(hex(&hasher.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "omnidisc-mls-file-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir.join(name)
    }

    fn write(path: &Path, bytes: &[u8]) {
        std::fs::write(path, bytes).expect("write");
    }

    #[test]
    fn round_trips_a_multi_chunk_file_and_keeps_the_hash() {
        let plain = temp("plain.bin");
        let enc = temp("enc.bin");
        let out = temp("out.bin");
        let data: Vec<u8> = (0..(CHUNK_SIZE * 2 + 4096))
            .map(|i| (i % 251) as u8)
            .collect();
        write(&plain, &data);
        let secret = new_file_secret();
        let (size, sha) = encrypt_file(&plain, &enc, &secret, "file-1").expect("encrypt");
        assert_eq!(size, data.len() as u64);
        assert_eq!(
            std::fs::metadata(&enc).expect("meta").len(),
            encrypted_size(size)
        );
        let back = decrypt_file(&enc, &out, &secret, "file-1", size).expect("decrypt");
        assert_eq!(back, sha);
        assert_eq!(std::fs::read(&out).expect("read"), data);
    }

    #[test]
    fn an_empty_file_is_still_one_authenticated_chunk() {
        let plain = temp("empty.bin");
        let enc = temp("empty.enc");
        let out = temp("empty.out");
        write(&plain, b"");
        let secret = new_file_secret();
        let (size, _) = encrypt_file(&plain, &enc, &secret, "file-e").expect("encrypt");
        assert_eq!(size, 0);
        assert_eq!(
            std::fs::metadata(&enc).expect("meta").len(),
            TAG_SIZE as u64
        );
        decrypt_file(&enc, &out, &secret, "file-e", 0).expect("decrypt");
        assert!(std::fs::read(&out).expect("read").is_empty());
    }

    #[test]
    fn tampering_dropping_and_swapping_chunks_are_all_rejected() {
        let plain = temp("t.bin");
        let enc = temp("t.enc");
        let out = temp("t.out");
        let data: Vec<u8> = (0..(CHUNK_SIZE * 2 + 10)).map(|i| (i % 97) as u8).collect();
        write(&plain, &data);
        let secret = new_file_secret();
        let (size, _) = encrypt_file(&plain, &enc, &secret, "file-2").expect("encrypt");
        let good = std::fs::read(&enc).expect("read");

        let mut tampered = good.clone();
        tampered[CHUNK_SIZE + 32] ^= 0xff;
        write(&enc, &tampered);
        assert!(decrypt_file(&enc, &out, &secret, "file-2", size).is_err());

        let cut = &good[..good.len() - (10 + TAG_SIZE)];
        write(&enc, cut);
        assert!(decrypt_file(&enc, &out, &secret, "file-2", size).is_err());

        let stride = CHUNK_SIZE + TAG_SIZE;
        let mut swapped = Vec::with_capacity(good.len());
        swapped.extend_from_slice(&good[stride..stride * 2]);
        swapped.extend_from_slice(&good[..stride]);
        swapped.extend_from_slice(&good[stride * 2..]);
        write(&enc, &swapped);
        assert!(decrypt_file(&enc, &out, &secret, "file-2", size).is_err());

        write(&enc, &good);
        assert!(decrypt_file(&enc, &out, &secret, "file-other", size).is_err());
        let other = new_file_secret();
        assert!(decrypt_file(&enc, &out, &other, "file-2", size).is_err());
    }

    #[test]
    fn chunk_counts_cover_the_edges() {
        assert_eq!(chunk_count(0), 1);
        assert_eq!(chunk_count(1), 1);
        assert_eq!(chunk_count(CHUNK_SIZE as u64), 1);
        assert_eq!(chunk_count(CHUNK_SIZE as u64 + 1), 2);
        assert_eq!(
            encrypted_size(CHUNK_SIZE as u64 + 1),
            CHUNK_SIZE as u64 + 1 + 32
        );
    }
}
