#![recursion_limit = "256"]

use hydracache_long_run_supervisor_074::host_receipt::{
    encode_canonical as encode_host_receipt, BinaryIdentity, HostObservationReceipt, MountIdentity,
    HOST_RECEIPT_HEAD_NAME, HOST_RECEIPT_NAME, SUPERVISOR_BINARY_PATH,
};
use hydracache_long_run_supervisor_074::manifest::{
    frozen_identity_from_manifest, parse_and_validate, ManifestError, MAX_MANIFEST_BYTES,
};
use hydracache_long_run_supervisor_074::manifest_evidence::{
    verify_manifest_evidence, ManifestEvidenceError,
};
use hydracache_long_run_supervisor_074::protocol::{ControllerIdentity, Operation, Request};
use hydracache_long_run_supervisor_074::start_evidence::{
    load_campaign_evidence, prepare_campaign_evidence, StartEvidenceError,
};
use hydracache_long_run_supervisor_074::state::{
    CampaignState, CheckpointHead, DurableCampaignState, FrozenIdentity,
};
use hydracache_long_run_supervisor_074::ProcessIdentity;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn manifest() -> Value {
    json!({
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
        "host_receipt_sha256": "f".repeat(64),
        "lease_id": "123e4567-e89b-42d3-a456-426614174000",
        "machine_id": "machine-a",
        "boot_id": "boot-a",
        "mount_identity": "dev=1;opts=rw",
        "isolated_cpuset": "2-7",
        "housekeeping_cpuset": "0-1",
        "seed": 740074,
        "checkpoint_cadence_seconds": 30,
        "progress_warning_gap_seconds": 90,
        "progress_rejection_gap_seconds": 180,
        "diagnostic_grace_seconds": 30,
        "product_lease_deadline_unix_seconds": 2_000,
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
        "campaign_id": "1".repeat(64),
        "nonce_sha256": "2".repeat(64),
        "dirty": false,
        "controller_history": [],
        "state": "PREPARED"
    })
}

fn encoded(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}

fn fixture_request(bytes: &[u8]) -> Request {
    Request {
        schema_version: 1,
        request_id: "123e4567-e89b-42d3-a456-426614174000".to_owned(),
        operation: Operation::Start,
        campaign_id: "1".repeat(64),
        expected_state_revision: 0,
        manifest_path: Some(format!(
            "/var/lib/hydracache-performance/staging/{}/campaign-start.json",
            "1".repeat(64)
        )),
        manifest_sha256: hex(&Sha256::digest(bytes)),
        controller: ControllerIdentity {
            repository_id: 10,
            run_id: 20,
            run_attempt: 1,
            actor_id: 30,
            authorization_sha256: "3".repeat(64),
        },
        abort_reason: None,
        approval_nonce_sha256: None,
    }
}

