use std::alloc::{GlobalAlloc, Layout, System};
use std::error::Error;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

use bytes::Bytes;
use hydracache::{
    CacheEventKind, CacheOptions, HydraCache, MemoryFootprintSnapshot, MemoryInstrumentationMode,
    MemorySnapshotRequest,
};
use serde::Serialize;

const PROFILE_ID: &str = "w3-tag-index-profile-073-v1";
const TAG_OWNER_ID: &str = "owner-f184fb44837b0512";
const ENTRY_COUNT: usize = 256;
const KEY_BYTES: usize = 32;
const TAG_BYTES: usize = 32;
const VALUE_BYTES: usize = 64;

struct ProfilingAllocator;

static COUNTING: AtomicBool = AtomicBool::new(false);
static GROSS_ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);
static LIVE_ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);

#[global_allocator]
static GLOBAL_ALLOCATOR: ProfilingAllocator = ProfilingAllocator;

unsafe impl GlobalAlloc for ProfilingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the request is delegated unchanged to the system allocator.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            record_alloc(layout.size());
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the request is delegated unchanged to the system allocator.
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            record_alloc(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        record_dealloc(layout.size());
        // SAFETY: pointer and layout came from this allocator and are unchanged.
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: the request is delegated unchanged to the system allocator.
        let new_pointer = unsafe { System.realloc(pointer, layout, new_size) };
        if !new_pointer.is_null() {
            if new_size >= layout.size() {
                LIVE_ALLOCATED_BYTES
                    .fetch_add((new_size - layout.size()) as u64, Ordering::Relaxed);
            } else {
                subtract_live((layout.size() - new_size) as u64);
            }
            if COUNTING.load(Ordering::Acquire) {
                GROSS_ALLOCATED_BYTES.fetch_add(new_size as u64, Ordering::Relaxed);
            }
        }
        new_pointer
    }
}

fn record_alloc(bytes: usize) {
    LIVE_ALLOCATED_BYTES.fetch_add(bytes as u64, Ordering::Relaxed);
    if COUNTING.load(Ordering::Acquire) {
        GROSS_ALLOCATED_BYTES.fetch_add(bytes as u64, Ordering::Relaxed);
    }
}

fn record_dealloc(bytes: usize) {
    subtract_live(bytes as u64);
}

fn subtract_live(bytes: u64) {
    let _ = LIVE_ALLOCATED_BYTES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
        Some(current.saturating_sub(bytes))
    });
}

struct AllocationScope {
    before_live: u64,
}

impl AllocationScope {
    fn start() -> Self {
        GROSS_ALLOCATED_BYTES.store(0, Ordering::Relaxed);
        let before_live = LIVE_ALLOCATED_BYTES.load(Ordering::Relaxed);
        assert!(!COUNTING.swap(true, Ordering::AcqRel));
        Self { before_live }
    }

    fn finish(self) -> AllocationSnapshot {
        COUNTING.store(false, Ordering::Release);
        let live_allocated_bytes = LIVE_ALLOCATED_BYTES.load(Ordering::Relaxed);
        AllocationSnapshot {
            gross_allocated_bytes: GROSS_ALLOCATED_BYTES.load(Ordering::Relaxed),
            live_allocated_bytes,
            live_allocated_delta_bytes: live_allocated_bytes as i128 - self.before_live as i128,
        }
    }
}

impl Drop for AllocationScope {
    fn drop(&mut self) {
        COUNTING.store(false, Ordering::Release);
    }
}

#[derive(Clone, Copy)]
struct AllocationSnapshot {
    gross_allocated_bytes: u64,
    live_allocated_bytes: u64,
    live_allocated_delta_bytes: i128,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Index,
    Event,
    Invalidate,
}

