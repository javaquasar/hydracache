use hydracache_long_run_supervisor_074::event::{
    append_lifecycle_event, request_sha256, verify_event_journal, LifecycleEvent, EVENT_HEAD_NAME,
    EVENT_JOURNAL_NAME,
};
use hydracache_long_run_supervisor_074::host_execution::HostExecutionClaim;
use hydracache_long_run_supervisor_074::protocol::{ControllerIdentity, Operation, Request};
use hydracache_long_run_supervisor_074::spawn::{
    prepare_spawn_intent, SpawnBackend, SpawnIntent, SpawnObservation,
};
use hydracache_long_run_supervisor_074::start_lifecycle::{
    drive_c74_start, drive_c74_start_request, drive_i74_start, drive_i74_start_request,
};
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
    lose_start_response: bool,
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
        if self.lose_start_response {
            Err("response lost after side effect")
        } else {
            Ok(exact)
        }
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

fn c74_request(expected_state_revision: u64) -> Request {
    let mut request = request();
    request.request_id = "223e4567-e89b-42d3-a456-426614174000".to_owned();
    request.expected_state_revision = expected_state_revision;
    request.manifest_path = None;
    request
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

fn seed_i74_state(lock: &CampaignLock, sealed: bool) -> DurableCampaignState {
    let request = request();
    let digest = request_sha256(&request).unwrap();
    let journal = lock.campaign_directory().join(EVENT_JOURNAL_NAME);
    let head = lock.campaign_directory().join(EVENT_HEAD_NAME);
    let mut state = prepared();
    append_lifecycle_event(
        &journal,
        &head,
        900,
        request.request_id.clone(),
        digest.clone(),
        LifecycleEvent::Prepared,
        state.clone(),
    )
    .unwrap();
    lock.initialize(&state).unwrap();
    state.revision = 1;
    state.campaign_state = CampaignState::I74Starting;
    append_lifecycle_event(
        &journal,
        &head,
        901,
        request.request_id.clone(),
        digest.clone(),
        LifecycleEvent::I74Starting,
        state.clone(),
    )
    .unwrap();
    lock.compare_and_swap(0, &state).unwrap();
    state.revision = 2;
    state.campaign_state = CampaignState::I74Running;
    state.harness = Some(process(
        &format!("hydracache-performance-074-i74-{}.service", hash('a')),
        100,
    ));
    state.daemon = Some(process(
        &format!("hydracache-performance-074-i74-{}.service", hash('a')),
        101,
    ));
    append_lifecycle_event(
        &journal,
        &head,
        902,
        request.request_id.clone(),
        digest.clone(),
        LifecycleEvent::I74Started,
        state.clone(),
    )
    .unwrap();
    lock.compare_and_swap(1, &state).unwrap();
    state.revision = 3;
    state.campaign_state = CampaignState::I74Terminal;
    append_lifecycle_event(
        &journal,
        &head,
        903,
        request.request_id.clone(),
        digest.clone(),
        LifecycleEvent::I74Terminal,
        state.clone(),
    )
    .unwrap();
    lock.compare_and_swap(2, &state).unwrap();
    if sealed {
        state.revision = 4;
        state.campaign_state = CampaignState::I74Sealed;
        state.harness = None;
        state.daemon = None;
        state.checkpoint = None;
        append_lifecycle_event(
            &journal,
            &head,
            904,
            request.request_id,
            digest,
            LifecycleEvent::I74Sealed,
            state.clone(),
        )
        .unwrap();
        lock.compare_and_swap(3, &state).unwrap();
    }
    state
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

#[test]
fn accepted_start_response_is_durable_and_replayed_without_touching_backend() {
    let (_temporary, claim, lock) = fixture();
    let request = request();
    let mut backend = FakeBackend::default();
    let response = drive_i74_start_request(
        &claim,
        &lock,
        &request,
        identity(),
        hash('6'),
        1_000,
        &mut backend,
    )
    .unwrap();
    assert!(response.body.ok);
    assert_eq!(response.body.state_revision, 2);
    let report = verify_event_journal(
        &lock.campaign_directory().join(EVENT_JOURNAL_NAME),
        &lock.campaign_directory().join(EVENT_HEAD_NAME),
    )
    .unwrap();
    assert_eq!(report.records, 4);
    assert_eq!(report.replay_index.len(), 1);

    let mut replay_backend = FakeBackend::default();
    let replay = drive_i74_start_request(
        &claim,
        &lock,
        &request,
        identity(),
        hash('6'),
        1_001,
        &mut replay_backend,
    )
    .unwrap();
    assert_eq!(replay, response);
    assert_eq!(replay_backend.starts, 0);
    assert_eq!(replay_backend.observations, 0);
}

#[test]
fn lost_backend_response_is_adopted_before_recording_success_response() {
    let (_temporary, claim, lock) = fixture();
    let request = request();
    let mut backend = FakeBackend {
        lose_start_response: true,
        ..FakeBackend::default()
    };
    assert!(drive_i74_start_request(
        &claim,
        &lock,
        &request,
        identity(),
        hash('6'),
        1_000,
        &mut backend,
    )
    .is_err());
    assert_eq!(backend.starts, 1);
    assert_eq!(
        lock.read().unwrap().campaign_state,
        CampaignState::I74Starting
    );

    backend.lose_start_response = false;
    let recovered = drive_i74_start_request(
        &claim,
        &lock,
        &request,
        identity(),
        hash('6'),
        1_001,
        &mut backend,
    )
    .unwrap();
    assert!(recovered.body.ok);
    assert_eq!(backend.starts, 1);
    assert_eq!(backend.observations, 1);
    assert_eq!(
        lock.read().unwrap().campaign_state,
        CampaignState::I74Running
    );
}

#[test]
fn c74_starts_only_from_exact_i74_seal_and_has_independent_spawn_evidence() {
    let (_temporary, claim, lock) = fixture();
    let sealed = seed_i74_state(&lock, true);
    let request = c74_request(sealed.revision);
    let mut backend = FakeBackend::default();
    let running = drive_c74_start(
        &claim,
        &lock,
        &request,
        identity(),
        hash('6'),
        1_000,
        &mut backend,
    )
    .unwrap();
    assert_eq!(running.revision, 6);
    assert_eq!(running.campaign_state, CampaignState::C74Running);
    assert_eq!(backend.starts, 1);
    assert!(lock
        .campaign_directory()
        .join("c74-spawn-intent.json")
        .exists());
    assert!(lock
        .campaign_directory()
        .join("c74-spawn-result.json")
        .exists());
    assert!(!lock
        .campaign_directory()
        .join("i74-spawn-intent.json")
        .exists());

    let mut replay_backend = FakeBackend::default();
    let replay = drive_c74_start(
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
fn c74_rejects_unsealed_or_stale_predecessor_without_backend_effect() {
    let (_temporary, claim, lock) = fixture();
    let terminal = seed_i74_state(&lock, false);
    let mut backend = FakeBackend::default();
    assert!(drive_c74_start(
        &claim,
        &lock,
        &c74_request(terminal.revision),
        identity(),
        hash('6'),
        1_000,
        &mut backend,
    )
    .is_err());
    assert_eq!(backend.starts, 0);

    let (_temporary, claim, lock) = fixture();
    let sealed = seed_i74_state(&lock, true);
    assert!(drive_c74_start(
        &claim,
        &lock,
        &c74_request(sealed.revision - 1),
        identity(),
        hash('6'),
        1_000,
        &mut backend,
    )
    .is_err());
    assert_eq!(backend.starts, 0);
}

#[test]
fn c74_accepted_response_is_durable_and_lost_side_effect_response_is_adopted() {
    let (_temporary, claim, lock) = fixture();
    let sealed = seed_i74_state(&lock, true);
    let request = c74_request(sealed.revision);
    let mut backend = FakeBackend {
        lose_start_response: true,
        ..FakeBackend::default()
    };
    assert!(drive_c74_start_request(
        &claim,
        &lock,
        &request,
        identity(),
        hash('6'),
        1_000,
        &mut backend,
    )
    .is_err());
    assert_eq!(backend.starts, 1);
    assert_eq!(
        lock.read().unwrap().campaign_state,
        CampaignState::C74Starting
    );

    backend.lose_start_response = false;
    let response = drive_c74_start_request(
        &claim,
        &lock,
        &request,
        identity(),
        hash('6'),
        1_001,
        &mut backend,
    )
    .unwrap();
    assert!(response.body.ok);
    assert_eq!(response.body.state_revision, 6);
    assert_eq!(backend.starts, 1);
    assert_eq!(backend.observations, 1);

    let mut replay_backend = FakeBackend::default();
    let replay = drive_c74_start_request(
        &claim,
        &lock,
        &request,
        identity(),
        hash('6'),
        1_002,
        &mut replay_backend,
    )
    .unwrap();
    assert_eq!(replay, response);
    assert_eq!(replay_backend.starts, 0);
    assert_eq!(replay_backend.observations, 0);
}
