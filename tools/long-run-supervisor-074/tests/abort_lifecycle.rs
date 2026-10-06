#![cfg(target_os = "linux")]

use hydracache_long_run_supervisor_074::abort_lifecycle::{
    drive_abort_request, AbortBackend, AbortLifecycleError, QuarantineRecoveryCause,
};
use hydracache_long_run_supervisor_074::event::{
    append_lifecycle_event, LifecycleEvent, EVENT_HEAD_NAME, EVENT_JOURNAL_NAME,
};
use hydracache_long_run_supervisor_074::host_execution::{
    HostExecutionClaim, ACTIVE_CAMPAIGN_NAME,
};
use hydracache_long_run_supervisor_074::protocol::{ControllerIdentity, Operation, Request};
use hydracache_long_run_supervisor_074::spawn::{
    start_or_recover, SpawnBackend, SpawnIntent, SpawnMismatch, SpawnObservation,
};
use hydracache_long_run_supervisor_074::state::{
    CampaignState, CheckpointHead, ControllerLease, DurableCampaignState, FrozenIdentity,
};
use hydracache_long_run_supervisor_074::state_store::CampaignLock;
use hydracache_long_run_supervisor_074::{ProcessIdentity, Role};
use std::path::Path;

fn hash(value: char) -> String {
    value.to_string().repeat(64)
}

fn process(unit: &str, pid: u32) -> ProcessIdentity {
    ProcessIdentity {
        boot_id: "boot-a".to_owned(),
        pid,
        start_ticks: u64::from(pid) * 10,
        process_group: 100,
        cgroup_path: format!("/system.slice/{unit}"),
        cgroup_inode: 500,
        unit_name: unit.to_owned(),
    }
}

fn state(campaign_id: &str) -> DurableCampaignState {
    let unit = format!("hydracache-performance-074-i74-{campaign_id}.service");
    DurableCampaignState {
        revision: 5,
        campaign_state: CampaignState::I74Running,
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
            lease_deadline_unix_seconds: 2_000,
        },
        harness: Some(process(&unit, 100)),
        daemon: Some(process(&unit, 101)),
        checkpoint: Some(CheckpointHead {
            sequence: 8,
            record_sha256: hash('5'),
            useful_progress_unix_seconds: 990,
        }),
        controller_lease: Some(ControllerLease {
            holder_request_id: "123e4567-e89b-42d3-a456-426614174000".to_owned(),
            authorization_sha256: hash('6'),
            repository_id: 10,
            run_id: 20,
            actor_id: 30,
            expires_unix_seconds: 1_100,
        }),
        recorded_failure: false,
        duplicate_executor: false,
        durable_history_corrupt: false,
    }
}

fn request(campaign_id: &str) -> Request {
    Request {
        schema_version: 1,
        request_id: "223e4567-e89b-42d3-a456-426614174000".to_owned(),
        operation: Operation::Abort,
        campaign_id: campaign_id.to_owned(),
        expected_state_revision: 5,
        manifest_path: None,
        manifest_sha256: hash('b'),
        controller: ControllerIdentity {
            repository_id: 10,
            run_id: 20,
            run_attempt: 1,
            actor_id: 30,
            authorization_sha256: hash('7'),
        },
        abort_reason: Some("operator-request".to_owned()),
        approval_nonce_sha256: Some(hash('8')),
    }
}

struct FakeAbortBackend {
    calls: usize,
    fail_next: bool,
    observed_requested_state: bool,
    quarantine_calls: usize,
}

impl AbortBackend for FakeAbortBackend {
    fn capture_and_stop(
        &mut self,
        campaign_directory: &Path,
        request: &Request,
        state: &DurableCampaignState,
    ) -> Result<(), String> {
        self.calls += 1;
        assert_eq!(
            campaign_directory
                .file_name()
                .and_then(|name| name.to_str()),
            Some(request.campaign_id.as_str())
        );
        self.observed_requested_state = state.campaign_state == CampaignState::AbortedIncomplete
            && state.revision == request.expected_state_revision + 1
            && state.harness.is_some()
            && state.daemon.is_some()
            && state.controller_lease.is_some();
        if self.fail_next {
            self.fail_next = false;
            return Err("injected abort interruption".to_owned());
        }
        Ok(())
    }

