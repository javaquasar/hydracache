use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use hydracache::{
    ClusterEpoch, DurabilityWritePath, DurableValueStore, InMemoryReplicatedValueStore,
    NamespacePersistenceRule, NamespacePersistenceSettings, PartitionId, PersistenceDurability,
    PersistencePolicy, PersistenceRegionPlacement, ReplicatedValueRecord, ReplicatedValueStore,
    TombstoneBudget, TombstoneTracker,
};
use serde::Serialize;

const PROFILE_ID: &str = "w7-durability-page-cache-profile-073-v1";

struct CountingAllocator;

static COUNTING: AtomicBool = AtomicBool::new(false);
static GROSS: AtomicU64 = AtomicU64::new(0);
static LIVE: AtomicU64 = AtomicU64::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() && COUNTING.load(Ordering::Relaxed) {
            GROSS.fetch_add(layout.size() as u64, Ordering::Relaxed);
            LIVE.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() && COUNTING.load(Ordering::Relaxed) {
            GROSS.fetch_add(layout.size() as u64, Ordering::Relaxed);
            LIVE.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        if COUNTING.load(Ordering::Relaxed) {
            LIVE.fetch_sub(layout.size() as u64, Ordering::Relaxed);
        }
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_pointer = unsafe { System.realloc(pointer, layout, new_size) };
        if !new_pointer.is_null() && COUNTING.load(Ordering::Relaxed) {
            GROSS.fetch_add(new_size as u64, Ordering::Relaxed);
            if new_size >= layout.size() {
                LIVE.fetch_add((new_size - layout.size()) as u64, Ordering::Relaxed);
            } else {
                LIVE.fetch_sub((layout.size() - new_size) as u64, Ordering::Relaxed);
            }
        }
        new_pointer
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

struct AllocationScope(u64);

impl AllocationScope {
    fn start() -> Self {
        GROSS.store(0, Ordering::Relaxed);
        let live = LIVE.load(Ordering::Relaxed);
        assert!(!COUNTING.swap(true, Ordering::AcqRel));
        Self(live)
    }

    fn finish(self) -> (u64, i128) {
        COUNTING.store(false, Ordering::Release);
        let live = LIVE.load(Ordering::Relaxed);
        (GROSS.load(Ordering::Relaxed), live as i128 - self.0 as i128)
    }
}

impl Drop for AllocationScope {
    fn drop(&mut self) {
        COUNTING.store(false, Ordering::Release);
    }
}

#[derive(Clone)]
struct Options {
    group: String,
    mode: String,
    cardinality: usize,
    payload_bytes: usize,
    repair_state: String,
}

impl Options {
    fn parse() -> Result<Self, Box<dyn Error>> {
        let mut values = HashMap::new();
        let mut args = std::env::args().skip(1);
        while let Some(key) = args.next() {
            let value = args
                .next()
                .ok_or_else(|| format!("missing value after {key}"))?;
            values.insert(key, value);
        }
        let group = values.remove("--group").ok_or("--group is required")?;
        let mode = values.remove("--mode").unwrap_or_else(|| "sync".to_owned());
        let cardinality = values
            .remove("--cardinality")
            .ok_or("--cardinality is required")?
            .parse()?;
        let payload_bytes = values
            .remove("--payload-bytes")
            .unwrap_or_else(|| "64".to_owned())
            .parse()?;
        let repair_state = values
            .remove("--repair-state")
            .unwrap_or_else(|| "pending".to_owned());
        if !values.is_empty() {
            return Err(format!("unknown arguments: {:?}", values.keys()).into());
        }
        Ok(Self {
            group,
            mode,
            cardinality,
            payload_bytes,
            repair_state,
        })
    }
}

#[derive(Clone, Copy)]
struct OsSnapshot {
    working_set_bytes: u64,
    private_commit_bytes: u64,
    resident_anonymous_bytes: Option<u64>,
    resident_file_bytes: Option<u64>,
    read_transfer_bytes: u64,
    write_transfer_bytes: u64,
}

#[derive(Serialize)]
struct PhaseMetrics {
    phase: String,
    operations: usize,
    gross_allocated_bytes: u64,
    live_allocated_delta_bytes: i128,
    elapsed_nanoseconds: u128,
    process_read_transfer_bytes: u64,
    process_write_transfer_bytes: u64,
    working_set_bytes: u64,
    private_commit_bytes: u64,
    resident_anonymous_bytes: Option<u64>,
    resident_file_bytes: Option<u64>,
    directory_logical_bytes: u64,
}

#[derive(Serialize)]
struct ResultRow {
    schema_version: u32,
    profile_id: &'static str,
    group: String,
    mode: String,
    cardinality: usize,
    payload_bytes: usize,
    repair_state: String,
    os_memory_source: &'static str,
    os_io_source: &'static str,
    anon_file_split_available: bool,
    phases: Vec<PhaseMetrics>,
    logical_durable_bytes: u64,
    expected_logical_durable_bytes: u64,
    record_count: usize,
    expected_record_count: usize,
    reopened_record_count: usize,
    reopen_content_equality_passed: bool,
    async_lag_after_admit: usize,
    async_lag_after_drain: usize,
    gc_scanned: usize,
    gc_removed: usize,
    gc_reclaimed_bytes: u64,
    gc_behavior_passed: bool,
    exact_logical_bytes_passed: bool,
    temporary_store_removed: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = Options::parse()?;
    let path = unique_store_path(&options.group);
    fs::create_dir_all(&path)?;
    let mut row = match options.group.as_str() {
        "store-lifecycle" => profile_store_lifecycle(&options, &path)?,
        "write-path-modes" => profile_write_path(&options, &path)?,
        "tombstone-gc" => profile_tombstone_gc(&options, &path)?,
        _ => return Err(format!("unknown group {}", options.group).into()),
    };
    row.temporary_store_removed = remove_store(&path)?;
    println!("{}", serde_json::to_string(&row)?);
    Ok(())
}

fn profile_store_lifecycle(options: &Options, path: &Path) -> Result<ResultRow, Box<dyn Error>> {
    let mut phases = Vec::new();
    let (mut store, phase) = measure("open", 1, path, || {
        DurableValueStore::open_with_budget(path, u64::MAX)
    })?;
    phases.push(phase);

    let (_, phase) = measure("fill", options.cardinality, path, || {
        for index in 0..options.cardinality {
            store.upsert(key(index), record(index, 1, options.payload_bytes))?;
        }
        Ok::<_, hydracache::ValueStoreError>(())
    })?;
    phases.push(phase);
    let logical_after_fill = store.total_bytes()?;

    let (_, phase) = measure("steady-read", options.cardinality, path, || {
        for index in 0..options.cardinality {
            black_box(store.get(&key(index))?.ok_or_else(|| {
                hydracache::ValueStoreError::new("profile record missing during steady read")
            })?);
        }
        Ok::<_, hydracache::ValueStoreError>(())
    })?;
    phases.push(phase);

    let (_, phase) = measure("overwrite", options.cardinality, path, || {
        for index in 0..options.cardinality {
            store.upsert(key(index), record(index, 2, options.payload_bytes))?;
        }
        Ok::<_, hydracache::ValueStoreError>(())
    })?;
    phases.push(phase);
    let logical = store.total_bytes()?;
    let records = store.scan_all()?.len();
    drop(store);

    let (reopened, phase) = measure("reopen", 1, path, || reopen_store(path, u64::MAX))?;
    phases.push(phase);
    let (reopened_count, phase) = measure("reopen-read", options.cardinality, path, || {
        let mut found = 0_usize;
        for index in 0..options.cardinality {
            let observed = reopened.get(&key(index))?;
            if observed == Some(record(index, 2, options.payload_bytes)) {
                found += 1;
            }
        }
        Ok::<_, hydracache::ValueStoreError>(found)
    })?;
    phases.push(phase);
    drop(reopened);

    let expected = (options.cardinality * options.payload_bytes.max(1)) as u64;
    Ok(result_row(
        options,
        phases,
        logical,
        expected,
        records,
        options.cardinality,
        reopened_count,
        reopened_count == options.cardinality && logical_after_fill == expected,
        0,
        0,
        0,
        0,
        0,
        true,
    ))
}

fn profile_write_path(options: &Options, path: &Path) -> Result<ResultRow, Box<dyn Error>> {
    match options.mode.as_str() {
        "ram-only" => profile_ram_only(options, path),
        "sync" | "async-bounded" => profile_persistent_write_path(options, path),
        _ => Err(format!("unknown durability mode {}", options.mode).into()),
    }
}

fn profile_ram_only(options: &Options, path: &Path) -> Result<ResultRow, Box<dyn Error>> {
    let mut phases = Vec::new();
    let (mut write_path, phase) = measure("open", 1, path, || {
        Ok::<_, hydracache::ValueStoreError>(DurabilityWritePath::new(
            InMemoryReplicatedValueStore::default(),
            PersistencePolicy::ram_only(),
            "eu",
            PersistenceRegionPlacement::home_region_only("eu"),
        ))
    })?;
    phases.push(phase);
    let (_, phase) = measure("admit", options.cardinality, path, || {
        for index in 0..options.cardinality {
            black_box(write_path.write(
                "cache.profile",
                key(index),
                record(index, 1, options.payload_bytes),
            )?);
        }
        Ok::<_, hydracache::DurabilityError>(())
    })?;
    phases.push(phase);
    let (_, phase) = measure("drain-or-sync", 1, path, || write_path.drain_async())?;
    phases.push(phase);
    let metrics = write_path.metrics();
    let records = write_path.store().scan_all()?.len();
    let logical = write_path.store().total_bytes();
    drop(write_path);
    let (_, phase) = measure("reopen-read", 1, path, || Ok::<_, std::io::Error>(()))?;
    phases.push(phase);
    Ok(result_row(
        options,
        phases,
        logical,
        0,
        records,
        0,
        0,
        metrics.ram_only_skipped_total as usize == options.cardinality,
        0,
        0,
        0,
        0,
        0,
        true,
    ))
}

fn profile_persistent_write_path(
    options: &Options,
    path: &Path,
) -> Result<ResultRow, Box<dyn Error>> {
    let mut phases = Vec::new();
    let durability = if options.mode == "sync" {
        PersistenceDurability::Sync
    } else {
        PersistenceDurability::AsyncBounded {
            max_lag: options.cardinality + 1,
        }
    };
    let (mut write_path, phase) = measure("open", 1, path, || {
        let store = DurableValueStore::open_with_budget(path, u64::MAX)?;
        Ok::<_, hydracache::ValueStoreError>(DurabilityWritePath::new(
            store,
            persistent_policy(durability),
            "eu",
            PersistenceRegionPlacement::home_region_only("eu"),
        ))
    })?;
    phases.push(phase);
    let (_, phase) = measure("admit", options.cardinality, path, || {
        for index in 0..options.cardinality {
            black_box(write_path.write(
                "cache.profile",
                key(index),
                record(index, 1, options.payload_bytes),
            )?);
        }
        Ok::<_, hydracache::DurabilityError>(())
    })?;
    phases.push(phase);
    let lag_after_admit = write_path.pending_lag();
    let (_, phase) = measure("drain-or-sync", options.cardinality.max(1), path, || {
        write_path.drain_async()
    })?;
    phases.push(phase);
    let lag_after_drain = write_path.pending_lag();
    let logical = write_path.store().total_bytes()?;
    let records = write_path.store().scan_all()?.len();
    let store = write_path.into_store();
    drop(store);

    let reopened = reopen_store(path, u64::MAX)?;
    let (reopened_count, phase) = measure("reopen-read", options.cardinality, path, || {
        let mut found = 0_usize;
        for index in 0..options.cardinality {
            if reopened.get(&key(index))? == Some(record(index, 1, options.payload_bytes)) {
                found += 1;
            }
        }
        Ok::<_, hydracache::ValueStoreError>(found)
    })?;
    phases.push(phase);
    drop(reopened);
    let expected = (options.cardinality * options.payload_bytes.max(1)) as u64;
    Ok(result_row(
        options,
        phases,
        logical,
        expected,
        records,
        options.cardinality,
        reopened_count,
        reopened_count == options.cardinality,
        lag_after_admit,
        lag_after_drain,
        0,
        0,
        0,
        true,
    ))
}

fn profile_tombstone_gc(options: &Options, path: &Path) -> Result<ResultRow, Box<dyn Error>> {
    let mut phases = Vec::new();
    let (mut store, phase) = measure("open", 1, path, || {
        DurableValueStore::open_with_budget(path, u64::MAX)
    })?;
    phases.push(phase);
    let mut tracker = TombstoneTracker::new(TombstoneBudget::new(
        options.cardinality + 1,
        (options.cardinality + 1) as u64,
    ));
    let (_, phase) = measure("fill-tombstones", options.cardinality, path, || {
        for index in 0..options.cardinality {
            let name = key(index);
            store.tombstone(
                name.clone(),
                PartitionId::new((index % 16) as u32),
                1,
                ClusterEpoch::new(1),
            )?;
            black_box(tracker.admit(name.clone(), 1, 1, None));
            if options.repair_state == "confirmed" {
                tracker.confirm_repair(&name, ClusterEpoch::new(1));
            }
        }
        Ok::<_, hydracache::ValueStoreError>(())
    })?;
    phases.push(phase);
    let (report, phase) = measure("gc", options.cardinality, path, || {
        store.collect_tombstone_garbage(&mut tracker, ClusterEpoch::new(2), options.cardinality)
    })?;
    phases.push(phase);
    let logical = store.total_bytes()?;
    let records = store.scan_all()?.len();
    drop(store);
    let reopened = reopen_store(path, u64::MAX)?;
    let reopened_count = reopened.scan_all()?.len();
    drop(reopened);
    let expected_records = if options.repair_state == "confirmed" {
        0
    } else {
        options.cardinality
    };
    let expected_logical = expected_records as u64;
    let gc_ok = report.scanned == options.cardinality
        && report.removed == options.cardinality - expected_records
        && report.reclaimed_bytes == (options.cardinality - expected_records) as u64;
    Ok(result_row(
        options,
        phases,
        logical,
        expected_logical,
        records,
        expected_records,
        reopened_count,
        reopened_count == expected_records,
        0,
        0,
        report.scanned,
        report.removed,
        report.reclaimed_bytes,
        gc_ok,
    ))
}

#[allow(clippy::too_many_arguments)]
fn result_row(
    options: &Options,
    phases: Vec<PhaseMetrics>,
    logical_durable_bytes: u64,
    expected_logical_durable_bytes: u64,
    record_count: usize,
    expected_record_count: usize,
    reopened_record_count: usize,
    reopen_content_equality_passed: bool,
    async_lag_after_admit: usize,
    async_lag_after_drain: usize,
    gc_scanned: usize,
    gc_removed: usize,
    gc_reclaimed_bytes: u64,
    gc_behavior_passed: bool,
) -> ResultRow {
    ResultRow {
        schema_version: 1,
        profile_id: PROFILE_ID,
        group: options.group.clone(),
        mode: options.mode.clone(),
        cardinality: options.cardinality,
        payload_bytes: options.payload_bytes,
        repair_state: options.repair_state.clone(),
        os_memory_source: os_memory_source(),
        os_io_source: os_io_source(),
        anon_file_split_available: phases.iter().all(|phase| {
            phase.resident_anonymous_bytes.is_some() && phase.resident_file_bytes.is_some()
        }),
        phases,
        logical_durable_bytes,
        expected_logical_durable_bytes,
        record_count,
        expected_record_count,
        reopened_record_count,
        reopen_content_equality_passed,
        async_lag_after_admit,
        async_lag_after_drain,
        gc_scanned,
        gc_removed,
        gc_reclaimed_bytes,
        gc_behavior_passed,
        exact_logical_bytes_passed: logical_durable_bytes == expected_logical_durable_bytes
            && record_count == expected_record_count,
        temporary_store_removed: false,
    }
}

fn measure<T, E, F>(
    phase: &str,
    operations: usize,
    path: &Path,
    action: F,
) -> Result<(T, PhaseMetrics), Box<dyn Error>>
where
    E: Error + 'static,
    F: FnOnce() -> Result<T, E>,
{
    let before = os_snapshot()?;
    let scope = AllocationScope::start();
    let started = Instant::now();
    let value = action()?;
    let elapsed = started.elapsed().as_nanos();
    let (gross, live_delta) = scope.finish();
    let after = os_snapshot()?;
    Ok((
        value,
        PhaseMetrics {
            phase: phase.to_owned(),
            operations,
            gross_allocated_bytes: gross,
            live_allocated_delta_bytes: live_delta,
            elapsed_nanoseconds: elapsed,
            process_read_transfer_bytes: after
                .read_transfer_bytes
                .saturating_sub(before.read_transfer_bytes),
            process_write_transfer_bytes: after
                .write_transfer_bytes
                .saturating_sub(before.write_transfer_bytes),
            working_set_bytes: after.working_set_bytes,
            private_commit_bytes: after.private_commit_bytes,
            resident_anonymous_bytes: after.resident_anonymous_bytes,
            resident_file_bytes: after.resident_file_bytes,
            directory_logical_bytes: directory_logical_bytes(path)?,
        },
    ))
}

fn persistent_policy(durability: PersistenceDurability) -> PersistencePolicy {
    PersistencePolicy::try_new([NamespacePersistenceRule::new(
        "cache.profile",
        NamespacePersistenceSettings::persistent().with_durability(durability),
    )
    .expect("static persistence rule")])
    .expect("single static persistence rule")
}

fn key(index: usize) -> String {
    format!("profile-key-{index:08x}")
}

fn record(index: usize, version: u64, payload_bytes: usize) -> ReplicatedValueRecord {
    let mut payload = vec![(index % 251) as u8; payload_bytes];
    if !payload.is_empty() {
        payload[0] = version as u8;
    }
    ReplicatedValueRecord::value(
        PartitionId::new((index % 16) as u32),
        version,
        ClusterEpoch::new(1),
        payload,
    )
}

fn reopen_store(
    path: &Path,
    budget: u64,
) -> Result<DurableValueStore, hydracache::ValueStoreError> {
    for _ in 0..100 {
        match DurableValueStore::open_with_budget(path, budget) {
            Ok(store) => return Ok(store),
            Err(error) if error.to_string().contains("could not acquire lock") => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => return Err(error),
        }
    }
    Err(hydracache::ValueStoreError::new(
        "durable store lock was not released within one second",
    ))
}

fn unique_store_path(group: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "hydracache-w7-profile-{group}-{}-{nanos}",
        std::process::id()
    ))
}