fn host_receipt() -> HostObservationReceipt {
    let campaign_mount = MountIdentity {
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
        serde_json::to_vec(&serde_json::to_value(&campaign_mount).unwrap()).unwrap(),
    ));
    HostObservationReceipt {
        schema_version: 1,
        machine_id: "machine-a".to_owned(),
        boot_id: "boot-a".to_owned(),
        kernel_release: "6.8.0-90-generic".to_owned(),
        kernel_command_line_sha256: "a".repeat(64),
        campaign_mount,
        mount_identity,
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
        .map(|key| (key.to_owned(), "0".to_owned()))
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

fn durable_state(identity: FrozenIdentity) -> DurableCampaignState {
    DurableCampaignState {
        revision: 0,
        campaign_state: CampaignState::I74Running,
        identity,
        harness: Some(process(100)),
        daemon: Some(process(101)),
        checkpoint: Some(CheckpointHead {
            sequence: 1,
            record_sha256: "9".repeat(64),
            useful_progress_unix_seconds: 1_000,
        }),
        controller_lease: None,
        recorded_failure: false,
        duplicate_executor: false,
        durable_history_corrupt: false,
    }
}

#[test]
fn exact_canonical_prepared_manifest_is_accepted() {
    let bytes = encoded(&manifest());
    let request = fixture_request(&bytes);
    assert!(parse_and_validate(&bytes, &request, 1_000).is_ok());
    let mut newline = bytes.clone();
    newline.push(b'\n');
    assert!(parse_and_validate(&newline, &request, 1_000).is_ok());
}

#[test]
fn start_evidence_is_validated_then_atomically_imported_once() {
    let temporary = tempfile::tempdir().unwrap();
    let campaign_root = temporary.path().join("campaigns");
    let staging_root = temporary.path().join("staging");
    fs::create_dir(&campaign_root).unwrap();
    fs::create_dir(&staging_root).unwrap();

    let receipt = host_receipt();
    let receipt_bytes = encode_host_receipt(&receipt).unwrap();
    let receipt_sha256 = hex(&Sha256::digest(&receipt_bytes));
    let mut value = manifest();
    value["host_receipt_sha256"] = json!(receipt_sha256);
    value["mount_identity"] = json!(receipt.mount_identity.clone());
    value["isolated_cpuset"] = json!(receipt.isolated_cpuset.clone());
    value["housekeeping_cpuset"] = json!(receipt.housekeeping_cpuset.clone());
    let manifest_bytes = encoded(&value);
    let mut request = fixture_request(&manifest_bytes);
    let staging = staging_root.join(&request.campaign_id);
    fs::create_dir(&staging).unwrap();
    request.manifest_path = Some(
        staging
            .join("campaign-start.json")
            .to_str()
            .unwrap()
            .to_owned(),
    );
    fs::write(staging.join("campaign-start.json"), &manifest_bytes).unwrap();
    fs::write(
        staging.join("campaign-start.sha256"),
        format!("{}\n", request.manifest_sha256),
    )
    .unwrap();
    fs::write(staging.join(HOST_RECEIPT_NAME), &receipt_bytes).unwrap();
    fs::write(
        staging.join(HOST_RECEIPT_HEAD_NAME),
        format!("{receipt_sha256}\n"),
    )
    .unwrap();

    let prepared =
        prepare_campaign_evidence(&campaign_root, &staging_root, &request, 1_000).unwrap();
    assert_eq!(prepared.manifest.campaign_id, request.campaign_id);
    assert_eq!(prepared.host_receipt, receipt);
    assert_eq!(
        fs::read(prepared.campaign_directory.join("campaign-start.json")).unwrap(),
        manifest_bytes
    );
    let loaded = load_campaign_evidence(&campaign_root, &request, 1_000).unwrap();
    assert_eq!(loaded.manifest, prepared.manifest);
    assert_eq!(loaded.host_receipt, prepared.host_receipt);
    assert!(matches!(
        prepare_campaign_evidence(&campaign_root, &staging_root, &request, 1_000),
        Err(StartEvidenceError::Path)
    ));
}

#[test]
fn digest_identity_and_frozen_field_drift_fail_closed() {
    let bytes = encoded(&manifest());
    let mut request = fixture_request(&bytes);
    request.controller.repository_id = 11;
    assert_eq!(
        parse_and_validate(&bytes, &request, 1_000),
        Err(ManifestError::Binding)
    );

    let mut changed = manifest();
    changed["progress_rejection_gap_seconds"] = json!(181);
    let bytes = encoded(&changed);
    let request = fixture_request(&bytes);
    assert_eq!(
        parse_and_validate(&bytes, &request, 1_000),
        Err(ManifestError::Invariant)
    );

    let mut request = fixture_request(&bytes);
    request.manifest_sha256 = "4".repeat(64);
    assert_eq!(
        parse_and_validate(&bytes, &request, 1_000),
        Err(ManifestError::Digest)
    );
}

#[test]
fn unknown_float_secret_value_and_oversize_fail_closed() {
    let mut unknown = manifest();
    unknown["command"] = json!("sh -c id");
    let bytes = encoded(&unknown);
    assert_eq!(
        parse_and_validate(&bytes, &fixture_request(&bytes), 1_000),
        Err(ManifestError::Document)
    );

    let mut secret = manifest();
    secret["secret_identifiers"] = json!(["TOKEN=plaintext"]);
    let bytes = encoded(&secret);
    assert_eq!(
        parse_and_validate(&bytes, &fixture_request(&bytes), 1_000),
        Err(ManifestError::Invariant)
    );

    let oversized = vec![b' '; MAX_MANIFEST_BYTES + 1];
    let request = fixture_request(&encoded(&manifest()));
    assert_eq!(
        parse_and_validate(&oversized, &request, 1_000),
        Err(ManifestError::Document)
    );
}

#[test]
fn execution_identity_and_nested_unknown_fields_fail_closed() {
    let mut dirty = manifest();
    dirty["i74_dirty"] = json!(true);
    let bytes = encoded(&dirty);
    assert_eq!(
        parse_and_validate(&bytes, &fixture_request(&bytes), 1_000),
        Err(ManifestError::Invariant)
    );

    let mut argv = manifest();
    argv["argv_templates"]["c74"][0] = json!("/tmp/unbound");
    let bytes = encoded(&argv);
    assert_eq!(
        parse_and_validate(&bytes, &fixture_request(&bytes), 1_000),
        Err(ManifestError::Invariant)
    );

    let mut nested_unknown = manifest();
    nested_unknown["output_limits"]["unbounded"] = json!(true);
    let bytes = encoded(&nested_unknown);
    assert_eq!(
        parse_and_validate(&bytes, &fixture_request(&bytes), 1_000),
        Err(ManifestError::Document)
    );
}

#[test]
fn persistent_manifest_reconstructs_the_exact_frozen_identity() {
    let bytes = encoded(&manifest());
    let mut request = fixture_request(&bytes);
    let parsed = parse_and_validate(&bytes, &request, 1_000).unwrap();
    let identity = frozen_identity_from_manifest(&parsed, &request.manifest_sha256).unwrap();
    assert_eq!(
        identity,
        frozen_identity_from_manifest(&parsed, &request.manifest_sha256).unwrap()
    );
    let state = durable_state(identity);
    request.operation = Operation::Attach;
    request.manifest_path = None;
    let temporary = tempfile::tempdir().unwrap();
    std::fs::write(temporary.path().join("campaign-start.json"), &bytes).unwrap();
    std::fs::write(
        temporary.path().join("campaign-start.sha256"),
        format!("{}\n", request.manifest_sha256),
    )
    .unwrap();

    assert_eq!(
        verify_manifest_evidence(temporary.path(), &request, &state, 1_000)
            .unwrap()
            .campaign_id,
        request.campaign_id
    );

    let mut drifted = state;
    drifted.identity.source_bundle_sha256 = "0".repeat(64);
    assert!(matches!(
        verify_manifest_evidence(temporary.path(), &request, &drifted, 1_000),
        Err(ManifestEvidenceError::Binding)
    ));
    std::fs::write(
        temporary.path().join("campaign-start.sha256"),
        format!("{}\n", "f".repeat(64)),
    )
    .unwrap();
    assert!(matches!(
        verify_manifest_evidence(temporary.path(), &request, &drifted, 1_000),
        Err(ManifestEvidenceError::Head)
    ));
}
