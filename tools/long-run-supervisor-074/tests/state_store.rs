use hydracache_long_run_supervisor_074::state::{
    CampaignState, CheckpointHead, DurableCampaignState, FrozenIdentity,
};
use hydracache_long_run_supervisor_074::state_store::{
    read_state, CampaignLock, StateStoreError, PREVIOUS_STATE_NAME, STATE_NAME,
};
use hydracache_long_run_supervisor_074::ProcessIdentity;
use std::fs;

fn hash(byte: char) -> String {
    byte.to_string().repeat(64)
}

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

fn state(revision: u64) -> DurableCampaignState {
    DurableCampaignState {
        revision,
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
            mount_identity: "dev=1;opts=rw".to_owned(),
            isolated_cpuset: "2-7".to_owned(),
            housekeeping_cpuset: "0-1".to_owned(),
            command_environment_sha256: hash('4'),
            lease_id: "00000000-0000-4000-8000-000000000074".to_owned(),
            lease_deadline_unix_seconds: 10_000,
        },
        harness: process(100),
        daemon: process(101),
        checkpoint: CheckpointHead {
            sequence: 8,
            record_sha256: hash('5'),
            useful_progress_unix_seconds: 1_000,
        },
        controller_lease: None,
        recorded_failure: false,
        duplicate_executor: false,
        durable_history_corrupt: false,
    }
}

fn fixture() -> (tempfile::TempDir, std::path::PathBuf) {
    let temporary = tempfile::tempdir().unwrap();
    let campaign = temporary.path().join(hash('a'));
    fs::create_dir(&campaign).unwrap();
    (temporary, campaign)
}

#[test]
fn initializes_and_compare_and_swaps_with_previous_snapshot() {
    let (temporary, campaign) = fixture();
    let lock = CampaignLock::acquire(temporary.path(), &hash('a')).unwrap();
    lock.initialize(&state(0)).unwrap();
    let mut next = state(1);
    next.checkpoint.sequence = 9;
    lock.compare_and_swap(0, &next).unwrap();

    assert_eq!(lock.read().unwrap(), next);
    assert_eq!(
        read_state(&campaign.join(PREVIOUS_STATE_NAME)).unwrap(),
        state(0)
    );
}

#[test]
fn stale_revision_and_identity_drift_do_not_mutate_state() {
    let (temporary, campaign) = fixture();
    let lock = CampaignLock::acquire(temporary.path(), &hash('a')).unwrap();
    lock.initialize(&state(0)).unwrap();
    let before = fs::read(campaign.join(STATE_NAME)).unwrap();

    let error = lock.compare_and_swap(1, &state(2)).unwrap_err();
    assert!(matches!(error, StateStoreError::Revision { .. }));
    let mut drift = state(1);
    drift.identity.manifest_sha256 = hash('9');
    assert!(matches!(
        lock.compare_and_swap(0, &drift),
        Err(StateStoreError::Revision { .. })
    ));
    assert_eq!(fs::read(campaign.join(STATE_NAME)).unwrap(), before);
}

#[test]
fn lock_is_exclusive_and_path_traversal_is_rejected() {
    let (temporary, _campaign) = fixture();
    let _lock = CampaignLock::acquire(temporary.path(), &hash('a')).unwrap();
    assert!(matches!(
        CampaignLock::acquire(temporary.path(), &hash('a')),
        Err(StateStoreError::Busy)
    ));
    assert!(matches!(
        CampaignLock::acquire(temporary.path(), "../escape"),
        Err(StateStoreError::Path)
    ));
}

#[test]
fn malformed_and_oversized_state_fail_closed() {
    let (temporary, campaign) = fixture();
    let lock = CampaignLock::acquire(temporary.path(), &hash('a')).unwrap();
    fs::write(campaign.join(STATE_NAME), b"{}\n").unwrap();
    assert!(matches!(lock.read(), Err(StateStoreError::Document)));
    fs::write(campaign.join(STATE_NAME), vec![b'x'; 65_537]).unwrap();
    assert!(matches!(lock.read(), Err(StateStoreError::Document)));
}
