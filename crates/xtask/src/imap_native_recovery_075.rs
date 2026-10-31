//! Independent fixed observations for controlled native listener recovery.
use serde::{Deserialize, Serialize};
use std::{error::Error, path::Path};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryProbe {
    pub schema: String,
    pub profile: String,
    pub artifact_sha256: String,
    pub accepted_writes: u32,
    pub callbacks_observed: u32,
    pub correlated_writes: u32,
    pub queue_capacity: u32,
    pub gap_before: String,
    pub old_generation: u64,
    pub new_generation: u64,
    pub baseline_key: String,
    pub baseline_value: String,
    pub baseline_digest: String,
    pub post_event_generation: u64,
    pub post_event_kind: String,
    pub post_event_key: String,
    pub post_event_old_value: String,
    pub post_event_value: String,
    pub state_after_recovery: String,
    pub state_after_close: String,
    pub removed_subscriptions: u32,
    pub client_alive_after_close: bool,
    pub map_value_after_close: String,
}
pub fn validate(probe: &RecoveryProbe) -> Result<(), Box<dyn Error>> {
    if probe.schema != "imap075-native-recovery-v1"
        || probe.profile != "controlled_unique_point_writers_queue_overflow"
        || probe.artifact_sha256 != crate::imap_pre074_075::HAZELCAST_SHA256
        || probe.accepted_writes != 8
        || probe.callbacks_observed != 8
        || probe.correlated_writes != 8
        || probe.queue_capacity != 2
        || probe.gap_before != "queue_overflow"
        || probe.old_generation != 1
        || probe.new_generation != 2
        || probe.baseline_key != "61"
        || probe.baseline_value != "07"
        || probe.baseline_digest
            != crate::imap_qualification_075::digest(&[0, 0, 0, 1, 97, 0, 0, 0, 1, 7, 0])
        || probe.post_event_generation != 2
        || probe.post_event_kind != "updated"
        || probe.post_event_key != "61"
        || probe.post_event_old_value != "07"
        || probe.post_event_value != "08"
        || probe.state_after_recovery != "healthy"
        || probe.state_after_close != "closed"
        || probe.removed_subscriptions != 2
        || !probe.client_alive_after_close
        || probe.map_value_after_close != "08"
    {
        return Err("controlled native recovery observations differ".into());
    }
    Ok(())
}
pub fn observe(root: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
    let bytes = crate::imap_native_probe_075::execute_native(
        root,
        "io.hydracache.imap.hazelcast.NativeListenerRecoveryProbeMain075",
    )?;
    validate(&serde_json::from_slice(&bytes)?)?;
    Ok(bytes)
}
pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if args != ["--release", "0.75"] {
        return Err("requires exactly --release 0.75".into());
    }
    println!(
        "{}",
        String::from_utf8(observe(&crate::doc_check::find_repo_root()?)?)?
    );
    Ok(())
}