impl Mode {
    fn parse(value: &str) -> Result<Self, Box<dyn Error>> {
        match value {
            "index" => Ok(Self::Index),
            "event" => Ok(Self::Event),
            "invalidate" => Ok(Self::Invalidate),
            _ => Err(format!("unknown mode {value}").into()),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Index => "index",
            Self::Event => "event",
            Self::Invalidate => "invalidate",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Topology {
    Shared,
    Unique,
}

impl Topology {
    fn parse(value: &str) -> Result<Self, Box<dyn Error>> {
        match value {
            "shared-tag-set" => Ok(Self::Shared),
            "unique-tag-set" => Ok(Self::Unique),
            _ => Err(format!("unknown topology {value}").into()),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Shared => "shared-tag-set",
            Self::Unique => "unique-tag-set",
        }
    }
}

struct Options {
    mode: Mode,
    tags: usize,
    topology: Topology,
    subscribers: usize,
    fanout: usize,
}

impl Options {
    fn parse() -> Result<Self, Box<dyn Error>> {
        let mut args = std::env::args().skip(1);
        let mut mode = None;
        let mut tags = None;
        let mut topology = None;
        let mut subscribers = None;
        let mut fanout = None;
        while let Some(argument) = args.next() {
            let value = args
                .next()
                .ok_or_else(|| format!("missing value after {argument}"))?;
            match argument.as_str() {
                "--mode" => mode = Some(Mode::parse(&value)?),
                "--tags" => tags = Some(value.parse()?),
                "--topology" => topology = Some(Topology::parse(&value)?),
                "--subscribers" => subscribers = Some(value.parse()?),
                "--fanout" => fanout = Some(value.parse()?),
                _ => return Err(format!("unknown argument {argument}").into()),
            }
        }

        let mode = mode.ok_or("--mode is required")?;
        let tags = tags.unwrap_or(0);
        let topology = topology.unwrap_or(Topology::Shared);
        let subscribers = subscribers.unwrap_or(0);
        let fanout = fanout.unwrap_or(0);
        match mode {
            Mode::Index => {
                require_member(tags, &[0, 1, 4, 16, 64], "--tags")?;
                require_zero(subscribers, "--subscribers")?;
                require_zero(fanout, "--fanout")?;
            }
            Mode::Event => {
                require_member(tags, &[0, 1, 4, 16, 64], "--tags")?;
                require_member(subscribers, &[0, 1, 8], "--subscribers")?;
                require_zero(fanout, "--fanout")?;
            }
            Mode::Invalidate => {
                require_member(fanout, &[1, 64, 1024], "--fanout")?;
                require_zero(tags, "--tags")?;
                require_zero(subscribers, "--subscribers")?;
            }
        }
        Ok(Self {
            mode,
            tags,
            topology,
            subscribers,
            fanout,
        })
    }
}

fn require_member(value: usize, allowed: &[usize], name: &str) -> Result<(), Box<dyn Error>> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(format!("{name} must be one of {allowed:?}").into())
    }
}

fn require_zero(value: usize, name: &str) -> Result<(), Box<dyn Error>> {
    if value == 0 {
        Ok(())
    } else {
        Err(format!("{name} is not valid for this mode").into())
    }
}

#[derive(Serialize)]
struct ProfileResult {
    schema_version: u32,
    profile_id: &'static str,
    mode: &'static str,
    topology: &'static str,
    entry_count: usize,
    tags_per_entry: usize,
    subscriber_count: usize,
    invalidation_fanout: usize,
    key_bytes: usize,
    tag_bytes: usize,
    gross_allocated_bytes: u64,
    live_allocated_bytes: u64,
    live_allocated_delta_bytes: i128,
    elapsed_nanoseconds: u128,
    estimated_tag_retained_bytes: u64,
    tag_owner_records: u64,
    tag_owner_logical_bytes: u64,
    live_entries: u64,
    tag_memberships: u64,
    tag_generation_records: u64,
    key_generation_records: u64,
    event_tag_payload_bytes: u64,
    event_delivery_count: u64,
    invalidation_removed_keys: u64,
    expected_memberships: u64,
    expected_event_deliveries: u64,
    expected_event_tag_payload_bytes: u64,
    exact_snapshot_passed: bool,
    exact_memory_reconciliation_passed: bool,
    membership_cardinality_passed: bool,
    event_content_equality_passed: bool,
    event_delivery_count_passed: bool,
    invalidation_fanout_passed: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let options = Options::parse()?;
    let result = match options.mode {
        Mode::Index | Mode::Event => profile_insertions(&options).await?,
        Mode::Invalidate => profile_invalidation(&options).await?,
    };
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}

async fn profile_insertions(options: &Options) -> Result<ProfileResult, Box<dyn Error>> {
    let cache = build_cache();
    let mut subscribers = (0..options.subscribers)
        .map(|_| cache.subscribe_mutations())
        .collect::<Vec<_>>();
    let value = Bytes::from_static(&[0x5a; VALUE_BYTES]);
    let expected_memberships = (ENTRY_COUNT * options.tags) as u64;
    let expected_event_deliveries = if options.mode == Mode::Event {
        (ENTRY_COUNT * options.subscribers) as u64
    } else {
        0
    };
    let expected_event_tag_payload_bytes =
        expected_event_deliveries * options.tags as u64 * TAG_BYTES as u64;

    let scope = AllocationScope::start();
    let started = Instant::now();
    let mut event_delivery_count = 0_u64;
    let mut event_tag_payload_bytes = 0_u64;
    let mut event_content_equality_passed = true;
    for entry_index in 0..ENTRY_COUNT {
        let key = fixed_id("key-", entry_index);
        let tags = tags_for(entry_index, options.tags, options.topology);
        cache
            .put_encoded(
                &key,
                value.clone(),
                CacheOptions::new().tags(tags.iter().cloned()),
            )
            .await?;

        if options.mode == Mode::Event {
            for subscriber in &mut subscribers {
                let event = subscriber.recv().await?;
                event_delivery_count += 1;
                event_tag_payload_bytes +=
                    event.tags().iter().map(|tag| tag.len() as u64).sum::<u64>();
                event_content_equality_passed &= event.kind() == CacheEventKind::Stored
                    && event.key() == Some(key.as_str())
                    && event.tags() == tags.as_slice();
            }
        }
    }
    let elapsed_nanoseconds = started.elapsed().as_nanos();
    let allocation = scope.finish();
    let memory = exact_memory(&cache).await?;

    Ok(ProfileResult {
        schema_version: 1,
        profile_id: PROFILE_ID,
        mode: options.mode.name(),
        topology: options.topology.name(),
        entry_count: ENTRY_COUNT,
        tags_per_entry: options.tags,
        subscriber_count: options.subscribers,
        invalidation_fanout: 0,
        key_bytes: KEY_BYTES,
        tag_bytes: TAG_BYTES,
        gross_allocated_bytes: allocation.gross_allocated_bytes,
        live_allocated_bytes: allocation.live_allocated_bytes,
        live_allocated_delta_bytes: allocation.live_allocated_delta_bytes,
        elapsed_nanoseconds,
        estimated_tag_retained_bytes: memory.estimated_tag_retained_bytes,
        tag_owner_records: memory.tag_owner_records,
        tag_owner_logical_bytes: memory.tag_owner_logical_bytes,
        live_entries: memory.snapshot.live_entries,
        tag_memberships: memory.snapshot.tag_memberships,
        tag_generation_records: memory.snapshot.tag_generation_records,
        key_generation_records: memory.snapshot.key_generation_records,
        event_tag_payload_bytes,
        event_delivery_count,
        invalidation_removed_keys: 0,
        expected_memberships,
        expected_event_deliveries,
        expected_event_tag_payload_bytes,
        exact_snapshot_passed: memory.exact_snapshot_passed,
        exact_memory_reconciliation_passed: memory.reconciliation_passed,
        membership_cardinality_passed: memory.snapshot.live_entries == ENTRY_COUNT as u64
            && memory.snapshot.tag_memberships == expected_memberships,
        event_content_equality_passed,
        event_delivery_count_passed: event_delivery_count == expected_event_deliveries
            && event_tag_payload_bytes == expected_event_tag_payload_bytes,
        invalidation_fanout_passed: true,
    })
}

async fn profile_invalidation(options: &Options) -> Result<ProfileResult, Box<dyn Error>> {
    let cache = build_cache();
    let shared_tag = fixed_id("fanout-tag-", 0);
    let value = Bytes::from_static(&[0x5a; VALUE_BYTES]);
    for entry_index in 0..options.fanout {
        cache
            .put_encoded(
                &fixed_id("key-", entry_index),
                value.clone(),
                CacheOptions::new().tag(shared_tag.clone()),
            )
            .await?;
    }
    let before = exact_memory(&cache).await?;
    if before.snapshot.live_entries != options.fanout as u64
        || before.snapshot.tag_memberships != options.fanout as u64
    {
        return Err("preloaded invalidation cardinality mismatch".into());
    }

    let scope = AllocationScope::start();
    let started = Instant::now();
    let invalidation_removed_keys = cache.invalidate_tag(&shared_tag).await?;
    let elapsed_nanoseconds = started.elapsed().as_nanos();
    let allocation = scope.finish();
    let memory = exact_memory(&cache).await?;
    let invalidation_fanout_passed = invalidation_removed_keys == options.fanout as u64
        && memory.snapshot.live_entries == 0
        && memory.snapshot.tag_memberships == 0
        && memory.snapshot.tag_generation_records == 1;

    Ok(ProfileResult {
        schema_version: 1,
        profile_id: PROFILE_ID,
        mode: options.mode.name(),
        topology: options.topology.name(),
        entry_count: options.fanout,
        tags_per_entry: 1,
        subscriber_count: 0,
        invalidation_fanout: options.fanout,
        key_bytes: KEY_BYTES,
        tag_bytes: TAG_BYTES,
        gross_allocated_bytes: allocation.gross_allocated_bytes,
        live_allocated_bytes: allocation.live_allocated_bytes,
        live_allocated_delta_bytes: allocation.live_allocated_delta_bytes,
        elapsed_nanoseconds,
        estimated_tag_retained_bytes: memory.estimated_tag_retained_bytes,
        tag_owner_records: memory.tag_owner_records,
        tag_owner_logical_bytes: memory.tag_owner_logical_bytes,
        live_entries: memory.snapshot.live_entries,
        tag_memberships: memory.snapshot.tag_memberships,
        tag_generation_records: memory.snapshot.tag_generation_records,
        key_generation_records: memory.snapshot.key_generation_records,
        event_tag_payload_bytes: 0,
        event_delivery_count: 0,
        invalidation_removed_keys,
        expected_memberships: 0,
        expected_event_deliveries: 0,
        expected_event_tag_payload_bytes: 0,
        exact_snapshot_passed: memory.exact_snapshot_passed,
        exact_memory_reconciliation_passed: memory.reconciliation_passed,
        membership_cardinality_passed: memory.snapshot.tag_memberships == 0,
        event_content_equality_passed: true,
        event_delivery_count_passed: true,
        invalidation_fanout_passed,
    })
}

fn build_cache() -> HydraCache {
    HydraCache::local()
        .max_capacity(16 * 1024 * 1024)
        .memory_instrumentation_mode(MemoryInstrumentationMode::Production)
        .event_buffer_capacity(2)
        .build()
}

fn tags_for(entry_index: usize, count: usize, topology: Topology) -> Vec<String> {
    (0..count)
        .map(|tag_index| match topology {
            Topology::Shared => fixed_id("shared-tag-", tag_index),
            Topology::Unique => fixed_id("unique-tag-", entry_index * 1_000 + tag_index),
        })
        .collect()
}

fn fixed_id(prefix: &str, number: usize) -> String {
    assert!(prefix.len() < KEY_BYTES);
    let width = KEY_BYTES - prefix.len();
    let value = format!("{prefix}{number:0width$}");
    assert_eq!(value.len(), KEY_BYTES);
    value
}

struct ExactMemory {
    snapshot: MemoryFootprintSnapshot,
    estimated_tag_retained_bytes: u64,
    tag_owner_records: u64,
    tag_owner_logical_bytes: u64,
    exact_snapshot_passed: bool,
    reconciliation_passed: bool,
}

async fn exact_memory(cache: &HydraCache) -> Result<ExactMemory, Box<dyn Error>> {
    let _ = cache.diagnostics().await;
    let reconciliation = cache.reconcile_memory_footprint().await?;
    let barrier = cache.memory_snapshot_barrier()?;
    let snapshot = cache
        .memory_footprint_snapshot(MemorySnapshotRequest::Exact {
            acknowledged_epoch: barrier.epoch,
        })
        .await?;
    let owner = snapshot
        .owners
        .iter()
        .find(|owner| owner.owner_id == TAG_OWNER_ID)
        .ok_or("tag-index memory owner missing")?;
    Ok(ExactMemory {
        estimated_tag_retained_bytes: owner.estimated_retained_bytes,
        tag_owner_records: owner.records,
        tag_owner_logical_bytes: owner.logical_bytes,
        exact_snapshot_passed: snapshot.workload_epoch_acknowledged
            && !snapshot.observed_non_atomic,
        reconciliation_passed: reconciliation.matched,
        snapshot,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_keys_and_tags_have_frozen_width() {
        assert_eq!(fixed_id("key-", 1).len(), KEY_BYTES);
        assert_eq!(fixed_id("shared-tag-", 63).len(), TAG_BYTES);
        assert_eq!(fixed_id("unique-tag-", 255_063).len(), TAG_BYTES);
    }

    #[test]
    fn shared_and_unique_topologies_have_expected_identity() {
        assert_eq!(
            tags_for(0, 4, Topology::Shared),
            tags_for(255, 4, Topology::Shared)
        );
        assert_ne!(
            tags_for(0, 4, Topology::Unique),
            tags_for(255, 4, Topology::Unique)
        );
    }
}
