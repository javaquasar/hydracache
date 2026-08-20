use std::path::{Path, PathBuf};
use std::time::Duration;

use hydracache::{
    CacheOptions, HydraCache, MemoryFootprintSnapshot, MemoryInstrumentationMode,
    MemorySnapshotConsistency, MemorySnapshotRequest,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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
    pub phase_count: usize,
    pub timeline: PathBuf,
    pub promotable: bool,
}

pub async fn run_and_write_memory_efficiency(
    profile: &str,
    provider: &str,
    output_dir: &Path,
) -> Result<MemoryEfficiencyReceipt, String> {
    std::fs::create_dir_all(output_dir)
        .map_err(|error| format!("unable to create {}: {error}", output_dir.display()))?;
    let snapshots_dir = output_dir.join("snapshots");
    std::fs::create_dir_all(&snapshots_dir)
        .map_err(|error| format!("unable to create {}: {error}", snapshots_dir.display()))?;

    eprintln!("hydracache-loadgen: initializing memory profile cache");
    let cache = HydraCache::local()
        .max_capacity(8 * 1024 * 1024)
        .memory_instrumentation_mode(MemoryInstrumentationMode::Profile)
        .build();
    eprintln!("hydracache-loadgen: initialized memory profile cache");
    let run_started = std::time::Instant::now();
    let mut timeline = Vec::with_capacity(MEMORY_PHASES.len());
    for (index, phase) in MEMORY_PHASES.into_iter().enumerate() {
        eprintln!("hydracache-loadgen: memory phase {}", phase.file_stem());
        run_phase_workload(&cache, phase).await?;
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
    }
    validate_timeline(&timeline)?;
    for record in &timeline {
        validate_schema(
            record,
            include_str!("../../../docs/testing/memory/0.71/memory-phase-timeline-v1.schema.json"),
            "memory phase timeline",
        )?;
    }
    cache
        .reconcile_memory_footprint()
        .await
        .map_err(|error| format!("final exact reconciliation failed: {error}"))?;

    let timeline_path = output_dir.join("phase-timeline.jsonl");
    let timeline_jsonl = timeline
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("timeline serialization failed: {error}"))?
        .join("\n")
        + "\n";
    write_bytes(&timeline_path, timeline_jsonl.as_bytes())?;
    let receipt = MemoryEfficiencyReceipt {
        schema_version: "hydracache-memory-efficiency-receipt-v1".to_owned(),
        profile: profile.to_owned(),
        provider: provider.to_owned(),
        phase_count: timeline.len(),
        timeline: timeline_path,
        promotable: false,
    };
    let receipt_bytes = serde_json::to_vec_pretty(&receipt)
        .map_err(|error| format!("receipt serialization failed: {error}"))?;
    write_bytes(&output_dir.join("receipt.json"), &receipt_bytes)?;
    Ok(receipt)
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
    for (index, (record, expected_phase)) in
        records.iter().zip(MEMORY_PHASES.into_iter()).enumerate()
    {
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
}