    fn capture_quarantine_and_stop(
        &mut self,
        campaign_directory: &Path,
        request: &Request,
        state: &DurableCampaignState,
        cause: &QuarantineRecoveryCause,
    ) -> Result<(), String> {
        self.quarantine_calls += 1;
        assert_eq!(
            campaign_directory.file_name().unwrap(),
            request.campaign_id.as_str()
        );
        assert_eq!(cause.role, Role::C74);
        assert_eq!(cause.intent.campaign_id, request.campaign_id);
        assert_eq!(cause.result.mismatch, Some(SpawnMismatch::Identity));
        self.observed_requested_state = state.campaign_state == CampaignState::AbortedIncomplete
            && state.revision == request.expected_state_revision + 1
            && state.harness.is_none()
            && state.daemon.is_none()
            && state.recorded_failure
            && state.durable_history_corrupt;
        if self.fail_next {
            self.fail_next = false;
            return Err("injected quarantine abort interruption".to_owned());
        }
        Ok(())
    }
}

struct Fixture {
    _temporary: tempfile::TempDir,
    claim: HostExecutionClaim,
    lock: CampaignLock,
    request: Request,
}

fn fixture() -> Fixture {
    let temporary = tempfile::tempdir().unwrap();
    let campaign_id = hash('a');
    let root = temporary.path().join("campaigns");
    std::fs::create_dir(&root).unwrap();
    let claim = HostExecutionClaim::acquire(&root, &campaign_id).unwrap();
    std::fs::create_dir(root.join(&campaign_id)).unwrap();
    let lock = CampaignLock::acquire(&root, &campaign_id).unwrap();
    let final_state = state(&campaign_id);
    let mut initial = final_state.clone();
    initial.revision = 0;
    lock.initialize(&initial).unwrap();
    for revision in 1..=final_state.revision {
        let mut next = final_state.clone();
        next.revision = revision;
        lock.compare_and_swap(revision - 1, &next).unwrap();
    }
    Fixture {
        _temporary: temporary,
        claim,
        lock,
        request: request(&campaign_id),
    }
}

#[test]
fn abort_commits_intent_runs_effect_once_and_replays_exact_response() {
    let fixture = fixture();
    let mut backend = FakeAbortBackend {
        calls: 0,
        fail_next: false,
        observed_requested_state: false,
        quarantine_calls: 0,
    };
    let response = drive_abort_request(
        &fixture.claim,
        &fixture.lock,
        &fixture.request,
        1_000,
        &mut backend,
    )
    .unwrap();
    assert!(response.body.ok);
    assert_eq!(response.body.state_revision, 7);
    assert_eq!(backend.calls, 1);
    assert!(backend.observed_requested_state);
    let final_state = fixture.lock.read().unwrap();
    assert_eq!(final_state.campaign_state, CampaignState::AbortedIncomplete);
    assert!(final_state.harness.is_none());
    assert!(final_state.controller_lease.is_none());
    assert!(!fixture
        .claim
        .campaign_root()
        .join(ACTIVE_CAMPAIGN_NAME)
        .exists());

    let replay = drive_abort_request(
        &fixture.claim,
        &fixture.lock,
        &fixture.request,
        1_001,
        &mut backend,
    )
    .unwrap();
    assert_eq!(replay, response);
    assert_eq!(backend.calls, 1);
}

