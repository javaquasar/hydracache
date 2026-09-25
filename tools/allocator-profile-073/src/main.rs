use std::collections::BTreeMap;
use std::error::Error;
use std::hint::black_box;
use std::time::{Duration, Instant};

use hydracache::{CacheOptions, HydraCache};
use serde::Serialize;
use serde_json::Value;

#[cfg(not(any(
    feature = "allocator-system",
    feature = "allocator-mimalloc",
    feature = "allocator-jemalloc"
)))]
compile_error!(
    "select exactly one allocator-system, allocator-mimalloc, or allocator-jemalloc feature"
);

#[cfg(any(
    all(feature = "allocator-system", feature = "allocator-mimalloc"),
    all(feature = "allocator-system", feature = "allocator-jemalloc"),
    all(feature = "allocator-mimalloc", feature = "allocator-jemalloc")
))]
compile_error!("allocator profile features are mutually exclusive");

const PROFILE_ID: &str = "w8-allocator-profile-073-v1";
const CARDINALITY: usize = 16_384;
const PAYLOAD_BYTES: usize = 4_096;
const STEADY_READ_OPERATIONS: usize = 65_536;
const IDLE_MILLISECONDS: u64 = 2_000;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    NoPurge,
    Purge,
}

impl Mode {
    fn parse() -> Result<Self, Box<dyn Error>> {
        let mut args = std::env::args().skip(1);
        let mut mode = Self::NoPurge;
        while let Some(argument) = args.next() {
            match argument.as_str() {
                "--mode" => {
                    mode = match args.next().as_deref() {
                        Some("no-purge") => Self::NoPurge,
                        Some("purge") => Self::Purge,
                        _ => return Err("--mode must be no-purge or purge".into()),
                    };
                }
                _ => return Err(format!("unsupported argument: {argument}").into()),
            }
        }
        Ok(mode)
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::NoPurge => "no-purge",
            Self::Purge => "purge",
        }
    }
}

#[derive(Serialize)]
struct Receipt {
    schema_version: &'static str,
    profile_id: &'static str,
    allocator: &'static str,
    allocator_feature: &'static str,
    allocator_native_version: Option<u64>,
    target: String,
    mode: &'static str,
    cardinality: usize,
    payload_bytes: usize,
    steady_read_operations: usize,
    idle_milliseconds: u64,
    elapsed_ns: u64,
    phases: Vec<PhaseSample>,
    invariants: Invariants,
    promotable: bool,
}

#[derive(Serialize)]
struct PhaseSample {
    phase: &'static str,
    elapsed_ns: u64,
    logical_entries: usize,
    process: ProcessSnapshot,
    allocator_native: NativeSnapshot,
}

#[derive(Serialize)]
struct ProcessSnapshot {
    source: &'static str,
    working_set_bytes: Option<u64>,
    peak_working_set_bytes: Option<u64>,
    private_commit_bytes: Option<u64>,
    page_fault_count: Option<u64>,
    unavailable: BTreeMap<&'static str, &'static str>,
}

#[derive(Serialize)]
struct NativeSnapshot {
    provider: &'static str,
    status: &'static str,
    allocated_or_live: Option<NativeMetric>,
    active_or_committed: Option<NativeMetric>,
    resident: Option<NativeMetric>,
    retained_or_reserved: Option<NativeMetric>,
    arenas: Option<u64>,
    thread_caches: Option<u64>,
    page_fault_count: Option<u64>,
    purge_calls: Option<u64>,
    purged_bytes: Option<u64>,
    unavailable: BTreeMap<&'static str, &'static str>,
    raw: Option<Value>,
}

#[derive(Serialize)]
struct NativeMetric {
    bytes: u64,
    source: &'static str,
    semantics: &'static str,
}

#[derive(Serialize)]
struct Invariants {
    exact_phase_order: bool,
    exact_logical_cardinality: bool,
    payload_shape_preserved: bool,
    rss_used_as_native_substitute: bool,
    purge_requested: bool,
    purge_api_invoked: bool,
    purge_calls_delta: Option<u64>,
    purged_bytes_delta: Option<u64>,
    second_refill_completed: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mode = Mode::parse()?;
    if mode == Mode::Purge && allocator_name() == "system" {
        return Err("system allocator has no reviewed portable purge API".into());
    }
    let run_started = Instant::now();
    let cache = HydraCache::local()
        .max_capacity((CARDINALITY * PAYLOAD_BYTES * 3) as u64)
        .build();
    let mut phases = Vec::with_capacity(if mode == Mode::Purge { 9 } else { 6 });

