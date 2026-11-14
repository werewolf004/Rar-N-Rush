//! Archive passwords kept in the Windows Credential Manager instead of the
//! settings file.
//!
//! A Windows credential holds at most 2,560 bytes, so the list (one password per
//! line, UTF-8) is split into chunks. Writes are crash-safe: a new *generation*
//! of chunks is written first, then the index entry is switched to it, and only
//! then is the previous generation deleted. A crash at any point leaves either
//! the old or the new list readable, never a mix.
//!
//! The credentials are named `<entry>.RarNRush` and are visible (and removable)
//! in **Control Panel → Credential Manager → Windows Credentials**.

use crate::sync::lock;
use keyring_core::Entry;
use keyring_core::api::CredentialStoreApi;
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

/// Service name of every credential written by Rar N Rush.
pub const SERVICE: &str = "RarNRush";
/// Name of the index entry: `<generation> <chunk count>`.
const INDEX: &str = "passwords";
/// Bytes per chunk, below the 2,560-byte limit of a Windows credential.
const CHUNK: usize = 2_048;
/// Upper bound on the number of chunks read back (guards against a corrupted index).
const MAX_CHUNKS: usize = 256;

/// Errors raised by the password vault.
#[derive(Debug, thiserror::Error)]
pub enum SecretError {
    /// The Credential Manager refused an operation.
    #[error("Windows Credential Manager: {0}")]
    Vault(String),
    /// The stored data is not in the expected format.
    #[error("unreadable password list in the Credential Manager: {0}")]
    Format(String),
}

/// Somewhere to keep the archive passwords.
pub trait SecretVault: Send + Sync + fmt::Debug {
    /// Reads the passwords. An empty vault gives an empty list.
    fn load(&self) -> Result<Vec<String>, SecretError>;
    /// Replaces the stored passwords with `passwords`.
    fn save(&self, passwords: &[String]) -> Result<(), SecretError>;
}

/// Raw key/value access used by [`ChunkedVault`]: the real Credential Manager, or
/// memory in tests.
trait RawStore: Send + Sync {
    fn get(&self, name: &str) -> Result<Option<Vec<u8>>, SecretError>;
    fn set(&self, name: &str, value: &[u8]) -> Result<(), SecretError>;
    fn delete(&self, name: &str) -> Result<(), SecretError>;
}

/// The Windows Credential Manager.
struct WindowsStore {
    store: Arc<windows_native_keyring_store::Store>,
}

impl WindowsStore {
    fn entry(&self, name: &str) -> Result<Entry, SecretError> {
        self.store
            .build(SERVICE, name, None)
            .map_err(|e| SecretError::Vault(e.to_string()))
    }
}

impl RawStore for WindowsStore {
    fn get(&self, name: &str) -> Result<Option<Vec<u8>>, SecretError> {
        match self.entry(name)?.get_secret() {
            Ok(bytes) => Ok(Some(bytes)),
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(e) => Err(SecretError::Vault(e.to_string())),
        }
    }

    fn set(&self, name: &str, value: &[u8]) -> Result<(), SecretError> {
        self.entry(name)?
            .set_secret(value)
            .map_err(|e| SecretError::Vault(e.to_string()))
    }

    fn delete(&self, name: &str) -> Result<(), SecretError> {
        match self.entry(name)?.delete_credential() {
            Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
            Err(e) => Err(SecretError::Vault(e.to_string())),
        }
    }
}

/// In-memory store for tests.
#[derive(Default)]
struct MemoryStore {
    entries: Mutex<HashMap<String, Vec<u8>>>,
}

impl RawStore for MemoryStore {
    fn get(&self, name: &str) -> Result<Option<Vec<u8>>, SecretError> {
        Ok(lock(&self.entries).get(name).cloned())
    }

    fn set(&self, name: &str, value: &[u8]) -> Result<(), SecretError> {
        lock(&self.entries).insert(name.to_owned(), value.to_vec());
        Ok(())
    }

    fn delete(&self, name: &str) -> Result<(), SecretError> {
        lock(&self.entries).remove(name);
        Ok(())
    }
}

/// Password list split into chunks over a key/value store (the Credential Manager, or memory).
pub struct ChunkedVault {
    raw: Box<dyn RawStore>,
    /// Prefix of every entry name, so tests never touch the real list.
    prefix: String,
}

impl fmt::Debug for ChunkedVault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChunkedVault")
            .field("prefix", &self.prefix)
            .finish_non_exhaustive()
    }
}

impl ChunkedVault {
    /// The Windows Credential Manager, under the `RarNRush` service.
    pub fn windows() -> Result<Self, SecretError> {
        Self::windows_with_prefix("")
    }

    /// The Windows Credential Manager with entry names starting with `prefix`
    /// (used by tests to stay away from the real list).
    pub fn windows_with_prefix(prefix: &str) -> Result<Self, SecretError> {
        let store =
            windows_native_keyring_store::Store::new().map_err(|e| SecretError::Vault(e.to_string()))?;
        Ok(Self {
            raw: Box::new(WindowsStore { store }),
            prefix: prefix.to_owned(),
        })
    }

    /// A vault kept in memory (tests).
    pub fn in_memory() -> Self {
        Self {
            raw: Box::new(MemoryStore::default()),
            prefix: String::new(),
        }
    }

    fn name(&self, entry: &str) -> String {
        format!("{}{entry}", self.prefix)
    }

    fn chunk_name(&self, generation: u64, index: usize) -> String {
        self.name(&format!("{INDEX}-{generation}-{index}"))
    }

