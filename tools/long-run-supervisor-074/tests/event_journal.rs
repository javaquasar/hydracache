use hydracache_long_run_supervisor_074::event::{
    append_or_replay, build_event, request_sha256, verify_event_bytes, verify_event_journal,
    EventAppend, EventError, EventOutcome,
};
use hydracache_long_run_supervisor_074::protocol::{
    sign_response, ControllerIdentity, Operation, Request, ResponseBody,
};
use hydracache_long_run_supervisor_074::GENESIS_HASH;
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
        ),
        Err(EventError::Outcome { .. })
    ));
    assert_eq!(request_sha256(&original).unwrap().len(), 64);
}

use std::io::Write;
