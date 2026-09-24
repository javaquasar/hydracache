//! Lab-only reference model for a bounded post-removal observer.
//!
//! This module deliberately does not modify HydraCache's production cache path. It makes the
//! ordering, saturation, reconciliation, and exact-snapshot rules executable before a Moka API
//! patch or product proposal can be authorized.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde::Serialize;

use crate::allocation::measure_allocations;

const MEASUREMENT_OPERATIONS: u64 = 1_024;
const MEASUREMENT_REPETITIONS: u64 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemovalKind {
    Explicit,
    Replaced,
    Expired,
    Capacity,
}

#[derive(Debug, Clone)]
pub struct VersionedEntry {
    pub key: String,
    pub version: u64,
    pub tags: Vec<String>,
    pub retained_bytes: u64,
    removal_accounted: Arc<AtomicBool>,
}

impl VersionedEntry {
    pub fn new(
        key: impl Into<String>,
        version: u64,
        tags: Vec<String>,
        retained_bytes: u64,
    ) -> Self {
        Self {
            key: key.into(),
            version,
            tags,
            retained_bytes,
            removal_accounted: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[derive(Debug, Clone)]
struct CleanupTicket {
    entry: VersionedEntry,
    kind: RemovalKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishOutcome {
    Accepted,
    Duplicate,
    Saturated,
    Closed,
    AccountingFault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExactSnapshotError {
    DirtyEpoch,
    PendingCleanup { accepted: u64, acknowledged: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExactSnapshot {
    pub retained_bytes: u64,
    pub memberships: u64,
    pub accepted: u64,
    pub acknowledged: u64,
}

#[derive(Debug, Default)]
struct VersionedMemberships {
    by_tag: HashMap<String, HashMap<String, u64>>,
}

impl VersionedMemberships {
    fn register(&mut self, entry: &VersionedEntry) {
        for tag in &entry.tags {
            self.by_tag
                .entry(tag.clone())
                .or_default()
                .insert(entry.key.clone(), entry.version);
        }
    }

    fn unregister_if_version(&mut self, entry: &VersionedEntry) {
        for tag in &entry.tags {
            let remove_tag = self.by_tag.get_mut(tag).is_some_and(|keys| {
                if keys.get(&entry.key) == Some(&entry.version) {
                    keys.remove(&entry.key);
                }
                keys.is_empty()
            });
            if remove_tag {
                self.by_tag.remove(tag);
            }
        }
    }

    fn membership_count(&self) -> u64 {
        self.by_tag.values().map(|keys| keys.len() as u64).sum()
    }

    fn contains(&self, tag: &str, key: &str, version: u64) -> bool {
        self.by_tag
            .get(tag)
            .and_then(|keys| keys.get(key))
            .is_some_and(|stored| *stored == version)
    }

    fn rebuild(&mut self, entries: &[VersionedEntry]) {
        self.by_tag.clear();
        for entry in entries {
            self.register(entry);
        }
    }
}

/// Bounded observer reference model. Publication is synchronous and contains no await point.
#[derive(Debug)]
pub struct RemovalObserver {
    queue: Mutex<VecDeque<CleanupTicket>>,
    queue_capacity: usize,
    closed: AtomicBool,
    memberships: Mutex<VersionedMemberships>,
    retained_bytes: AtomicU64,
    accepted: AtomicU64,
    acknowledged: AtomicU64,
    dirty: AtomicBool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ObserverPrototypeReceipt {
    pub schema_version: &'static str,
    pub release: &'static str,
    pub experiment: &'static str,
    pub operations_per_case: u64,
    pub repetitions: u64,
    pub cases: Vec<ObserverPrototypeCase>,
    pub diagnostic_only: bool,
    pub product_semantics_eligible: bool,
    pub promotable: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ObserverPrototypeCase {
    pub path: &'static str,
    pub repetition: u64,
    pub position: u64,
    pub gross_allocated_bytes: u64,
    pub gross_allocated_bytes_per_operation: f64,
    pub elapsed_ns: u64,
}

impl RemovalObserver {
    pub fn new(queue_capacity: usize) -> Self {
        assert!(
            queue_capacity > 0,
            "observer queue must be bounded and non-empty"
        );
        Self {
            queue: Mutex::new(VecDeque::with_capacity(queue_capacity)),
            queue_capacity,
            closed: AtomicBool::new(false),
            memberships: Mutex::new(VersionedMemberships::default()),
            retained_bytes: AtomicU64::new(0),
            accepted: AtomicU64::new(0),
            acknowledged: AtomicU64::new(0),
            dirty: AtomicBool::new(false),
        }
    }

    pub fn register(&self, entry: &VersionedEntry) {
        self.memberships.lock().unwrap().register(entry);
        if self
            .retained_bytes
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                current.checked_add(entry.retained_bytes)
            })
            .is_err()
        {
            self.dirty.store(true, Ordering::Release);
        }
    }

    /// Publish a removal without awaiting, spawning, or allocating a boxed future.
    pub fn publish(&self, entry: VersionedEntry, kind: RemovalKind) -> PublishOutcome {
        if entry
            .removal_accounted
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return PublishOutcome::Duplicate;
        }
        if self
            .retained_bytes
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                current.checked_sub(entry.retained_bytes)
            })
            .is_err()
        {
            self.dirty.store(true, Ordering::Release);
            return PublishOutcome::AccountingFault;
        }

        self.accepted.fetch_add(1, Ordering::AcqRel);
        if self.closed.load(Ordering::Acquire) {
            self.dirty.store(true, Ordering::Release);
            return PublishOutcome::Closed;
        }
        let mut queue = self.queue.lock().unwrap();
        if queue.len() == self.queue_capacity {
            self.dirty.store(true, Ordering::Release);
            PublishOutcome::Saturated
        } else {
            queue.push_back(CleanupTicket { entry, kind });
            PublishOutcome::Accepted
        }
    }

    pub fn drain_available(&self) -> usize {
        let mut drained = 0;
        loop {
            let ticket = self.queue.lock().unwrap().pop_front();
            let Some(ticket) = ticket else {
                break;
            };
            let _kind = ticket.kind;
            self.memberships
                .lock()
                .unwrap()
                .unregister_if_version(&ticket.entry);
            self.acknowledged.fetch_add(1, Ordering::AcqRel);
            drained += 1;
        }
        drained
    }

    pub fn exact_snapshot(&self) -> Result<ExactSnapshot, ExactSnapshotError> {
        if self.dirty.load(Ordering::Acquire) {
            return Err(ExactSnapshotError::DirtyEpoch);
        }
        let accepted = self.accepted.load(Ordering::Acquire);
        let acknowledged = self.acknowledged.load(Ordering::Acquire);
        if accepted != acknowledged {
            return Err(ExactSnapshotError::PendingCleanup {
                accepted,
                acknowledged,
            });
        }
        Ok(ExactSnapshot {
            retained_bytes: self.retained_bytes.load(Ordering::Acquire),
            memberships: self.memberships.lock().unwrap().membership_count(),
            accepted,
            acknowledged,
        })
    }

    /// Rebuild from an authoritative quiescent entry list after saturation or accounting failure.
    pub fn reconcile(&self, entries: &[VersionedEntry]) {
        self.queue.lock().unwrap().clear();

        self.memberships.lock().unwrap().rebuild(entries);
        let retained_bytes = entries.iter().fold(0_u64, |total, entry| {
            total.saturating_add(entry.retained_bytes)
        });
        self.retained_bytes.store(retained_bytes, Ordering::Release);
        let accepted = self.accepted.load(Ordering::Acquire);
        self.acknowledged.store(accepted, Ordering::Release);
        self.dirty.store(false, Ordering::Release);
    }

    pub fn close_and_drain(&self) -> usize {
        self.closed.store(true, Ordering::Release);
        self.drain_available()
    }

    pub fn contains_membership(&self, tag: &str, key: &str, version: u64) -> bool {
        self.memberships.lock().unwrap().contains(tag, key, version)
    }
}

pub async fn run_and_write_prototype(output: &Path) -> Result<ObserverPrototypeReceipt, String> {
    if output.exists() {
        return Err(format!(
            "append-only observer prototype output already exists: {}",
            output.display()
        ));
    }
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let mut cases = Vec::with_capacity((MEASUREMENT_REPETITIONS * 2) as usize);
    for repetition in 1..=MEASUREMENT_REPETITIONS {
        for (position, observer_enabled) in measurement_order(repetition).into_iter().enumerate() {
            cases.push(
                measure_prototype_case(observer_enabled, repetition, position as u64 + 1).await?,
            );
        }
    }
    let receipt = ObserverPrototypeReceipt {
        schema_version: "hydracache-notification-observer-prototype-073-v1",
        release: "0.73",
        experiment: "versioned-bounded-post-removal-observer",
        operations_per_case: MEASUREMENT_OPERATIONS,
        repetitions: MEASUREMENT_REPETITIONS,
        cases,
        diagnostic_only: true,
        product_semantics_eligible: false,
        promotable: false,
    };
    std::fs::write(
        output,
        serde_json::to_vec_pretty(&receipt).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    Ok(receipt)
}

async fn measure_prototype_case(
    observer_enabled: bool,
    repetition: u64,
    position: u64,
) -> Result<ObserverPrototypeCase, String> {
    let mut entries: Vec<_> = (0..MEASUREMENT_OPERATIONS)
        .map(|index| {
            Some(VersionedEntry::new(
                format!("key-{index}"),
                index,
                vec!["tag".to_owned()],
                64,
            ))
        })
        .collect();
    let observer = RemovalObserver::new(MEASUREMENT_OPERATIONS as usize);
    if observer_enabled {
        for entry in entries.iter().flatten() {
            observer.register(entry);
        }
    }
    let counters_only = AtomicU64::new(MEASUREMENT_OPERATIONS * 64);
    let started = Instant::now();
    let (_, allocation) = measure_allocations(MEASUREMENT_OPERATIONS, async {
        if observer_enabled {
            for entry in &mut entries {
                let outcome = observer.publish(
                    entry.take().expect("prototype entry"),
                    RemovalKind::Explicit,
                );
                assert_eq!(outcome, PublishOutcome::Accepted);
            }
            assert_eq!(observer.drain_available(), MEASUREMENT_OPERATIONS as usize);
        } else {
            for _ in 0..MEASUREMENT_OPERATIONS {
                counters_only.fetch_sub(64, Ordering::Relaxed);
            }
        }
    })
    .await;
    if observer_enabled {
        observer
            .exact_snapshot()
            .map_err(|error| format!("observer prototype did not drain exactly: {error:?}"))?;
    }
    Ok(ObserverPrototypeCase {
        path: if observer_enabled {
            "versioned-observer"
        } else {
            "atomic-counter-only"
        },
        repetition,
        position,
        gross_allocated_bytes: allocation.gross_allocated_bytes,
        gross_allocated_bytes_per_operation: allocation.gross_allocated_bytes_per_operation,
        elapsed_ns: u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX),
    })
}

fn measurement_order(repetition: u64) -> [bool; 2] {
    if repetition & 1 == 0 {
        [true, false]
    } else {
        [false, true]
    }
}

pub fn default_prototype_output() -> PathBuf {
    PathBuf::from("target/performance-evidence/0.73/local/notification-observer-prototype.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(key: &str, version: u64, tags: &[&str], retained_bytes: u64) -> VersionedEntry {
        VersionedEntry::new(
            key,
            version,
            tags.iter().map(|tag| (*tag).to_owned()).collect(),
            retained_bytes,
        )
    }

    #[test]
    fn delayed_cleanup_cannot_remove_newer_membership() {
        let observer = RemovalObserver::new(4);
        let old = entry("key", 41, &["blue"], 100);
        observer.register(&old);
        assert_eq!(
            observer.publish(old, RemovalKind::Replaced),
            PublishOutcome::Accepted
        );

        let new = entry("key", 42, &["blue"], 120);
        observer.register(&new);
        observer.drain_available();

        assert!(observer.contains_membership("blue", "key", 42));
        assert_eq!(observer.exact_snapshot().unwrap().retained_bytes, 120);
    }

    #[test]
    fn duplicate_delivery_cannot_decrement_twice() {
        let observer = RemovalObserver::new(2);
        let removed = entry("key", 1, &["tag"], 64);
        observer.register(&removed);
        assert_eq!(
            observer.publish(removed.clone(), RemovalKind::Explicit),
            PublishOutcome::Accepted
        );
        assert_eq!(
            observer.publish(removed, RemovalKind::Explicit),
            PublishOutcome::Duplicate
        );
        observer.drain_available();
        assert_eq!(observer.exact_snapshot().unwrap().retained_bytes, 0);
    }

    #[test]
    fn saturation_fails_exact_snapshot_closed_until_reconciliation() {
        let observer = RemovalObserver::new(1);
        let first = entry("first", 1, &["tag"], 10);
        let second = entry("second", 2, &["tag"], 20);
        observer.register(&first);
        observer.register(&second);
        assert_eq!(
            observer.publish(first, RemovalKind::Capacity),
            PublishOutcome::Accepted
        );
        assert_eq!(
            observer.publish(second.clone(), RemovalKind::Expired),
            PublishOutcome::Saturated
        );
        assert_eq!(
            observer.exact_snapshot(),
            Err(ExactSnapshotError::DirtyEpoch)
        );

        observer.reconcile(&[]);
        assert_eq!(observer.exact_snapshot().unwrap().retained_bytes, 0);
        assert!(!observer.contains_membership("tag", "second", 2));
    }

    #[test]
    fn pending_cleanup_is_not_reported_as_exact() {
        let observer = RemovalObserver::new(2);
        let removed = entry("key", 1, &["tag"], 32);
        observer.register(&removed);
        observer.publish(removed, RemovalKind::Explicit);
        assert_eq!(
            observer.exact_snapshot(),
            Err(ExactSnapshotError::PendingCleanup {
                accepted: 1,
                acknowledged: 0,
            })
        );
    }

    #[test]
    fn every_removal_kind_uses_the_same_exact_pipeline() {
        for (index, kind) in [
            RemovalKind::Explicit,
            RemovalKind::Replaced,
            RemovalKind::Expired,
            RemovalKind::Capacity,
        ]
        .into_iter()
        .enumerate()
        {
            let observer = RemovalObserver::new(1);
            let removed = entry("key", index as u64, &["tag"], 16);
            observer.register(&removed);
            assert_eq!(observer.publish(removed, kind), PublishOutcome::Accepted);
            observer.drain_available();
            assert_eq!(observer.exact_snapshot().unwrap().memberships, 0);
        }
    }

    #[test]
    fn publication_has_no_cancellation_point_and_shutdown_drains_acknowledged_work() {
        let observer = RemovalObserver::new(1);
        let removed = entry("key", 1, &["tag"], 16);
        observer.register(&removed);
        assert_eq!(
            observer.publish(removed, RemovalKind::Explicit),
            PublishOutcome::Accepted
        );
        assert_eq!(observer.close_and_drain(), 1);
        assert!(observer.exact_snapshot().is_ok());

        let late = entry("late", 2, &["tag"], 16);
        observer.register(&late);
        assert_eq!(
            observer.publish(late, RemovalKind::Explicit),
            PublishOutcome::Closed
        );
        assert_eq!(
            observer.exact_snapshot(),
            Err(ExactSnapshotError::DirtyEpoch)
        );
    }

    #[tokio::test]
    async fn prototype_measurement_is_counterbalanced_and_non_promotable() {
        let mut cases = Vec::new();
        for repetition in 1..=MEASUREMENT_REPETITIONS {
            for (position, observer_enabled) in
                measurement_order(repetition).into_iter().enumerate()
            {
                cases.push(
                    measure_prototype_case(observer_enabled, repetition, position as u64 + 1)
                        .await
                        .unwrap(),
                );
            }
        }
        assert_eq!(cases.len(), 6);
        assert_eq!(measurement_order(1), [false, true]);
        assert_eq!(measurement_order(2), [true, false]);
        assert_eq!(
            cases
                .iter()
                .filter(|case| case.path == "versioned-observer")
                .count(),
            3
        );
    }
}
