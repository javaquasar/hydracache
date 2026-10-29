use hydracache_long_run_supervisor_074::spawn::{
    apply_spawn_result, prepare_spawn_intent, start_or_recover, IntentDisposition, SpawnBackend,
    SpawnError, SpawnIntent, SpawnMismatch, SpawnObservation, SpawnResolution,
};
use hydracache_long_run_supervisor_074::state::{
    CampaignState, DurableCampaignState, FrozenIdentity,
};
use hydracache_long_run_supervisor_074::{ProcessIdentity, Role};
use std::convert::Infallible;
use std::fs;

#[derive(Default)]
struct FakeBackend {
    start_calls: usize,
    observe_calls: usize,
    live: Option<SpawnObservation>,
    lose_start_response: bool,
}

impl SpawnBackend for FakeBackend {
    type Error = &'static str;

    fn start_once(&mut self, intent: &SpawnIntent) -> Result<SpawnObservation, Self::Error> {
        self.start_calls += 1;
        let observation = exact(&intent.unit_name);
        self.live = Some(observation.clone());
        if self.lose_start_response {
            Err("response lost after side effect")
        } else {
            Ok(observation)
        }
    }

    fn observe(&mut self, _unit_name: &str) -> Result<SpawnObservation, Self::Error> {
        self.observe_calls += 1;
        Ok(self.live.clone().unwrap_or(SpawnObservation::Absent))
    }
}

fn hash(value: char) -> String {
    value.to_string().repeat(64)
}

fn intent(role: Role) -> SpawnIntent {
    SpawnIntent::new(
        hash('a'),
        "123e4567-e89b-42d3-a456-426614174000".to_owned(),
        hash('b'),
        hash('c'),
        hash('d'),
        role,
    )
    .unwrap()
}

fn process(unit_name: &str, pid: u32) -> ProcessIdentity {
    ProcessIdentity {
        boot_id: "boot".to_owned(),
        pid,
        start_ticks: u64::from(pid) * 10,
        process_group: i64::from(pid),
        cgroup_path: format!("/system.slice/{unit_name}"),
        cgroup_inode: u64::from(pid) * 100,
        unit_name: unit_name.to_owned(),
    }
}

fn exact(unit_name: &str) -> SpawnObservation {
    SpawnObservation::Exact {
        harness: process(unit_name, 10),
        daemon: process(unit_name, 11),
    }
}

