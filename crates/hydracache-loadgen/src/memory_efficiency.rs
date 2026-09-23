use std::path::{Path, PathBuf};
use std::time::Duration;

use hydracache::{
    CacheOptions, HydraCache, MemoryFootprintSnapshot, MemoryInstrumentationMode,
    MemorySnapshotConsistency, MemorySnapshotRequest,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::allocation::{measure_allocations, AllocationMeasurement};

const OVERHEAD_PROFILE_073: &str = "instrumentation-overhead-073-v1";
const COUNTERS_ONLY_PROFILE_073: &str = "instrumentation-overhead-counters-only-073-v1";

pub const MEMORY_PHASES: [MemoryPhase; 8] = [
    MemoryPhase::Cold,
    MemoryPhase::Fill,
    MemoryPhase::Steady,
    MemoryPhase::ExpireOrDelete,
    MemoryPhase::Reset,
    MemoryPhase::Refill,
    MemoryPhase::PostIdle,
    MemoryPhase::Shutdown,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryPhase {
    Cold,
    Fill,
    Steady,
    ExpireOrDelete,
    Reset,
    Refill,
    PostIdle,
    Shutdown,
}

impl MemoryPhase {
    fn file_stem(self) -> &'static str {
        match self {
            Self::Cold => "cold",
            Self::Fill => "fill",
            Self::Steady => "steady",
            Self::ExpireOrDelete => "expire_or_delete",
            Self::Reset => "reset",
            Self::Refill => "refill",
            Self::PostIdle => "post_idle",
            Self::Shutdown => "shutdown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryPhaseTimelineRecord {
    pub schema_version: String,
    pub sequence: u64,
    pub phase: MemoryPhase,
    pub epoch: u64,
    pub monotonic_ns: u64,
    pub owner_snapshot_digest: String,
    pub telemetry_checkpoint: String,
    pub provider_mark: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryEfficiencyReceipt {
    pub schema_version: String,
    pub profile: String,
    pub provider: String,
    pub instrumentation_mode: MemoryInstrumentationMode,
    pub phase_count: usize,
    pub elapsed_ns: u64,
    pub timeline: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_series: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostic_variant: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counter_correctness_eligible: Option<bool>,
    pub promotable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryPhaseResourceRecord {
    pub schema_version: String,
    pub sequence: u64,
    pub phase: MemoryPhase,
    pub operations: u64,
    pub gross_allocated_bytes: Option<u64>,
    pub gross_allocated_bytes_per_operation: Option<f64>,
    pub rss_bytes: Option<u64>,
    pub peak_rss_bytes: Option<u64>,
    pub process_memory_available: bool,
    pub process_memory_unavailable_reason: Option<String>,
}

pub async fn run_and_write_memory_efficiency(
    profile: &str,
    provider: &str,
    instrumentation_mode: &str,
    output_dir: &Path,
) -> Result<MemoryEfficiencyReceipt, String> {
    std::fs::create_dir_all(output_dir)
        .map_err(|error| format!("unable to create {}: {error}", output_dir.display()))?;
    let snapshots_dir = output_dir.join("snapshots");
    std::fs::create_dir_all(&snapshots_dir)
        .map_err(|error| format!("unable to create {}: {error}", snapshots_dir.display()))?;

    eprintln!("hydracache-loadgen: initializing memory profile cache");
    let instrumentation_mode = parse_instrumentation_mode(instrumentation_mode)?;
    let mut builder = HydraCache::local()
        .max_capacity(8 * 1024 * 1024)
        .memory_instrumentation_mode(instrumentation_mode);
    let counters_only = profile == COUNTERS_ONLY_PROFILE_073;
    if counters_only {
        builder = builder.instrumentation_lab_eviction_listener(false);
    }
    let cache = builder.build();
    eprintln!("hydracache-loadgen: initialized memory profile cache");
    let run_started = std::time::Instant::now();
    let mut timeline = Vec::with_capacity(MEMORY_PHASES.len());
    let collect_resources = matches!(profile, OVERHEAD_PROFILE_073 | COUNTERS_ONLY_PROFILE_073);
    let mut resource_series = Vec::with_capacity(MEMORY_PHASES.len());
    for (index, phase) in MEMORY_PHASES.into_iter().enumerate() {
        eprintln!("hydracache-loadgen: memory phase {}", phase.file_stem());
        let operations = phase.operations();
        let allocation = if collect_resources && operations > 0 {
            let (result, measurement) =
                measure_allocations(operations, run_phase_workload(&cache, phase)).await;
            result?;
            Some(measurement)
        } else {
            run_phase_workload(&cache, phase).await?;
            None
        };
        cache.diagnostics().await;
        let barrier = cache
            .memory_snapshot_barrier()
            .map_err(|error| format!("{} barrier failed: {error}", phase.file_stem()))?;
        let snapshot = cache
            .memory_footprint_snapshot(MemorySnapshotRequest::Exact {
                acknowledged_epoch: barrier.epoch,
            })
            .await
            .map_err(|error| format!("{} snapshot failed: {error}", phase.file_stem()))?;
        let snapshot_bytes = serde_json::to_vec_pretty(&snapshot)
            .map_err(|error| format!("snapshot serialization failed: {error}"))?;
        validate_schema(
            &snapshot,
            include_str!("../../../docs/testing/memory/0.71/memory-footprint-v1.schema.json"),
            "memory footprint",
        )?;
        let digest = format!(
            "sha256:{}",
            Sha256::digest(&snapshot_bytes)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        );
        write_bytes(
            &snapshots_dir.join(format!("{}.json", phase.file_stem())),
            &snapshot_bytes,
        )?;
        timeline.push(MemoryPhaseTimelineRecord {
            schema_version: "hydracache-memory-phase-v1".to_owned(),
            sequence: (index + 1) as u64,
            phase,
            epoch: snapshot.epoch,
            monotonic_ns: u64::try_from(run_started.elapsed().as_nanos()).unwrap_or(u64::MAX),
            owner_snapshot_digest: digest,
            telemetry_checkpoint: format!("memory.phase.{}", phase.file_stem()),
            provider_mark: format!("{provider}:{}", phase.file_stem()),
        });
        if collect_resources {
            resource_series.push(resource_record(index, phase, operations, allocation));
        }
    }
    validate_timeline(&timeline)?;
    for record in &timeline {
        validate_schema(
            record,
            include_str!("../../../docs/testing/memory/0.71/memory-phase-timeline-v1.schema.json"),
            "memory phase timeline",
        )?;
    }
    if instrumentation_mode != MemoryInstrumentationMode::Off && !counters_only {
        cache
            .reconcile_memory_footprint()
            .await
            .map_err(|error| format!("final exact reconciliation failed: {error}"))?;
    }

    let timeline_path = output_dir.join("phase-timeline.jsonl");
    let timeline_jsonl = timeline
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("timeline serialization failed: {error}"))?
        .join("\n")
        + "\n";
    write_bytes(&timeline_path, timeline_jsonl.as_bytes())?;
    let resource_series_path = if collect_resources {
        validate_resource_series(&resource_series)?;
        for record in &resource_series {
            validate_schema(
                record,
                include_str!(
                    "../../../docs/testing/performance/0.73/resource-phase-v1.schema.json"
                ),
                "0.73 resource phase",
            )?;
        }
        let path = output_dir.join("resource-series.jsonl");
        let bytes = resource_series
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("resource series serialization failed: {error}"))?
            .join("\n")
            + "\n";
        write_bytes(&path, bytes.as_bytes())?;
        Some(path)
    } else {
        None
    };
    let receipt = MemoryEfficiencyReceipt {
        schema_version: "hydracache-memory-efficiency-receipt-v1".to_owned(),
        profile: profile.to_owned(),
        provider: provider.to_owned(),
        instrumentation_mode,
        phase_count: timeline.len(),
        elapsed_ns: u64::try_from(run_started.elapsed().as_nanos()).unwrap_or(u64::MAX),
        timeline: timeline_path,
        resource_series: resource_series_path,
        diagnostic_variant: collect_resources.then(|| {
            if counters_only {
                "production_counters_without_eviction_listener"
            } else {
                "full"
            }
            .to_owned()
        }),
        counter_correctness_eligible: collect_resources.then_some(!counters_only),
        promotable: false,
    };
    let receipt_bytes = serde_json::to_vec_pretty(&receipt)
        .map_err(|error| format!("receipt serialization failed: {error}"))?;
    write_bytes(&output_dir.join("receipt.json"), &receipt_bytes)?;
    Ok(receipt)
}

impl MemoryPhase {
    fn operations(self) -> u64 {
        match self {
            Self::Cold | Self::PostIdle => 0,
            Self::Fill | Self::Steady => 128,
            Self::ExpireOrDelete | Self::Refill => 64,
            Self::Reset | Self::Shutdown => 1,
        }
    }
}

fn resource_record(
    index: usize,
    phase: MemoryPhase,
    operations: u64,
    allocation: Option<AllocationMeasurement>,
) -> MemoryPhaseResourceRecord {
    let process = process_memory();
    MemoryPhaseResourceRecord {
        schema_version: "hydracache-performance-resource-phase-073-v1".to_owned(),
        sequence: (index + 1) as u64,
        phase,
        operations,
        gross_allocated_bytes: allocation.map(|value| value.gross_allocated_bytes),
        gross_allocated_bytes_per_operation: allocation
            .map(|value| value.gross_allocated_bytes_per_operation),
        rss_bytes: process.as_ref().ok().map(|value| value.0),
        peak_rss_bytes: process.as_ref().ok().map(|value| value.1),
        process_memory_available: process.is_ok(),
        process_memory_unavailable_reason: process.err(),
    }
}

fn validate_resource_series(records: &[MemoryPhaseResourceRecord]) -> Result<(), String> {
    if records.len() != MEMORY_PHASES.len() {
        return Err("resource series must contain every memory phase".to_owned());
    }
    for (index, (record, phase)) in records.iter().zip(MEMORY_PHASES).enumerate() {
        if record.sequence != (index + 1) as u64 || record.phase != phase {
            return Err("resource series phase is missing or reordered".to_owned());
        }
        if record.operations == 0
            && (record.gross_allocated_bytes.is_some()
                || record.gross_allocated_bytes_per_operation.is_some())
        {
            return Err("idle resource phase must not invent per-operation allocation".to_owned());
        }
        if record.operations > 0
            && (record.gross_allocated_bytes.is_none()
                || record.gross_allocated_bytes_per_operation.is_none())
        {
            return Err("active resource phase is missing allocation accounting".to_owned());
        }
        if record.process_memory_available
            != (record.rss_bytes.is_some()
                && record.peak_rss_bytes.is_some()
                && record.process_memory_unavailable_reason.is_none())
        {
            return Err("resource series process-memory availability is inconsistent".to_owned());
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn process_memory() -> Result<(u64, u64), String> {
    let status = std::fs::read_to_string("/proc/self/status")
        .map_err(|error| format!("cannot read /proc/self/status: {error}"))?;
    let value = |name: &str| {
        status.lines().find_map(|line| {
            let rest = line.strip_prefix(name)?.trim();
            let kib = rest.split_whitespace().next()?.parse::<u64>().ok()?;
            kib.checked_mul(1024)
        })
    };
    Ok((
        value("VmRSS:").ok_or("VmRSS is unavailable")?,
        value("VmHWM:").ok_or("VmHWM is unavailable")?,
    ))
}

#[cfg(target_os = "windows")]
fn process_memory() -> Result<(u64, u64), String> {
    #[repr(C)]
    struct ProcessMemoryCounters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> isize;
    }
    #[link(name = "psapi")]
    unsafe extern "system" {
        fn GetProcessMemoryInfo(
            process: isize,
            counters: *mut ProcessMemoryCounters,
            size: u32,
        ) -> i32;
    }
    let mut counters = ProcessMemoryCounters {
        cb: std::mem::size_of::<ProcessMemoryCounters>() as u32,
        page_fault_count: 0,
        peak_working_set_size: 0,
        working_set_size: 0,
        quota_peak_paged_pool_usage: 0,
        quota_paged_pool_usage: 0,
        quota_peak_non_paged_pool_usage: 0,
        quota_non_paged_pool_usage: 0,
        pagefile_usage: 0,
        peak_pagefile_usage: 0,
    };
    // SAFETY: The pseudo-handle is valid for the current process and the
    // writable structure has the exact size reported in its `cb` field.
    let success = unsafe {
        GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut counters,
            std::mem::size_of::<ProcessMemoryCounters>() as u32,
        )
    };
    if success == 0 {
        return Err(format!(
            "GetProcessMemoryInfo failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok((
        counters.working_set_size as u64,
        counters.peak_working_set_size as u64,
    ))
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn process_memory() -> Result<(u64, u64), String> {
    Err("process RSS probe is unsupported on this platform".to_owned())
}

fn parse_instrumentation_mode(value: &str) -> Result<MemoryInstrumentationMode, String> {
    match value {
        "off" => Ok(MemoryInstrumentationMode::Off),
        "production" => Ok(MemoryInstrumentationMode::Production),
        "profile" => Ok(MemoryInstrumentationMode::Profile),
        _ => Err("instrumentation mode must be off, production, or profile".to_owned()),
    }
}

async fn run_phase_workload(cache: &HydraCache, phase: MemoryPhase) -> Result<(), String> {
    match phase {
        MemoryPhase::Cold | MemoryPhase::PostIdle => {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        MemoryPhase::Fill => {
            for index in 0..128_u64 {
                cache
                    .put(
                        &format!("memory:{index}"),
                        vec![index as u8; 256],
                        CacheOptions::new().tag(format!("bucket:{}", index % 8)),
                    )
                    .await
                    .map_err(|error| error.to_string())?;
            }
        }
        MemoryPhase::Steady => {
            for index in 0..128_u64 {
                let _: Option<Vec<u8>> = cache
                    .get(&format!("memory:{index}"))
                    .await
                    .map_err(|error| error.to_string())?;
            }
        }
        MemoryPhase::ExpireOrDelete => {
            for index in 0..64_u64 {
                cache
                    .remove(&format!("memory:{index}"))
                    .await
                    .map_err(|error| error.to_string())?;
            }
        }
        MemoryPhase::Reset | MemoryPhase::Shutdown => {
            cache.flush().await.map_err(|error| error.to_string())?;
        }
        MemoryPhase::Refill => {
            for index in 0..64_u64 {
                cache
                    .put(
                        &format!("refill:{index}"),
                        vec![index as u8; 128],
                        CacheOptions::new(),
                    )
                    .await
                    .map_err(|error| error.to_string())?;
            }
        }
    }
    Ok(())
}

pub fn validate_timeline(records: &[MemoryPhaseTimelineRecord]) -> Result<(), String> {
    if records.len() != MEMORY_PHASES.len() {
        return Err(format!(
            "memory timeline must contain exactly {} phases, found {}",
            MEMORY_PHASES.len(),
            records.len()
        ));
    }
    let mut previous_epoch = 0;
    for (index, (record, expected_phase)) in records.iter().zip(MEMORY_PHASES).enumerate() {
        let expected_sequence = (index + 1) as u64;
        if record.sequence != expected_sequence || record.phase != expected_phase {
            return Err(format!(
                "memory timeline phase {expected_sequence} is missing or reordered"
            ));
        }
        if record.epoch <= previous_epoch {
            return Err("memory timeline epochs must increase monotonically".to_owned());
        }
        if index > 0 && record.monotonic_ns <= records[index - 1].monotonic_ns {
            return Err("memory timeline timestamps must increase monotonically".to_owned());
        }
        if !record.owner_snapshot_digest.starts_with("sha256:")
            || record.owner_snapshot_digest.len() != 71
        {
            return Err("memory timeline owner digest is invalid".to_owned());
        }
        if record.telemetry_checkpoint.is_empty() || record.provider_mark.is_empty() {
            return Err("memory timeline checkpoint/mark is missing".to_owned());
        }
        previous_epoch = record.epoch;
    }
    Ok(())
}

pub fn snapshot_is_promotable(snapshot: &MemoryFootprintSnapshot) -> bool {
    snapshot.consistency == MemorySnapshotConsistency::Exact
        && snapshot.workload_epoch_acknowledged
        && !snapshot.observed_non_atomic
}

fn write_bytes(path: &Path, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(path, bytes)
        .map_err(|error| format!("unable to write {}: {error}", path.display()))
}

fn validate_schema<T: Serialize>(value: &T, schema_text: &str, label: &str) -> Result<(), String> {
    let schema: serde_json::Value = serde_json::from_str(schema_text)
        .map_err(|error| format!("invalid {label} schema: {error}"))?;
    let value = serde_json::to_value(value)
        .map_err(|error| format!("unable to serialize {label}: {error}"))?;
    let validator = jsonschema::validator_for(&schema)
        .map_err(|error| format!("unable to compile {label} schema: {error}"))?;
    validator
        .validate(&value)
        .map_err(|error| format!("{label} failed schema validation: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validator_rejects_missing_and_reordered_phases() {
        let records = MEMORY_PHASES
            .into_iter()
            .enumerate()
            .map(|(index, phase)| MemoryPhaseTimelineRecord {
                schema_version: "hydracache-memory-phase-v1".to_owned(),
                sequence: (index + 1) as u64,
                phase,
                epoch: (index + 1) as u64,
                monotonic_ns: (index + 1) as u64,
                owner_snapshot_digest: format!("sha256:{}", "0".repeat(64)),
                telemetry_checkpoint: "checkpoint".to_owned(),
                provider_mark: "system:mark".to_owned(),
            })
            .collect::<Vec<_>>();
        assert!(validate_timeline(&records).is_ok());
        assert!(validate_timeline(&records[..7]).is_err());
        let mut reordered = records;
        reordered.swap(1, 2);
        assert!(validate_timeline(&reordered).is_err());
    }

    #[test]
    fn resource_series_requires_allocations_only_for_active_phases() {
        let records = MEMORY_PHASES
            .into_iter()
            .enumerate()
            .map(|(index, phase)| {
                let operations = phase.operations();
                MemoryPhaseResourceRecord {
                    schema_version: "hydracache-performance-resource-phase-073-v1".to_owned(),
                    sequence: (index + 1) as u64,
                    phase,
                    operations,
                    gross_allocated_bytes: (operations > 0).then_some(operations * 10),
                    gross_allocated_bytes_per_operation: (operations > 0).then_some(10.0),
                    rss_bytes: Some(1),
                    peak_rss_bytes: Some(2),
                    process_memory_available: true,
                    process_memory_unavailable_reason: None,
                }
            })
            .collect::<Vec<_>>();
        assert!(validate_resource_series(&records).is_ok());
        let mut invalid = records;
        invalid[0].gross_allocated_bytes = Some(1);
        assert!(validate_resource_series(&invalid).is_err());
    }
}
