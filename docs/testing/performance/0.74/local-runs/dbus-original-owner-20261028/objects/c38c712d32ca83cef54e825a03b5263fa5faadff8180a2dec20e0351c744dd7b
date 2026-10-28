use hydracache_long_run_supervisor_074::event::{
    append_or_replay, request_sha256, EventOutcome, EVENT_HEAD_NAME, EVENT_JOURNAL_NAME,
};
use hydracache_long_run_supervisor_074::mutation::{
    begin_attach, reconcile_campaign, BeginAttach, MutationError,
};
use hydracache_long_run_supervisor_074::protocol::{
    sign_response, ControllerIdentity, Operation, Request, ResponseBody,
};
use hydracache_long_run_supervisor_074::state::{
    apply_attach, AttachRequest, CampaignState, CheckpointHead, DurableCampaignState,
    FrozenIdentity,
};
use hydracache_long_run_supervisor_074::state_store::CampaignLock;
use hydracache_long_run_supervisor_074::ProcessIdentity;
use std::fs;

fn process(pid: u32) -> ProcessIdentity {
    ProcessIdentity {
        boot_id: "boot-a".to_owned(),
        pid,
        start_ticks: u64::from(pid) * 100,
        process_group: 10,
        cgroup_path: "/hc/a".to_owned(),
        cgroup_inode: 50,
        unit_name: "hc-a.service".to_owned(),
    }
}

fn state() -> DurableCampaignState {
    let hash = |byte: char| byte.to_string().repeat(64);
    DurableCampaignState {
        revision: 0,
        campaign_state: CampaignState::I74Running,
        identity: FrozenIdentity {
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
            lease_deadline_unix_seconds: 2_000,
        },
        harness: Some(process(100)),
        daemon: Some(process(101)),
        checkpoint: Some(CheckpointHead {
            sequence: 8,
            record_sha256: hash('5'),
            useful_progress_unix_seconds: 1_000,
        }),
        controller_lease: None,
        recorded_failure: false,
        duplicate_executor: false,
        durable_history_corrupt: false,
    }
}

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

fn next_state(current: &DurableCampaignState, request: &Request) -> DurableCampaignState {
    apply_attach(
        current,
        &AttachRequest {
            request_id: request.request_id.clone(),
            request_sha256: request_sha256(request).unwrap(),
            expected_revision: request.expected_state_revision,
            authorization_sha256: request.controller.authorization_sha256.clone(),
            repository_id: request.controller.repository_id,
            run_id: request.controller.run_id,
            actor_id: request.controller.actor_id,
            identity: current.identity.clone(),
            harness: current.harness.clone().unwrap(),
            daemon: current.daemon.clone().unwrap(),
            checkpoint: current.checkpoint.clone().unwrap(),
            now_unix_seconds: 1_010,
            requested_controller_lease_seconds: 60,
        },
        180,
    )
    .unwrap()
}

fn campaign() -> (tempfile::TempDir, CampaignLock) {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("campaigns");
    fs::create_dir(&root).unwrap();
    fs::create_dir(root.join("a".repeat(64))).unwrap();
    let lock = CampaignLock::acquire(&root, &"a".repeat(64)).unwrap();
    lock.initialize(&state()).unwrap();
    (temporary, lock)
}

#[test]
fn accepted_attach_commits_event_before_state_and_replays_exact_response() {
    let (_temporary, lock) = campaign();
    let request = request("123e4567-e89b-42d3-a456-426614174000", 0);
    let transaction = match begin_attach(&lock, &request, 1_010).unwrap() {
        BeginAttach::New(transaction) => transaction,
        BeginAttach::Replayed(_) => panic!("fresh request replayed"),
    };
    let next = next_state(transaction.state(), &request);
    let response = transaction.accept(next.clone()).unwrap();
    assert!(response.body.ok);
    assert_eq!(lock.read().unwrap(), next);

    let replay = match begin_attach(&lock, &request, 1_011).unwrap() {
        BeginAttach::Replayed(response) => response,
        BeginAttach::New(_) => panic!("committed request was not replayed"),
    };
    assert_eq!(replay, response);
}

#[test]
fn rejected_attach_is_durable_and_does_not_change_state() {
    let (_temporary, lock) = campaign();
    let request = request("123e4567-e89b-42d3-a456-426614174000", 9);
    let transaction = match begin_attach(&lock, &request, 1_010).unwrap() {
        BeginAttach::New(transaction) => transaction,
        BeginAttach::Replayed(_) => panic!("fresh request replayed"),
    };
    let response = transaction.reject(6).unwrap();
    assert_eq!(response.body.error_code, Some(6));
    assert_eq!(lock.read().unwrap(), state());
    assert!(matches!(
        begin_attach(&lock, &request, 1_011).unwrap(),
        BeginAttach::Replayed(replayed) if replayed == response
    ));
}

#[test]
fn recovery_finishes_state_commit_after_event_sync_crash_window() {
    let (_temporary, lock) = campaign();
    let request = request("123e4567-e89b-42d3-a456-426614174000", 0);
    let next = next_state(&state(), &request);
    let response = sign_response(ResponseBody {
        schema_version: 1,
        request_id: request.request_id.clone(),
        campaign_id: request.campaign_id.clone(),
        ok: true,
        state_revision: 1,
        server_time_unix_seconds: 1_010,
        result: Some(serde_json::to_value(&next).unwrap()),
        error_code: None,
    })
    .unwrap();
    append_or_replay(
        &lock.campaign_directory().join(EVENT_JOURNAL_NAME),
        &lock.campaign_directory().join(EVENT_HEAD_NAME),
        1_010,
        request.clone(),
        EventOutcome::Accepted,
        response.clone(),
        Some(next.clone()),
    )
    .unwrap();
    assert_eq!(lock.read().unwrap().revision, 0);

    assert_eq!(reconcile_campaign(&lock).unwrap(), next);

    assert!(matches!(
        begin_attach(&lock, &request, 1_011).unwrap(),
        BeginAttach::Replayed(replayed) if replayed == response
    ));
    assert_eq!(lock.read().unwrap(), next);
}

#[test]
fn same_revision_snapshot_drift_is_never_repaired_from_a_copy() {
    let (_temporary, lock) = campaign();
    let request = request("123e4567-e89b-42d3-a456-426614174000", 0);
    let authoritative = next_state(&state(), &request);
    let response = sign_response(ResponseBody {
        schema_version: 1,
        request_id: request.request_id.clone(),
        campaign_id: request.campaign_id.clone(),
        ok: true,
        state_revision: 1,
        server_time_unix_seconds: 1_010,
        result: Some(serde_json::to_value(&authoritative).unwrap()),
        error_code: None,
    })
    .unwrap();
    append_or_replay(
        &lock.campaign_directory().join(EVENT_JOURNAL_NAME),
        &lock.campaign_directory().join(EVENT_HEAD_NAME),
        1_010,
        request.clone(),
        EventOutcome::Accepted,
        response,
        Some(authoritative.clone()),
    )
    .unwrap();
    let mut drifted = authoritative;
    drifted.recorded_failure = true;
    lock.compare_and_swap(0, &drifted).unwrap();

    assert!(matches!(
        begin_attach(&lock, &request, 1_011),
        Err(MutationError::Diverged)
    ));
}
