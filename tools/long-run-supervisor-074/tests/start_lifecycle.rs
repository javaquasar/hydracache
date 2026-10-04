use hydracache_long_run_supervisor_074::event::{
    append_lifecycle_event, request_sha256, verify_event_journal, LifecycleEvent, EVENT_HEAD_NAME,
    EVENT_JOURNAL_NAME,
};
use hydracache_long_run_supervisor_074::host_execution::HostExecutionClaim;
use hydracache_long_run_supervisor_074::protocol::{ControllerIdentity, Operation, Request};
use hydracache_long_run_supervisor_074::spawn::{
    prepare_spawn_intent, SpawnBackend, SpawnIntent, SpawnObservation,
};
use hydracache_long_run_supervisor_074::start_lifecycle::drive_i74_start;
use hydracache_long_run_supervisor_074::state::{
    CampaignState, DurableCampaignState, FrozenIdentity,
};
use hydracache_long_run_supervisor_074::state_store::CampaignLock;
use hydracache_long_run_supervisor_074::{ProcessIdentity, Role};
use std::fs;

#[derive(Default)]
struct FakeBackend {
    starts: usize,
    observations: usize,
    live: Option<SpawnObservation>,
}

impl SpawnBackend for FakeBackend {
    type Error = &'static str;

    fn start_once(&mut self, intent: &SpawnIntent) -> Result<SpawnObservation, Self::Error> {
        self.starts += 1;
        let exact = SpawnObservation::Exact {
            harness: process(&intent.unit_name, 100),
            daemon: process(&intent.unit_name, 101),
        };
        self.live = Some(exact.clone());
        Ok(exact)
    }

    fn observe(&mut self, _unit_name: &str) -> Result<SpawnObservation, Self::Error> {
        self.observations += 1;
        Ok(self.live.clone().unwrap_or(SpawnObservation::Absent))
    }
}

fn hash(value: char) -> String {
    value.to_string().repeat(64)
}

fn process(unit_name: &str, pid: u32) -> ProcessIdentity {
    ProcessIdentity {
        boot_id: "boot-a".to_owned(),
        pid,
        start_ticks: u64::from(pid) * 100,
        process_group: 100,
        cgroup_path: format!("/system.slice/{unit_name}"),
        cgroup_inode: u64::from(pid) * 10,
        unit_name: unit_name.to_owned(),
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
        mount_identity: "mount-a".to_owned(),
        isolated_cpuset: "2-7".to_owned(),
        housekeeping_cpuset: "0-1".to_owned(),
        command_environment_sha256: hash('4'),
        lease_id: "00000000-0000-4000-8000-000000000074".to_owned(),
        lease_deadline_unix_seconds: 2_000_000_000,
    }
}

fn request() -> Request {
    Request {
        schema_version: 1,
        request_id: "123e4567-e89b-42d3-a456-426614174000".to_owned(),
        operation: Operation::Start,
        campaign_id: hash('a'),
        expected_state_revision: 0,
        manifest_path: Some(format!("/staging/{}/campaign-start.json", hash('a'))),
        manifest_sha256: hash('b'),
        controller: ControllerIdentity {
            repository_id: 1,
            run_id: 2,
            run_attempt: 1,
            actor_id: 3,
            authorization_sha256: hash('5'),
        },
        abort_reason: None,
        approval_nonce_sha256: None,
    }
}

fn fixture() -> (tempfile::TempDir, HostExecutionClaim, CampaignLock) {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("campaigns");
    fs::create_dir(&root).unwrap();
    let claim = HostExecutionClaim::acquire(&root, &hash('a')).unwrap();
    fs::create_dir(root.join(hash('a'))).unwrap();
    let lock = CampaignLock::acquire(&root, &hash('a')).unwrap();
    (temporary, claim, lock)
}

