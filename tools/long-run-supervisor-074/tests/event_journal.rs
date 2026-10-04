use hydracache_long_run_supervisor_074::event::{
    append_or_replay, build_event, request_sha256, verify_event_bytes, verify_event_journal,
    EventAppend, EventError, EventOutcome,
};
use hydracache_long_run_supervisor_074::protocol::{
    sign_response, ControllerIdentity, Operation, Request, ResponseBody,
};
use hydracache_long_run_supervisor_074::state::{
    CampaignState, CheckpointHead, ControllerLease, DurableCampaignState, FrozenIdentity,
};
use hydracache_long_run_supervisor_074::{ProcessIdentity, GENESIS_HASH};
use serde_json::json;
use std::fs;
use tempfile::tempdir;

fn request(request_id: &str, revision: u64) -> Request {
    Request {
        schema_version: 1,
        request_id: request_id.to_owned(),
        operation: Operation::Attach,
        campaign_id: "a".repeat(64),
        expected_state_revision: revision,
        manifest_path: None,
        manifest_sha256: "b".repeat(64),
        controller: ControllerIdentity {
            repository_id: 1,
            run_id: 2,
            run_attempt: 1,
            actor_id: 3,
            authorization_sha256: "c".repeat(64),
        },
        abort_reason: None,
        approval_nonce_sha256: None,
    }
}

fn response(
    request: &Request,
    revision: u64,
    ok: bool,
) -> hydracache_long_run_supervisor_074::protocol::Response {
    sign_response(ResponseBody {
        schema_version: 1,
        request_id: request.request_id.clone(),
        campaign_id: request.campaign_id.clone(),
        ok,
        state_revision: revision,
        server_time_unix_seconds: 1_000 + revision,
        result: ok.then(|| json!({"state": "I74_RUNNING"})),
        error_code: (!ok).then_some(11),
    })
    .unwrap()
}

fn state_after(request: &Request, revision: u64) -> DurableCampaignState {
    let hash = |byte: char| byte.to_string().repeat(64);
    let process = |pid| ProcessIdentity {
        boot_id: "boot-a".to_owned(),
        pid,
        start_ticks: u64::from(pid) * 100,
        process_group: 10,
        cgroup_path: "/hc/a".to_owned(),
        cgroup_inode: 50,
        unit_name: "hc-a.service".to_owned(),
    };
    DurableCampaignState {
        revision,
        campaign_state: CampaignState::I74Running,
        identity: FrozenIdentity {
            campaign_id: request.campaign_id.clone(),
            manifest_sha256: request.manifest_sha256.clone(),
            contract_sha256: hash('d'),
            scenario_sha256: hash('e'),
            tooling_sha256: hash('f'),
            source_bundle_sha256: hash('1'),
            binary_bundle_sha256: hash('2'),
            workload_bundle_sha256: hash('3'),
            machine_id: "machine-a".to_owned(),
            boot_id: "boot-a".to_owned(),
            host_receipt_sha256: hash('4'),
            mount_identity: "mount-a".to_owned(),
            isolated_cpuset: "2-7".to_owned(),
            housekeeping_cpuset: "0-1".to_owned(),
            command_environment_sha256: hash('5'),
            lease_id: "00000000-0000-4000-8000-000000000074".to_owned(),
            lease_deadline_unix_seconds: 2_000_000_000,
        },
        harness: Some(process(100)),
        daemon: Some(process(101)),
        checkpoint: Some(CheckpointHead {
            sequence: 8,
            record_sha256: hash('6'),
            useful_progress_unix_seconds: 1_000,
        }),
        controller_lease: Some(ControllerLease {
            holder_request_id: request.request_id.clone(),
            authorization_sha256: request.controller.authorization_sha256.clone(),
            expires_unix_seconds: 1_300,
        }),
        recorded_failure: false,
        duplicate_executor: false,
        durable_history_corrupt: false,
    }
}

#[test]
fn appends_and_verifies_a_durable_event_chain() {
    let directory = tempdir().unwrap();
    let journal = directory.path().join("events.jsonl");
    let head = directory.path().join("events.head");
    let first = request("123e4567-e89b-42d3-a456-426614174000", 0);
    let second = request("223e4567-e89b-42d3-a456-426614174000", 1);

    assert!(matches!(
        append_or_replay(
            &journal,
            &head,
            10,
            first.clone(),
            EventOutcome::Accepted,
            response(&first, 1, true),
            Some(state_after(&first, 1)),
        )
        .unwrap(),
        EventAppend::Appended(_)
    ));
    assert!(matches!(
        append_or_replay(
            &journal,
            &head,
            11,
            second.clone(),
            EventOutcome::Rejected,
            response(&second, 1, false),
            None,
        )
        .unwrap(),
        EventAppend::Appended(_)
    ));

    let report = verify_event_journal(&journal, &head).unwrap();
    assert_eq!(report.records, 2);
    assert_eq!(report.campaign_id, "a".repeat(64));
    assert_eq!(report.last_occurred_at_unix_seconds, 11);
    assert_eq!(report.recovered_incomplete_trailing_bytes, 0);
    assert_eq!(report.replay_index.len(), 2);
    assert_eq!(
        fs::read_to_string(head).unwrap(),
        format!("{}\n", report.head_sha256)
    );
}

