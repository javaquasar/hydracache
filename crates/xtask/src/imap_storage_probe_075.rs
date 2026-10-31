//! Characterize the existing durable v1 store. This does not persist an IMap outcome ledger.
use std::error::Error;
use std::path::Path;
use std::time::{Duration, Instant};

use hydracache::{
    ClusterEpoch, DurableValueStore, PartitionId, ReplicatedValueRecord, ReplicatedValueStore,
    DURABLE_VALUE_FORMAT_VERSION,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageProbe {
    pub scope: String,
    pub format_version: u32,
    pub observations: Vec<String>,
    pub unavailable_imap_guarantees: Vec<String>,
}

fn require(condition: bool, reason: &str) -> Result<(), Box<dyn Error>> {
    if condition {
        Ok(())
    } else {
        Err(reason.into())
    }
}

fn reopen(path: &Path, budget: u64) -> Result<DurableValueStore, Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match DurableValueStore::open_with_budget(path, budget) {
            Ok(store) => return Ok(store),
            Err(error)
                if error.to_string().contains("could not acquire lock")
                    && Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => return Err(error.into()),
        }
    }
}

/// All fixtures live in a fresh temporary child of `scratch`; no existing store is opened.
pub fn characterize(scratch: &Path) -> Result<StorageProbe, Box<dyn Error>> {
    let temp = tempfile::tempdir_in(scratch)?;
    let path = temp.path().join("records");
    let value = ReplicatedValueRecord::value(
        PartitionId::new(7),
        3,
        ClusterEpoch::new(9),
        vec![0, 128, 255],
    );
    let mut store = DurableValueStore::open_with_budget(&path, 3)?;
    store.upsert("development:opaque-key", value.clone())?;
    store.flush()?;
    drop(store);
    let mut store = reopen(&path, 3)?;
    require(
        store.get("development:opaque-key")? == Some(value.clone()),
        "value/version/epoch changed after reopen",
    )?;
    require(
        store.upsert("over-budget", value.clone()).is_err(),
        "budget failed to reject growth",
    )?;
    require(
        store.get("over-budget")?.is_none() && store.total_bytes()? == 3,
        "budget refusal changed records",
    )?;
    let tombstone = ReplicatedValueRecord::tombstone(
        PartitionId::new(7),
        4,
        ClusterEpoch::new(10),
        Some(ClusterEpoch::new(12)),
    );
    store.upsert("development:opaque-key", tombstone.clone())?;
    store.upsert("development:opaque-key", value)?;
    store.flush()?;
    drop(store);
    let store = reopen(&path, 3)?;
    require(
        store.get("development:opaque-key")? == Some(tombstone),
        "stale value resurrected a durable tombstone",
    )?;
    let raw = store
        .raw_record_for_test("development:opaque-key")?
        .ok_or("missing raw record")?;
    let mut corrupted = raw.clone();
    let last = corrupted.last_mut().ok_or("empty sealed record")?;
    *last ^= 1;
    store.put_raw_record_for_test("development:opaque-key", corrupted)?;
    require(
        store.get("development:opaque-key").is_err(),
        "corruption was accepted",
    )?;
    store.put_raw_record_for_test("development:opaque-key", &raw[..raw.len() / 2])?;
    require(
        store.get("development:opaque-key").is_err(),
        "truncated record was accepted",
    )?;
    drop(store);
    require(
        reopen(&path, 3).is_err(),
        "corrupted store reopened successfully",
    )?;

    let future = temp.path().join("future");
    DurableValueStore::write_format_marker_for_test(&future, DURABLE_VALUE_FORMAT_VERSION + 1)?;
    require(reopen(&future, 3).is_err(), "future format was accepted")?;
    let non_directory = temp.path().join("not-a-directory");
    std::fs::write(&non_directory, b"occupied")?;
    require(
        DurableValueStore::open(&non_directory).is_err(),
        "invalid storage path was accepted",
    )?;
    Ok(StorageProbe {
        scope: "existing_durable_v1_characterization".into(),
        format_version: DURABLE_VALUE_FORMAT_VERSION,
        observations: [
            "flush_reopen_value_version_epoch",
            "budget_refusal_without_mutation",
            "tombstone_survives_reopen_and_stale_merge",
            "checksum_rejection",
            "truncation_rejection",
            "corrupt_reopen_rejection",
            "future_format_rejection",
            "invalid_storage_path_rejection",
        ]
        .map(String::from)
        .to_vec(),
        unavailable_imap_guarantees: [
            "atomic_value_ttl_dedup_outcome_commit",
            "namespace_generation_and_map_incarnation",
            "absolute_imap_expiry",
            "retained_invocation_identity_and_result",
            "imap_crash_phase_recovery",
            "injected_short_write_and_fsync_failure",
        ]
        .map(String::from)
        .to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_observation_cannot_be_reported_as_passed() {
        assert!(require(false, "broken observation").is_err());
        assert!(require(true, "accepted observation").is_ok());
    }
}