fn prepared() -> DurableCampaignState {
    DurableCampaignState {
        revision: 0,
        campaign_state: CampaignState::Prepared,
        identity: identity(),
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
fn drives_prepared_start_to_running_once_and_replays_without_backend_calls() {
    let (_temporary, claim, lock) = fixture();
    let request = request();
    let mut backend = FakeBackend::default();
    let running = drive_i74_start(
        &claim,
        &lock,
        &request,
        identity(),
        hash('6'),
        1_000,
        &mut backend,
    )
    .unwrap();
    assert_eq!(running.revision, 2);
    assert_eq!(running.campaign_state, CampaignState::I74Running);
    assert!(running.checkpoint.is_none());
    assert_eq!(backend.starts, 1);
    assert_eq!(backend.observations, 0);
    assert_eq!(
        verify_event_journal(
            &lock.campaign_directory().join(EVENT_JOURNAL_NAME),
            &lock.campaign_directory().join(EVENT_HEAD_NAME),
        )
        .unwrap()
        .records,
        3
    );

    let mut replay_backend = FakeBackend::default();
    let replay = drive_i74_start(
        &claim,
        &lock,
        &request,
        identity(),
        hash('6'),
        1_001,
        &mut replay_backend,
    )
    .unwrap();
    assert_eq!(replay, running);
    assert_eq!(replay_backend.starts, 0);
    assert_eq!(replay_backend.observations, 0);
}

#[test]
fn recovers_event_ahead_of_missing_or_stale_state_before_spawning() {
    let (_temporary, claim, lock) = fixture();
    let request = request();
    let digest = request_sha256(&request).unwrap();
    let prepared = prepared();
    append_lifecycle_event(
        &lock.campaign_directory().join(EVENT_JOURNAL_NAME),
        &lock.campaign_directory().join(EVENT_HEAD_NAME),
        1_000,
        request.request_id.clone(),
        digest,
        LifecycleEvent::Prepared,
        prepared,
    )
    .unwrap();

    let mut backend = FakeBackend::default();
    let running = drive_i74_start(
        &claim,
        &lock,
        &request,
        identity(),
        hash('6'),
        1_001,
        &mut backend,
    )
    .unwrap();
    assert_eq!(running.campaign_state, CampaignState::I74Running);
    assert_eq!(backend.starts, 1);
    assert_eq!(lock.read().unwrap(), running);
}

#[test]
fn recovers_starting_event_ahead_of_prepared_snapshot_before_spawning() {
    let (_temporary, claim, lock) = fixture();
    let request = request();
    let digest = request_sha256(&request).unwrap();
    let prepared = prepared();
    append_lifecycle_event(
        &lock.campaign_directory().join(EVENT_JOURNAL_NAME),
        &lock.campaign_directory().join(EVENT_HEAD_NAME),
        1_000,
        request.request_id.clone(),
        digest.clone(),
        LifecycleEvent::Prepared,
        prepared.clone(),
    )
    .unwrap();
    lock.initialize(&prepared).unwrap();
    let mut starting = prepared;
    starting.revision = 1;
    starting.campaign_state = CampaignState::I74Starting;
    append_lifecycle_event(
        &lock.campaign_directory().join(EVENT_JOURNAL_NAME),
        &lock.campaign_directory().join(EVENT_HEAD_NAME),
        1_001,
        request.request_id.clone(),
        digest,
        LifecycleEvent::I74Starting,
        starting,
    )
    .unwrap();

    let mut backend = FakeBackend::default();
    let running = drive_i74_start(
        &claim,
        &lock,
        &request,
        identity(),
        hash('6'),
        1_002,
        &mut backend,
    )
    .unwrap();
    assert_eq!(running.campaign_state, CampaignState::I74Running);
    assert_eq!(backend.starts, 1);
    assert_eq!(lock.read().unwrap(), running);
}

#[test]
fn durable_intent_without_side_effect_fails_incomplete_and_never_starts_on_recovery() {
    let (_temporary, claim, lock) = fixture();
    let request = request();
    let digest = request_sha256(&request).unwrap();
    let intent = SpawnIntent::new(
        request.campaign_id.clone(),
        request.request_id.clone(),
        digest,
        request.manifest_sha256.clone(),
        hash('6'),
        Role::I74,
    )
    .unwrap();
    prepare_spawn_intent(lock.campaign_directory(), &intent).unwrap();

    let mut backend = FakeBackend::default();
    let failed = drive_i74_start(
        &claim,
        &lock,
        &request,
        identity(),
        hash('6'),
        1_000,
        &mut backend,
    )
    .unwrap();
    assert_eq!(failed.campaign_state, CampaignState::FailedIncomplete);
    assert!(failed.recorded_failure);
    assert_eq!(backend.starts, 0);
    assert_eq!(backend.observations, 1);
}
