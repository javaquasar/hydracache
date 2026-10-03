use hydracache_long_run_supervisor_074::state::{
    apply_attach, evaluate_attach, transition, AttachFailure, AttachRequest, CampaignState,
    CheckpointHead, DurableCampaignState, FrozenIdentity, ReplayDecision, ReplayMap, Transition,
};
use hydracache_long_run_supervisor_074::ProcessIdentity;

fn hash(byte: char) -> String {
    byte.to_string().repeat(64)
}

fn process(pid: u32) -> ProcessIdentity {
    ProcessIdentity {
        boot_id: "boot-a".to_owned(),
        pid,
        start_ticks: pid as u64 * 100,
        process_group: 10,
        cgroup_path: "/hc/a".to_owned(),
        cgroup_inode: 50,
        unit_name: "hc-a.service".to_owned(),
    }
}

fn identity() -> FrozenIdentity {
    FrozenIdentity {
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
        lease_id: "lease-a".to_owned(),
        lease_deadline_unix_seconds: 10_000,
    }
}

fn state() -> DurableCampaignState {
    DurableCampaignState {
        revision: 7,
        campaign_state: CampaignState::I74Running,
        identity: identity(),
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

fn request(state: &DurableCampaignState) -> AttachRequest {
    AttachRequest {
        request_id: "request-a".to_owned(),
        request_sha256: hash('6'),
        expected_revision: state.revision,
        authorization_sha256: hash('7'),
        identity: state.identity.clone(),
        harness: state.harness.clone(),
        daemon: state.daemon.clone(),
        checkpoint: state.checkpoint.clone(),
        now_unix_seconds: 1_010,
        requested_controller_lease_seconds: 60,
    }
}

#[test]
fn exact_attach_only_changes_controller_revision_and_lease() {
    let state = state();
    let request = request(&state);
    let next = apply_attach(&state, &request, 180).unwrap();
    assert_eq!(next.revision, state.revision + 1);
    assert_eq!(next.campaign_state, state.campaign_state);
    assert_eq!(next.harness, state.harness);
    assert_eq!(next.daemon, state.daemon);
    assert_eq!(next.checkpoint, state.checkpoint);
    assert_eq!(
        next.controller_lease.unwrap().holder_request_id,
        request.request_id
    );
}

#[test]
fn attach_fails_closed_for_host_process_checkpoint_lease_and_failure_drift() {
    let state = state();
    let mut request = request(&state);
    request.expected_revision += 1;
    request.identity.boot_id = "boot-b".to_owned();
    request.harness.start_ticks += 1;
    request.checkpoint.sequence += 1;
    request.now_unix_seconds = 20_000;
    let mut state = state;
    state.recorded_failure = true;
    state.duplicate_executor = true;
    state.durable_history_corrupt = true;
    let decision = evaluate_attach(&state, &request, 180);
    assert!(!decision.admitted);
    for failure in [
        AttachFailure::StaleRevision,
        AttachFailure::CampaignIdentityDrift,
        AttachFailure::HostOrBootDrift,
        AttachFailure::ProcessIdentityDrift,
        AttachFailure::CheckpointDrift,
        AttachFailure::CheckpointStale,
        AttachFailure::ProductLeaseExpired,
        AttachFailure::RecordedFailure,
        AttachFailure::DuplicateExecutor,
        AttachFailure::CorruptHistory,
    ] {
        assert!(decision.failures.contains(&failure), "missing {failure:?}");
    }
}

#[test]
fn state_machine_cannot_skip_roles_or_restart_terminal_work() {
    let state = transition(CampaignState::Prepared, Transition::StartI74).unwrap();
    let state = transition(state, Transition::MarkI74Running).unwrap();
    let state = transition(state, Transition::MarkI74Terminal).unwrap();
    let state = transition(state, Transition::SealI74).unwrap();
    let state = transition(state, Transition::StartC74).unwrap();
    let state = transition(state, Transition::MarkC74Running).unwrap();
    let state = transition(state, Transition::MarkC74Terminal).unwrap();
    assert_eq!(
        transition(state, Transition::SealComplete).unwrap(),
        CampaignState::CompleteSealed
    );
    assert!(transition(CampaignState::Prepared, Transition::StartC74).is_err());
    assert!(transition(CampaignState::I74Terminal, Transition::StartI74).is_err());
    assert!(transition(CampaignState::CompleteSealed, Transition::StartI74).is_err());
}

#[test]
fn request_replay_is_idempotent_only_for_identical_digest() {
    let mut replay = ReplayMap::default();
    assert_eq!(replay.observe("request-a", &hash('a')), ReplayDecision::New);
    assert_eq!(
        replay.observe("request-a", &hash('a')),
        ReplayDecision::Idempotent
    );
    assert_eq!(
        replay.observe("request-a", &hash('b')),
        ReplayDecision::DigestConflict
    );
}

#[test]
fn lease_expiry_is_terminal_and_never_restarts() {
    let expired = transition(CampaignState::C74Running, Transition::ExpireLease).unwrap();
    assert_eq!(expired, CampaignState::LeaseExpiredIncomplete);
    assert!(transition(expired, Transition::StartC74).is_err());
}