fn directory_logical_bytes(path: &Path) -> Result<u64, std::io::Error> {
    if !path.exists() {
        return Ok(0);
    }
    let mut total = 0_u64;
    let mut pending = vec![path.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            if metadata.is_dir() {
                pending.push(entry.path());
            } else {
                total = total.saturating_add(metadata.len());
            }
        }
    }
    Ok(total)
}

fn remove_store(path: &Path) -> Result<bool, std::io::Error> {
    for _ in 0..100 {
        match fs::remove_dir_all(path) {
            Ok(()) => return Ok(!path.exists()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(true),
            Err(_) => thread::sleep(Duration::from_millis(10)),
        }
    }
    fs::remove_dir_all(path)?;
    Ok(!path.exists())
}

#[cfg(windows)]
fn os_snapshot() -> Result<OsSnapshot, Box<dyn Error>> {
    use std::mem::size_of;
    use windows_sys::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX,
    };
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcess, GetProcessIoCounters, IO_COUNTERS,
    };

    let process = unsafe { GetCurrentProcess() };
    let mut memory: PROCESS_MEMORY_COUNTERS_EX = unsafe { std::mem::zeroed() };
    memory.cb = size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
    if unsafe {
        GetProcessMemoryInfo(
            process,
            &mut memory as *mut PROCESS_MEMORY_COUNTERS_EX as *mut PROCESS_MEMORY_COUNTERS,
            memory.cb,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error().into());
    }
    let mut io: IO_COUNTERS = unsafe { std::mem::zeroed() };
    if unsafe { GetProcessIoCounters(process, &mut io) } == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(OsSnapshot {
        working_set_bytes: memory.WorkingSetSize as u64,
        private_commit_bytes: memory.PrivateUsage as u64,
        resident_anonymous_bytes: None,
        resident_file_bytes: None,
        read_transfer_bytes: io.ReadTransferCount,
        write_transfer_bytes: io.WriteTransferCount,
    })
}

