#![cfg(target_os = "linux")]
#![recursion_limit = "256"]

use ed25519_dalek::{Signer, SigningKey};
use hydracache_long_run_supervisor_074::abort_lifecycle::AbortBackend;
use hydracache_long_run_supervisor_074::artifact::PacketResult;
use hydracache_long_run_supervisor_074::auth::{
    canonical_document, canonical_message, AuthorizationBody, SignedAuthorization,
};
use hydracache_long_run_supervisor_074::client::{exchange, exchange_start_with_evidence};
use hydracache_long_run_supervisor_074::config::ServerConfig;
use hydracache_long_run_supervisor_074::event::{
    append_lifecycle_event, append_or_replay, request_sha256, verify_event_journal, EventOutcome,
    LifecycleEvent, EVENT_HEAD_NAME, EVENT_JOURNAL_NAME,
};
use hydracache_long_run_supervisor_074::host_execution::ACTIVE_CAMPAIGN_NAME;
use hydracache_long_run_supervisor_074::host_receipt::{
    encode_canonical as encode_host_receipt, mount_identity_digest, BinaryIdentity,
    HostObservationReceipt, MountIdentity, HOST_RECEIPT_HEAD_NAME, HOST_RECEIPT_NAME,
    SUPERVISOR_BINARY_PATH,
};
use hydracache_long_run_supervisor_074::lease_expiry::{
    LeaseExpiryBackend, LeaseExpiryCause, LeaseExpiryOutcome,
};
use hydracache_long_run_supervisor_074::manifest::CampaignManifest;
use hydracache_long_run_supervisor_074::measurement_loss::{
    MeasurementLossBackend, MeasurementLossCause, MeasurementLossOutcome, MeasurementLossReason,
    MeasurementObservation,
};
use hydracache_long_run_supervisor_074::progress_loss::{
    ProgressLossBackend, ProgressLossCause, ProgressLossOutcome,
};
use hydracache_long_run_supervisor_074::protocol::{
    sign_response, ControllerIdentity, HostObservationResult, Operation, Request, ResponseBody,
    WireRequest, HOST_OBSERVATION_CAMPAIGN_SCOPE_SHA256, HOST_OBSERVATION_MANIFEST_SCOPE_SHA256,
    MAX_PACKET_BYTES,
};
use hydracache_long_run_supervisor_074::seal_input::{
    InventoryGuardEvidence, SealInputInventory, SEAL_INPUT_INVENTORY_NAME,
};
use hydracache_long_run_supervisor_074::server::{
    AbortObservationBackend, HostObservationBackend, LeaseExpiryObservationBackend,
    MeasurementLossObservationBackend, ProgressLossObservationBackend, SealObservationBackend,
    StartObservationBackend, SupervisorServer,
};
use hydracache_long_run_supervisor_074::spawn::{SpawnBackend, SpawnIntent, SpawnObservation};
use hydracache_long_run_supervisor_074::state::{
    CampaignState, CheckpointHead, ControllerLease, DurableCampaignState, FrozenIdentity,
};
use hydracache_long_run_supervisor_074::state_store::CampaignLock;
use hydracache_long_run_supervisor_074::systemd_unit::UnitSnapshot;
use hydracache_long_run_supervisor_074::unix_transport::SeqpacketConnection;
use hydracache_long_run_supervisor_074::{
    append_record, build_record, CheckpointPayload, Phase, ProcessIdentity, Role, GENESIS_HASH,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
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

fn host_observation_request() -> Vec<u8> {
    serde_json::to_vec(&WireRequest {
        request: Request {
            schema_version: 1,
            request_id: "323e4567-e89b-42d3-a456-426614174000".to_owned(),
            operation: Operation::HostObservation,
            campaign_id: HOST_OBSERVATION_CAMPAIGN_SCOPE_SHA256.to_owned(),
            expected_state_revision: 0,
            manifest_path: None,
            manifest_sha256: HOST_OBSERVATION_MANIFEST_SCOPE_SHA256.to_owned(),
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

#[test]
fn diagnostic_only_reservation_skips_campaign_maintenance_without_busy_or_backend_calls() {
    use hydracache_long_run_supervisor_074::diagnostic_lease::{
        DiagnosticClock, DiagnosticCoordinator, DiagnosticIdentity,
    };
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("campaigns");
    fs::create_dir(&root).unwrap();
    fs::create_dir(temporary.path().join("staging")).unwrap();
    fs::create_dir(temporary.path().join("seals")).unwrap();
    let boot = "00000000-0000-4000-8000-000000000074";
    let _lease = DiagnosticCoordinator::reserve(
        &root,
        DiagnosticIdentity {
            lease_id: "a".repeat(64),
            boot_id: boot.into(),
            binary_sha256: "b".repeat(64),
            build_provenance_sha256: "c".repeat(64),
        },
        &DiagnosticClock {
            boot_id: boot.into(),
            monotonic_ns: 0,
        },
    )
    .unwrap();
    let server = SupervisorServer::bind(server_config(
        &temporary.path().join("diagnostic.sock"),
        &root,
    ))
    .unwrap();
    let mut expiry = FakeLeaseExpiryBackend::default();
    let mut progress = FakeProgressLossBackend::default();
    let mut measurement = FakeMeasurementLossBackend::new(MeasurementObservation::Healthy);
    assert!(server
        .maintain_lease_expiry_with_backend(1, &mut expiry)
        .unwrap()
        .is_none());
    assert!(server
        .maintain_measurement_loss_with_backend(1, &mut measurement)
        .unwrap()
        .is_none());
    assert!(server
        .maintain_progress_loss_with_backend(1, &mut progress)
        .unwrap()
        .is_none());
    assert_eq!(
        expiry.host_checks
            + expiry.stop_calls
            + progress.host_checks
            + progress.stop_calls
            + measurement.prepare_calls
            + measurement.observe_calls
            + measurement.stop_calls,
        0
    );
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

fn exchange_start_once<B: SpawnBackend + StartObservationBackend + Send>(
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

fn exchange_host_observation_once<B: HostObservationBackend + Send>(
    server: &SupervisorServer,
    socket: &Path,
    packet: &[u8],
    backend: &mut B,
) -> hydracache_long_run_supervisor_074::protocol::Response {
    std::thread::scope(|scope| {
        let client = scope.spawn(|| exchange(socket, packet));
        server
            .serve_one_with_host_observation_backend(backend)
            .unwrap();
        client.join().unwrap().unwrap()
    })
}

fn exchange_uploaded_start_once<B: SpawnBackend + StartObservationBackend + Send>(
    server: &SupervisorServer,
    socket: &Path,
    packet: &[u8],
    manifest: &[u8],
    host_receipt: &[u8],
    backend: &mut B,
) -> hydracache_long_run_supervisor_074::protocol::Response {
    std::thread::scope(|scope| {
        let client =
            scope.spawn(|| exchange_start_with_evidence(socket, packet, manifest, host_receipt));
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

#[derive(Default)]
struct FakeLeaseExpiryBackend {
    host_checks: usize,
    stop_calls: usize,
}

impl LeaseExpiryBackend for FakeLeaseExpiryBackend {
    fn capture_and_stop_expired(
        &mut self,
        _campaign_directory: &Path,
        cause: &LeaseExpiryCause,
        state: &DurableCampaignState,
    ) -> Result<(), String> {
        self.stop_calls += 1;
        assert_eq!(cause.campaign_id, state.identity.campaign_id);
        assert_eq!(cause.lease_id, state.identity.lease_id);
        assert_eq!(state.campaign_state, CampaignState::LeaseExpiredIncomplete);
        assert!(state.harness.is_some());
        assert!(state.daemon.is_some());
        Ok(())
    }
}

impl LeaseExpiryObservationBackend for FakeLeaseExpiryBackend {
    fn verify_host(
        &mut self,
        _campaign_directory: &Path,
        _manifest: &CampaignManifest,
        _state: &DurableCampaignState,
    ) -> Result<(), String> {
        self.host_checks += 1;
        Ok(())
    }
}

#[derive(Default)]
struct FakeProgressLossBackend {
    host_checks: usize,
    stop_calls: usize,
    startup_causes: usize,
}

impl ProgressLossBackend for FakeProgressLossBackend {
    fn capture_and_stop_stalled(
        &mut self,
        _campaign_directory: &Path,
        cause: &ProgressLossCause,
        state: &DurableCampaignState,
    ) -> Result<(), String> {
        self.stop_calls += 1;
        assert_eq!(cause.campaign_id, state.identity.campaign_id);
        assert_eq!(state.campaign_state, CampaignState::FailedIncomplete);
        assert_eq!(state.checkpoint, cause.checkpoint);
        assert!(state.harness.is_some());
        assert!(state.daemon.is_some());
        if cause.checkpoint.is_none() {
            self.startup_causes += 1;
        }
        Ok(())
    }
}

impl ProgressLossObservationBackend for FakeProgressLossBackend {
    fn verify_host(
        &mut self,
        _campaign_directory: &Path,
        _manifest: &CampaignManifest,
        _state: &DurableCampaignState,
    ) -> Result<(), String> {
        self.host_checks += 1;
        Ok(())
    }
}

struct FakeMeasurementLossBackend {
    observation: MeasurementObservation,
    prepare_calls: usize,
    observe_calls: usize,
    stop_calls: usize,
    fail_next: bool,
}

impl FakeMeasurementLossBackend {
    fn new(observation: MeasurementObservation) -> Self {
        Self {
            observation,
            prepare_calls: 0,
            observe_calls: 0,
            stop_calls: 0,
            fail_next: false,
        }
    }
}

impl MeasurementLossBackend for FakeMeasurementLossBackend {
    fn capture_and_stop_lost(
        &mut self,
        _campaign_directory: &Path,
        cause: &MeasurementLossCause,
        state: &DurableCampaignState,
    ) -> Result<(), String> {
        self.stop_calls += 1;
        assert_eq!(state.campaign_state, CampaignState::FailedIncomplete);
        assert!(state.recorded_failure);
        assert_eq!(cause.campaign_id, state.identity.campaign_id);
        assert_eq!(cause.harness, state.harness.clone().unwrap());
        assert_eq!(cause.daemon, state.daemon.clone().unwrap());
        if self.fail_next {
            self.fail_next = false;
            return Err("injected measurement-loss interruption".to_owned());
        }
        Ok(())
    }
}

impl MeasurementLossObservationBackend for FakeMeasurementLossBackend {
    fn prepare_measurement(
        &mut self,
        manifest: &CampaignManifest,
        state: &DurableCampaignState,
    ) -> Result<(), String> {
        self.prepare_calls += 1;
        assert_eq!(manifest.campaign_id, state.identity.campaign_id);
        Ok(())
    }

    fn observe_measurement(
        &mut self,
        _campaign_directory: &Path,
        _manifest: &CampaignManifest,
        _state: &DurableCampaignState,
    ) -> Result<MeasurementObservation, String> {
        self.observe_calls += 1;
        Ok(self.observation)
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
    host_checks: usize,
    fail_host: bool,
}

struct FakeHostObservationBackend {
    receipt: HostObservationReceipt,
    calls: usize,
    fail: bool,
}

impl HostObservationBackend for FakeHostObservationBackend {
    fn collect(
        &mut self,
        _campaign_root: &Path,
    ) -> Result<(HostObservationReceipt, String, BinaryIdentity), String> {
        self.calls += 1;
        if self.fail {
            Err("host observation unavailable".to_owned())
        } else {
            let mut fixture = self.receipt.supervisor_binary.clone();
            fixture.path = "/opt/hydracache-performance/0.74/campaign-lifecycle-fixture".to_owned();
            Ok((self.receipt.clone(), "d".repeat(40), fixture))
        }
    }
}

impl StartObservationBackend for FakeStartBackend {
    fn verify_host(
        &mut self,
        _campaign_directory: &Path,
        _manifest: &CampaignManifest,
        _admitted: &HostObservationReceipt,
    ) -> Result<(), String> {
        self.host_checks += 1;
        if self.fail_host {
            Err("host observation drifted".to_owned())
        } else {
            Ok(())
        }
    }
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

fn sample_host_receipt() -> HostObservationReceipt {
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
    let mount_identity = mount_identity_digest(&mount).unwrap();
    HostObservationReceipt {
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
    }
}

fn stage_start(staging_root: &Path, key: &SigningKey, now: u64) -> Vec<u8> {
    let campaign_id = "1".repeat(64);
    let campaign = staging_root.join(&campaign_id);
    fs::create_dir(&campaign).unwrap();
    let receipt = sample_host_receipt();
    let mount_identity = receipt.mount_identity.clone();
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
    let now = durable_event_time(&lock, now);
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
            observed_unix_seconds: now,
            useful_progress_unix_seconds: now,
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
    let now = durable_event_time(&lock, now);
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

fn durable_event_time(lock: &CampaignLock, proposed: u64) -> u64 {
    verify_event_journal(
        &lock.campaign_directory().join(EVENT_JOURNAL_NAME),
        &lock.campaign_directory().join(EVENT_HEAD_NAME),
    )
    .map_or(proposed, |report| {
        proposed.max(report.last_occurred_at_unix_seconds)
    })
}

#[test]
fn host_observation_returns_digest_bound_receipt_without_campaign_mutation() {
    let temporary = tempfile::tempdir().unwrap();
    let campaign_root = temporary.path().join("campaigns");
    fs::create_dir(&campaign_root).unwrap();
    let socket = temporary.path().join("supervisor-host-observation.sock");
    let server = SupervisorServer::bind(server_config(&socket, &campaign_root)).unwrap();
    let receipt = sample_host_receipt();
    let expected_digest = hex(&Sha256::digest(encode_host_receipt(&receipt).unwrap()));
    let mut backend = FakeHostObservationBackend {
        receipt: receipt.clone(),
        calls: 0,
        fail: false,
    };

    let response =
        exchange_host_observation_once(&server, &socket, &host_observation_request(), &mut backend);
    assert!(response.body.ok);
    assert_eq!(response.body.state_revision, 0);
    assert_eq!(backend.calls, 1);
    let result: HostObservationResult =
        serde_json::from_value(response.body.result.clone().unwrap()).unwrap();
    assert_eq!(result.schema_version, 1);
    assert_eq!(result.installed_source_commit, "d".repeat(40));
    assert!(result.active_campaign_absent);
    assert_eq!(
        result.fixture_binary.path,
        "/opt/hydracache-performance/0.74/campaign-lifecycle-fixture"
    );
    assert_eq!(result.receipt_sha256, expected_digest);
    assert_eq!(result.receipt, receipt);
    assert!(serde_json::to_vec(&response).unwrap().len() <= MAX_PACKET_BYTES);
    assert!(!campaign_root
        .join(HOST_OBSERVATION_CAMPAIGN_SCOPE_SHA256)
        .exists());
    assert!(!campaign_root.join(ACTIVE_CAMPAIGN_NAME).exists());
}

#[test]
fn host_observation_rejects_an_active_campaign_before_collecting() {
    let temporary = tempfile::tempdir().unwrap();
    let campaign_root = temporary.path().join("campaigns");
    fs::create_dir(&campaign_root).unwrap();
    fs::write(
        campaign_root.join(ACTIVE_CAMPAIGN_NAME),
        format!("{}\n", "a".repeat(64)),
    )
    .unwrap();
    let socket = temporary.path().join("supervisor-host-claimed.sock");
    let server = SupervisorServer::bind(server_config(&socket, &campaign_root)).unwrap();
    let mut backend = FakeHostObservationBackend {
        receipt: sample_host_receipt(),
        calls: 0,
        fail: false,
    };

    let response =
        exchange_host_observation_once(&server, &socket, &host_observation_request(), &mut backend);
    assert!(!response.body.ok);
    assert_eq!(response.body.error_code, Some(6));
    assert_eq!(backend.calls, 1);
}

#[test]
fn host_observation_failure_is_bounded_and_does_not_poison_the_next_request() {
    let temporary = tempfile::tempdir().unwrap();
    let campaign_root = temporary.path().join("campaigns");
    fs::create_dir(&campaign_root).unwrap();
    let socket = temporary
        .path()
        .join("supervisor-host-observation-retry.sock");
    let server = SupervisorServer::bind(server_config(&socket, &campaign_root)).unwrap();
    let mut backend = FakeHostObservationBackend {
        receipt: sample_host_receipt(),
        calls: 0,
        fail: true,
    };

    let rejected =
        exchange_host_observation_once(&server, &socket, &host_observation_request(), &mut backend);
    assert!(!rejected.body.ok);
    assert_eq!(rejected.body.error_code, Some(6));
    assert_eq!(rejected.body.state_revision, 0);
    assert_eq!(backend.calls, 1);

    backend.fail = false;
    let accepted =
        exchange_host_observation_once(&server, &socket, &host_observation_request(), &mut backend);
    assert!(accepted.body.ok);
    assert_eq!(backend.calls, 2);
    assert!(!campaign_root
        .join(HOST_OBSERVATION_CAMPAIGN_SCOPE_SHA256)
        .exists());
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
fn maintenance_expires_the_active_campaign_without_a_controller_request() {
    let temporary = tempfile::tempdir().unwrap();
    let campaign_root = temporary.path().join("campaigns");
    let staging_root = temporary.path().join("staging");
    let seal_root = temporary.path().join("seals");
    fs::create_dir(&campaign_root).unwrap();
    fs::create_dir(&staging_root).unwrap();
    fs::create_dir(&seal_root).unwrap();
    let socket = temporary.path().join("supervisor-expiry.sock");
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

    let mut backend = FakeLeaseExpiryBackend::default();
    assert_eq!(
        server
            .maintain_lease_expiry_with_backend(now + 10_000, &mut backend)
            .unwrap(),
        Some(LeaseExpiryOutcome::NotDue)
    );
    assert_eq!(backend.host_checks, 0);
    assert_eq!(backend.stop_calls, 0);
    assert_eq!(
        server
            .maintain_lease_expiry_with_backend(now + 10_001, &mut backend)
            .unwrap(),
        Some(LeaseExpiryOutcome::Completed { state_revision: 4 })
    );
    assert_eq!(backend.host_checks, 1);
    assert_eq!(backend.stop_calls, 1);
    let lock = CampaignLock::acquire(&campaign_root, &"1".repeat(64)).unwrap();
    let expired = lock.read().unwrap();
    assert_eq!(
        expired.campaign_state,
        CampaignState::LeaseExpiredIncomplete
    );
    assert!(expired.harness.is_none());
    assert!(!campaign_root.join(ACTIVE_CAMPAIGN_NAME).exists());
    drop(lock);

    assert_eq!(
        server
            .maintain_lease_expiry_with_backend(now + 10_002, &mut backend)
            .unwrap(),
        None
    );
    assert_eq!(backend.stop_calls, 1);
}

#[test]
fn measurement_maintenance_distinguishes_healthy_terminal_and_lost_processes() {
    let temporary = tempfile::tempdir().unwrap();
    let campaign_root = temporary.path().join("campaigns");
    let staging_root = temporary.path().join("staging");
    let seal_root = temporary.path().join("seals");
    fs::create_dir(&campaign_root).unwrap();
    fs::create_dir(&staging_root).unwrap();
    fs::create_dir(&seal_root).unwrap();
    let socket = temporary.path().join("supervisor-measurement-loss.sock");
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

    let mut backend = FakeMeasurementLossBackend::new(MeasurementObservation::Healthy);
    assert_eq!(
        server
            .maintain_measurement_loss_with_backend(now + 1, &mut backend)
            .unwrap(),
        None
    );
    backend.observation = MeasurementObservation::Terminal;
    assert_eq!(
        server
            .maintain_measurement_loss_with_backend(now + 2, &mut backend)
            .unwrap(),
        None
    );
    assert_eq!(backend.prepare_calls, 2);
    assert_eq!(backend.observe_calls, 2);
    assert_eq!(backend.stop_calls, 0);
    assert!(campaign_root.join(ACTIVE_CAMPAIGN_NAME).exists());

    backend.observation = MeasurementObservation::Lost(MeasurementLossReason::ProcessIdentityDrift);
    assert_eq!(
        server
            .maintain_measurement_loss_with_backend(now + 3, &mut backend)
            .unwrap(),
        Some(MeasurementLossOutcome::Completed { state_revision: 4 })
    );
    assert_eq!(backend.prepare_calls, 3);
    assert_eq!(backend.observe_calls, 3);
    assert_eq!(backend.stop_calls, 1);
    let lock = CampaignLock::acquire(&campaign_root, &"1".repeat(64)).unwrap();
    let failed = lock.read().unwrap();
    assert_eq!(failed.campaign_state, CampaignState::FailedIncomplete);
    assert!(failed.recorded_failure);
    assert!(failed.harness.is_none());
    assert!(failed.daemon.is_none());
    assert!(failed.checkpoint.is_none());
    assert!(failed.controller_lease.is_none());
    assert!(!campaign_root.join(ACTIVE_CAMPAIGN_NAME).exists());
}

#[test]
fn measurement_maintenance_recovers_committed_intent_before_lease_expiry() {
    let temporary = tempfile::tempdir().unwrap();
    let campaign_root = temporary.path().join("campaigns");
    let staging_root = temporary.path().join("staging");
    let seal_root = temporary.path().join("seals");
    fs::create_dir(&campaign_root).unwrap();
    fs::create_dir(&staging_root).unwrap();
    fs::create_dir(&seal_root).unwrap();
    let socket = temporary
        .path()
        .join("supervisor-measurement-loss-recovery.sock");
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

    let mut backend = FakeMeasurementLossBackend::new(MeasurementObservation::Lost(
        MeasurementLossReason::UnitAbsent,
    ));
    backend.fail_next = true;
    assert!(server
        .maintain_measurement_loss_with_backend(now + 1, &mut backend)
        .is_err());
    assert_eq!(backend.prepare_calls, 1);
    assert_eq!(backend.observe_calls, 1);
    assert_eq!(backend.stop_calls, 1);
    let lock = CampaignLock::acquire(&campaign_root, &"1".repeat(64)).unwrap();
    let requested = lock.read().unwrap();
    assert_eq!(requested.revision, 3);
    assert_eq!(requested.campaign_state, CampaignState::FailedIncomplete);
    assert!(requested.recorded_failure);
    assert!(requested.harness.is_some());
    drop(lock);

    let mut lease_backend = FakeLeaseExpiryBackend::default();
    assert_eq!(
        server
            .maintain_lease_expiry_with_backend(now + 10_001, &mut lease_backend)
            .unwrap(),
        Some(LeaseExpiryOutcome::NotDue)
    );
    assert_eq!(lease_backend.host_checks, 0);
    assert_eq!(lease_backend.stop_calls, 0);

    backend.observation = MeasurementObservation::Healthy;
    assert_eq!(
        server
            .maintain_measurement_loss_with_backend(now + 10_001, &mut backend)
            .unwrap(),
        Some(MeasurementLossOutcome::Completed { state_revision: 4 })
    );
    assert_eq!(backend.prepare_calls, 2);
    assert_eq!(backend.observe_calls, 1);
    assert_eq!(backend.stop_calls, 2);
    assert!(!campaign_root.join(ACTIVE_CAMPAIGN_NAME).exists());
}

#[test]
fn maintenance_fails_startup_that_never_publishes_a_first_checkpoint() {
    let temporary = tempfile::tempdir().unwrap();
    let campaign_root = temporary.path().join("campaigns");
    let staging_root = temporary.path().join("staging");
    let seal_root = temporary.path().join("seals");
    fs::create_dir(&campaign_root).unwrap();
    fs::create_dir(&staging_root).unwrap();
    fs::create_dir(&seal_root).unwrap();
    let socket = temporary
        .path()
        .join("supervisor-startup-progress-loss.sock");
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

    let campaign_directory = campaign_root.join("1".repeat(64));
    let report = verify_event_journal(
        &campaign_directory.join(EVENT_JOURNAL_NAME),
        &campaign_directory.join(EVENT_HEAD_NAME),
    )
    .unwrap();
    let startup_progress = report
        .latest_lifecycle
        .as_ref()
        .unwrap()
        .occurred_at_unix_seconds;
    let mut backend = FakeProgressLossBackend::default();
    assert_eq!(
        server
            .maintain_progress_loss_with_backend(startup_progress + 180, &mut backend)
            .unwrap(),
        Some(ProgressLossOutcome::NotDue)
    );
    assert_eq!(backend.host_checks, 0);
    assert_eq!(backend.stop_calls, 0);
    let role = campaign_directory.join("roles/i74");
    fs::create_dir_all(&role).unwrap();
    fs::File::create(role.join("checkpoints.jsonl")).unwrap();

    assert_eq!(
        server
            .maintain_progress_loss_with_backend(startup_progress + 181, &mut backend)
            .unwrap(),
        Some(ProgressLossOutcome::Completed { state_revision: 4 })
    );
    assert_eq!(backend.host_checks, 1);
    assert_eq!(backend.stop_calls, 1);
    assert_eq!(backend.startup_causes, 1);
    let lock = CampaignLock::acquire(&campaign_root, &"1".repeat(64)).unwrap();
    let failed = lock.read().unwrap();
    assert_eq!(failed.campaign_state, CampaignState::FailedIncomplete);
    assert!(failed.harness.is_none());
    assert!(failed.daemon.is_none());
    assert!(failed.checkpoint.is_none());
    assert!(!campaign_root.join(ACTIVE_CAMPAIGN_NAME).exists());
}

#[test]
fn maintenance_fails_stalled_progress_from_the_live_checkpoint_chain() {
    let temporary = tempfile::tempdir().unwrap();
    let campaign_root = temporary.path().join("campaigns");
    let staging_root = temporary.path().join("staging");
    let seal_root = temporary.path().join("seals");
    fs::create_dir(&campaign_root).unwrap();
    fs::create_dir(&staging_root).unwrap();
    fs::create_dir(&seal_root).unwrap();
    let socket = temporary.path().join("supervisor-progress-loss.sock");
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

    let lock = CampaignLock::acquire(&campaign_root, &"1".repeat(64)).unwrap();
    let running = lock.read().unwrap();
    let role = lock.campaign_directory().join("roles/i74");
    fs::create_dir_all(&role).unwrap();
    let record = build_record(
        1,
        GENESIS_HASH,
        CheckpointPayload {
            campaign_id: running.identity.campaign_id.clone(),
            role: Role::I74,
            phase: Phase::Measured,
            phase_epoch: 1,
            monotonic_elapsed_ns: 1_000,
            wall_clock_utc: "2026-10-05T00:00:01Z".to_owned(),
            observed_unix_seconds: now,
            useful_progress_unix_seconds: now,
            completed: 1,
            failed: 0,
            rejected: 0,
            timed_out: 0,
            outstanding: 0,
            telemetry_sequence: 1,
            milestone: "measured".to_owned(),
            surface_counters: BTreeMap::new(),
            resource_counters: BTreeMap::new(),
            owner_counters: BTreeMap::new(),
            harness: running.harness.clone().unwrap(),
            daemon: running.daemon.clone().unwrap(),
        },
    )
    .unwrap();
    append_record(
        &role.join("checkpoints.jsonl"),
        &role.join("checkpoints.head"),
        &record,
    )
    .unwrap();
    drop(lock);

    let mut backend = FakeProgressLossBackend::default();
    assert_eq!(
        server
            .maintain_progress_loss_with_backend(now + 180, &mut backend)
            .unwrap(),
        Some(ProgressLossOutcome::NotDue)
    );
    assert_eq!(backend.host_checks, 0);
    assert_eq!(backend.stop_calls, 0);
    let second = build_record(
        2,
        &record.record_sha256,
        CheckpointPayload {
            campaign_id: running.identity.campaign_id.clone(),
            role: Role::I74,
            phase: Phase::Measured,
            phase_epoch: 1,
            monotonic_elapsed_ns: 2_000,
            wall_clock_utc: "2026-10-05T00:00:02Z".to_owned(),
            observed_unix_seconds: now + 170,
            useful_progress_unix_seconds: now + 170,
            completed: 2,
            failed: 0,
            rejected: 0,
            timed_out: 0,
            outstanding: 0,
            telemetry_sequence: 2,
            milestone: "measured".to_owned(),
            surface_counters: BTreeMap::new(),
            resource_counters: BTreeMap::new(),
            owner_counters: BTreeMap::new(),
            harness: running.harness.clone().unwrap(),
            daemon: running.daemon.clone().unwrap(),
        },
    )
    .unwrap();
    append_record(
        &role.join("checkpoints.jsonl"),
        &role.join("checkpoints.head"),
        &second,
    )
    .unwrap();
    assert_eq!(
        server
            .maintain_progress_loss_with_backend(now + 181, &mut backend)
            .unwrap(),
        Some(ProgressLossOutcome::NotDue)
    );
    assert_eq!(
        server
            .maintain_progress_loss_with_backend(now + 351, &mut backend)
            .unwrap(),
        Some(ProgressLossOutcome::Completed { state_revision: 4 })
    );
    assert_eq!(backend.host_checks, 1);
    assert_eq!(backend.stop_calls, 1);
    let lock = CampaignLock::acquire(&campaign_root, &"1".repeat(64)).unwrap();
    let failed = lock.read().unwrap();
    assert_eq!(failed.campaign_state, CampaignState::FailedIncomplete);
    assert!(failed.harness.is_none());
    assert!(failed.checkpoint.is_none());
    assert!(!campaign_root.join(ACTIVE_CAMPAIGN_NAME).exists());
    drop(lock);

    assert_eq!(
        server
            .maintain_progress_loss_with_backend(now + 352, &mut backend)
            .unwrap(),
        None
    );
    assert_eq!(backend.stop_calls, 1);
}

#[test]
fn revision_zero_start_uploads_evidence_through_the_privileged_supervisor() {
    let temporary = tempfile::tempdir().unwrap();
    let campaign_root = temporary.path().join("campaigns");
    let staging_root = temporary.path().join("staging");
    fs::create_dir(&campaign_root).unwrap();
    fs::create_dir(&staging_root).unwrap();
    let socket = temporary.path().join("supervisor-upload-start.sock");
    let key = SigningKey::from_bytes(&[7; 32]);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let packet = stage_start(&staging_root, &key, now);
    let staged = staging_root.join("1".repeat(64));
    let manifest = fs::read(staged.join("campaign-start.json")).unwrap();
    let host_receipt = fs::read(staged.join(HOST_RECEIPT_NAME)).unwrap();
    fs::remove_dir_all(&staged).unwrap();

    let server = SupervisorServer::bind(server_config(&socket, &campaign_root)).unwrap();
    let mut backend = FakeStartBackend::default();
    let response = exchange_uploaded_start_once(
        &server,
        &socket,
        &packet,
        &manifest,
        &host_receipt,
        &mut backend,
    );

    assert!(response.body.ok);
    assert_eq!(backend.starts, 1);
    assert_eq!(
        fs::read(staged.join("campaign-start.json")).unwrap(),
        manifest
    );
    assert_eq!(
        fs::read(staged.join(HOST_RECEIPT_NAME)).unwrap(),
        host_receipt
    );
    assert_eq!(
        fs::metadata(&staged).unwrap().permissions().mode() & 0o777,
        0o750
    );
    assert_eq!(
        fs::metadata(staged.join("campaign-start.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o400
    );
    assert!(campaign_root
        .join("1".repeat(64))
        .join("state.json")
        .exists());
}

#[test]
fn revision_zero_upload_rejects_tampered_receipt_before_staging_or_spawn() {
    let temporary = tempfile::tempdir().unwrap();
    let campaign_root = temporary.path().join("campaigns");
    let staging_root = temporary.path().join("staging");
    fs::create_dir(&campaign_root).unwrap();
    fs::create_dir(&staging_root).unwrap();
    let socket = temporary.path().join("supervisor-upload-reject.sock");
    let key = SigningKey::from_bytes(&[7; 32]);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let packet = stage_start(&staging_root, &key, now);
    let staged = staging_root.join("1".repeat(64));
    let manifest = fs::read(staged.join("campaign-start.json")).unwrap();
    let mut host_receipt = fs::read(staged.join(HOST_RECEIPT_NAME)).unwrap();
    host_receipt[0] ^= 1;
    fs::remove_dir_all(&staged).unwrap();

    let server = SupervisorServer::bind(server_config(&socket, &campaign_root)).unwrap();
    let mut backend = FakeStartBackend::default();
    let response = exchange_uploaded_start_once(
        &server,
        &socket,
        &packet,
        &manifest,
        &host_receipt,
        &mut backend,
    );

    assert!(!response.body.ok);
    assert_eq!(response.body.error_code, Some(9));
    assert_eq!(backend.starts, 0);
    assert!(!staged.exists());
    assert!(!campaign_root.join("1".repeat(64)).exists());
}

#[test]
fn disconnected_partial_upload_does_not_kill_the_next_start_request() {
    let temporary = tempfile::tempdir().unwrap();
    let campaign_root = temporary.path().join("campaigns");
    let staging_root = temporary.path().join("staging");
    fs::create_dir(&campaign_root).unwrap();
    fs::create_dir(&staging_root).unwrap();
    let socket = temporary.path().join("supervisor-upload-disconnect.sock");
    let key = SigningKey::from_bytes(&[7; 32]);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let packet = stage_start(&staging_root, &key, now);
    let staged = staging_root.join("1".repeat(64));
    let manifest = fs::read(staged.join("campaign-start.json")).unwrap();
    let host_receipt = fs::read(staged.join(HOST_RECEIPT_NAME)).unwrap();
    fs::remove_dir_all(&staged).unwrap();
    let server = SupervisorServer::bind(server_config(&socket, &campaign_root)).unwrap();
    let mut backend = FakeStartBackend::default();

    let abandoned = SeqpacketConnection::connect(&socket).unwrap();
    abandoned.send_packet(&packet).unwrap();
    abandoned.send_packet(&manifest).unwrap();
    drop(abandoned);
    server.serve_one_with_start_backend(&mut backend).unwrap();
    assert_eq!(backend.starts, 0);
    assert!(!staged.exists());

    let response = exchange_uploaded_start_once(
        &server,
        &socket,
        &packet,
        &manifest,
        &host_receipt,
        &mut backend,
    );
    assert!(response.body.ok);
    assert_eq!(backend.starts, 1);
}

#[test]
fn new_start_revalidates_host_before_claim_while_exact_replay_skips_live_observation() {
    let temporary = tempfile::tempdir().unwrap();
    let campaign_root = temporary.path().join("campaigns");
    let staging_root = temporary.path().join("staging");
    fs::create_dir(&campaign_root).unwrap();
    fs::create_dir(&staging_root).unwrap();
    let socket = temporary
        .path()
        .join("supervisor-start-host-observation.sock");
    let key = SigningKey::from_bytes(&[7; 32]);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let packet = stage_start(&staging_root, &key, now);
    let server = SupervisorServer::bind(server_config(&socket, &campaign_root)).unwrap();
    let mut backend = FakeStartBackend {
        fail_host: true,
        ..FakeStartBackend::default()
    };

    let rejected = exchange_start_once(&server, &socket, &packet, &mut backend);
    assert!(!rejected.body.ok);
    assert_eq!(rejected.body.error_code, Some(6));
    assert_eq!(backend.host_checks, 1);
    assert_eq!(backend.starts, 0);
    assert!(!campaign_root.join(ACTIVE_CAMPAIGN_NAME).exists());
    assert!(!campaign_root
        .join("1".repeat(64))
        .join("state.json")
        .exists());

    backend.fail_host = false;
    let accepted = exchange_start_once(&server, &socket, &packet, &mut backend);
    assert!(accepted.body.ok);
    assert_eq!(backend.host_checks, 2);
    assert_eq!(backend.starts, 1);

    backend.fail_host = true;
    let replay = exchange_start_once(&server, &socket, &packet, &mut backend);
    assert_eq!(replay, accepted);
    assert_eq!(backend.host_checks, 2);
    assert_eq!(backend.starts, 1);
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
