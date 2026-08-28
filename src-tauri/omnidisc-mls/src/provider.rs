use crate::{MlsError, Result};
use openmls_memory_storage::MemoryStorage;
use openmls_rust_crypto::RustCrypto;
use openmls_traits::OpenMlsProvider;
use std::collections::HashMap;

// OpenMlsRustCrypto keeps its storage private, so a restart could never hand it
// back. Our own provider is the only way to serialise and restore group state.
pub struct Provider {
    crypto: RustCrypto,
    storage: MemoryStorage,
}

impl Provider {
    pub fn fresh() -> Self {
        Self { crypto: RustCrypto::default(), storage: MemoryStorage::default() }
    }

    pub fn from_blob(blob: &[u8]) -> Result<Self> {
        Ok(Self { crypto: RustCrypto::default(), storage: storage_from_blob(blob)? })
    }

    pub fn to_blob(&self) -> Vec<u8> {
        storage_to_blob(&self.storage)
    }
}

impl OpenMlsProvider for Provider {
    type CryptoProvider = RustCrypto;
    type RandProvider = RustCrypto;
    type StorageProvider = MemoryStorage;

    fn storage(&self) -> &MemoryStorage {
        &self.storage
    }

    fn crypto(&self) -> &RustCrypto {
        &self.crypto
    }

    fn rand(&self) -> &RustCrypto {
        &self.crypto
    }
}

fn storage_to_blob(storage: &MemoryStorage) -> Vec<u8> {
    let Ok(values) = storage.values.read() else {
        return (0u64).to_be_bytes().to_vec();
    };
    let mut out = Vec::new();
    out.extend_from_slice(&(values.len() as u64).to_be_bytes());
    // The map has no order of its own; sort so an unchanged state produces an
    // unchanged blob and a redundant disk write can be skipped.
    let mut entries: Vec<_> = values.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    for (k, v) in entries {
        out.extend_from_slice(&(k.len() as u64).to_be_bytes());
        out.extend_from_slice(&(v.len() as u64).to_be_bytes());
        out.extend_from_slice(k);
        out.extend_from_slice(v);
    }
    out
}

fn storage_from_blob(mut blob: &[u8]) -> Result<MemoryStorage> {
    fn take_u64(b: &mut &[u8]) -> Result<usize> {
        if b.len() < 8 {
            return Err(MlsError::State("truncated storage blob".into()));
        }
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[..8]);
        *b = &b[8..];
        Ok(u64::from_be_bytes(a) as usize)
    }
    fn take(b: &mut &[u8], n: usize) -> Result<Vec<u8>> {
        if b.len() < n {
            return Err(MlsError::State("truncated storage blob".into()));
        }
        let v = b[..n].to_vec();
        *b = &b[n..];
        Ok(v)
    }
    let count = take_u64(&mut blob)?;
    let mut map = HashMap::with_capacity(count.min(4096));
    for _ in 0..count {
        let kl = take_u64(&mut blob)?;
        let vl = take_u64(&mut blob)?;
        let k = take(&mut blob, kl)?;
        let v = take(&mut blob, vl)?;
        map.insert(k, v);
    }
    Ok(MemoryStorage { values: std::sync::RwLock::new(map) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_blob_round_trips_and_rejects_truncation() {
        let provider = Provider::fresh();
        {
            let mut values = provider.storage.values.write().expect("lock");
            values.insert(b"a".to_vec(), b"one".to_vec());
            values.insert(b"bb".to_vec(), b"two".to_vec());
        }
        let blob = provider.to_blob();
        let restored = Provider::from_blob(&blob).expect("restore");
        let values = restored.storage.values.read().expect("lock");
        assert_eq!(values.get(b"a".as_slice()).map(Vec::as_slice), Some(b"one".as_slice()));
        assert_eq!(values.len(), 2);
        assert!(Provider::from_blob(&blob[..blob.len() - 1]).is_err());
    }
}
