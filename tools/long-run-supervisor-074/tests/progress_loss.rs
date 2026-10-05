#![cfg(target_os = "linux")]

use hydracache_long_run_supervisor_074::host_execution::{
    HostExecutionClaim, ACTIVE_CAMPAIGN_NAME,
};
use hydracache_long_run_supervisor_074::progress_loss::{
    drive_progress_loss, ProgressLossBackend, ProgressLossError, ProgressLossOutcome,
};
use hydracache_long_run_supervisor_074::state::{
    CampaignState, CheckpointHead, ControllerLease, DurableCampaignState, FrozenIdentity,
};
use hydracache_long_run_supervisor_074::state_store::CampaignLock;
use hydracache_long_run_supervisor_074::ProcessIdentity;
use std::path::Path;

fn hash(value: char) -> String {
    value.to_string().repeat(64)
}

fn state(campaign_id: &str) -> DurableCampaignState {
    let unit = format!("hydracache-performance-074-i74-{campaign_id}.service");
    let process = |pid| ProcessIdentity {
        boot_id: "boot-a".to_owned(),
        pid,
        start_ticks: u64::from(pid) * 10,
        process_group: 100,
        cgroup_path: format!("/system.slice/{unit}"),
        cgroup_inode: 500,
        unit_name: unit.clone(),
    };
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
        harness: Some(process(100)),
        daemon: Some(process(101)),
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
            expires_unix_seconds: 1_500,
        }),
        recorded_failure: false,
        duplicate_executor: false,
        durable_history_corrupt: false,
    }
}

#[derive(Default)]
struct FakeBackend {
    calls: usize,
    fail_next: bool,
    observed_retained_identity: bool,
}

impl ProgressLossBackend for FakeBackend {
    fn capture_and_stop_stalled(
        &mut self,
        campaign_directory: &Path,
        cause: &hydracache_long_run_supervisor_074::progress_loss::ProgressLossCause,
        state: &DurableCampaignState,
    ) -> Result<(), String> {
        self.calls += 1;
        assert_eq!(
            campaign_directory
                .file_name()
                .and_then(|name| name.to_str()),
            Some(cause.campaign_id.as_str())
        );
        assert_eq!(cause.checkpoint.sequence, 8);
        assert_eq!(cause.rejection_gap_seconds, 180);
        assert_eq!(cause.rejection_deadline_unix_seconds, 1_170);
        self.observed_retained_identity = state.campaign_state == CampaignState::FailedIncomplete
            && state.harness.is_some()
            && state.daemon.is_some()
            && state.checkpoint.is_some();
        if self.fail_next {
            self.fail_next = false;
            return Err("injected progress-loss interruption".to_owned());
        }
        Ok(())
    }
}

struct Fixture {
    _temporary: tempfile::TempDir,
    claim: HostExecutionClaim,
    lock: CampaignLock,
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
    }
}

#[test]
fn progress_deadline_commits_intent_stops_and_releases_only_after_completion() {
    let fixture = fixture();
    let mut backend = FakeBackend::default();
    assert_eq!(
        drive_progress_loss(
            &fixture.claim,
            &fixture.lock,
            1_170,
            fixture.lock.read().unwrap().checkpoint,
            180,
            &mut backend,
        )
        .unwrap(),
        ProgressLossOutcome::NotDue
    );
    assert_eq!(backend.calls, 0);

    assert_eq!(
        drive_progress_loss(
            &fixture.claim,
            &fixture.lock,
            1_171,
            fixture.lock.read().unwrap().checkpoint,
            180,
            &mut backend,
        )
        .unwrap(),
        ProgressLossOutcome::Completed { state_revision: 7 }
    );
    assert_eq!(backend.calls, 1);
    assert!(backend.observed_retained_identity);
    let completed = fixture.lock.read().unwrap();
    assert_eq!(completed.campaign_state, CampaignState::FailedIncomplete);
    assert!(completed.harness.is_none());
    assert!(completed.controller_lease.is_none());
    assert!(!fixture
        .claim
        .campaign_root()
        .join(ACTIVE_CAMPAIGN_NAME)
        .exists());
}

#[test]
fn backend_failure_retains_progress_cause_and_retry_completes_once() {
    let fixture = fixture();
    let mut backend = FakeBackend {
        fail_next: true,
        ..FakeBackend::default()
    };
    assert!(matches!(
        drive_progress_loss(
            &fixture.claim,
            &fixture.lock,
            1_171,
            fixture.lock.read().unwrap().checkpoint,
            180,
            &mut backend,
        ),
        Err(ProgressLossError::Backend(_))
    ));
    let requested = fixture.lock.read().unwrap();
    assert_eq!(requested.revision, 6);
    assert_eq!(requested.campaign_state, CampaignState::FailedIncomplete);
    assert!(requested.harness.is_some());
    assert!(fixture
        .claim
        .campaign_root()
        .join(ACTIVE_CAMPAIGN_NAME)
        .exists());

    assert_eq!(
        drive_progress_loss(
            &fixture.claim,
            &fixture.lock,
            1_172,
            fixture.lock.read().unwrap().checkpoint,
            180,
            &mut backend,
        )
        .unwrap(),
        ProgressLossOutcome::Completed { state_revision: 7 }
    );
    assert_eq!(backend.calls, 2);
}

#[test]
fn completed_progress_failure_recovers_marker_release_without_repeating_effect() {
    let fixture = fixture();
    let mut backend = FakeBackend::default();
    drive_progress_loss(
        &fixture.claim,
        &fixture.lock,
        1_171,
        fixture.lock.read().unwrap().checkpoint,
        180,
        &mut backend,
    )
    .unwrap();
    assert_eq!(backend.calls, 1);

    let root = fixture.claim.campaign_root().to_owned();
    let campaign = fixture.claim.campaign_id().to_owned();
    drop(fixture.claim);
    let recreated = HostExecutionClaim::acquire(&root, &campaign).unwrap();
    assert_eq!(
        drive_progress_loss(&recreated, &fixture.lock, 1_172, None, 180, &mut backend).unwrap(),
        ProgressLossOutcome::Completed { state_revision: 7 }
    );
    assert_eq!(backend.calls, 1);
}
