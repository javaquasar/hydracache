use hydracache_long_run_supervisor_074::{
    append_record, build_record, verify_journal, verify_journal_bytes, ChainError,
    CheckpointPayload, Phase, ProcessIdentity, Role, GENESIS_HASH,
};
use std::collections::BTreeMap;
use std::io::Write;

fn identity(pid: u32) -> ProcessIdentity {
    ProcessIdentity {
        boot_id: "boot-a".to_owned(),
        pid,
        start_ticks: pid as u64 * 10,
        process_group: 77,
        cgroup_path: "/hc074/campaign-a".to_owned(),
        cgroup_inode: 99,
        unit_name: "hydracache-performance-074-campaign-a.service".to_owned(),
    }
}

fn payload(elapsed: u64, phase: Phase) -> CheckpointPayload {
    CheckpointPayload {
        campaign_id: "a".repeat(64),
        role: Role::I74,
        phase,
        phase_epoch: 1,
        monotonic_elapsed_ns: elapsed,
        wall_clock_utc: format!("2026-10-04T00:00:{:02}Z", elapsed / 1_000),
        completed: elapsed,
        failed: 0,
        rejected: 0,
        timed_out: 0,
        outstanding: 0,
        telemetry_sequence: elapsed,
        milestone: "progress".to_owned(),
        surface_counters: BTreeMap::from([("resp".to_owned(), elapsed)]),
        resource_counters: BTreeMap::new(),
        owner_counters: BTreeMap::new(),
        harness: identity(100),
        daemon: identity(101),
    }
}

fn encoded_chain() -> Vec<u8> {
    let first = build_record(1, GENESIS_HASH, payload(1_000, Phase::Warmup)).unwrap();
    let second = build_record(2, &first.record_sha256, payload(2_000, Phase::Measured)).unwrap();
    let mut bytes = serde_json::to_vec(&first).unwrap();
    bytes.push(b'\n');
    bytes.extend(serde_json::to_vec(&second).unwrap());
    bytes.push(b'\n');
    bytes
}

#[test]
fn verifies_complete_chain_and_recovers_only_incomplete_tail() {
    let bytes = encoded_chain();
    let report = verify_journal_bytes(&bytes).unwrap();
    assert_eq!(report.records, 2);
    assert_eq!(report.recovered_incomplete_trailing_bytes, 0);
    assert_eq!(report.harness, identity(100));
    assert_eq!(report.daemon, identity(101));

    let mut torn = bytes;
    torn.extend(br#"{"schema_version":1"#);
    let report = verify_journal_bytes(&torn).unwrap();
    assert_eq!(report.records, 2);
    assert!(report.recovered_incomplete_trailing_bytes > 0);
}

#[test]
fn complete_json_without_terminal_newline_is_still_a_torn_record() {
    let first = build_record(1, GENESIS_HASH, payload(1_000, Phase::Warmup)).unwrap();
    let bytes = serde_json::to_vec(&first).unwrap();
    assert!(matches!(
        verify_journal_bytes(&bytes),
        Err(ChainError::TornTailRequiresRecovery { bytes: count }) if count == bytes.len()
    ));
}

#[test]
fn rejects_middle_corruption_sequence_gap_hash_break_and_identity_drift() {
    let bytes = encoded_chain();
    let mut corrupt = bytes.clone();
    let position = corrupt.iter().position(|byte| *byte == b'w').unwrap();
    corrupt[position] = b'x';
    assert!(verify_journal_bytes(&corrupt).is_err());

    let first_end = bytes.iter().position(|byte| *byte == b'\n').unwrap();
    let first: hydracache_long_run_supervisor_074::RecordEnvelope =
        serde_json::from_slice(&bytes[..first_end]).unwrap();
    let gap = build_record(3, &first.record_sha256, payload(3_000, Phase::Measured)).unwrap();
    let mut gap_bytes = bytes[..=first_end].to_vec();
    gap_bytes.extend(serde_json::to_vec(&gap).unwrap());
    gap_bytes.push(b'\n');
    assert!(matches!(
        verify_journal_bytes(&gap_bytes),
        Err(ChainError::Sequence { .. })
    ));

    let mut drifted = payload(2_000, Phase::Measured);
    drifted.harness.pid = 900;
    let drift = build_record(2, &first.record_sha256, drifted).unwrap();
    let mut drift_bytes = bytes[..=first_end].to_vec();
    drift_bytes.extend(serde_json::to_vec(&drift).unwrap());
    drift_bytes.push(b'\n');
    assert!(matches!(
        verify_journal_bytes(&drift_bytes),
        Err(ChainError::IdentityDrift { .. })
    ));
}

#[test]
fn append_syncs_chain_and_exact_head() {
    let directory = tempfile::tempdir().unwrap();
    let journal = directory.path().join("checkpoints.jsonl");
    let head = directory.path().join("checkpoints.head");
    let first = build_record(1, GENESIS_HASH, payload(1_000, Phase::Warmup)).unwrap();
    append_record(&journal, &head, &first).unwrap();
    let second = build_record(2, &first.record_sha256, payload(2_000, Phase::Measured)).unwrap();
    append_record(&journal, &head, &second).unwrap();
    assert_eq!(verify_journal(&journal).unwrap().records, 2);
    assert_eq!(
        std::fs::read_to_string(head).unwrap(),
        format!("{}\n", second.record_sha256)
    );
}

#[test]
fn refuses_replaying_a_sequence_or_reusing_a_stale_head() {
    let directory = tempfile::tempdir().unwrap();
    let journal = directory.path().join("checkpoints.jsonl");
    let head = directory.path().join("checkpoints.head");
    let first = build_record(1, GENESIS_HASH, payload(1_000, Phase::Warmup)).unwrap();
    append_record(&journal, &head, &first).unwrap();
    assert!(matches!(
        append_record(&journal, &head, &first),
        Err(ChainError::Sequence { .. })
    ));
}

#[test]
fn append_refuses_to_write_after_a_torn_tail_without_explicit_recovery() {
    let directory = tempfile::tempdir().unwrap();
    let journal = directory.path().join("checkpoints.jsonl");
    let head = directory.path().join("checkpoints.head");
    let first = build_record(1, GENESIS_HASH, payload(1_000, Phase::Warmup)).unwrap();
    append_record(&journal, &head, &first).unwrap();
    std::fs::OpenOptions::new()
        .append(true)
        .open(&journal)
        .unwrap()
        .write_all(b"{\"torn\":")
        .unwrap();
    let second = build_record(2, &first.record_sha256, payload(2_000, Phase::Measured)).unwrap();
    assert!(matches!(
        append_record(&journal, &head, &second),
        Err(ChainError::TornTailRequiresRecovery { .. })
    ));
}