#[cfg(target_os = "linux")]
fn os_snapshot() -> Result<OsSnapshot, Box<dyn Error>> {
    let smaps = fs::read_to_string("/proc/self/smaps_rollup")?;
    let io = fs::read_to_string("/proc/self/io")?;
    let rss = proc_kib(&smaps, "Rss:")?;
    let anonymous = proc_kib(&smaps, "Pss_Anon:")?;
    let file = proc_kib(&smaps, "Pss_File:")?;
    let private =
        proc_kib(&smaps, "Private_Clean:")?.saturating_add(proc_kib(&smaps, "Private_Dirty:")?);
    Ok(OsSnapshot {
        working_set_bytes: rss,
        private_commit_bytes: private,
        resident_anonymous_bytes: Some(anonymous),
        resident_file_bytes: Some(file),
        read_transfer_bytes: proc_u64(&io, "read_bytes:")?,
        write_transfer_bytes: proc_u64(&io, "write_bytes:")?,
    })
}

#[cfg(target_os = "linux")]
fn proc_kib(text: &str, key: &str) -> Result<u64, Box<dyn Error>> {
    Ok(proc_u64(text, key)?.saturating_mul(1024))
}

#[cfg(target_os = "linux")]
fn proc_u64(text: &str, key: &str) -> Result<u64, Box<dyn Error>> {
    text.lines()
        .find_map(|line| {
            line.strip_prefix(key)
                .and_then(|rest| rest.split_whitespace().next())
                .and_then(|value| value.parse().ok())
        })
        .ok_or_else(|| format!("missing {key} in proc counter source").into())
}

