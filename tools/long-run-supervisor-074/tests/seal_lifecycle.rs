#![cfg(target_os = "linux")]

use hydracache_long_run_supervisor_074::archive::ArchiveLimits;
use hydracache_long_run_supervisor_074::artifact::{
    GuardEvidenceInput, PacketLimits, PacketPlan, PacketResult, RoleEvidenceInput,
};
use hydracache_long_run_supervisor_074::event::{
    append_lifecycle_event, append_or_replay, request_sha256, verify_event_journal, EventOutcome,
    LifecycleEvent, EVENT_HEAD_NAME, EVENT_JOURNAL_NAME,
};
use hydracache_long_run_supervisor_074::host_execution::{
    HostExecutionClaim, ACTIVE_CAMPAIGN_NAME,
};
use hydracache_long_run_supervisor_074::protocol::{
    sign_response, ControllerIdentity, Operation, Request, ResponseBody,
};
use hydracache_long_run_supervisor_074::seal_artifact::SealResponseResult;
use hydracache_long_run_supervisor_074::seal_lifecycle::drive_seal_request;
use hydracache_long_run_supervisor_074::state::{
    CampaignState, CheckpointHead, ControllerLease, DurableCampaignState, FrozenIdentity,
};
use hydracache_long_run_supervisor_074::state_store::CampaignLock;
use hydracache_long_run_supervisor_074::systemd_unit::UnitSnapshot;
use hydracache_long_run_supervisor_074::{
    append_record, build_record, CheckpointPayload, Phase, ProcessIdentity, Role, GENESIS_HASH,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

fn hash(value: char) -> String {
    value.to_string().repeat(64)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn process(unit: &str, pid: u32) -> ProcessIdentity {
    ProcessIdentity {
        boot_id: "boot-a".to_owned(),
        pid,
        start_ticks: u64::from(pid) * 100,
        process_group: 10,
        cgroup_path: format!("/system.slice/{unit}"),
        cgroup_inode: 50 + u64::from(pid),
        unit_name: unit.to_owned(),
    }
}

fn payload(
    campaign_id: &str,
    role: Role,
    unit: &str,
    harness_pid: u32,
    sequence: u64,
) -> CheckpointPayload {
    CheckpointPayload {
        campaign_id: campaign_id.to_owned(),
        role,
        phase: if sequence == 2 {
            Phase::Terminal
        } else {
            Phase::Measured
        },
        phase_epoch: 1,
        monotonic_elapsed_ns: sequence * 1_000,
        wall_clock_utc: format!("2026-10-04T00:00:{sequence:02}Z"),
        completed: sequence,
        failed: 0,
        rejected: 0,
        timed_out: 0,
        outstanding: 0,
        telemetry_sequence: sequence,
        milestone: if sequence == 2 {
            "terminal"
        } else {
            "measured"
        }
        .to_owned(),
        surface_counters: BTreeMap::from([("resp".to_owned(), sequence)]),
        resource_counters: BTreeMap::new(),
        owner_counters: BTreeMap::new(),
        harness: process(unit, harness_pid),
        daemon: process(unit, harness_pid + 1),
    }
}

struct Fixture {
    _temporary: tempfile::TempDir,
    claim: HostExecutionClaim,
    lock: CampaignLock,
    request: Request,
    unit: UnitSnapshot,
    plan: PacketPlan,
    seal_root: PathBuf,
}

fn fixture() -> Fixture {
    let temporary = tempfile::tempdir().unwrap();
    let campaign_id = hash('a');
    let root = temporary.path().join("campaigns");
    fs::create_dir(&root).unwrap();
    let claim = HostExecutionClaim::acquire(&root, &campaign_id).unwrap();
    let campaign = root.join(&campaign_id);
    fs::create_dir(&campaign).unwrap();
    let lock = CampaignLock::acquire(&root, &campaign_id).unwrap();
    let role = campaign.join("roles/i74");
    fs::create_dir_all(role.join("guards")).unwrap();

    let manifest = serde_json::to_vec(&serde_json::json!({
        "campaign_id": campaign_id,
        "release": "0.74"
    }))
    .unwrap();
    let manifest_sha256 = hex(&Sha256::digest(&manifest));
    fs::write(campaign.join("campaign-start.json"), &manifest).unwrap();
    fs::write(
        role.join("guards/semantic.json"),
        serde_json::to_vec(&serde_json::json!({"id": "semantic", "passed": true})).unwrap(),
    )
    .unwrap();

    let unit_name = format!("hydracache-performance-074-i74-{campaign_id}.service");
    let journal = role.join("checkpoints.jsonl");
    let head = role.join("checkpoints.head");
    let first = build_record(
        1,
        GENESIS_HASH,
        payload(&campaign_id, Role::I74, &unit_name, 100, 1),
    )
    .unwrap();
    append_record(&journal, &head, &first).unwrap();
    let second = build_record(
        2,
        &first.record_sha256,
        payload(&campaign_id, Role::I74, &unit_name, 100, 2),
    )
    .unwrap();
    append_record(&journal, &head, &second).unwrap();

    let identity = FrozenIdentity {
        campaign_id: campaign_id.clone(),
        manifest_sha256: manifest_sha256.clone(),
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
    };
    let start_id = "123e4567-e89b-42d3-a456-426614174000";
    let start_digest = hash('5');
    let events = campaign.join(EVENT_JOURNAL_NAME);
    let event_head = campaign.join(EVENT_HEAD_NAME);
    let mut state = DurableCampaignState {
        revision: 0,
        campaign_state: CampaignState::Prepared,
        identity,
        harness: None,
        daemon: None,
        checkpoint: None,
        controller_lease: None,
        recorded_failure: false,
        duplicate_executor: false,
        durable_history_corrupt: false,
    };
    append_lifecycle_event(
        &events,
        &event_head,
        900,
        start_id.to_owned(),
        start_digest.clone(),
        LifecycleEvent::Prepared,
        state.clone(),
    )
    .unwrap();
    lock.initialize(&state).unwrap();
    state.revision = 1;
    state.campaign_state = CampaignState::I74Starting;
    append_lifecycle_event(
        &events,
        &event_head,
        901,
        start_id.to_owned(),
        start_digest.clone(),
        LifecycleEvent::I74Starting,
        state.clone(),
    )
    .unwrap();
    lock.compare_and_swap(0, &state).unwrap();
    state.revision = 2;
    state.campaign_state = CampaignState::I74Running;
    state.harness = Some(process(&unit_name, 100));
    state.daemon = Some(process(&unit_name, 101));
    append_lifecycle_event(
        &events,
        &event_head,
        902,
        start_id.to_owned(),
        start_digest,
        LifecycleEvent::I74Started,
        state.clone(),
    )
    .unwrap();
    lock.compare_and_swap(1, &state).unwrap();

    let attach_request = Request {
        schema_version: 1,
        request_id: "023e4567-e89b-42d3-a456-426614174000".to_owned(),
        operation: Operation::Attach,
        campaign_id: campaign_id.clone(),
        expected_state_revision: 2,
        manifest_path: None,
        manifest_sha256: manifest_sha256.clone(),
        controller: ControllerIdentity {
            repository_id: 1,
            run_id: 2,
            run_attempt: 1,
            actor_id: 3,
            authorization_sha256: hash('6'),
        },
        abort_reason: None,
        approval_nonce_sha256: None,
    };
    state.revision = 3;
    state.checkpoint = Some(CheckpointHead {
        sequence: 2,
        record_sha256: second.record_sha256,
        useful_progress_unix_seconds: 999,
    });
    state.controller_lease = Some(ControllerLease {
        holder_request_id: attach_request.request_id.clone(),
        authorization_sha256: hash('6'),
        repository_id: 1,
        run_id: 2,
        actor_id: 3,
        expires_unix_seconds: 1_500,
    });
    let attach_response = sign_response(ResponseBody {
        schema_version: 1,
        request_id: attach_request.request_id.clone(),
        campaign_id: campaign_id.clone(),
        ok: true,
        state_revision: 3,
        server_time_unix_seconds: 903,
        result: Some(serde_json::to_value(&state).unwrap()),
        error_code: None,
    })
    .unwrap();
    append_or_replay(
        &events,
        &event_head,
        903,
        attach_request,
        EventOutcome::Accepted,
        attach_response,
        Some(state.clone()),
    )
    .unwrap();
    lock.compare_and_swap(2, &state).unwrap();

    let request = Request {
        schema_version: 1,
        request_id: "223e4567-e89b-42d3-a456-426614174000".to_owned(),
        operation: Operation::Seal,
        campaign_id: campaign_id.clone(),
        expected_state_revision: 3,
        manifest_path: None,
        manifest_sha256: manifest_sha256.clone(),
        controller: ControllerIdentity {
            repository_id: 1,
            run_id: 2,
            run_attempt: 1,
            actor_id: 3,
            authorization_sha256: hash('6'),
        },
        abort_reason: None,
        approval_nonce_sha256: None,
    };
    let plan = PacketPlan {
        campaign_id: campaign_id.clone(),
        campaign_manifest_sha256: manifest_sha256,
        result: PacketResult::Complete,
        promotable: false,
        terminal_reason: None,
        required_final_guards: vec!["semantic".to_owned()],
        guard_evidence: vec![GuardEvidenceInput {
            id: "semantic".to_owned(),
            passed: true,
            source_relative_path: PathBuf::from("roles/i74/guards/semantic.json"),
        }],
        roles: vec![RoleEvidenceInput {
            role: Role::I74,
            result: PacketResult::Complete,
            journal_relative_path: PathBuf::from("roles/i74/checkpoints.jsonl"),
        }],
        raw_files: vec![
            PathBuf::from("campaign-start.json"),
            PathBuf::from("roles/i74/checkpoints.jsonl"),
            PathBuf::from("roles/i74/guards/semantic.json"),
        ],
    };
    let seal_root = temporary.path().join("seals");
    fs::create_dir(&seal_root).unwrap();
    Fixture {
        _temporary: temporary,
        claim,
        lock,
        request,
        unit: UnitSnapshot {
            unit_name,
            active_state: "active".to_owned(),
            sub_state: "exited".to_owned(),
            main_pid: 0,
            control_group: state.harness.unwrap().cgroup_path,
            result: "success".to_owned(),
        },
        plan,
        seal_root,
    }
}

fn limits() -> (PacketLimits, ArchiveLimits) {
    (
        PacketLimits {
            maximum_files: 2_000,
            maximum_bytes: 1_073_741_824,
        },
        ArchiveLimits {
            maximum_files: 2_002,
            maximum_uncompressed_bytes: 1_073_741_824,
            maximum_archive_bytes: 1_073_741_824,
        },
    )
}

fn seal(fixture: &Fixture) -> hydracache_long_run_supervisor_074::protocol::Response {
    let (packet_limits, archive_limits) = limits();
    drive_seal_request(
        &fixture.claim,
        &fixture.lock,
        &fixture.request,
        1_000,
        &fixture.unit,
        &fixture.plan,
        &fixture.seal_root,
        packet_limits,
        archive_limits,
    )
    .unwrap()
}

#[test]
fn terminal_artifact_sealed_response_is_durable_and_exactly_replayed() {
    let fixture = fixture();
    let first = seal(&fixture);
    let result: SealResponseResult =
        serde_json::from_value(first.body.result.clone().unwrap()).unwrap();
    assert_eq!(result.state.campaign_state, CampaignState::I74Sealed);
    assert_eq!(result.state.revision, 5);
    assert!(result.state.harness.is_none());
    assert!(result.state.controller_lease.is_none());
    assert_eq!(fixture.lock.read().unwrap(), result.state);

    let replay = seal(&fixture);
    assert_eq!(replay, first);
    let report = verify_event_journal(
        &fixture.lock.campaign_directory().join(EVENT_JOURNAL_NAME),
        &fixture.lock.campaign_directory().join(EVENT_HEAD_NAME),
    )
    .unwrap();
    assert_eq!(report.records, 7);
    assert_eq!(report.replay_index.len(), 2);

    let mut conflict = fixture.request.clone();
    conflict.controller.run_attempt = 2;
    let (packet_limits, archive_limits) = limits();
    assert!(drive_seal_request(
        &fixture.claim,
        &fixture.lock,
        &conflict,
        1_001,
        &fixture.unit,
        &fixture.plan,
        &fixture.seal_root,
        packet_limits,
        archive_limits,
    )
    .is_err());
}

#[test]
fn terminal_event_ahead_of_state_is_reconciled_before_artifact_creation() {
    let fixture = fixture();
    let digest = request_sha256(&fixture.request).unwrap();
    let mut terminal = fixture.lock.read().unwrap();
    terminal.revision = 4;
    terminal.campaign_state = CampaignState::I74Terminal;
    append_lifecycle_event(
        &fixture.lock.campaign_directory().join(EVENT_JOURNAL_NAME),
        &fixture.lock.campaign_directory().join(EVENT_HEAD_NAME),
        999,
        fixture.request.request_id.clone(),
        digest,
        LifecycleEvent::I74Terminal,
        terminal,
    )
    .unwrap();

    let response = seal(&fixture);
    let result: SealResponseResult =
        serde_json::from_value(response.body.result.clone().unwrap()).unwrap();
    assert_eq!(result.state.campaign_state, CampaignState::I74Sealed);
    assert_eq!(result.state.revision, 5);
}

#[test]
fn terminal_unit_drift_fails_before_state_or_artifact_mutation() {
    let fixture = fixture();
    let mut drift = fixture.unit.clone();
    drift.result = "failed".to_owned();
    let (packet_limits, archive_limits) = limits();
    assert!(drive_seal_request(
        &fixture.claim,
        &fixture.lock,
        &fixture.request,
        1_000,
        &drift,
        &fixture.plan,
        &fixture.seal_root,
        packet_limits,
        archive_limits,
    )
    .is_err());
    assert_eq!(
        fixture.lock.read().unwrap().campaign_state,
        CampaignState::I74Running
    );
    assert!(fs::read_dir(&fixture.seal_root).unwrap().next().is_none());
    assert!(!fixture
        .lock
        .campaign_directory()
        .join("i74-seal-intent.json")
        .exists());
}

#[test]
fn c74_complete_seal_releases_host_claim_and_replays_after_release() {
    let fixture = fixture();
    seal(&fixture);
    let campaign = fixture.lock.campaign_directory();
    let campaign_id = fixture.request.campaign_id.clone();
    let events = campaign.join(EVENT_JOURNAL_NAME);
    let event_head = campaign.join(EVENT_HEAD_NAME);
    let c74_start_id = "323e4567-e89b-42d3-a456-426614174000";
    let c74_start_digest = hash('7');
    let c74_unit_name = format!("hydracache-performance-074-c74-{campaign_id}.service");

    let mut state = fixture.lock.read().unwrap();
    state.revision = 6;
    state.campaign_state = CampaignState::C74Starting;
    append_lifecycle_event(
        &events,
        &event_head,
        1_100,
        c74_start_id.to_owned(),
        c74_start_digest.clone(),
        LifecycleEvent::C74Starting,
        state.clone(),
    )
    .unwrap();
    fixture.lock.compare_and_swap(5, &state).unwrap();
    state.revision = 7;
    state.campaign_state = CampaignState::C74Running;
    state.harness = Some(process(&c74_unit_name, 200));
    state.daemon = Some(process(&c74_unit_name, 201));
    append_lifecycle_event(
        &events,
        &event_head,
        1_101,
        c74_start_id.to_owned(),
        c74_start_digest,
        LifecycleEvent::C74Started,
        state.clone(),
    )
    .unwrap();
    fixture.lock.compare_and_swap(6, &state).unwrap();

    let c74_role = campaign.join("roles/c74");
    fs::create_dir_all(c74_role.join("guards")).unwrap();
    let c74_journal = c74_role.join("checkpoints.jsonl");
    let c74_head = c74_role.join("checkpoints.head");
    let first = build_record(
        1,
        GENESIS_HASH,
        payload(&campaign_id, Role::C74, &c74_unit_name, 200, 1),
    )
    .unwrap();
    append_record(&c74_journal, &c74_head, &first).unwrap();
    let second = build_record(
        2,
        &first.record_sha256,
        payload(&campaign_id, Role::C74, &c74_unit_name, 200, 2),
    )
    .unwrap();
    append_record(&c74_journal, &c74_head, &second).unwrap();

    let attach_request = Request {
        schema_version: 1,
        request_id: "423e4567-e89b-42d3-a456-426614174000".to_owned(),
        operation: Operation::Attach,
        campaign_id: campaign_id.clone(),
        expected_state_revision: 7,
        manifest_path: None,
        manifest_sha256: fixture.request.manifest_sha256.clone(),
        controller: fixture.request.controller.clone(),
        abort_reason: None,
        approval_nonce_sha256: None,
    };
    state.revision = 8;
    state.checkpoint = Some(CheckpointHead {
        sequence: 2,
        record_sha256: second.record_sha256,
        useful_progress_unix_seconds: 1_199,
    });
    state.controller_lease = Some(ControllerLease {
        holder_request_id: attach_request.request_id.clone(),
        authorization_sha256: fixture.request.controller.authorization_sha256.clone(),
        repository_id: 1,
        run_id: 2,
        actor_id: 3,
        expires_unix_seconds: 1_500,
    });
    let attach_response = sign_response(ResponseBody {
        schema_version: 1,
        request_id: attach_request.request_id.clone(),
        campaign_id: campaign_id.clone(),
        ok: true,
        state_revision: 8,
        server_time_unix_seconds: 1_102,
        result: Some(serde_json::to_value(&state).unwrap()),
        error_code: None,
    })
    .unwrap();
    append_or_replay(
        &events,
        &event_head,
        1_102,
        attach_request,
        EventOutcome::Accepted,
        attach_response,
        Some(state.clone()),
    )
    .unwrap();
    fixture.lock.compare_and_swap(7, &state).unwrap();

    let request = Request {
        schema_version: 1,
        request_id: "523e4567-e89b-42d3-a456-426614174000".to_owned(),
        operation: Operation::Seal,
        campaign_id: campaign_id.clone(),
        expected_state_revision: 8,
        manifest_path: None,
        manifest_sha256: fixture.request.manifest_sha256.clone(),
        controller: fixture.request.controller.clone(),
        abort_reason: None,
        approval_nonce_sha256: None,
    };
    let plan = PacketPlan {
        campaign_id,
        campaign_manifest_sha256: fixture.plan.campaign_manifest_sha256.clone(),
        result: PacketResult::Complete,
        promotable: true,
        terminal_reason: None,
        required_final_guards: fixture.plan.required_final_guards.clone(),
        guard_evidence: fixture.plan.guard_evidence.clone(),
        roles: vec![
            fixture.plan.roles[0].clone(),
            RoleEvidenceInput {
                role: Role::C74,
                result: PacketResult::Complete,
                journal_relative_path: PathBuf::from("roles/c74/checkpoints.jsonl"),
            },
        ],
        raw_files: vec![
            PathBuf::from("campaign-start.json"),
            PathBuf::from("roles/i74/checkpoints.jsonl"),
            PathBuf::from("roles/i74/guards/semantic.json"),
            PathBuf::from("roles/c74/checkpoints.jsonl"),
        ],
    };
    let unit = UnitSnapshot {
        unit_name: c74_unit_name,
        active_state: "active".to_owned(),
        sub_state: "exited".to_owned(),
        main_pid: 0,
        control_group: state.harness.unwrap().cgroup_path,
        result: "success".to_owned(),
    };
    let (packet_limits, archive_limits) = limits();
    let first = drive_seal_request(
        &fixture.claim,
        &fixture.lock,
        &request,
        1_200,
        &unit,
        &plan,
        &fixture.seal_root,
        packet_limits,
        archive_limits,
    )
    .unwrap();
    let result: SealResponseResult =
        serde_json::from_value(first.body.result.clone().unwrap()).unwrap();
    assert_eq!(result.state.campaign_state, CampaignState::CompleteSealed);
    assert_eq!(result.state.revision, 10);
    assert!(!fixture
        .claim
        .campaign_root()
        .join(ACTIVE_CAMPAIGN_NAME)
        .exists());

    let replay = drive_seal_request(
        &fixture.claim,
        &fixture.lock,
        &request,
        1_201,
        &unit,
        &plan,
        &fixture.seal_root,
        packet_limits,
        archive_limits,
    )
    .unwrap();
    assert_eq!(replay, first);
}