    assert_entries(&cache, 0).await?;
    sample(&mut phases, "cold", 0, run_started)?;

    put_range(&cache, "fill", 0, CARDINALITY).await?;
    assert_entries(&cache, CARDINALITY).await?;
    sample(&mut phases, "fill", CARDINALITY, run_started)?;

    for index in 0..STEADY_READ_OPERATIONS {
        let key = format!("fill:{}", index % CARDINALITY);
        let value: Option<Vec<u8>> = cache.get(&key).await?;
        let value = value.ok_or_else(|| format!("steady read missed {key}"))?;
        if value.len() != PAYLOAD_BYTES {
            return Err(format!("steady read payload changed for {key}").into());
        }
        black_box(value);
    }
    assert_entries(&cache, CARDINALITY).await?;
    sample(&mut phases, "steady_read", CARDINALITY, run_started)?;

    for index in 0..CARDINALITY {
        let key = format!("fill:{index}");
        if !cache.remove(&key).await? {
            return Err(format!("delete missed {key}").into());
        }
    }
    assert_entries(&cache, 0).await?;
    sample(&mut phases, "delete", 0, run_started)?;

    put_range(&cache, "refill", 0, CARDINALITY).await?;
    assert_entries(&cache, CARDINALITY).await?;
    sample(&mut phases, "refill", CARDINALITY, run_started)?;

    tokio::time::sleep(Duration::from_millis(IDLE_MILLISECONDS)).await;
    sample(&mut phases, "post_idle", CARDINALITY, run_started)?;

    let mut purge_api_invoked = false;
    let mut purge_calls_delta = None;
    let mut purged_bytes_delta = None;
    let mut second_refill_completed = false;
    if mode == Mode::Purge {
        sample(&mut phases, "pre_purge", CARDINALITY, run_started)?;
        let before = native_purge_counters()?;
        invoke_purge()?;
        purge_api_invoked = true;
        let after = native_purge_counters()?;
        purge_calls_delta = optional_delta(before.0, after.0);
        purged_bytes_delta = optional_delta(before.1, after.1);
        sample(&mut phases, "post_purge", CARDINALITY, run_started)?;

        put_range(&cache, "second-refill", 0, CARDINALITY).await?;
        assert_entries(&cache, CARDINALITY * 2).await?;
        second_refill_completed = true;
        sample(&mut phases, "second_refill", CARDINALITY * 2, run_started)?;
    }

    let expected = if mode == Mode::Purge {
        vec![
            "cold",
            "fill",
            "steady_read",
            "delete",
            "refill",
            "post_idle",
            "pre_purge",
            "post_purge",
            "second_refill",
        ]
    } else {
        vec![
            "cold",
            "fill",
            "steady_read",
            "delete",
            "refill",
            "post_idle",
        ]
    };
    let observed = phases.iter().map(|phase| phase.phase).collect::<Vec<_>>();
    let exact_phase_order = observed == expected;
    if !exact_phase_order {
        return Err("phase order changed".into());
    }

    let receipt = Receipt {
        schema_version: "hydracache-w8-allocator-profile-v1",
        profile_id: PROFILE_ID,
        allocator: allocator_name(),
        allocator_feature: allocator_feature(),
        allocator_native_version: allocator_native_version(),
        target: format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS),
        mode: mode.as_str(),
        cardinality: CARDINALITY,
        payload_bytes: PAYLOAD_BYTES,
        steady_read_operations: STEADY_READ_OPERATIONS,
        idle_milliseconds: IDLE_MILLISECONDS,
        elapsed_ns: u64::try_from(run_started.elapsed().as_nanos()).unwrap_or(u64::MAX),
        phases,
        invariants: Invariants {
            exact_phase_order,
            exact_logical_cardinality: true,
            payload_shape_preserved: true,
            rss_used_as_native_substitute: false,
            purge_requested: mode == Mode::Purge,
            purge_api_invoked,
            purge_calls_delta,
            purged_bytes_delta,
            second_refill_completed,
        },
        promotable: false,
    };
    serde_json::to_writer(std::io::stdout().lock(), &receipt)?;
    println!();
    Ok(())
}

async fn assert_entries(cache: &HydraCache, expected: usize) -> Result<(), Box<dyn Error>> {
    let observed = cache.diagnostics().await.estimated_entries;
    if observed != expected as u64 {
        return Err(format!(
            "logical cardinality changed: expected {expected}, observed {observed}"
        )
        .into());
    }
    Ok(())
}

