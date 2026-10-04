use hydracache_long_run_supervisor_074::checkpoint_evidence::{
    verify_checkpoint_evidence, CheckpointEvidenceError,
};
use hydracache_long_run_supervisor_074::state::{
    CampaignState, CheckpointHead, DurableCampaignState, FrozenIdentity,
};
use hydracache_long_run_supervisor_074::{
    append_record, build_record, ChainError, CheckpointPayload, Phase, ProcessIdentity, Role,
    GENESIS_HASH,
};
use std::collections::BTreeMap;
use std::fs;

fn process(pid: u32) -> ProcessIdentity {
    ProcessIdentity {
        boot_id: "boot-a".to_owned(),
        pid,
        start_ticks: u64::from(pid) * 100,
        process_group: 10,
        cgroup_path: "/hc/a".to_owned(),
        cgroup_inode: 50,
        unit_name: "hc-a.service".to_owned(),
    }
}

fn payload(role: Role, sequence: u64) -> CheckpointPayload {
    CheckpointPayload {
        campaign_id: "a".repeat(64),
        role,
        phase: Phase::Measured,
        phase_epoch: 1,
        monotonic_elapsed_ns: sequence * 1_000,
        wall_clock_utc: format!("2026-10-04T00:00:{sequence:02}Z"),
        completed: sequence,
        failed: 0,
        rejected: 0,
        timed_out: 0,
        outstanding: 0,
        telemetry_sequence: sequence,
        milestone: "measured".to_owned(),
        surface_counters: BTreeMap::from([("resp".to_owned(), sequence)]),
        resource_counters: BTreeMap::new(),
        owner_counters: BTreeMap::new(),
        harness: process(100),
        daemon: process(101),
    }
}

fn state(head: String) -> DurableCampaignState {
    let hash = |byte: char| byte.to_string().repeat(64);
    DurableCampaignState {
        revision: 0,
        campaign_state: CampaignState::I74Running,
        identity: FrozenIdentity {
            campaign_id: hash('a'),
            manifest_sha256: hash('b'),
            contract_sha256: hash('c'),
            scenario_sha256: hash('d'),
            tooling_sha256: hash('e'),
            source_bundle_sha256: hash('f'),
            binary_bundle_sha256: hash('1'),
            workload_bundle_sha256: hash('2'),
            machine_id: "machine-a".to_owned(),
            boot_id: "boot-a".to_owned(),
            host_receipt_sha256: hash('3'),
            mount_identity: "mount-a".to_owned(),
            isolated_cpuset: "2-7".to_owned(),
            housekeeping_cpuset: "0-1".to_owned(),
            command_environment_sha256: hash('4'),
            lease_id: "00000000-0000-4000-8000-000000000074".to_owned(),
            lease_deadline_unix_seconds: 2_000,
        },
        harness: Some(process(100)),
        daemon: Some(process(101)),
        checkpoint: Some(CheckpointHead {
            sequence: 2,
            record_sha256: head,
            useful_progress_unix_seconds: 1_000,
        }),
        controller_lease: None,
        recorded_failure: false,
        duplicate_executor: false,
        durable_history_corrupt: false,
    }
}

fn fixture() -> (tempfile::TempDir, DurableCampaignState) {
    let temporary = tempfile::tempdir().unwrap();
    let role = temporary.path().join("roles").join("i74");
    fs::create_dir_all(&role).unwrap();
    let journal = role.join("checkpoints.jsonl");
    let head = role.join("checkpoints.head");
    let first = build_record(1, GENESIS_HASH, payload(Role::I74, 1)).unwrap();
    append_record(&journal, &head, &first).unwrap();
    let second = build_record(2, &first.record_sha256, payload(Role::I74, 2)).unwrap();
    append_record(&journal, &head, &second).unwrap();
    (temporary, state(second.record_sha256))
}

#[test]
fn exact_chain_head_role_and_process_identity_are_required() {
    let (temporary, state) = fixture();
    let report = verify_checkpoint_evidence(temporary.path(), &state).unwrap();
    assert_eq!(report.records, 2);
    assert_eq!(Some(report.harness), state.harness);
    assert_eq!(Some(report.daemon), state.daemon);

    let mut drifted = state.clone();
    drifted.checkpoint.as_mut().unwrap().sequence += 1;
    assert!(matches!(
        verify_checkpoint_evidence(temporary.path(), &drifted),
        Err(CheckpointEvidenceError::Binding)
    ));
    drifted = state.clone();
    drifted.harness.as_mut().unwrap().start_ticks += 1;
    assert!(matches!(
        verify_checkpoint_evidence(temporary.path(), &drifted),
        Err(CheckpointEvidenceError::Binding)
    ));
}

#[test]
fn stale_head_and_non_live_state_fail_closed() {
    let (temporary, mut state) = fixture();
    fs::write(
        temporary.path().join("roles/i74").join("checkpoints.head"),
        format!("{}\n", "f".repeat(64)),
    )
    .unwrap();
    assert!(matches!(
        verify_checkpoint_evidence(temporary.path(), &state),
        Err(CheckpointEvidenceError::Head)
    ));
    state.campaign_state = CampaignState::I74Sealed;
    assert!(matches!(
        verify_checkpoint_evidence(temporary.path(), &state),
        Err(CheckpointEvidenceError::State)
    ));
}

#[test]
fn non_regular_checkpoint_journal_is_rejected_before_parsing() {
    let (temporary, state) = fixture();
    let journal = temporary.path().join("roles/i74/checkpoints.jsonl");
    fs::remove_file(&journal).unwrap();
    fs::create_dir(&journal).unwrap();
    assert!(matches!(
        verify_checkpoint_evidence(temporary.path(), &state),
        Err(CheckpointEvidenceError::Chain(ChainError::UnsafeJournal))
    ));
}