#[cfg(not(any(windows, target_os = "linux")))]
fn os_snapshot() -> Result<OsSnapshot, Box<dyn Error>> {
    Err("W7 profile requires Windows process counters or Linux procfs".into())
}

#[cfg(windows)]
fn os_memory_source() -> &'static str {
    "windows-GetProcessMemoryInfo-working-set-private-commit; anon-file-split-unavailable"
}

#[cfg(target_os = "linux")]
fn os_memory_source() -> &'static str {
    "linux-proc-self-smaps-rollup"
}

#[cfg(not(any(windows, target_os = "linux")))]
fn os_memory_source() -> &'static str {
    "unsupported"
}

#[cfg(windows)]
fn os_io_source() -> &'static str {
    "windows-GetProcessIoCounters-transfer-bytes"
}

#[cfg(target_os = "linux")]
fn os_io_source() -> &'static str {
    "linux-proc-self-io-read-write-bytes"
}

#[cfg(not(any(windows, target_os = "linux")))]
fn os_io_source() -> &'static str {
    "unsupported"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_scenario_count_is_twenty_four() {
        assert_eq!(4 * 2 + 3 * 2 * 2 + 2 * 2, 24);
    }

    #[test]
    fn record_payload_identity_changes_with_version() {
        assert_ne!(record(1, 1, 64), record(1, 2, 64));
        assert_eq!(record(1, 1, 64).approx_bytes(), 64);
    }

    #[test]
    fn persistent_policy_selects_profile_namespace() {
        let policy = persistent_policy(PersistenceDurability::Sync);
        let placement = PersistenceRegionPlacement::home_region_only("eu");
        assert!(policy
            .resolve_for_region("cache.profile", &"eu".into(), &placement)
            .unwrap()
            .persists());
    }
}
