#![cfg(target_os = "linux")]
#![recursion_limit = "256"]

use ed25519_dalek::{Signer, SigningKey};
use hydracache_long_run_supervisor_074::abort_lifecycle::AbortBackend;
use hydracache_long_run_supervisor_074::artifact::PacketResult;
use hydracache_long_run_supervisor_074::auth::{
    canonical_document, canonical_message, AuthorizationBody, SignedAuthorization,
};
use hydracache_long_run_supervisor_074::client::exchange;
use hydracache_long_run_supervisor_074::config::ServerConfig;
use hydracache_long_run_supervisor_074::event::{
    append_lifecycle_event, append_or_replay, request_sha256, EventOutcome, LifecycleEvent,
    EVENT_HEAD_NAME, EVENT_JOURNAL_NAME,
};
use hydracache_long_run_supervisor_074::host_execution::ACTIVE_CAMPAIGN_NAME;
use hydracache_long_run_supervisor_074::host_receipt::{
    encode_canonical as encode_host_receipt, BinaryIdentity, HostObservationReceipt, MountIdentity,
    HOST_RECEIPT_HEAD_NAME, HOST_RECEIPT_NAME, SUPERVISOR_BINARY_PATH,
};
use hydracache_long_run_supervisor_074::manifest::CampaignManifest;
use hydracache_long_run_supervisor_074::protocol::{
    sign_response, ControllerIdentity, Operation, Request, ResponseBody, WireRequest,
};
use hydracache_long_run_supervisor_074::seal_input::{
    InventoryGuardEvidence, SealInputInventory, SEAL_INPUT_INVENTORY_NAME,
};
use hydracache_long_run_supervisor_074::server::{
    AbortObservationBackend, SealObservationBackend, SupervisorServer,
};
use hydracache_long_run_supervisor_074::spawn::{SpawnBackend, SpawnIntent, SpawnObservation};
use hydracache_long_run_supervisor_074::state::{
    CampaignState, CheckpointHead, ControllerLease, DurableCampaignState, FrozenIdentity,
};
use hydracache_long_run_supervisor_074::state_store::CampaignLock;
use hydracache_long_run_supervisor_074::systemd_unit::UnitSnapshot;
use hydracache_long_run_supervisor_074::{
    append_record, build_record, CheckpointPayload, Phase, ProcessIdentity, Role, GENESIS_HASH,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn canonical<T: Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(&serde_json::to_value(value).unwrap()).unwrap()
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
    let staging_root = campaign_root.parent().unwrap().join("staging");
    let seal_root = campaign_root.parent().unwrap().join("seals");
    let document = format!(
        "schema_version=1\nsocket_path={:?}\ncampaign_root={:?}\nstaging_root={:?}\nseal_root={:?}\nsocket_mode=432\nexpected_repository_id=10\nallowed_actor_ids=[30]\nallowed_client_uids=[{uid}]\nrequired_client_gid={gid}\nverification_key_hex=\"{}\"\n",
        socket.as_os_str().as_bytes().escape_ascii().to_string(),
        campaign_root.as_os_str().as_bytes().escape_ascii().to_string(),
        staging_root.as_os_str().as_bytes().escape_ascii().to_string(),
        seal_root.as_os_str().as_bytes().escape_ascii().to_string(),
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

fn exchange_start_once<B: SpawnBackend + Send>(
    server: &SupervisorServer,
    socket: &Path,
    packet: &[u8],
    backend: &mut B,
) -> hydracache_long_run_supervisor_074::protocol::Response {
    std::thread::scope(|scope| {
        let client = scope.spawn(|| exchange(socket, packet));
        server.serve_one_with_start_backend(backend).unwrap();
        client.join().unwrap().unwrap()
    })
}

fn exchange_seal_once(
    server: &SupervisorServer,
    socket: &Path,
    packet: &[u8],
    backend: &mut dyn SealObservationBackend,
) -> hydracache_long_run_supervisor_074::protocol::Response {
    std::thread::scope(|scope| {
        let client = scope.spawn(|| exchange(socket, packet));
        server.serve_one_with_seal_backend(backend).unwrap();
        client.join().unwrap().unwrap()
    })
}

fn exchange_abort_once(
    server: &SupervisorServer,
    socket: &Path,
    packet: &[u8],
    backend: &mut dyn AbortObservationBackend,
) -> hydracache_long_run_supervisor_074::protocol::Response {
    std::thread::scope(|scope| {
        let client = scope.spawn(|| exchange(socket, packet));
        server.serve_one_with_abort_backend(backend).unwrap();
        client.join().unwrap().unwrap()
    })
}

#[derive(Default)]
struct FakeAbortBackend {
    calls: usize,
    fail_next: bool,
}

impl AbortBackend for FakeAbortBackend {
    fn capture_and_stop(
        &mut self,
        _campaign_directory: &Path,
        _request: &Request,
        state: &DurableCampaignState,
    ) -> Result<(), String> {
        self.calls += 1;
        assert_eq!(state.campaign_state, CampaignState::AbortedIncomplete);
        assert!(state.harness.is_some());
        if self.fail_next {
            self.fail_next = false;
            return Err("injected abort interruption".to_owned());
        }
        Ok(())
    }
}

impl AbortObservationBackend for FakeAbortBackend {
    fn verify_host(
        &mut self,
        _campaign_directory: &Path,
        _manifest: &CampaignManifest,
        _state: &DurableCampaignState,
    ) -> Result<(), String> {
        Ok(())
    }
}

struct FakeSealBackend {
    snapshot: UnitSnapshot,
    host_checks: usize,
    unit_checks: usize,
}

impl SealObservationBackend for FakeSealBackend {
    fn verify_host(
        &mut self,
        _campaign_directory: &Path,
        _manifest: &CampaignManifest,
        _state: &DurableCampaignState,
    ) -> Result<(), String> {
        self.host_checks += 1;
        Ok(())
    }

    fn inspect_terminal(&mut self, _harness: &ProcessIdentity) -> Result<UnitSnapshot, String> {
        self.unit_checks += 1;
        Ok(self.snapshot.clone())
    }
}

#[derive(Default)]
struct FakeStartBackend {
    starts: usize,
}

impl SpawnBackend for FakeStartBackend {
    type Error = &'static str;

    fn start_once(&mut self, intent: &SpawnIntent) -> Result<SpawnObservation, Self::Error> {
        self.starts += 1;
        Ok(SpawnObservation::Exact {
            harness: transient_process(&intent.unit_name, 100),
            daemon: transient_process(&intent.unit_name, 101),
        })
    }

    fn observe(&mut self, _unit_name: &str) -> Result<SpawnObservation, Self::Error> {
        Err("unexpected observation")
    }
}

fn transient_process(unit_name: &str, pid: u32) -> ProcessIdentity {
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

fn stage_start(staging_root: &Path, key: &SigningKey, now: u64) -> Vec<u8> {
    let campaign_id = "1".repeat(64);
    let campaign = staging_root.join(&campaign_id);
    fs::create_dir(&campaign).unwrap();
    let mount = MountIdentity {
        mount_id: 31,
        device_major_minor: "8:2".to_owned(),
        root: "/".to_owned(),
        mount_point: "/var/lib/hydracache-performance".to_owned(),
        mount_options: vec!["relatime".to_owned(), "rw".to_owned()],
        filesystem_type: "ext4".to_owned(),
        source: "/dev/nvme0n1p2".to_owned(),
        super_options: vec!["errors=remount-ro".to_owned(), "rw".to_owned()],
    };
    let mount_identity = hex(&Sha256::digest(
        serde_json::to_vec(&serde_json::to_value(&mount).unwrap()).unwrap(),
    ));
    let receipt = HostObservationReceipt {
        schema_version: 1,
        machine_id: "machine-a".to_owned(),
        boot_id: "boot-a".to_owned(),
        kernel_release: "6.8.0-90-generic".to_owned(),
        kernel_command_line_sha256: "a".repeat(64),
        campaign_mount: mount,
        mount_identity: mount_identity.clone(),
        online_cpuset: "0-3".to_owned(),
        isolated_cpuset: "1-2".to_owned(),
        housekeeping_cpuset: "0,3".to_owned(),
        cpu_governors: (0..4)
            .map(|cpu| (format!("cpu{cpu}"), "performance".to_owned()))
            .collect(),
        kernel_tunables: [
            "kernel.numa_balancing",
            "kernel.sched_autogroup_enabled",
            "kernel.sched_migration_cost_ns",
            "kernel.watchdog",
            "vm.dirty_background_ratio",
            "vm.dirty_ratio",
            "vm.swappiness",
        ]
        .into_iter()
        .map(|name| (name.to_owned(), "0".to_owned()))
        .collect::<BTreeMap<_, _>>(),
        supervisor_binary: BinaryIdentity {
            path: SUPERVISOR_BINARY_PATH.to_owned(),
            sha256: "b".repeat(64),
            size: 1_024,
            inode: 44,
            device: 8,
            uid: 0,
            gid: 0,
            mode: 0o755,
        },
        reference_host_freeze_sha256: "c".repeat(64),
    };
    let receipt_bytes = encode_host_receipt(&receipt).unwrap();
    let receipt_sha256 = hex(&Sha256::digest(&receipt_bytes));
    let manifest = serde_json::json!({
        "schema_version": 1,
        "repository_id": 10,
        "authorization_identity": "protected-performance-074",
        "contract_sha256": "a".repeat(64),
        "tooling_sha": "b".repeat(40),
        "i74_source_sha": "c".repeat(40),
        "c74_source_sha": "d".repeat(40),
        "i74_tree_sha": "1".repeat(40),
        "c74_tree_sha": "2".repeat(40),
        "i74_cargo_lock_sha256": "3".repeat(64),
        "c74_cargo_lock_sha256": "4".repeat(64),
        "i74_dirty": false,
        "c74_dirty": false,
        "scenario_sha256": "e".repeat(64),
        "workload_sha256": "5".repeat(64),
        "offered_load_sha256": "6".repeat(64),
        "estimator_sha256": "7".repeat(64),
        "thresholds_sha256": "8".repeat(64),
        "host_receipt_sha256": receipt_sha256,
        "lease_id": "123e4567-e89b-42d3-a456-426614174000",
        "machine_id": "machine-a",
        "boot_id": "boot-a",
        "mount_identity": mount_identity,
        "isolated_cpuset": "1-2",
        "housekeeping_cpuset": "0,3",
        "seed": 740074,
        "checkpoint_cadence_seconds": 30,
        "progress_warning_gap_seconds": 90,
        "progress_rejection_gap_seconds": 180,
        "diagnostic_grace_seconds": 30,
        "product_lease_deadline_unix_seconds": now + 10_000,
        "maximum_campaign_bytes": 21_474_836_480_u64,
        "maximum_campaign_files": 20_000,
        "installed_binaries": [
            {"role": "i74", "path": "/opt/hydracache-performance/0.74/i74/hydracache", "sha256": "9".repeat(64), "size": 1, "inode": 2, "device": 3, "uid": 1001, "gid": 1001, "mode": 365},
            {"role": "c74", "path": "/opt/hydracache-performance/0.74/c74/hydracache", "sha256": "a".repeat(64), "size": 1, "inode": 4, "device": 3, "uid": 1001, "gid": 1001, "mode": 365}
        ],
        "argv_templates": {
            "i74": ["/opt/hydracache-performance/0.74/i74/hydracache", "--role", "i74"],
            "c74": ["/opt/hydracache-performance/0.74/c74/hydracache", "--role", "c74"]
        },
        "command_environment_sha256": "b".repeat(64),
        "role_order": ["i74", "c74"],
        "phase_durations_seconds": {"warmup": 60, "measured": 300, "drain": 30, "durable_companion": 30, "post_work_idle": 60, "reconciliation": 30},
        "output_limits": {"stdout_bytes": 1_048_576, "stderr_bytes": 1_048_576, "diagnostic_bytes": 1_048_576, "final_artifact_bytes": 1_073_741_824, "files": 2_000},
        "expected_output_schema_sha256s": {"checkpoint": "c".repeat(64), "measurement": "d".repeat(64), "reconciliation": "e".repeat(64), "raw_manifest": "1".repeat(64), "packet_manifest": "f".repeat(64)},
        "required_final_guards": ["semantic", "native-non-regression", "retention"],
        "secret_identifiers": ["github-environment-key-v1"],
        "release": "0.74",
        "campaign_id": campaign_id,
        "nonce_sha256": "2".repeat(64),
        "dirty": false,
        "controller_history": [],
        "state": "PREPARED"
    });
    let manifest_bytes = serde_json::to_vec(&manifest).unwrap();
    let manifest_sha256 = hex(&Sha256::digest(&manifest_bytes));
    fs::write(campaign.join("campaign-start.json"), &manifest_bytes).unwrap();
    fs::write(
        campaign.join("campaign-start.sha256"),
        format!("{manifest_sha256}\n"),
    )
    .unwrap();
    fs::write(campaign.join(HOST_RECEIPT_NAME), &receipt_bytes).unwrap();
    fs::write(
        campaign.join(HOST_RECEIPT_HEAD_NAME),
        format!("{}\n", hex(&Sha256::digest(&receipt_bytes))),
    )
    .unwrap();

    let request = Request {
        schema_version: 1,
        request_id: "323e4567-e89b-42d3-a456-426614174000".to_owned(),
        operation: Operation::Start,
        campaign_id: "1".repeat(64),
        expected_state_revision: 0,
        manifest_path: Some(
            campaign
                .join("campaign-start.json")
                .to_string_lossy()
                .into_owned(),
        ),
        manifest_sha256,
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
    signed_packet(request, key, now)
}

fn signed_packet(mut request: Request, key: &SigningKey, now: u64) -> Vec<u8> {
    request.controller.authorization_sha256 = "0".repeat(64);
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

fn seal_packet(start_packet: &[u8], key: &SigningKey, revision: u64, now: u64) -> Vec<u8> {
    let mut request: WireRequest = serde_json::from_slice(start_packet).unwrap();
    request.request.request_id = "623e4567-e89b-42d3-a456-426614174000".to_owned();
    request.request.operation = Operation::Seal;
    request.request.expected_state_revision = revision;
    request.request.manifest_path = None;
    signed_packet(request.request, key, now)
}

fn abort_packet(start_packet: &[u8], key: &SigningKey, revision: u64, now: u64) -> Vec<u8> {
    let mut request: WireRequest = serde_json::from_slice(start_packet).unwrap();
    request.request.request_id = "823e4567-e89b-42d3-a456-426614174000".to_owned();
    request.request.operation = Operation::Abort;
    request.request.expected_state_revision = revision;
    request.request.manifest_path = None;
    request.request.abort_reason = Some("operator-request".to_owned());
    request.request.approval_nonce_sha256 = Some("8".repeat(64));
    signed_packet(request.request, key, now)
}

fn prepare_i74_terminal_evidence(
    campaign_root: &Path,
    seal_packet: &[u8],
    now: u64,
) -> UnitSnapshot {
    let wire: WireRequest = serde_json::from_slice(seal_packet).unwrap();
    let request = wire.request;
    let lock = CampaignLock::acquire(campaign_root, &request.campaign_id).unwrap();
    let mut next = lock.read().unwrap();
    assert_eq!(next.revision, 2);
    assert_eq!(next.campaign_state, CampaignState::I74Running);
    let harness = next.harness.clone().unwrap();
    let daemon = next.daemon.clone().unwrap();
    let role_root = lock.campaign_directory().join("roles/i74");
    fs::create_dir_all(role_root.join("guards")).unwrap();
    let record = build_record(
        1,
        GENESIS_HASH,
        CheckpointPayload {
            campaign_id: request.campaign_id.clone(),
            role: Role::I74,
            phase: Phase::Terminal,
            phase_epoch: 1,
            monotonic_elapsed_ns: 1_000,
            wall_clock_utc: "2026-10-05T00:00:01Z".to_owned(),
            completed: 1,
            failed: 0,
            rejected: 0,
            timed_out: 0,
            outstanding: 0,
            telemetry_sequence: 1,
            milestone: "terminal".to_owned(),
            surface_counters: BTreeMap::new(),
            resource_counters: BTreeMap::new(),
            owner_counters: BTreeMap::new(),
            harness: harness.clone(),
            daemon: daemon.clone(),
        },
    )
    .unwrap();
    append_record(
        &role_root.join("checkpoints.jsonl"),
        &role_root.join("checkpoints.head"),
        &record,
    )
    .unwrap();
    for guard in ["semantic", "native-non-regression", "retention"] {
        fs::write(
            role_root.join(format!("guards/{guard}.json")),
            format!("{{\"guard\":\"{guard}\",\"passed\":true}}"),
        )
        .unwrap();
    }
    let inventory = SealInputInventory {
        schema_version: 1,
        release: "0.74".to_owned(),
        campaign_id: request.campaign_id.clone(),
        campaign_manifest_sha256: request.manifest_sha256.clone(),
        role: Role::I74,
        result: PacketResult::Complete,
        terminal_reason: None,
        journal_relative_path: "roles/i74/checkpoints.jsonl".into(),
        guard_evidence: ["semantic", "native-non-regression", "retention"]
            .into_iter()
            .map(|guard| InventoryGuardEvidence {
                id: guard.to_owned(),
                passed: true,
                source_relative_path: format!("roles/i74/guards/{guard}.json").into(),
            })
            .collect(),
        raw_files: [
            "campaign-start.json",
            "roles/i74/checkpoints.jsonl",
            "roles/i74/guards/native-non-regression.json",
            "roles/i74/guards/retention.json",
            "roles/i74/guards/semantic.json",
            "roles/i74/seal-input-inventory.json",
        ]
        .into_iter()
        .map(Into::into)
        .collect(),
    };
    fs::write(
        role_root.join(SEAL_INPUT_INVENTORY_NAME),
        canonical(&inventory),
    )
    .unwrap();

    next.revision = 3;
    next.checkpoint = Some(CheckpointHead {
        sequence: 1,
        record_sha256: record.record_sha256,
        useful_progress_unix_seconds: now,
    });
    next.controller_lease = Some(ControllerLease {
        holder_request_id: "723e4567-e89b-42d3-a456-426614174000".to_owned(),
        authorization_sha256: request.controller.authorization_sha256.clone(),
        repository_id: request.controller.repository_id,
        run_id: request.controller.run_id,
        actor_id: request.controller.actor_id,
        expires_unix_seconds: now + 300,
    });
    let attach_request = Request {
        schema_version: 1,
        request_id: "723e4567-e89b-42d3-a456-426614174000".to_owned(),
        operation: Operation::Attach,
        campaign_id: request.campaign_id.clone(),
        expected_state_revision: 2,
        manifest_path: None,
        manifest_sha256: request.manifest_sha256,
        controller: request.controller,
        abort_reason: None,
        approval_nonce_sha256: None,
    };
    let response = sign_response(ResponseBody {
        schema_version: 1,
        request_id: attach_request.request_id.clone(),
        campaign_id: attach_request.campaign_id.clone(),
        ok: true,
        state_revision: 3,
        server_time_unix_seconds: now,
        result: Some(serde_json::to_value(&next).unwrap()),
        error_code: None,
    })
    .unwrap();
    append_or_replay(
        &lock.campaign_directory().join(EVENT_JOURNAL_NAME),
        &lock.campaign_directory().join(EVENT_HEAD_NAME),
        now,
        attach_request,
        EventOutcome::Accepted,
        response,
        Some(next.clone()),
    )
    .unwrap();
    lock.compare_and_swap(2, &next).unwrap();
    UnitSnapshot {
        unit_name: harness.unit_name,
        active_state: "active".to_owned(),
        sub_state: "exited".to_owned(),
        main_pid: 0,
        control_group: harness.cgroup_path,
        result: "success".to_owned(),
    }
}

fn c74_start_packet(i74_packet: &[u8], key: &SigningKey, revision: u64, now: u64) -> Vec<u8> {
    let mut request: WireRequest = serde_json::from_slice(i74_packet).unwrap();
    request.request.request_id = "423e4567-e89b-42d3-a456-426614174000".to_owned();
    request.request.expected_state_revision = revision;
    request.request.manifest_path = None;
    signed_packet(request.request, key, now)
}

fn seal_i74_for_c74(campaign_root: &Path, i74_packet: &[u8], now: u64) {
    let request: WireRequest = serde_json::from_slice(i74_packet).unwrap();
    let digest = request_sha256(&request.request).unwrap();
    let lock = CampaignLock::acquire(campaign_root, &request.request.campaign_id).unwrap();
    let mut terminal = lock.read().unwrap();
    assert_eq!(terminal.revision, 2);
    terminal.revision = 3;
    terminal.campaign_state = CampaignState::I74Terminal;
    append_lifecycle_event(
        &lock.campaign_directory().join(EVENT_JOURNAL_NAME),
        &lock.campaign_directory().join(EVENT_HEAD_NAME),
        now,
        request.request.request_id.clone(),
        digest.clone(),
        LifecycleEvent::I74Terminal,
        terminal.clone(),
    )
    .unwrap();
    lock.compare_and_swap(2, &terminal).unwrap();

    let mut sealed = terminal;
    sealed.revision = 4;
    sealed.campaign_state = CampaignState::I74Sealed;
    sealed.harness = None;
    sealed.daemon = None;
    sealed.checkpoint = None;
    append_lifecycle_event(
        &lock.campaign_directory().join(EVENT_JOURNAL_NAME),
        &lock.campaign_directory().join(EVENT_HEAD_NAME),
        now,
        request.request.request_id,
        digest,
        LifecycleEvent::I74Sealed,
        sealed.clone(),
    )
    .unwrap();
    lock.compare_and_swap(3, &sealed).unwrap();
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

#[test]
fn seal_dispatch_fails_closed_before_dbus_or_host_claim_on_missing_evidence() {
    let temporary = tempfile::tempdir().unwrap();
    let campaign_root = temporary.path().join("campaigns");
    fs::create_dir(&campaign_root).unwrap();
    fs::create_dir(campaign_root.join("a".repeat(64))).unwrap();
    let lock = CampaignLock::acquire(&campaign_root, &"a".repeat(64)).unwrap();
    lock.initialize(&state(0)).unwrap();
    drop(lock);

    let key = SigningKey::from_bytes(&[7; 32]);
    let socket = temporary.path().join("supervisor-seal.sock");
    let server = SupervisorServer::bind(server_config(&socket, &campaign_root)).unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let packet = signed_packet(
        Request {
            schema_version: 1,
            request_id: "523e4567-e89b-42d3-a456-426614174000".to_owned(),
            operation: Operation::Seal,
            campaign_id: "a".repeat(64),
            expected_state_revision: 0,
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
        },
        &key,
        now,
    );
    let mut backend = FakeSealBackend {
        snapshot: UnitSnapshot {
            unit_name: "hc-a.service".to_owned(),
            active_state: "active".to_owned(),
            sub_state: "exited".to_owned(),
            main_pid: 0,
            control_group: "/hc/a".to_owned(),
            result: "success".to_owned(),
        },
        host_checks: 0,
        unit_checks: 0,
    };
    let response = exchange_seal_once(&server, &socket, &packet, &mut backend);
    assert!(!response.body.ok);
    assert_eq!(response.body.error_code, Some(5));
    assert_eq!(response.body.state_revision, 0);
    assert_eq!(backend.host_checks, 0);
    assert_eq!(backend.unit_checks, 0);
    assert!(!campaign_root.join(ACTIVE_CAMPAIGN_NAME).exists());
}

#[test]
fn seal_dispatch_composes_observation_inventory_and_durable_lifecycle() {
    let temporary = tempfile::tempdir().unwrap();
    let campaign_root = temporary.path().join("campaigns");
    let staging_root = temporary.path().join("staging");
    let seal_root = temporary.path().join("seals");
    fs::create_dir(&campaign_root).unwrap();
    fs::create_dir(&staging_root).unwrap();
    fs::create_dir(&seal_root).unwrap();
    let socket = temporary.path().join("supervisor-seal-positive.sock");
    let key = SigningKey::from_bytes(&[7; 32]);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let start_packet = stage_start(&staging_root, &key, now);
    let server = SupervisorServer::bind(server_config(&socket, &campaign_root)).unwrap();
    let mut start_backend = FakeStartBackend::default();
    let started = exchange_start_once(&server, &socket, &start_packet, &mut start_backend);
    assert!(started.body.ok);

    let packet = seal_packet(&start_packet, &key, 3, now);
    let snapshot = prepare_i74_terminal_evidence(&campaign_root, &packet, now);
    let mut backend = FakeSealBackend {
        snapshot,
        host_checks: 0,
        unit_checks: 0,
    };
    let packet_directory = seal_root.join(format!("{}-i74-packet", "1".repeat(64)));
    fs::create_dir(&packet_directory).unwrap();
    let interrupted = exchange_seal_once(&server, &socket, &packet, &mut backend);
    assert!(!interrupted.body.ok);
    assert_eq!(interrupted.body.state_revision, 4);
    assert_eq!(interrupted.body.error_code, Some(11));
    fs::remove_dir(&packet_directory).unwrap();

    let sealed = exchange_seal_once(&server, &socket, &packet, &mut backend);
    assert!(sealed.body.ok);
    assert_eq!(sealed.body.state_revision, 5);
    assert_eq!(
        sealed.body.result.as_ref().unwrap()["state"]["campaign_state"],
        "I74_SEALED"
    );
    assert_eq!(backend.host_checks, 2);
    assert_eq!(backend.unit_checks, 2);
    assert!(packet_directory.is_dir());
    assert!(seal_root
        .join(format!("{}-i74-archive", "1".repeat(64)))
        .is_dir());

    let replay = exchange_seal_once(&server, &socket, &packet, &mut backend);
    assert_eq!(replay, sealed);
    assert_eq!(backend.host_checks, 3);
    assert_eq!(backend.unit_checks, 2);
}

#[test]
fn abort_dispatch_reports_committed_intent_and_identical_retry_completes() {
    let temporary = tempfile::tempdir().unwrap();
    let campaign_root = temporary.path().join("campaigns");
    let staging_root = temporary.path().join("staging");
    let seal_root = temporary.path().join("seals");
    fs::create_dir(&campaign_root).unwrap();
    fs::create_dir(&staging_root).unwrap();
    fs::create_dir(&seal_root).unwrap();
    let socket = temporary.path().join("supervisor-abort.sock");
    let key = SigningKey::from_bytes(&[7; 32]);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let start_packet = stage_start(&staging_root, &key, now);
    let server = SupervisorServer::bind(server_config(&socket, &campaign_root)).unwrap();
    let mut start_backend = FakeStartBackend::default();
    let started = exchange_start_once(&server, &socket, &start_packet, &mut start_backend);
    assert!(started.body.ok);

    let seal = seal_packet(&start_packet, &key, 3, now);
    prepare_i74_terminal_evidence(&campaign_root, &seal, now);
    let abort = abort_packet(&start_packet, &key, 3, now);
    let mut backend = FakeAbortBackend {
        calls: 0,
        fail_next: true,
    };
    let interrupted = exchange_abort_once(&server, &socket, &abort, &mut backend);
    assert!(!interrupted.body.ok);
    assert_eq!(interrupted.body.error_code, Some(11));
    assert_eq!(interrupted.body.state_revision, 4);
    assert!(campaign_root.join(ACTIVE_CAMPAIGN_NAME).exists());

    let completed = exchange_abort_once(&server, &socket, &abort, &mut backend);
    assert!(completed.body.ok);
    assert_eq!(completed.body.state_revision, 5);
    assert_eq!(
        completed.body.result.as_ref().unwrap()["campaign_state"],
        "ABORTED_INCOMPLETE"
    );
    assert_eq!(backend.calls, 2);
    assert!(!campaign_root.join(ACTIVE_CAMPAIGN_NAME).exists());

    let replay = exchange_abort_once(&server, &socket, &abort, &mut backend);
    assert_eq!(replay, completed);
    assert_eq!(backend.calls, 2);
    assert!(!campaign_root.join(ACTIVE_CAMPAIGN_NAME).exists());
}

#[test]
fn authorized_role_starts_import_evidence_execute_once_and_replay_exact_responses() {
    let temporary = tempfile::tempdir().unwrap();
    let campaign_root = temporary.path().join("campaigns");
    let staging_root = temporary.path().join("staging");
    fs::create_dir(&campaign_root).unwrap();
    fs::create_dir(&staging_root).unwrap();
    let socket = temporary.path().join("supervisor-start.sock");
    let key = SigningKey::from_bytes(&[7; 32]);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let packet = stage_start(&staging_root, &key, now);
    let server = SupervisorServer::bind(server_config(&socket, &campaign_root)).unwrap();
    let mut backend = FakeStartBackend::default();

    let first = exchange_start_once(&server, &socket, &packet, &mut backend);
    assert!(first.body.ok);
    assert_eq!(first.body.state_revision, 2);
    assert_eq!(backend.starts, 1);
    let campaign = campaign_root.join("1".repeat(64));
    assert!(campaign.join("state.json").exists());
    assert!(campaign.join("i74-spawn-intent.json").exists());
    assert!(campaign.join("i74-spawn-result.json").exists());

    let replay = exchange_start_once(&server, &socket, &packet, &mut backend);
    assert_eq!(replay, first);
    assert_eq!(backend.starts, 1);

    seal_i74_for_c74(&campaign_root, &packet, now);
    let c74_packet = c74_start_packet(&packet, &key, 4, now);
    let c74 = exchange_start_once(&server, &socket, &c74_packet, &mut backend);
    assert!(c74.body.ok);
    assert_eq!(c74.body.state_revision, 6);
    assert_eq!(backend.starts, 2);
    assert!(campaign.join("c74-spawn-intent.json").exists());
    assert!(campaign.join("c74-spawn-result.json").exists());

    let c74_replay = exchange_start_once(&server, &socket, &c74_packet, &mut backend);
    assert_eq!(c74_replay, c74);
    assert_eq!(backend.starts, 2);
}
