#![cfg(target_os = "linux")]

use hydracache_long_run_supervisor_074::host_execution::{
    HostExecutionClaim, ACTIVE_CAMPAIGN_NAME,
};
use hydracache_long_run_supervisor_074::measurement_loss::{
    drive_measurement_loss, MeasurementLossBackend, MeasurementLossError, MeasurementLossOutcome,
    MeasurementLossReason,
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

impl MeasurementLossBackend for FakeBackend {
    fn capture_and_stop_lost(
        &mut self,
        campaign_directory: &Path,
        cause: &hydracache_long_run_supervisor_074::measurement_loss::MeasurementLossCause,
        state: &DurableCampaignState,
    ) -> Result<(), String> {
        self.calls += 1;
        assert_eq!(
            campaign_directory
                .file_name()
                .and_then(|name| name.to_str()),
            Some(cause.campaign_id.as_str())
        );
        assert_eq!(cause.reason, MeasurementLossReason::ProcessIdentityDrift);
        assert_eq!(cause.observed_unix_seconds, 1_010);
        assert_eq!(cause.harness, state.harness.clone().unwrap());
        assert_eq!(cause.daemon, state.daemon.clone().unwrap());
        assert_eq!(cause.checkpoint, state.checkpoint);
        self.observed_retained_identity = state.campaign_state == CampaignState::FailedIncomplete
            && state.recorded_failure
            && state.harness.is_some()
            && state.daemon.is_some();
        if self.fail_next {
            self.fail_next = false;
            return Err("injected measurement-loss interruption".to_owned());
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
fn measurement_loss_commits_cause_stops_and_releases_after_completion() {
    let fixture = fixture();
    let mut backend = FakeBackend::default();
    assert_eq!(
        drive_measurement_loss(
            &fixture.claim,
            &fixture.lock,
            1_010,
            Some(MeasurementLossReason::ProcessIdentityDrift),
            Some(1_010),
            &mut backend,
        )
        .unwrap(),
        MeasurementLossOutcome::Completed { state_revision: 7 }
    );
    assert_eq!(backend.calls, 1);
    assert!(backend.observed_retained_identity);
    let completed = fixture.lock.read().unwrap();
    assert_eq!(completed.campaign_state, CampaignState::FailedIncomplete);
    assert!(completed.recorded_failure);
    assert!(completed.harness.is_none());
    assert!(completed.daemon.is_none());
    assert!(completed.checkpoint.is_none());
    assert!(completed.controller_lease.is_none());
    assert!(!fixture
        .claim
        .campaign_root()
        .join(ACTIVE_CAMPAIGN_NAME)
        .exists());
}

#[test]
fn interrupted_backend_recovers_exact_measurement_cause_once() {
    let fixture = fixture();
    let mut backend = FakeBackend {
        fail_next: true,
        ..FakeBackend::default()
    };
    assert!(matches!(
        drive_measurement_loss(
            &fixture.claim,
            &fixture.lock,
            1_010,
            Some(MeasurementLossReason::ProcessIdentityDrift),
            Some(1_010),
            &mut backend,
        ),
        Err(MeasurementLossError::Backend(_))
    ));
    let requested = fixture.lock.read().unwrap();
    assert_eq!(requested.revision, 6);
    assert_eq!(requested.campaign_state, CampaignState::FailedIncomplete);
    assert!(requested.recorded_failure);
    assert!(requested.harness.is_some());
    assert!(fixture
        .claim
        .campaign_root()
        .join(ACTIVE_CAMPAIGN_NAME)
        .exists());

    assert_eq!(
        drive_measurement_loss(
            &fixture.claim,
            &fixture.lock,
            1_011,
            None,
            None,
            &mut backend,
        )
        .unwrap(),
        MeasurementLossOutcome::Completed { state_revision: 7 }
    );
    assert_eq!(backend.calls, 2);
}

#[test]
fn completed_measurement_failure_recovers_marker_without_repeating_effect() {
    let fixture = fixture();
    let mut backend = FakeBackend::default();
    drive_measurement_loss(
        &fixture.claim,
        &fixture.lock,
        1_010,
        Some(MeasurementLossReason::ProcessIdentityDrift),
        Some(1_010),
        &mut backend,
    )
    .unwrap();
    assert_eq!(backend.calls, 1);

    let root = fixture.claim.campaign_root().to_owned();
    let campaign = fixture.claim.campaign_id().to_owned();
    drop(fixture.claim);
    let recreated = HostExecutionClaim::acquire(&root, &campaign).unwrap();
    assert_eq!(
        drive_measurement_loss(&recreated, &fixture.lock, 1_011, None, None, &mut backend,)
            .unwrap(),
        MeasurementLossOutcome::Completed { state_revision: 7 }
    );
    assert_eq!(backend.calls, 1);
}

#[test]
fn future_measurement_observation_fails_before_intent_or_backend() {
    let fixture = fixture();
    let mut backend = FakeBackend::default();
    assert!(matches!(
        drive_measurement_loss(
            &fixture.claim,
            &fixture.lock,
            1_010,
            Some(MeasurementLossReason::UnitAbsent),
            Some(1_011),
            &mut backend,
        ),
        Err(MeasurementLossError::Binding)
    ));
    assert_eq!(fixture.lock.read().unwrap().revision, 5);
    assert_eq!(backend.calls, 0);
}