#[test]
fn backend_failure_retains_abort_intent_and_identical_retry_finishes() {
    let fixture = fixture();
    let mut backend = FakeAbortBackend {
        calls: 0,
        fail_next: true,
        observed_requested_state: false,
        quarantine_calls: 0,
    };
    assert!(matches!(
        drive_abort_request(
            &fixture.claim,
            &fixture.lock,
            &fixture.request,
            1_000,
            &mut backend
        ),
        Err(AbortLifecycleError::Backend(_))
    ));
    let requested = fixture.lock.read().unwrap();
    assert_eq!(requested.revision, 6);
    assert_eq!(requested.campaign_state, CampaignState::AbortedIncomplete);
    assert!(requested.harness.is_some());
    assert!(fixture
        .claim
        .campaign_root()
        .join(ACTIVE_CAMPAIGN_NAME)
        .exists());

    let response = drive_abort_request(
        &fixture.claim,
        &fixture.lock,
        &fixture.request,
        1_001,
        &mut backend,
    )
    .unwrap();
    assert!(response.body.ok);
    assert_eq!(response.body.state_revision, 7);
    assert_eq!(backend.calls, 2);
}

#[test]
fn stale_or_unauthorized_abort_does_not_record_intent_or_call_backend() {
    let fixture = fixture();
    let mut stale = fixture.request.clone();
    stale.expected_state_revision = 4;
    let mut backend = FakeAbortBackend {
        calls: 0,
        fail_next: false,
        observed_requested_state: false,
        quarantine_calls: 0,
    };
    assert!(matches!(
        drive_abort_request(&fixture.claim, &fixture.lock, &stale, 1_000, &mut backend),
        Err(AbortLifecycleError::Binding)
    ));
    assert_eq!(fixture.lock.read().unwrap().revision, 5);
    assert_eq!(backend.calls, 0);
    assert!(!fixture
        .lock
        .campaign_directory()
        .join("events.jsonl")
        .exists());
}

struct MismatchBackend(SpawnMismatch);

impl SpawnBackend for MismatchBackend {
    type Error = &'static str;

    fn start_once(&mut self, _intent: &SpawnIntent) -> Result<SpawnObservation, Self::Error> {
        Ok(SpawnObservation::Mismatch {
            reason: self.0,
            harness: None,
            daemon: None,
        })
    }

    fn observe(&mut self, _unit_name: &str) -> Result<SpawnObservation, Self::Error> {
        Err("unexpected observe")
    }
}

fn quarantine_fixture() -> Fixture {
    quarantine_fixture_with(SpawnMismatch::Identity)
}

fn quarantine_fixture_with(reason: SpawnMismatch) -> Fixture {
    let temporary = tempfile::tempdir().unwrap();
    let campaign_id = hash('a');
    let root = temporary.path().join("campaigns");
    std::fs::create_dir(&root).unwrap();
    let claim = HostExecutionClaim::acquire(&root, &campaign_id).unwrap();
    std::fs::create_dir(root.join(&campaign_id)).unwrap();
    let lock = CampaignLock::acquire(&root, &campaign_id).unwrap();
    let mut prepared = state(&campaign_id);
    prepared.revision = 0;
    prepared.campaign_state = CampaignState::Prepared;
    prepared.harness = None;
    prepared.daemon = None;
    prepared.checkpoint = None;
    prepared.controller_lease = None;
    lock.initialize(&prepared).unwrap();
    append_lifecycle_event(
        &lock.campaign_directory().join(EVENT_JOURNAL_NAME),
        &lock.campaign_directory().join(EVENT_HEAD_NAME),
        900,
        "prepare-cause".to_owned(),
        hash('9'),
        LifecycleEvent::Prepared,
        prepared.clone(),
    )
    .unwrap();
    let mut starting = prepared.clone();
    starting.revision = 1;
    starting.campaign_state = CampaignState::C74Starting;
    lock.compare_and_swap(0, &starting).unwrap();
    let start_request_id = "323e4567-e89b-42d3-a456-426614174000".to_owned();
    let start_request_sha256 = hash('0');
    append_lifecycle_event(
        &lock.campaign_directory().join(EVENT_JOURNAL_NAME),
        &lock.campaign_directory().join(EVENT_HEAD_NAME),
        901,
        start_request_id.clone(),
        start_request_sha256.clone(),
        LifecycleEvent::C74Starting,
        starting.clone(),
    )
    .unwrap();
    let intent = SpawnIntent::new(
        campaign_id.clone(),
        start_request_id.clone(),
        start_request_sha256.clone(),
        prepared.identity.manifest_sha256.clone(),
        hash('8'),
        Role::C74,
    )
    .unwrap();
    let result = start_or_recover(
        lock.campaign_directory(),
        &intent,
        &mut MismatchBackend(reason),
    )
    .unwrap();
    let mut quarantined = starting;
    quarantined.revision = 2;
    quarantined.campaign_state = CampaignState::CorruptQuarantined;
    quarantined.recorded_failure = true;
    quarantined.durable_history_corrupt = true;
    quarantined.duplicate_executor = reason == SpawnMismatch::MultipleExecutors;
    lock.compare_and_swap(1, &quarantined).unwrap();
    append_lifecycle_event(
        &lock.campaign_directory().join(EVENT_JOURNAL_NAME),
        &lock.campaign_directory().join(EVENT_HEAD_NAME),
        902,
        start_request_id,
        start_request_sha256,
        LifecycleEvent::C74SpawnMismatch,
        quarantined,
    )
    .unwrap();
    assert_eq!(result.mismatch, Some(reason));
    let mut abort = request(&campaign_id);
    abort.expected_state_revision = 2;
    abort.abort_reason = Some("guard-failure".to_owned());
    Fixture {
        _temporary: temporary,
        claim,
        lock,
        request: abort,
    }
}

