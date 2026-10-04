#![cfg(target_os = "linux")]

use ed25519_dalek::{Signer, SigningKey};
use hydracache_long_run_supervisor_074::auth::{
    canonical_document, canonical_message, AuthorizationBody, SignedAuthorization,
};
use hydracache_long_run_supervisor_074::client::exchange;
use hydracache_long_run_supervisor_074::config::ServerConfig;
use hydracache_long_run_supervisor_074::protocol::{
    ControllerIdentity, Operation, Request, WireRequest,
};
use hydracache_long_run_supervisor_074::server::SupervisorServer;
use hydracache_long_run_supervisor_074::state::{
    CampaignState, CheckpointHead, DurableCampaignState, FrozenIdentity,
};
use hydracache_long_run_supervisor_074::state_store::CampaignLock;
use hydracache_long_run_supervisor_074::ProcessIdentity;
use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

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

fn state(revision: u64) -> DurableCampaignState {
    let hash = |byte: char| byte.to_string().repeat(64);
    DurableCampaignState {
        revision,
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
            lease_deadline_unix_seconds: 2_000_000_000,
        },
        harness: process(100),
        daemon: process(101),
        checkpoint: CheckpointHead {
            sequence: 8,
            record_sha256: hash('5'),
            useful_progress_unix_seconds: 1_000,
        },
        controller_lease: None,
        recorded_failure: false,
        duplicate_executor: false,
        durable_history_corrupt: false,
    }
}

fn request(revision: u64) -> Vec<u8> {
    serde_json::to_vec(&WireRequest {
        request: Request {
            schema_version: 1,
            request_id: "123e4567-e89b-42d3-a456-426614174000".to_owned(),
            operation: Operation::Status,
            campaign_id: "a".repeat(64),
            expected_state_revision: revision,
            manifest_path: None,
            manifest_sha256: "b".repeat(64),
            controller: ControllerIdentity {
                repository_id: 10,
                run_id: 20,
                run_attempt: 1,
                actor_id: 30,
                authorization_sha256: "c".repeat(64),
            },
            abort_reason: None,
            approval_nonce_sha256: None,
        },
        authorization: None,
    })
    .unwrap()
}

fn attach_request(key: &SigningKey, revision: u64, now: u64) -> Vec<u8> {
    let mut request = Request {
        schema_version: 1,
        request_id: "223e4567-e89b-42d3-a456-426614174000".to_owned(),
        operation: Operation::Attach,
        campaign_id: "a".repeat(64),
        expected_state_revision: revision,
        manifest_path: None,
        manifest_sha256: "b".repeat(64),
        controller: ControllerIdentity {
            repository_id: 10,
            run_id: 20,
            run_attempt: 1,
            actor_id: 30,
            authorization_sha256: "0".repeat(64),
        },
        abort_reason: None,
        approval_nonce_sha256: None,
    };
    let body = AuthorizationBody {
        schema_version: 1,
        request_id: request.request_id.clone(),
        operation: request.operation,
        campaign_id: request.campaign_id.clone(),
        manifest_sha256: request.manifest_sha256.clone(),
        repository_id: request.controller.repository_id,
        run_id: request.controller.run_id,
        actor_id: request.controller.actor_id,
        issued_at_unix_seconds: now.saturating_sub(1),
        expires_at_unix_seconds: now + 300,
    };
    let authorization = SignedAuthorization {
        signature_hex: hex(&key.sign(&canonical_message(&body).unwrap()).to_bytes()),
        body,
    };
    request.controller.authorization_sha256 =
        hex(&Sha256::digest(canonical_document(&authorization).unwrap()));
    serde_json::to_vec(&WireRequest {
        request,
        authorization: Some(authorization),
    })
    .unwrap()
}

fn server_config(socket: &Path, campaign_root: &Path) -> ServerConfig {
    let key = SigningKey::from_bytes(&[7; 32]);
    let uid = unsafe { libc::geteuid() };
    let gid = unsafe { libc::getegid() };
    let document = format!(
        "schema_version=1\nsocket_path={:?}\ncampaign_root={:?}\nsocket_mode=432\nexpected_repository_id=10\nallowed_actor_ids=[30]\nallowed_client_uids=[{uid}]\nrequired_client_gid={gid}\nverification_key_hex=\"{}\"\n",
        socket.as_os_str().as_bytes().escape_ascii().to_string(),
        campaign_root.as_os_str().as_bytes().escape_ascii().to_string(),
        hex(key.verifying_key().as_bytes())
    );
    ServerConfig::parse(document.as_bytes(), false).unwrap()
}

fn exchange_once(
    server: &SupervisorServer,
    socket: &Path,
    packet: &[u8],
) -> hydracache_long_run_supervisor_074::protocol::Response {
    std::thread::scope(|scope| {
        let handle = scope.spawn(|| server.serve_one().unwrap());
        let response = exchange(socket, packet).unwrap();
        handle.join().unwrap();
        response
    })
}

#[test]
fn status_round_trips_exact_durable_state_and_rejects_stale_revision() {
    let temporary = tempfile::tempdir().unwrap();
    let campaign_root = temporary.path().join("campaigns");
    fs::create_dir(&campaign_root).unwrap();
    fs::create_dir(campaign_root.join("a".repeat(64))).unwrap();
    let lock = CampaignLock::acquire(&campaign_root, &"a".repeat(64)).unwrap();
    lock.initialize(&state(0)).unwrap();
    for revision in 1..=7 {
        lock.compare_and_swap(revision - 1, &state(revision))
            .unwrap();
    }
    drop(lock);

    let socket = temporary.path().join("supervisor.sock");
    let server = SupervisorServer::bind(server_config(&socket, &campaign_root)).unwrap();
    let response = exchange_once(&server, &socket, &request(7));
    assert!(response.body.ok);
    assert_eq!(response.body.state_revision, 7);
    assert_eq!(
        response.body.result.unwrap()["campaign_state"],
        "I74_RUNNING"
    );

    let stale = exchange_once(&server, &socket, &request(6));
    assert!(!stale.body.ok);
    assert_eq!(stale.body.error_code, Some(5));
    assert_eq!(stale.body.state_revision, 7);
}

#[test]
fn attach_guard_rejection_is_durable_and_exactly_replayed() {
    let temporary = tempfile::tempdir().unwrap();
    let campaign_root = temporary.path().join("campaigns");
    fs::create_dir(&campaign_root).unwrap();
    fs::create_dir(campaign_root.join("a".repeat(64))).unwrap();
    let lock = CampaignLock::acquire(&campaign_root, &"a".repeat(64)).unwrap();
    lock.initialize(&state(0)).unwrap();
    drop(lock);

    let key = SigningKey::from_bytes(&[7; 32]);
    let socket = temporary.path().join("supervisor.sock");
    let server = SupervisorServer::bind(server_config(&socket, &campaign_root)).unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let packet = attach_request(&key, 0, now);
    let response = exchange_once(&server, &socket, &packet);
    assert!(!response.body.ok);
    assert_eq!(response.body.error_code, Some(6));
    let failures = response.body.result.as_ref().unwrap()["failures"]
        .as_array()
        .unwrap();
    assert!(failures.len() >= 4);
    assert!(failures
        .iter()
        .any(|failure| failure.as_str() == Some("host:manifest-unavailable")));

    let replay = exchange_once(&server, &socket, &packet);
    assert_eq!(replay, response);
    let lock = CampaignLock::acquire(&campaign_root, &"a".repeat(64)).unwrap();
    assert_eq!(lock.read().unwrap(), state(0));
    assert!(lock.campaign_directory().join("events.jsonl").exists());
}
