use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Weak};
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_util::future::Shared;
use hydracache_core::CacheError;
use tokio::sync::RwLock;

use crate::tag_index::LoadGenerationSnapshot;

pub(crate) type BoxedLoadFuture = Pin<Box<dyn Future<Output = LoadResult> + Send + 'static>>;
pub(crate) type SharedLoadFuture = Shared<BoxedLoadFuture>;
pub(crate) type LoadResult = std::result::Result<Bytes, Arc<CacheError>>;

pub(crate) struct SharedLoadHandle {
    load: SharedLoadFuture,
    map: Option<Weak<InFlightMap>>,
    key: String,
    generation: LoadGenerationSnapshot,
    waiters: Option<Arc<AtomicUsize>>,
}

impl SharedLoadHandle {
    pub(crate) fn detached(load: SharedLoadFuture) -> Self {
        Self {
            load,
            map: None,
            key: String::new(),
            generation: LoadGenerationSnapshot {
                global: 0,
                key: String::new(),
                key_generation: 0,
                tags: Vec::new(),
            },
            waiters: None,
        }
    }
}

impl Future for SharedLoadHandle {
    type Output = LoadResult;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.load).poll(context)
    }
}

impl Drop for SharedLoadHandle {
    fn drop(&mut self) {
        let (Some(map), Some(waiters)) = (self.map.as_ref(), self.waiters.as_ref()) else {
            return;
        };
        if waiters.fetch_sub(1, Ordering::AcqRel) != 1 {
            return;
        }
        let Some(map) = map.upgrade() else {
            return;
        };
        let key = self.key.clone();
        let generation = self.generation.clone();
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            runtime.spawn(async move {
                map.remove_if_idle(&key, &generation).await;
            });
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct InFlightMap {
    loads: RwLock<HashMap<String, InFlightEntry>>,
}

#[derive(Debug, Clone)]
struct InFlightEntry {
    load: SharedLoadFuture,
    generation: LoadGenerationSnapshot,
    waiters: Arc<AtomicUsize>,
}

impl InFlightMap {
    pub(crate) async fn get_current(
        self: &Arc<Self>,
        key: &str,
        generation: &LoadGenerationSnapshot,
    ) -> Option<SharedLoadHandle> {
        let guard = self.loads.read().await;
        let entry = guard
            .get(key)
            .filter(|entry| &entry.generation == generation)?;
        entry.waiters.fetch_add(1, Ordering::AcqRel);
        Some(SharedLoadHandle {
            load: entry.load.clone(),
            map: Some(Arc::downgrade(self)),
            key: key.to_owned(),
            generation: generation.clone(),
            waiters: Some(entry.waiters.clone()),
        })
    }

    pub(crate) async fn insert_or_get_current(
        self: &Arc<Self>,
        key: String,
        load: SharedLoadFuture,
        generation: LoadGenerationSnapshot,
    ) -> (SharedLoadHandle, bool) {
        let mut guard = self.loads.write().await;
        if let Some(existing) = guard.get(&key) {
            if existing.generation == generation {
                existing.waiters.fetch_add(1, Ordering::AcqRel);
                return (
                    SharedLoadHandle {
                        load: existing.load.clone(),
                        map: Some(Arc::downgrade(self)),
                        key,
                        generation,
                        waiters: Some(existing.waiters.clone()),
                    },
                    false,
                );
            }
        }

        let waiters = Arc::new(AtomicUsize::new(1));
        guard.insert(
            key.clone(),
            InFlightEntry {
                load: load.clone(),
                generation: generation.clone(),
                waiters: waiters.clone(),
            },
        );
        (
            SharedLoadHandle {
                load,
                map: Some(Arc::downgrade(self)),
                key,
                generation,
                waiters: Some(waiters),
            },
            true,
        )
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

    async fn remove_if_idle(&self, key: &str, generation: &LoadGenerationSnapshot) {
        let mut guard = self.loads.write().await;
        if guard.get(key).is_some_and(|entry| {
            &entry.generation == generation && entry.waiters.load(Ordering::Acquire) == 0
        }) {
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
            key: "key".to_owned(),
            key_generation: 0,
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
        let map = Arc::new(InFlightMap::default());
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
