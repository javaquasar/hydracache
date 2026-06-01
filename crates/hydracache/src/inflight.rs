use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use bytes::Bytes;
use futures_util::future::Shared;
use hydracache_core::CacheError;
use tokio::sync::RwLock;

use crate::tag_index::LoadGenerationSnapshot;

pub(crate) type BoxedLoadFuture = Pin<Box<dyn Future<Output = LoadResult> + Send + 'static>>;
pub(crate) type SharedLoadFuture = Shared<BoxedLoadFuture>;
pub(crate) type LoadResult = std::result::Result<Bytes, Arc<CacheError>>;

#[derive(Debug, Default)]
pub(crate) struct InFlightMap {
    loads: RwLock<HashMap<String, InFlightEntry>>,
}

#[derive(Debug, Clone)]
struct InFlightEntry {
    load: SharedLoadFuture,
    generation: LoadGenerationSnapshot,
}

impl InFlightMap {
    pub(crate) async fn get_current(
        &self,
        key: &str,
        generation: &LoadGenerationSnapshot,
    ) -> Option<SharedLoadFuture> {
        self.loads
            .read()
            .await
            .get(key)
            .filter(|entry| &entry.generation == generation)
            .map(|entry| entry.load.clone())
    }

    pub(crate) async fn insert_or_get_current(
        &self,
        key: String,
        load: SharedLoadFuture,
        generation: LoadGenerationSnapshot,
    ) -> (SharedLoadFuture, bool) {
        let mut guard = self.loads.write().await;
        if let Some(existing) = guard.get(&key) {
            if existing.generation == generation {
                return (existing.load.clone(), false);
            }
        }

        guard.insert(
            key,
            InFlightEntry {
                load: load.clone(),
                generation,
            },
        );
        (load, true)
    }

    pub(crate) async fn remove_if_generation_matches(
        &self,
        key: &str,
        generation: &LoadGenerationSnapshot,
    ) {
        let mut guard = self.loads.write().await;
        if guard
            .get(key)
            .map(|entry| &entry.generation == generation)
            .unwrap_or(false)
        {
            guard.remove(key);
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use futures_util::FutureExt;

    use super::*;

    fn snapshot(global: u64) -> LoadGenerationSnapshot {
        LoadGenerationSnapshot {
            global,
            tags: Vec::new(),
        }
    }

    fn shared_bytes(value: &'static [u8]) -> SharedLoadFuture {
        async move { Ok(Bytes::from_static(value)) }
            .boxed()
            .shared()
    }

    #[tokio::test]
    async fn insert_or_get_current_returns_existing_load_for_same_generation() {
        let map = InFlightMap::default();
        let generation = snapshot(1);
        let first = shared_bytes(b"first");
        let second = shared_bytes(b"second");

        let (_, inserted) = map
            .insert_or_get_current("key".to_owned(), first.clone(), generation.clone())
            .await;
        let (existing, inserted_again) = map
            .insert_or_get_current("key".to_owned(), second, generation)
            .await;

        assert!(inserted);
        assert!(!inserted_again);
        assert_eq!(existing.await.unwrap(), Bytes::from_static(b"first"));
    }
}