async fn put_range(
    cache: &HydraCache,
    prefix: &str,
    start: usize,
    count: usize,
) -> Result<(), Box<dyn Error>> {
    for index in start..start + count {
        let key = format!("{prefix}:{index}");
        let payload = vec![(index % 251) as u8; PAYLOAD_BYTES];
        cache.put(&key, payload, CacheOptions::new()).await?;
    }
    Ok(())
}

fn sample(
    phases: &mut Vec<PhaseSample>,
    phase: &'static str,
    logical_entries: usize,
    started: Instant,
) -> Result<(), Box<dyn Error>> {
    phases.push(PhaseSample {
        phase,
        elapsed_ns: u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX),
        logical_entries,
        process: process_snapshot()?,
        allocator_native: native_snapshot()?,
    });
    Ok(())
}

fn optional_delta(before: Option<u64>, after: Option<u64>) -> Option<u64> {
    Some(after?.saturating_sub(before?))
}

#[cfg(feature = "allocator-system")]
fn allocator_name() -> &'static str {
    "system"
}

#[cfg(feature = "allocator-mimalloc")]
fn allocator_name() -> &'static str {
    "mimalloc"
}

#[cfg(feature = "allocator-jemalloc")]
fn allocator_name() -> &'static str {
    "jemalloc"
}

fn allocator_feature() -> &'static str {
    match allocator_name() {
        "system" => "allocator-system",
        "mimalloc" => "allocator-mimalloc",
        "jemalloc" => "allocator-jemalloc",
        _ => unreachable!(),
    }
}

#[cfg(feature = "allocator-mimalloc")]
fn allocator_native_version() -> Option<u64> {
    Some(u64::from(mimalloc::MiMalloc.version()))
}

#[cfg(feature = "allocator-jemalloc")]
fn allocator_native_version() -> Option<u64> {
    None
}

#[cfg(feature = "allocator-system")]
fn allocator_native_version() -> Option<u64> {
    None
}

#[cfg(feature = "allocator-system")]
fn native_snapshot() -> Result<NativeSnapshot, Box<dyn Error>> {
    let reason = "system allocator exposes no portable allocator-native byte accounting";
    Ok(NativeSnapshot {
        provider: "system",
        status: "unavailable-with-reason",
        allocated_or_live: None,
        active_or_committed: None,
        resident: None,
        retained_or_reserved: None,
        arenas: None,
        thread_caches: None,
        page_fault_count: None,
        purge_calls: None,
        purged_bytes: None,
        unavailable: BTreeMap::from([
            ("allocated_or_live", reason),
            ("active_or_committed", reason),
            ("resident", reason),
            ("retained_or_reserved", reason),
            ("arenas", "system allocator arena count is not portable"),
            (
                "thread_caches",
                "system allocator thread-cache state is not portable",
            ),
        ]),
        raw: None,
    })
}

#[cfg(feature = "allocator-mimalloc")]
fn native_snapshot() -> Result<NativeSnapshot, Box<dyn Error>> {
    let stats = mimalloc::MiMalloc::stats_json().map_err(std::io::Error::other)?;
    let raw: Value = serde_json::from_str(stats.to_str()?)?;
    let committed = stat_current(&raw, "committed")?;
    let reserved = stat_current(&raw, "reserved")?;
    let resident = raw_u64(&raw, &["process", "rss_current"])?;
    Ok(NativeSnapshot {
        provider: "mimalloc",
        status: "partial-native-statistics",
        allocated_or_live: None,
        active_or_committed: Some(NativeMetric {
            bytes: committed,
            source: "mi_stats_get_json.committed.current",
            semantics: "current bytes committed by mimalloc",
        }),
        resident: Some(NativeMetric {
            bytes: resident,
            source: "mi_stats_get_json.process.rss_current",
            semantics:
                "process RSS reported by mimalloc process information; not allocator-owned RSS",
        }),
        retained_or_reserved: Some(NativeMetric {
            bytes: reserved,
            source: "mi_stats_get_json.reserved.current",
            semantics: "current virtual bytes reserved by mimalloc",
        }),
        arenas: raw.get("arena_count").and_then(Value::as_u64),
        thread_caches: None,
        page_fault_count: raw.pointer("/process/page_faults").and_then(Value::as_u64),
        purge_calls: raw.get("purge_calls").and_then(Value::as_u64),
        purged_bytes: raw.get("purged").and_then(Value::as_u64),
        unavailable: BTreeMap::from([
            (
                "allocated_or_live",
                "release mimalloc v3 JSON reports malloc_requested.current as zero without a supported process merge API; zero is retained only in raw evidence",
            ),
            (
                "thread_caches",
                "mimalloc JSON statistics do not expose a comparable thread-cache count",
            ),
        ]),
        raw: Some(raw),
    })
}