#[test]
fn returns_the_original_response_for_an_identical_request_without_appending() {
    let directory = tempdir().unwrap();
    let journal = directory.path().join("events.jsonl");
    let head = directory.path().join("events.head");
    let original = request("123e4567-e89b-42d3-a456-426614174000", 4);
    let original_response = response(&original, 5, true);
    append_or_replay(
        &journal,
        &head,
        10,
        original.clone(),
        EventOutcome::Accepted,
        original_response.clone(),
        Some(state_after(&original, 5)),
    )
    .unwrap();
    let before = fs::read(&journal).unwrap();

    let replay = append_or_replay(
        &journal,
        &head,
        99,
        original,
        EventOutcome::Rejected,
        response(
            &request("123e4567-e89b-42d3-a456-426614174000", 4),
            4,
            false,
        ),
        None,
    )
    .unwrap();
    assert_eq!(replay, EventAppend::Replayed(original_response));
    assert_eq!(fs::read(&journal).unwrap(), before);
}

#[test]
fn rejects_request_id_reuse_with_different_request_bytes() {
    let directory = tempdir().unwrap();
    let journal = directory.path().join("events.jsonl");
    let head = directory.path().join("events.head");
    let original = request("123e4567-e89b-42d3-a456-426614174000", 4);
    append_or_replay(
        &journal,
        &head,
        10,
        original.clone(),
        EventOutcome::Accepted,
        response(&original, 5, true),
        Some(state_after(&original, 5)),
    )
    .unwrap();
    let changed = request("123e4567-e89b-42d3-a456-426614174000", 5);

    assert!(matches!(
        append_or_replay(
            &journal,
            &head,
            11,
            changed.clone(),
            EventOutcome::Accepted,
            response(&changed, 6, true),
            Some(state_after(&changed, 6)),
        ),
        Err(EventError::ReplayConflict { .. })
    ));
}

#[test]
fn detects_middle_corruption_and_a_mismatched_head() {
    let directory = tempdir().unwrap();
    let journal = directory.path().join("events.jsonl");
    let head = directory.path().join("events.head");
    let first = request("123e4567-e89b-42d3-a456-426614174000", 0);
    append_or_replay(
        &journal,
        &head,
        10,
        first.clone(),
        EventOutcome::Accepted,
        response(&first, 1, true),
        Some(state_after(&first, 1)),
    )
    .unwrap();

    let mut corrupted = fs::read(&journal).unwrap();
    let position = corrupted
        .windows(64)
        .position(|window| window == "b".repeat(64).as_bytes())
        .unwrap();
    corrupted[position] = b'c';
    assert!(matches!(
        verify_event_bytes(&corrupted),
        Err(EventError::Binding { .. }) | Err(EventError::PayloadHash { .. })
    ));

    fs::write(&head, format!("{}\n", "f".repeat(64))).unwrap();
    assert!(matches!(
        verify_event_journal(&journal, &head),
        Err(EventError::Head)
    ));
}

#[test]
fn reports_a_torn_final_record_and_refuses_to_append_over_it() {
    let directory = tempdir().unwrap();
    let journal = directory.path().join("events.jsonl");
    let head = directory.path().join("events.head");
    let first = request("123e4567-e89b-42d3-a456-426614174000", 0);
    append_or_replay(
        &journal,
        &head,
        10,
        first.clone(),
        EventOutcome::Accepted,
        response(&first, 1, true),
        Some(state_after(&first, 1)),
    )
    .unwrap();
    fs::OpenOptions::new()
        .append(true)
        .open(&journal)
        .unwrap()
        .write_all(br#"{"schema_version":1"#)
        .unwrap();

    let report = verify_event_journal(&journal, &head).unwrap();
    assert!(report.recovered_incomplete_trailing_bytes > 0);
    let second = request("223e4567-e89b-42d3-a456-426614174000", 1);
    assert!(matches!(
        append_or_replay(
            &journal,
            &head,
            11,
            second.clone(),
            EventOutcome::Accepted,
            response(&second, 2, true),
            Some(state_after(&second, 2)),
        ),
        Err(EventError::TornTailRequiresRecovery { .. })
    ));
}

#[test]
fn treats_a_complete_json_record_without_its_newline_as_a_torn_write() {
    let original = request("123e4567-e89b-42d3-a456-426614174000", 0);
    let event = build_event(
        1,
        GENESIS_HASH,
        10,
        original.clone(),
        EventOutcome::Accepted,
        response(&original, 1, true),
        Some(state_after(&original, 1)),
    )
    .unwrap();
    let bytes = serde_json::to_vec(&event).unwrap();

    assert!(matches!(
        verify_event_bytes(&bytes),
        Err(EventError::TornTailRequiresRecovery { bytes: count }) if count == bytes.len()
    ));
}

#[test]
fn rejects_invalid_response_binding_and_outcome() {
    let original = request("123e4567-e89b-42d3-a456-426614174000", 0);
    let different = request("223e4567-e89b-42d3-a456-426614174000", 0);
    assert!(matches!(
        build_event(
            1,
            GENESIS_HASH,
            10,
            original.clone(),
            EventOutcome::Accepted,
            response(&different, 1, true),
            Some(state_after(&original, 1)),
        ),
        Err(EventError::Binding { .. })
    ));
    assert!(matches!(
        build_event(
            1,
            GENESIS_HASH,
            10,
            original.clone(),
            EventOutcome::Rejected,
            response(&original, 1, true),
            None,
        ),
        Err(EventError::Outcome { .. })
    ));
    assert_eq!(request_sha256(&original).unwrap().len(), 64);
}

use std::io::Write;
