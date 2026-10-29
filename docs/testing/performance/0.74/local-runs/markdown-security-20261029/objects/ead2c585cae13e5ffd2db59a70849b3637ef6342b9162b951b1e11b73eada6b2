use hydracache_long_run_supervisor_074::host_execution::{
    ClaimDisposition, HostExecutionClaim, HostExecutionError, ACTIVE_CAMPAIGN_NAME,
    HOST_EXECUTION_LOCK_NAME,
};
use hydracache_long_run_supervisor_074::state::{
    CampaignState, DurableCampaignState, FrozenIdentity,
};
use std::fs;

fn sealed_state(campaign_id: &str) -> DurableCampaignState {
    let hash = |byte: char| byte.to_string().repeat(64);
    DurableCampaignState {
        revision: 8,
        campaign_state: CampaignState::CompleteSealed,
        identity: FrozenIdentity {
            campaign_id: campaign_id.to_owned(),
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
            lease_deadline_unix_seconds: 2_000_000_000,
        },
        harness: None,
        daemon: None,
        checkpoint: None,
        controller_lease: None,
        recorded_failure: false,
        duplicate_executor: false,
        durable_history_corrupt: false,
    }
}

#[test]
fn one_campaign_claim_survives_restart_and_blocks_every_other_campaign() {
    let temporary = tempfile::tempdir().unwrap();
    let first_id = "a".repeat(64);
    let second_id = "b".repeat(64);
    let first = HostExecutionClaim::acquire(temporary.path(), &first_id).unwrap();
    assert_eq!(first.disposition(), ClaimDisposition::Created);
    assert_eq!(first.campaign_id(), first_id);
    assert_eq!(
        first.campaign_root(),
        temporary.path().canonicalize().unwrap()
    );
    assert!(matches!(
        HostExecutionClaim::acquire(temporary.path(), &first_id),
        Err(HostExecutionError::Busy)
    ));
    drop(first);

    let recovered = HostExecutionClaim::acquire(temporary.path(), &first_id).unwrap();
    assert_eq!(recovered.disposition(), ClaimDisposition::Recovered);
    drop(recovered);
    assert!(matches!(
        HostExecutionClaim::acquire(temporary.path(), &second_id),
        Err(HostExecutionError::Conflict { campaign_id }) if campaign_id == first_id
    ));
    assert_eq!(
        fs::read_to_string(temporary.path().join(ACTIVE_CAMPAIGN_NAME)).unwrap(),
        format!("{first_id}\n")
    );
}

#[test]
fn recovery_never_creates_a_missing_active_campaign_marker() {
    let temporary = tempfile::tempdir().unwrap();
    assert!(HostExecutionClaim::recover_active(temporary.path())
        .unwrap()
        .is_none());
    assert!(matches!(
        HostExecutionClaim::recover(temporary.path(), &"a".repeat(64)),
        Err(HostExecutionError::Path)
    ));
    assert!(!temporary.path().join(ACTIVE_CAMPAIGN_NAME).exists());
}

#[test]
fn active_recovery_returns_the_validated_marker_identity_under_the_host_lock() {
    let temporary = tempfile::tempdir().unwrap();
    let campaign = "a".repeat(64);
    let created = HostExecutionClaim::acquire(temporary.path(), &campaign).unwrap();
    drop(created);

    let recovered = HostExecutionClaim::recover_active(temporary.path())
        .unwrap()
        .unwrap();
    assert_eq!(recovered.campaign_id(), campaign);
    assert_eq!(recovered.disposition(), ClaimDisposition::Recovered);
    assert!(matches!(
        HostExecutionClaim::recover_active(temporary.path()),
        Err(HostExecutionError::Busy)
    ));
}

#[test]
fn unsafe_marker_lock_and_campaign_identity_fail_closed() {
    let temporary = tempfile::tempdir().unwrap();
    assert!(matches!(
        HostExecutionClaim::acquire(temporary.path(), "../escape"),
        Err(HostExecutionError::Path)
    ));
    fs::write(temporary.path().join(ACTIVE_CAMPAIGN_NAME), b"invalid\n").unwrap();
    assert!(matches!(
        HostExecutionClaim::acquire(temporary.path(), &"a".repeat(64)),
        Err(HostExecutionError::Path)
    ));

    let other = tempfile::tempdir().unwrap();
    fs::create_dir(other.path().join(HOST_EXECUTION_LOCK_NAME)).unwrap();
    assert!(matches!(
        HostExecutionClaim::acquire(other.path(), &"a".repeat(64)),
        Err(HostExecutionError::Path)
    ));
}

#[cfg(unix)]
#[test]
fn symlink_and_hardlink_markers_are_rejected() {
    use std::os::unix::fs::symlink;

    let hardlink_root = tempfile::tempdir().unwrap();
    let marker = hardlink_root.path().join(ACTIVE_CAMPAIGN_NAME);
    fs::write(&marker, format!("{}\n", "a".repeat(64))).unwrap();
    fs::hard_link(&marker, hardlink_root.path().join("alias")).unwrap();
    assert!(matches!(
        HostExecutionClaim::acquire(hardlink_root.path(), &"a".repeat(64)),
        Err(HostExecutionError::Path)
    ));

    let symlink_root = tempfile::tempdir().unwrap();
    let target = symlink_root.path().join("target");
    fs::write(&target, format!("{}\n", "a".repeat(64))).unwrap();
    symlink(&target, symlink_root.path().join(ACTIVE_CAMPAIGN_NAME)).unwrap();
    assert!(matches!(
        HostExecutionClaim::acquire(symlink_root.path(), &"a".repeat(64)),
        Err(HostExecutionError::Path)
    ));
}

#[test]
fn host_marker_is_released_only_after_a_clean_complete_seal() {
    let temporary = tempfile::tempdir().unwrap();
    let campaign = "a".repeat(64);
    let claim = HostExecutionClaim::acquire(temporary.path(), &campaign).unwrap();
    let marker = temporary.path().join(ACTIVE_CAMPAIGN_NAME);
    let mut state = sealed_state(&campaign);

    state.campaign_state = CampaignState::I74Sealed;
    assert!(matches!(
        claim.release_after_complete_seal(&state),
        Err(HostExecutionError::Path)
    ));
    assert!(marker.exists());

    state.campaign_state = CampaignState::CompleteSealed;
    state.identity.campaign_id = "b".repeat(64);
    assert!(matches!(
        claim.release_after_complete_seal(&state),
        Err(HostExecutionError::Path)
    ));
    assert!(marker.exists());

    state.identity.campaign_id = campaign;
    claim.release_after_complete_seal(&state).unwrap();
    assert!(!marker.exists());
    drop(claim);

    let next = HostExecutionClaim::acquire(temporary.path(), &"b".repeat(64)).unwrap();
    assert_eq!(next.disposition(), ClaimDisposition::Created);
}