#[cfg(feature = "allocator-jemalloc")]
fn native_snapshot() -> Result<NativeSnapshot, Box<dyn Error>> {
    use tikv_jemalloc_ctl::{epoch, stats};

    epoch::advance()?;
    Ok(NativeSnapshot {
        provider: "jemalloc",
        status: "available",
        allocated_or_live: Some(NativeMetric {
            bytes: stats::allocated::read()? as u64,
            source: "mallctl.stats.allocated",
            semantics: "bytes allocated by the application",
        }),
        active_or_committed: Some(NativeMetric {
            bytes: stats::active::read()? as u64,
            source: "mallctl.stats.active",
            semantics: "bytes in active pages",
        }),
        resident: Some(NativeMetric {
            bytes: stats::resident::read()? as u64,
            source: "mallctl.stats.resident",
            semantics: "maximum bytes in physically resident allocator mappings",
        }),
        retained_or_reserved: Some(NativeMetric {
            bytes: stats::retained::read()? as u64,
            source: "mallctl.stats.retained",
            semantics: "virtual memory retained by jemalloc",
        }),
        arenas: None,
        thread_caches: None,
        page_fault_count: None,
        purge_calls: None,
        purged_bytes: None,
        unavailable: BTreeMap::from([
            (
                "arenas",
                "arena count is not yet normalized by this profile revision",
            ),
            (
                "thread_caches",
                "thread-cache count is not exposed as a comparable scalar",
            ),
            (
                "page_fault_count",
                "jemalloc mallctl does not expose process page faults",
            ),
            (
                "purge_calls",
                "jemalloc purge counters are not normalized by this profile revision",
            ),
            (
                "purged_bytes",
                "jemalloc purge bytes are not normalized by this profile revision",
            ),
        ]),
        raw: None,
    })
}

#[cfg(feature = "allocator-mimalloc")]
fn stat_current(raw: &Value, field: &str) -> Result<u64, Box<dyn Error>> {
    raw.get(field)
        .and_then(|value| value.get("current"))
        .and_then(Value::as_i64)
        .and_then(|value| u64::try_from(value).ok())
        .ok_or_else(|| format!("mimalloc field {field}.current is unavailable").into())
}

#[cfg(feature = "allocator-mimalloc")]
fn raw_u64(raw: &Value, path: &[&str]) -> Result<u64, Box<dyn Error>> {
    let mut value = raw;
    for field in path {
        value = value
            .get(*field)
            .ok_or_else(|| format!("mimalloc field {} is unavailable", path.join(".")))?;
    }
    value
        .as_u64()
        .ok_or_else(|| format!("mimalloc field {} is not u64", path.join(".")).into())
}

#[cfg(feature = "allocator-mimalloc")]
fn native_purge_counters() -> Result<(Option<u64>, Option<u64>), Box<dyn Error>> {
    let snapshot = native_snapshot()?;
    Ok((snapshot.purge_calls, snapshot.purged_bytes))
}

#[cfg(feature = "allocator-jemalloc")]
fn native_purge_counters() -> Result<(Option<u64>, Option<u64>), Box<dyn Error>> {
    Ok((None, None))
}

#[cfg(feature = "allocator-system")]
fn native_purge_counters() -> Result<(Option<u64>, Option<u64>), Box<dyn Error>> {
    Err("system allocator has no reviewed portable purge counters".into())
}

#[cfg(feature = "allocator-mimalloc")]
fn invoke_purge() -> Result<(), Box<dyn Error>> {
    unsafe { libmimalloc_sys::mi_collect(true) };
    Ok(())
}

#[cfg(feature = "allocator-jemalloc")]
fn invoke_purge() -> Result<(), Box<dyn Error>> {
    Err("jemalloc purge is intentionally unavailable until its rate-limited arena contract is implemented".into())
}

#[cfg(feature = "allocator-system")]
fn invoke_purge() -> Result<(), Box<dyn Error>> {
    Err("system allocator has no reviewed portable purge API".into())
}

