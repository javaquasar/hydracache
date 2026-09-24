use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use moka::notification::RemovalCause;
use tokio::sync::{mpsc, Mutex};

use crate::entry::CacheEntry;
use crate::memory_footprint::{EntryMemoryDelta, MemoryFootprintCounters, MemoryFootprintError};
use crate::tag_index::TagIndex;

const DEFAULT_CLEANUP_CAPACITY: usize = 4_096;

#[derive(Debug)]
struct CleanupTicket {
    key: Arc<String>,
    tags: Box<[String]>,
    version: u64,
}

#[derive(Debug)]
pub(crate) struct RemovalObserver {
    sender: mpsc::Sender<CleanupTicket>,
    receiver: Mutex<mpsc::Receiver<CleanupTicket>>,
    memory: Arc<MemoryFootprintCounters>,
    accepted: AtomicU64,
    acknowledged: AtomicU64,
    dirty: AtomicBool,
    inflight_versions: Box<[AtomicU64]>,
}

impl RemovalObserver {
    pub(crate) fn new(memory: Arc<MemoryFootprintCounters>) -> Self {
        Self::with_capacity(memory, DEFAULT_CLEANUP_CAPACITY)
    }

    fn with_capacity(memory: Arc<MemoryFootprintCounters>, capacity: usize) -> Self {
        assert!(capacity > 0, "removal cleanup queue must be non-empty");
        let (sender, receiver) = mpsc::channel(capacity);
        Self {
            sender,
            receiver: Mutex::new(receiver),
            memory,
            accepted: AtomicU64::new(0),
            acknowledged: AtomicU64::new(0),
            dirty: AtomicBool::new(false),
            inflight_versions: (0..capacity)
                .map(|_| AtomicU64::new(0))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        }
    }

