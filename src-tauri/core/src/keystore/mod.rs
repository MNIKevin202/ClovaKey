//! OS-backed secure key storage.
//!
//! ClovaKey's vault master key is protected by the operating system's native
//! credential store — the login Keychain on macOS and the Credential Manager
//! (DPAPI-protected) on Windows — accessed through the maintained `keyring`
//! crate. A pure in-memory implementation backs the unit tests so they never
//! touch the real OS store.

use crate::error::{CoreError, Result};
use data_encoding::BASE64;
use zeroize::Zeroizing;

/// Abstraction over a secure secret store keyed by a string label.
///
/// Values are arbitrary bytes (we store them base64-encoded internally so the
/// backend only ever sees a string, which is the most portable option).
pub trait KeyStore: Send + Sync {
    fn store(&self, label: &str, secret: &[u8]) -> Result<()>;
    fn retrieve(&self, label: &str) -> Result<Option<Zeroizing<Vec<u8>>>>;
    fn delete(&self, label: &str) -> Result<()>;
}

/// The default OS keychain-backed store.
pub struct OsKeyStore {
    service: String,
}

impl OsKeyStore {
    /// Create a store namespaced under `service` (e.g. a reverse-DNS app id).
    pub fn new(service: impl Into<String>) -> Self {
        OsKeyStore {
            service: service.into(),
        }
    }

    fn entry(&self, label: &str) -> Result<keyring::Entry> {
        keyring::Entry::new(&self.service, label).map_err(|e| CoreError::KeyStore(e.to_string()))
    }
}

impl KeyStore for OsKeyStore {
    fn store(&self, label: &str, secret: &[u8]) -> Result<()> {
        let encoded = BASE64.encode(secret);
        self.entry(label)?
            .set_password(&encoded)
            .map_err(|e| CoreError::KeyStore(e.to_string()))
    }

    fn retrieve(&self, label: &str) -> Result<Option<Zeroizing<Vec<u8>>>> {
        match self.entry(label)?.get_password() {
            Ok(encoded) => {
                let bytes = BASE64
                    .decode(encoded.as_bytes())
                    .map_err(|_| CoreError::KeyStore("stored key is corrupt".into()))?;
                Ok(Some(Zeroizing::new(bytes)))
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(CoreError::KeyStore(e.to_string())),
        }
    }

    fn delete(&self, label: &str) -> Result<()> {
        match self.entry(label)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(CoreError::KeyStore(e.to_string())),
        }
    }
}

/// In-memory key store used exclusively by tests.
#[derive(Default)]
pub struct MemoryKeyStore {
    inner: std::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>,
}

impl MemoryKeyStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl KeyStore for MemoryKeyStore {
    fn store(&self, label: &str, secret: &[u8]) -> Result<()> {
        self.inner
            .lock()
            .unwrap()
            .insert(label.to_string(), secret.to_vec());
        Ok(())
    }

    fn retrieve(&self, label: &str) -> Result<Option<Zeroizing<Vec<u8>>>> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .get(label)
            .map(|v| Zeroizing::new(v.clone())))
    }

    fn delete(&self, label: &str) -> Result<()> {
        self.inner.lock().unwrap().remove(label);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_store_round_trip() {
        let store = MemoryKeyStore::new();
        assert!(store.retrieve("k").unwrap().is_none());
        store.store("k", &[1, 2, 3]).unwrap();
        assert_eq!(&store.retrieve("k").unwrap().unwrap()[..], &[1, 2, 3]);
        store.delete("k").unwrap();
        assert!(store.retrieve("k").unwrap().is_none());
        // Deleting a missing entry is a no-op.
        store.delete("k").unwrap();
    }
}
