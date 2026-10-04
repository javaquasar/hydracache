#![cfg(target_os = "linux")]

use hydracache_long_run_supervisor_074::abort_lifecycle::{
    drive_abort_request, AbortBackend, AbortLifecycleError,
};
use hydracache_long_run_supervisor_074::host_execution::{
    HostExecutionClaim, ACTIVE_CAMPAIGN_NAME,
};
use hydracache_long_run_supervisor_074::protocol::{ControllerIdentity, Operation, Request};
use hydracache_long_run_supervisor_074::state::{
    CampaignState, CheckpointHead, ControllerLease, DurableCampaignState, FrozenIdentity,
};
use hydracache_long_run_supervisor_074::state_store::CampaignLock;
use hydracache_long_run_supervisor_074::ProcessIdentity;
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