    pub(crate) fn observe(&self, key: Arc<String>, entry: CacheEntry, _cause: RemovalCause) {
        let slot_index = (entry.version % self.inflight_versions.len() as u64) as usize;
        let slot = &self.inflight_versions[slot_index];
        let slot_claimed =
            match slot.compare_exchange(0, entry.version, Ordering::AcqRel, Ordering::Acquire) {
                Ok(_) => true,
                Err(version) if version == entry.version => return,
                Err(_) => {
                    self.dirty.store(true, Ordering::Release);
                    false
                }
            };

        let _mutation = self.memory.mutation();
        match EntryMemoryDelta::new(
            &key,
            entry.value.len(),
            &entry.tags,
            entry.expires_at.is_some(),
        ) {
            Ok(delta) => self.memory.remove(delta),
            Err(_) => {
                self.memory.mark_fault();
                self.dirty.store(true, Ordering::Release);
            }
        }

        if self
            .accepted
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                value.checked_add(1)
            })
            .is_err()
        {
            self.dirty.store(true, Ordering::Release);
            return;
        }

        if !slot_claimed {
            return;
        }

        let ticket = CleanupTicket {
            key,
            tags: entry.tags,
            version: entry.version,
        };
        if self.sender.try_send(ticket).is_err() {
            self.dirty.store(true, Ordering::Release);
        }
    }

    pub(crate) async fn drain(&self, tag_index: &TagIndex) {
        let mut receiver = self.receiver.lock().await;
        while let Ok(ticket) = receiver.try_recv() {
            tag_index
                .unregister_if_version(&ticket.key, &ticket.tags, ticket.version)
                .await;
            if self
                .acknowledged
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                    value.checked_add(1)
                })
                .is_err()
            {
                self.dirty.store(true, Ordering::Release);
            }
            let slot_index = (ticket.version % self.inflight_versions.len() as u64) as usize;
            let slot = &self.inflight_versions[slot_index];
            if slot
                .compare_exchange(ticket.version, 0, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
            {
                self.dirty.store(true, Ordering::Release);
            }
        }
    }

    pub(crate) fn ensure_clean(&self) -> Result<(), MemoryFootprintError> {
        if self.dirty.load(Ordering::Acquire) {
            return Err(MemoryFootprintError::RemovalObserverDirty);
        }
        let accepted = self.accepted.load(Ordering::Acquire);
        let acknowledged = self.acknowledged.load(Ordering::Acquire);
        if accepted == acknowledged {
            Ok(())
        } else {
            Err(MemoryFootprintError::RemovalCleanupPending {
                accepted,
                acknowledged,
            })
        }
    }

    pub(crate) fn is_clean(&self) -> bool {
        self.ensure_clean().is_ok()
    }

    pub(crate) async fn reset_after_reconcile(&self) {
        let mut receiver = self.receiver.lock().await;
        while receiver.try_recv().is_ok() {}
        for slot in &self.inflight_versions {
            slot.store(0, Ordering::Release);
        }
        let accepted = self.accepted.load(Ordering::Acquire);
        self.acknowledged.store(accepted, Ordering::Release);
        self.dirty.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use bytes::Bytes;

    use super::*;
    use crate::memory_footprint::MemoryInstrumentationMode;

    fn entry(version: u64, value_len: usize, tags: &[&str]) -> CacheEntry {
        CacheEntry::new(
            Bytes::from(vec![7_u8; value_len]),
            tags.iter().map(|tag| (*tag).to_owned()).collect(),
            Some(Instant::now()),
            version,
        )
    }

    #[tokio::test]
    async fn delayed_old_cleanup_preserves_new_membership() {
        let memory = Arc::new(MemoryFootprintCounters::new(
            MemoryInstrumentationMode::Production,
        ));
        let observer = RemovalObserver::with_capacity(memory.clone(), 2);
        let index = TagIndex::default();
        let old = entry(41, 8, &["blue"]);
        memory.insert(EntryMemoryDelta::new("key", 8, &old.tags, true).unwrap());
        index.register("key", &old.tags, old.version).await;

        observer.observe(Arc::new("key".to_owned()), old, RemovalCause::Replaced);
        let new = entry(42, 16, &["blue"]);
        memory.insert(EntryMemoryDelta::new("key", 16, &new.tags, true).unwrap());
        index.register("key", &new.tags, new.version).await;
        observer.drain(&index).await;

        assert!(index.contains_version("blue", "key", 42).await);
        assert!(observer.ensure_clean().is_ok());
    }

    #[tokio::test]
    async fn duplicate_delivery_is_idempotent_and_saturation_fails_closed() {
        let memory = Arc::new(MemoryFootprintCounters::new(
            MemoryInstrumentationMode::Production,
        ));
        let observer = RemovalObserver::with_capacity(memory.clone(), 1);
        let index = TagIndex::default();
        let first = entry(1, 8, &["tag"]);
        let second = entry(2, 8, &["tag"]);
        for (key, item) in [("first", &first), ("second", &second)] {
            memory.insert(EntryMemoryDelta::new(key, 8, &item.tags, true).unwrap());
            index.register(key, &item.tags, item.version).await;
        }

        observer.observe(
            Arc::new("first".to_owned()),
            first.clone(),
            RemovalCause::Explicit,
        );
        observer.observe(Arc::new("first".to_owned()), first, RemovalCause::Explicit);
        observer.observe(Arc::new("second".to_owned()), second, RemovalCause::Size);

        assert_eq!(
            observer.ensure_clean(),
            Err(MemoryFootprintError::RemovalObserverDirty)
        );
        observer.drain(&index).await;
        observer.reset_after_reconcile().await;
        assert!(observer.ensure_clean().is_ok());
    }

    #[tokio::test]
    async fn pending_cleanup_fails_closed_until_acknowledged() {
        let memory = Arc::new(MemoryFootprintCounters::new(
            MemoryInstrumentationMode::Production,
        ));
        let observer = RemovalObserver::with_capacity(memory.clone(), 1);
        let index = TagIndex::default();
        let removed = entry(7, 8, &["tag"]);
        memory.insert(EntryMemoryDelta::new("key", 8, &removed.tags, true).unwrap());
        index.register("key", &removed.tags, removed.version).await;

        observer.observe(Arc::new("key".to_owned()), removed, RemovalCause::Explicit);
        assert_eq!(
            observer.ensure_clean(),
            Err(MemoryFootprintError::RemovalCleanupPending {
                accepted: 1,
                acknowledged: 0,
            })
        );

        observer.drain(&index).await;
        assert!(observer.ensure_clean().is_ok());
    }
}