fn starting_state(role: &Role) -> DurableCampaignState {
    DurableCampaignState {
        revision: 1,
        campaign_state: match role {
            Role::I74 => CampaignState::I74Starting,
            Role::C74 => CampaignState::C74Starting,
        },
        identity: FrozenIdentity {
            campaign_id: hash('a'),
            manifest_sha256: hash('c'),
            contract_sha256: hash('e'),
            scenario_sha256: hash('f'),
            tooling_sha256: hash('1'),
            source_bundle_sha256: hash('2'),
            binary_bundle_sha256: hash('3'),
            workload_bundle_sha256: hash('4'),
            machine_id: "machine-a".to_owned(),
            boot_id: "boot-a".to_owned(),
            host_receipt_sha256: hash('5'),
            mount_identity: "mount-a".to_owned(),
            isolated_cpuset: "2-7".to_owned(),
            housekeeping_cpuset: "0-1".to_owned(),
            command_environment_sha256: hash('6'),
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
fn normal_start_is_durable_and_replay_never_calls_backend_again() {
    let temporary = tempfile::tempdir().unwrap();
    let intent = intent(Role::I74);
    let mut backend = FakeBackend::default();
    let started = start_or_recover(temporary.path(), &intent, &mut backend).unwrap();
    assert_eq!(started.resolution, SpawnResolution::Started);
    assert!(started.exact_identities().is_some());
    assert_eq!(backend.start_calls, 1);
    assert_eq!(backend.observe_calls, 0);

    backend.live = Some(SpawnObservation::Absent);
    let replay = start_or_recover(temporary.path(), &intent, &mut backend).unwrap();
    assert_eq!(replay, started);
    assert_eq!(backend.start_calls, 1);
    assert_eq!(backend.observe_calls, 0);
}

#[test]
fn lost_start_response_is_recovered_by_observation_without_second_start() {
    let temporary = tempfile::tempdir().unwrap();
    let intent = intent(Role::I74);
    let mut backend = FakeBackend {
        lose_start_response: true,
        ..FakeBackend::default()
    };
    assert!(matches!(
        start_or_recover(temporary.path(), &intent, &mut backend),
        Err(SpawnError::Backend(_))
    ));
    assert_eq!(backend.start_calls, 1);

    backend.lose_start_response = false;
    let recovered = start_or_recover(temporary.path(), &intent, &mut backend).unwrap();
    assert_eq!(recovered.resolution, SpawnResolution::Adopted);
    assert_eq!(backend.start_calls, 1);
    assert_eq!(backend.observe_calls, 1);
}

#[test]
fn crash_before_side_effect_records_absence_instead_of_starting_on_retry() {
    let temporary = tempfile::tempdir().unwrap();
    let intent = intent(Role::I74);
    assert_eq!(
        prepare_spawn_intent(temporary.path(), &intent).unwrap(),
        IntentDisposition::Created
    );
    let mut backend = FakeBackend::default();
    let recovered = start_or_recover(temporary.path(), &intent, &mut backend).unwrap();
    assert_eq!(recovered.resolution, SpawnResolution::Absent);
    assert_eq!(backend.start_calls, 0);
    assert_eq!(backend.observe_calls, 1);
}

#[test]
fn crash_after_side_effect_adopts_exact_existing_unit() {
    let temporary = tempfile::tempdir().unwrap();
    let intent = intent(Role::I74);
    prepare_spawn_intent(temporary.path(), &intent).unwrap();
    let mut backend = FakeBackend {
        live: Some(exact(&intent.unit_name)),
        ..FakeBackend::default()
    };
    let recovered = start_or_recover(temporary.path(), &intent, &mut backend).unwrap();
    assert_eq!(recovered.resolution, SpawnResolution::Adopted);
    assert_eq!(backend.start_calls, 0);
    assert_eq!(backend.observe_calls, 1);
}

#[test]
fn mismatched_existing_unit_is_preserved_as_terminal_evidence() {
    let temporary = tempfile::tempdir().unwrap();
    let intent = intent(Role::I74);
    prepare_spawn_intent(temporary.path(), &intent).unwrap();
    let mut backend = FakeBackend {
        live: Some(SpawnObservation::Mismatch {
            reason: SpawnMismatch::Identity,
            harness: Some(process("reused.service", 10)),
            daemon: None,
        }),
        ..FakeBackend::default()
    };
    let result = start_or_recover(temporary.path(), &intent, &mut backend).unwrap();
    assert_eq!(result.resolution, SpawnResolution::Mismatch);
    assert_eq!(result.mismatch, Some(SpawnMismatch::Identity));
    assert_eq!(result.harness.unwrap().unit_name, "reused.service");
    assert_eq!(backend.start_calls, 0);
}

#[test]
fn spawn_outcomes_advance_state_without_placeholder_processes_or_checkpoints() {
    let started_root = tempfile::tempdir().unwrap();
    let i74 = intent(Role::I74);
    let mut started_backend = FakeBackend::default();
    let started = start_or_recover(started_root.path(), &i74, &mut started_backend).unwrap();
    let running = apply_spawn_result(&starting_state(&Role::I74), &Role::I74, &started).unwrap();
    assert_eq!(running.revision, 2);
    assert_eq!(running.campaign_state, CampaignState::I74Running);
    assert!(running.harness.is_some());
    assert!(running.daemon.is_some());
    assert!(running.checkpoint.is_none());

    let absent_root = tempfile::tempdir().unwrap();
    prepare_spawn_intent(absent_root.path(), &i74).unwrap();
    let mut absent_backend = FakeBackend::default();
    let absent = start_or_recover(absent_root.path(), &i74, &mut absent_backend).unwrap();
    let failed = apply_spawn_result(&starting_state(&Role::I74), &Role::I74, &absent).unwrap();
    assert_eq!(failed.campaign_state, CampaignState::FailedIncomplete);
    assert!(failed.recorded_failure);
    assert!(failed.harness.is_none());
    assert!(failed.checkpoint.is_none());

    let mismatch_root = tempfile::tempdir().unwrap();
    prepare_spawn_intent(mismatch_root.path(), &i74).unwrap();
    let mut mismatch_backend = FakeBackend {
        live: Some(SpawnObservation::Mismatch {
            reason: SpawnMismatch::MultipleExecutors,
            harness: Some(process("reused.service", 10)),
            daemon: Some(process("reused.service", 11)),
        }),
        ..FakeBackend::default()
    };
    let mismatch = start_or_recover(mismatch_root.path(), &i74, &mut mismatch_backend).unwrap();
    let quarantined =
        apply_spawn_result(&starting_state(&Role::I74), &Role::I74, &mismatch).unwrap();
    assert_eq!(
        quarantined.campaign_state,
        CampaignState::CorruptQuarantined
    );
    assert!(quarantined.recorded_failure);
    assert!(quarantined.durable_history_corrupt);
    assert!(quarantined.duplicate_executor);
    assert!(quarantined.harness.is_none());
}

#[test]
fn roles_have_independent_evidence_and_deterministic_units() {
    let temporary = tempfile::tempdir().unwrap();
    let i74 = intent(Role::I74);
    let c74 = intent(Role::C74);
    let mut backend = FakeBackend::default();
    let first = start_or_recover(temporary.path(), &i74, &mut backend).unwrap();
    let second = start_or_recover(temporary.path(), &c74, &mut backend).unwrap();
    assert_eq!(first.resolution, SpawnResolution::Started);
    assert_eq!(second.resolution, SpawnResolution::Started);
    assert_ne!(i74.unit_name, c74.unit_name);
    assert_eq!(backend.start_calls, 2);
}

#[test]
fn conflicting_or_corrupt_intent_fails_closed() {
    let temporary = tempfile::tempdir().unwrap();
    let intent = intent(Role::I74);
    prepare_spawn_intent(temporary.path(), &intent).unwrap();
    let mut conflict = intent.clone();
    conflict.request_sha256 = hash('e');
    assert!(matches!(
        prepare_spawn_intent(temporary.path(), &conflict),
        Err(SpawnError::IntentConflict)
    ));

    let head = temporary.path().join("i74-spawn-intent.sha256");
    fs::remove_file(&head).unwrap();
    fs::write(head, format!("{}\n", hash('f'))).unwrap();
    assert!(matches!(
        prepare_spawn_intent(temporary.path(), &intent),
        Err(SpawnError::Digest)
    ));
}

#[test]
fn backend_error_type_need_only_be_displayable() {
    struct Never;
    impl SpawnBackend for Never {
        type Error = Infallible;

        fn start_once(&mut self, _intent: &SpawnIntent) -> Result<SpawnObservation, Self::Error> {
            unreachable!()
        }

        fn observe(&mut self, _unit_name: &str) -> Result<SpawnObservation, Self::Error> {
            unreachable!()
        }
    }
    let _ = std::mem::size_of::<Never>();
}

#[cfg(unix)]
#[test]
fn symlink_and_hardlink_evidence_are_rejected() {
    use std::os::unix::fs::symlink;

    let hardlink_root = tempfile::tempdir().unwrap();
    let intent = intent(Role::I74);
    prepare_spawn_intent(hardlink_root.path(), &intent).unwrap();
    let document = hardlink_root.path().join("i74-spawn-intent.json");
    fs::hard_link(&document, hardlink_root.path().join("alias")).unwrap();
    assert!(matches!(
        prepare_spawn_intent(hardlink_root.path(), &intent),
        Err(SpawnError::Path)
    ));

    let symlink_root = tempfile::tempdir().unwrap();
    let target = symlink_root.path().join("target");
    fs::write(&target, b"{}").unwrap();
    symlink(&target, symlink_root.path().join("i74-spawn-intent.json")).unwrap();
    assert!(matches!(
        prepare_spawn_intent(symlink_root.path(), &intent),
        Err(SpawnError::Path)
    ));
}