#[test]
fn exact_identity_mismatch_quarantine_is_aborted_and_claim_is_released() {
    let fixture = quarantine_fixture();
    let mut backend = FakeAbortBackend {
        calls: 0,
        fail_next: false,
        observed_requested_state: false,
        quarantine_calls: 0,
    };
    let response = drive_abort_request(
        &fixture.claim,
        &fixture.lock,
        &fixture.request,
        1_000,
        &mut backend,
    )
    .unwrap();
    assert_eq!(response.body.state_revision, 4);
    assert_eq!(backend.calls, 0);
    assert_eq!(backend.quarantine_calls, 1);
    assert!(backend.observed_requested_state);
    let final_state = fixture.lock.read().unwrap();
    assert_eq!(final_state.campaign_state, CampaignState::AbortedIncomplete);
    assert!(final_state.recorded_failure);
    assert!(final_state.durable_history_corrupt);
    assert!(!fixture
        .claim
        .campaign_root()
        .join(ACTIVE_CAMPAIGN_NAME)
        .exists());
}

#[test]
fn quarantine_backend_failure_retains_intent_and_exact_retry_completes() {
    let fixture = quarantine_fixture();
    let mut backend = FakeAbortBackend {
        calls: 0,
        fail_next: true,
        observed_requested_state: false,
        quarantine_calls: 0,
    };
    assert!(matches!(
        drive_abort_request(
            &fixture.claim,
            &fixture.lock,
            &fixture.request,
            1_000,
            &mut backend
        ),
        Err(AbortLifecycleError::Backend(_))
    ));
    assert_eq!(fixture.lock.read().unwrap().revision, 3);
    assert!(fixture
        .claim
        .campaign_root()
        .join(ACTIVE_CAMPAIGN_NAME)
        .exists());
    let response = drive_abort_request(
        &fixture.claim,
        &fixture.lock,
        &fixture.request,
        1_001,
        &mut backend,
    )
    .unwrap();
    assert_eq!(response.body.state_revision, 4);
    assert_eq!(backend.quarantine_calls, 2);
}

#[test]
fn duplicate_executor_quarantine_remains_fail_closed() {
    let fixture = quarantine_fixture_with(SpawnMismatch::MultipleExecutors);
    let mut backend = FakeAbortBackend {
        calls: 0,
        fail_next: false,
        observed_requested_state: false,
        quarantine_calls: 0,
    };
    assert!(drive_abort_request(
        &fixture.claim,
        &fixture.lock,
        &fixture.request,
        1_000,
        &mut backend
    )
    .is_err());
    assert_eq!(backend.quarantine_calls, 0);
}