#[cfg(windows)]
fn process_snapshot() -> Result<ProcessSnapshot, Box<dyn Error>> {
    use std::mem::size_of;
    use windows_sys::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    let mut memory: PROCESS_MEMORY_COUNTERS_EX = unsafe { std::mem::zeroed() };
    memory.cb = size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
    let process = unsafe { GetCurrentProcess() };
    let success = unsafe {
        GetProcessMemoryInfo(
            process,
            &mut memory as *mut PROCESS_MEMORY_COUNTERS_EX as *mut PROCESS_MEMORY_COUNTERS,
            memory.cb,
        )
    };
    if success == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(ProcessSnapshot {
        source: "windows-GetProcessMemoryInfo",
        working_set_bytes: Some(memory.WorkingSetSize as u64),
        peak_working_set_bytes: Some(memory.PeakWorkingSetSize as u64),
        private_commit_bytes: Some(memory.PrivateUsage as u64),
        page_fault_count: Some(u64::from(memory.PageFaultCount)),
        unavailable: BTreeMap::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_delta_requires_both_counters_and_never_underflows() {
        assert_eq!(optional_delta(Some(10), Some(13)), Some(3));
        assert_eq!(optional_delta(Some(13), Some(10)), Some(0));
        assert_eq!(optional_delta(None, Some(10)), None);
    }

    #[cfg(feature = "allocator-system")]
    #[test]
    fn system_native_fields_are_explicitly_unavailable() {
        let snapshot = native_snapshot().expect("system capability snapshot");
        assert_eq!(snapshot.status, "unavailable-with-reason");
        assert!(snapshot.allocated_or_live.is_none());
        assert!(snapshot.active_or_committed.is_none());
        assert!(snapshot.resident.is_none());
        assert!(snapshot.retained_or_reserved.is_none());
        assert!(snapshot.unavailable.contains_key("allocated_or_live"));
        assert!(snapshot.raw.is_none());
    }

    #[cfg(feature = "allocator-mimalloc")]
    #[test]
    fn mimalloc_release_stats_keep_partial_fields_and_raw_payload() {
        let snapshot = native_snapshot().expect("mimalloc native snapshot");
        assert_eq!(snapshot.status, "partial-native-statistics");
        assert!(snapshot.allocated_or_live.is_none());
        assert!(snapshot.active_or_committed.is_some());
        assert!(snapshot.resident.is_some());
        assert!(snapshot.retained_or_reserved.is_some());
        assert!(snapshot.unavailable.contains_key("allocated_or_live"));
        assert!(snapshot.raw.is_some());
    }

    #[cfg(feature = "allocator-mimalloc")]
    #[test]
    fn mimalloc_purge_api_has_monotonic_call_counter() {
        let before = native_purge_counters().expect("before purge").0;
        invoke_purge().expect("mimalloc purge");
        let after = native_purge_counters().expect("after purge").0;
        assert!(before
            .zip(after)
            .is_some_and(|(before, after)| after > before));
    }
}

#[cfg(target_os = "linux")]
fn process_snapshot() -> Result<ProcessSnapshot, Box<dyn Error>> {
    let status = std::fs::read_to_string("/proc/self/status")?;
    let stat = std::fs::read_to_string("/proc/self/stat")?;
    let fields = stat.split_whitespace().collect::<Vec<_>>();
    let faults = fields
        .get(9)
        .and_then(|value| value.parse::<u64>().ok())
        .zip(fields.get(11).and_then(|value| value.parse::<u64>().ok()))
        .map(|(minor, major)| minor.saturating_add(major));
    Ok(ProcessSnapshot {
        source: "linux-proc-self-status-and-stat",
        working_set_bytes: status_kib(&status, "VmRSS:").map(|value| value * 1_024),
        peak_working_set_bytes: status_kib(&status, "VmHWM:").map(|value| value * 1_024),
        private_commit_bytes: None,
        page_fault_count: faults,
        unavailable: BTreeMap::from([(
            "private_commit_bytes",
            "Linux procfs has no process-private commit metric equivalent to Windows PrivateUsage",
        )]),
    })
}

#[cfg(target_os = "linux")]
fn status_kib(status: &str, name: &str) -> Option<u64> {
    status.lines().find_map(|line| {
        line.strip_prefix(name)?
            .split_whitespace()
            .next()?
            .parse::<u64>()
            .ok()
    })
}

#[cfg(not(any(windows, target_os = "linux")))]
fn process_snapshot() -> Result<ProcessSnapshot, Box<dyn Error>> {
    Ok(ProcessSnapshot {
        source: "unsupported",
        working_set_bytes: None,
        peak_working_set_bytes: None,
        private_commit_bytes: None,
        page_fault_count: None,
        unavailable: BTreeMap::from([
            ("working_set_bytes", "unsupported target"),
            ("peak_working_set_bytes", "unsupported target"),
            ("private_commit_bytes", "unsupported target"),
            ("page_fault_count", "unsupported target"),
        ]),
    })
}
