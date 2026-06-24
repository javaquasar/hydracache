use std::collections::BTreeMap;

use super::BackupError;

/// Minimal object-store contract for backup backends.
pub trait ObjectStore {
    /// Store bytes under a key.
    fn put(&mut self, key: &str, bytes: Vec<u8>) -> Result<(), BackupError>;

    /// Load bytes by key.
    fn get(&self, key: &str) -> Result<Vec<u8>, BackupError>;

    /// List keys with a prefix.
    fn list(&self, prefix: &str) -> Result<Vec<String>, BackupError>;
}

/// In-memory object store for tests and examples.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InMemoryObjectStore {
    objects: BTreeMap<String, Vec<u8>>,
}

impl InMemoryObjectStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mutate an object in place for corruption/fault tests.
    pub fn mutate(
        &mut self,
        key: &str,
        mutate: impl FnOnce(&mut Vec<u8>),
    ) -> Result<(), BackupError> {
        let bytes = self
            .objects
            .get_mut(key)
            .ok_or_else(|| BackupError::MissingObject(key.to_owned()))?;
        mutate(bytes);
        Ok(())
    }
}

impl ObjectStore for InMemoryObjectStore {
    fn put(&mut self, key: &str, bytes: Vec<u8>) -> Result<(), BackupError> {
        if key.trim().is_empty() {
            return Err(BackupError::InvalidName("object key"));
        }
        self.objects.insert(key.to_owned(), bytes);
        Ok(())
    }

    fn get(&self, key: &str) -> Result<Vec<u8>, BackupError> {
        self.objects
            .get(key)
            .cloned()
            .ok_or_else(|| BackupError::MissingObject(key.to_owned()))
    }

    fn list(&self, prefix: &str) -> Result<Vec<String>, BackupError> {
        Ok(self
            .objects
            .keys()
            .filter(|key| key.starts_with(prefix))
            .cloned()
            .collect())
    }
}