    /// `(generation, chunk count)` of the current list, if any.
    fn read_index(&self) -> Result<Option<(u64, usize)>, SecretError> {
        let Some(bytes) = self.raw.get(&self.name(INDEX))? else {
            return Ok(None);
        };
        let text = String::from_utf8(bytes).map_err(|e| SecretError::Format(e.to_string()))?;
        let mut parts = text.split_whitespace().map(str::parse::<u64>);
        match (parts.next(), parts.next(), parts.next()) {
            (Some(Ok(generation)), Some(Ok(count)), None) => {
                let count = usize::try_from(count)
                    .ok()
                    .filter(|c| *c <= MAX_CHUNKS)
                    .ok_or_else(|| SecretError::Format(format!("{count} chunks")))?;
                Ok(Some((generation, count)))
            }
            _ => Err(SecretError::Format(format!("index {text:?}"))),
        }
    }

    fn delete_generation(&self, generation: u64, count: usize) -> Result<(), SecretError> {
        (0..count).try_for_each(|i| self.raw.delete(&self.chunk_name(generation, i)))
    }
}

impl SecretVault for ChunkedVault {
    fn load(&self) -> Result<Vec<String>, SecretError> {
        let Some((generation, count)) = self.read_index()? else {
            return Ok(Vec::new());
        };
        let mut bytes = Vec::new();
        for i in 0..count {
            let chunk = self
                .raw
                .get(&self.chunk_name(generation, i))?
                .ok_or_else(|| SecretError::Format(format!("chunk {i} of {count} is missing")))?;
            bytes.extend(chunk);
        }
        let text = String::from_utf8(bytes).map_err(|e| SecretError::Format(e.to_string()))?;
        Ok(text
            .lines()
            .filter(|l| !l.is_empty())
            .map(str::to_owned)
            .collect())
    }

    fn save(&self, passwords: &[String]) -> Result<(), SecretError> {
        let previous = self.read_index().unwrap_or(None);
        let passwords: Vec<&str> = passwords
            .iter()
            .map(|p| p.trim())
            .filter(|p| !p.is_empty() && !p.contains(['\r', '\n']))
            .collect();
        if passwords.is_empty() {
            self.raw.delete(&self.name(INDEX))?;
        } else {
            let generation = previous.map_or(1, |(g, _)| g.wrapping_add(1));
            let data = passwords.join("\n").into_bytes();
            let chunks: Vec<&[u8]> = data.chunks(CHUNK).collect();
            if chunks.len() > MAX_CHUNKS {
                return Err(SecretError::Vault(format!(
                    "password list too long ({} bytes)",
                    data.len()
                )));
            }
            for (i, chunk) in chunks.iter().enumerate() {
                self.raw.set(&self.chunk_name(generation, i), chunk)?;
            }
            // Switching the index is the commit point.
            self.raw.set(
                &self.name(INDEX),
                format!("{generation} {}", chunks.len()).as_bytes(),
            )?;
        }
        if let Some((generation, count)) = previous {
            self.delete_generation(generation, count)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn empty_vault_gives_no_password() {
        assert_eq!(ChunkedVault::in_memory().load().unwrap(), Vec::<String>::new());
    }

    #[test]
    fn round_trip_keeps_order_and_unicode() {
        let vault = ChunkedVault::in_memory();
        let passwords = list(&["abc", "p é 2", "日本語", "with space"]);
        vault.save(&passwords).unwrap();
        assert_eq!(vault.load().unwrap(), passwords);
    }

    #[test]
    fn long_lists_are_split_and_old_chunks_removed() {
        let vault = ChunkedVault::in_memory();
        let long: Vec<String> = (0..2_000).map(|i| format!("password-number-{i:05}")).collect();
        vault.save(&long).unwrap();
        assert_eq!(vault.load().unwrap(), long);
        let (generation, count) = vault.read_index().unwrap().unwrap();
        assert!(count > 1, "the list must span several chunks");

        vault.save(&list(&["short"])).unwrap();
        assert_eq!(vault.load().unwrap(), list(&["short"]));
        for i in 0..count {
            assert_eq!(vault.raw.get(&vault.chunk_name(generation, i)).unwrap(), None);
        }
    }

    #[test]
    fn saving_nothing_clears_the_vault() {
        let vault = ChunkedVault::in_memory();
        vault.save(&list(&["a"])).unwrap();
        vault.save(&[]).unwrap();
        assert_eq!(vault.load().unwrap(), Vec::<String>::new());
        assert_eq!(vault.raw.get(INDEX).unwrap(), None);
    }

    #[test]
    fn interrupted_save_keeps_the_previous_list() {
        let vault = ChunkedVault::in_memory();
        vault.save(&list(&["old"])).unwrap();
        // Simulate a crash after the new chunks were written, before the index switch.
        vault.raw.set(&vault.chunk_name(2, 0), b"new").unwrap();
        assert_eq!(vault.load().unwrap(), list(&["old"]));
    }

    #[test]
    fn corrupted_index_is_reported() {
        let vault = ChunkedVault::in_memory();
        vault.raw.set(INDEX, b"not an index").unwrap();
        assert!(matches!(vault.load(), Err(SecretError::Format(_))));
        vault.raw.set(INDEX, b"1 999999").unwrap();
        assert!(matches!(vault.load(), Err(SecretError::Format(_))));
    }

    /// Writes to the real Credential Manager under a unique test prefix and
    /// removes everything afterwards.
    #[test]
    #[cfg(windows)]
    fn windows_credential_manager_round_trip() {
        let prefix = format!("test-{}-", std::process::id());
        let vault = ChunkedVault::windows_with_prefix(&prefix).unwrap();
        let passwords = list(&["secret-1", "sécret-2"]);
        vault.save(&passwords).unwrap();
        let loaded = vault.load();
        vault.save(&[]).unwrap();
        assert_eq!(loaded.unwrap(), passwords);
        assert_eq!(vault.load().unwrap(), Vec::<String>::new());
    }
}
