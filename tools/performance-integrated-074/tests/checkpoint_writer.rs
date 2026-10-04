#![recursion_limit = "256"]

use hydracache_long_run_supervisor_074::artifact::PacketResult;
use hydracache_long_run_supervisor_074::manifest::CampaignManifest;
use hydracache_long_run_supervisor_074::seal_input::{
    resolve_packet_plan, SealInputInventory, SEAL_INPUT_INVENTORY_NAME,
};
use hydracache_long_run_supervisor_074::{CheckpointPayload, Phase, ProcessIdentity, Role};
use hydracache_performance_integrated_074::{
    CheckpointWriterError, DurableCheckpointWriter, TerminalGuardResult, TerminalSealInput,
    CHECKPOINT_JOURNAL_NAME,
};
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

fn process(pid: u32) -> ProcessIdentity {
    ProcessIdentity {
        boot_id: "boot-a".to_owned(),
        pid,
        start_ticks: u64::from(pid) * 10,
        process_group: 100,
        cgroup_path: "/system.slice/hydracache-perf.scope".to_owned(),
        cgroup_inode: 500,
        unit_name: "hydracache-perf-i74.service".to_owned(),
    }
}

fn payload(phase: Phase, elapsed: u64, completed: u64, outstanding: u64) -> CheckpointPayload {
    CheckpointPayload {
        campaign_id: "a".repeat(64),
        role: Role::I74,
        phase,
        phase_epoch: elapsed,
        monotonic_elapsed_ns: elapsed * 1_000_000_000,
        wall_clock_utc: format!("2026-10-04T00:{elapsed:02}:00Z"),
        completed,
        failed: 0,
        rejected: 0,
        timed_out: 0,
        outstanding,
        telemetry_sequence: elapsed,
        milestone: format!("milestone-{elapsed}"),
        surface_counters: BTreeMap::from([("resp".to_owned(), completed)]),
        resource_counters: BTreeMap::from([("process_cpu_time_ns".to_owned(), elapsed * 1_000)]),
        owner_counters: BTreeMap::new(),
        harness: process(100),
        daemon: process(101),
    }
}

fn canonical<T: Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(&serde_json::to_value(value).unwrap()).unwrap()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn manifest(campaign_id: &str) -> CampaignManifest {
    serde_json::from_value(json!({
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
        "required_final_guards": ["semantic"],
        "secret_identifiers": ["github-environment-key-v1"],
        "release": "0.74",
        "campaign_id": campaign_id,
        "nonce_sha256": "2".repeat(64),
        "dirty": false,
        "controller_history": [],
        "state": "PREPARED"
    }))
    .unwrap()
}

fn terminal_writer(role: &Path) -> DurableCheckpointWriter {
    let mut writer =
        DurableCheckpointWriter::start(role, payload(Phase::Startup, 1, 0, 10), 1).unwrap();
    for (phase, elapsed, completed, outstanding) in [
        (Phase::Warmup, 2, 10, 10),
        (Phase::Measured, 3, 20, 10),
        (Phase::Drain, 4, 20, 5),
        (Phase::DurableCompanion, 5, 20, 5),
        (Phase::PostWorkIdle, 6, 20, 5),
        (Phase::Reconciliation, 7, 20, 0),
        (Phase::Terminal, 8, 20, 0),
    ] {
        writer
            .append(payload(phase, elapsed, completed, outstanding), elapsed)
            .unwrap();
    }
    writer
}

fn publication_fixture() -> (
    tempfile::TempDir,
    PathBuf,
    PathBuf,
    CampaignManifest,
    String,
) {
    let temporary = tempfile::tempdir().unwrap();
    let campaign_id = "a".repeat(64);
    let campaign = temporary.path().join(&campaign_id);
    let role = campaign.join("roles/i74");
    fs::create_dir_all(role.join("measurements")).unwrap();
    let manifest = manifest(&campaign_id);
    let bytes = canonical(&manifest);
    let digest = hex(&Sha256::digest(&bytes));
    fs::write(campaign.join("campaign-start.json"), bytes).unwrap();
    fs::write(
        role.join("measurements/semantic-proof.json"),
        canonical(&json!({"exact_final_state": true})),
    )
    .unwrap();
    (temporary, campaign, role, manifest, digest)
}

fn terminal_input(manifest: CampaignManifest, digest: String) -> TerminalSealInput {
    TerminalSealInput {
        campaign_manifest: manifest,
        campaign_manifest_sha256: digest,
        result: PacketResult::Complete,
        terminal_reason: None,
        guards: vec![TerminalGuardResult {
            id: "semantic".to_owned(),
            passed: true,
            evidence_relative_paths: vec![PathBuf::from(
                "roles/i74/measurements/semantic-proof.json",
            )],
        }],
        additional_raw_files: Vec::new(),
    }
}

#[test]
fn writes_complete_phase_aware_terminal_chain() {
    let temporary = tempfile::tempdir().unwrap();
    let mut writer =
        DurableCheckpointWriter::start(temporary.path(), payload(Phase::Startup, 1, 0, 10), 1)
            .unwrap();
    for (phase, elapsed, completed, outstanding) in [
        (Phase::Warmup, 2, 10, 10),
        (Phase::Measured, 3, 20, 10),
        (Phase::Drain, 4, 20, 5),
        (Phase::DurableCompanion, 5, 20, 5),
        (Phase::PostWorkIdle, 6, 20, 5),
        (Phase::Reconciliation, 7, 20, 0),
        (Phase::Terminal, 8, 20, 0),
    ] {
        writer
            .append(payload(phase, elapsed, completed, outstanding), elapsed)
            .unwrap();
    }
    let report = writer.finish().unwrap();
    assert_eq!(report.records, 8);
    assert_eq!(report.last_phase, Phase::Terminal);
    assert_eq!(report.recovered_incomplete_trailing_bytes, 0);
}

#[test]
fn rejected_progress_does_not_advance_or_append() {
    let temporary = tempfile::tempdir().unwrap();
    let mut writer =
        DurableCheckpointWriter::start(temporary.path(), payload(Phase::Startup, 1, 0, 10), 1)
            .unwrap();
    let mut drift = payload(Phase::Warmup, 2, 10, 10);
    drift.harness.start_ticks += 1;
    assert!(matches!(
        writer.append(drift, 2),
        Err(CheckpointWriterError::Progress(_))
    ));
    assert_eq!(writer.sequence(), 1);
    assert_eq!(writer.report().unwrap().records, 1);
}

#[test]
fn existing_or_torn_journal_cannot_be_reused_as_a_new_process() {
    let temporary = tempfile::tempdir().unwrap();
    let writer =
        DurableCheckpointWriter::start(temporary.path(), payload(Phase::Startup, 1, 0, 10), 1)
            .unwrap();
    assert!(matches!(
        DurableCheckpointWriter::start(temporary.path(), payload(Phase::Startup, 1, 0, 10), 1),
        Err(CheckpointWriterError::Initialization)
    ));
    let mut journal = OpenOptions::new()
        .append(true)
        .open(temporary.path().join(CHECKPOINT_JOURNAL_NAME))
        .unwrap();
    journal.write_all(b"{\"torn\":").unwrap();
    journal.sync_all().unwrap();
    assert!(writer.finish().is_err());
}

#[test]
fn terminal_writer_publishes_resolver_ready_guard_results_and_inventory() {
    let (_temporary, campaign, role, manifest, digest) = publication_fixture();
    let receipt = terminal_writer(&role)
        .finish_and_publish(terminal_input(manifest.clone(), digest.clone()))
        .unwrap();
    assert_eq!(receipt.report.last_phase, Phase::Terminal);
    assert_eq!(
        receipt.inventory_path,
        fs::canonicalize(&role)
            .unwrap()
            .join(SEAL_INPUT_INVENTORY_NAME)
    );
    assert_eq!(receipt.inventory_sha256.len(), 64);

    let inventory_bytes = fs::read(&receipt.inventory_path).unwrap();
    assert_eq!(
        inventory_bytes,
        canonical(&serde_json::from_slice::<serde_json::Value>(&inventory_bytes).unwrap())
    );
    let inventory: SealInputInventory = serde_json::from_slice(&inventory_bytes).unwrap();
    assert_eq!(inventory.result, PacketResult::Complete);
    assert_eq!(inventory.guard_evidence.len(), 1);
    assert!(inventory
        .raw_files
        .contains(&PathBuf::from("roles/i74/measurements/semantic-proof.json")));
    assert!(campaign
        .join(&inventory.guard_evidence[0].source_relative_path)
        .is_file());
    assert!(!role
        .join(format!("{SEAL_INPUT_INVENTORY_NAME}.pending"))
        .exists());

    let seal_root = campaign.parent().unwrap().join("seals");
    fs::create_dir(&seal_root).unwrap();
    let plan = resolve_packet_plan(&campaign, &seal_root, &manifest, &digest, Role::I74).unwrap();
    assert_eq!(plan.result, PacketResult::Complete);
    assert_eq!(plan.guard_evidence.len(), 1);
    assert!(plan.guard_evidence[0].passed);
}

#[test]
fn complete_publication_rejects_failed_guard_without_inventory() {
    let (_temporary, _campaign, role, manifest, digest) = publication_fixture();
    let mut input = terminal_input(manifest, digest);
    input.guards[0].passed = false;
    assert!(matches!(
        terminal_writer(&role).finish_and_publish(input),
        Err(CheckpointWriterError::SealBinding)
    ));
    assert!(!role.join(SEAL_INPUT_INVENTORY_NAME).exists());
    assert!(!role.join("guards").exists());
}

#[test]
fn publication_rejects_cross_role_evidence_without_inventory() {
    let (_temporary, campaign, role, manifest, digest) = publication_fixture();
    let c74 = campaign.join("roles/c74/measurements");
    fs::create_dir_all(&c74).unwrap();
    fs::write(c74.join("semantic-proof.json"), b"{}").unwrap();
    let mut input = terminal_input(manifest, digest);
    input.guards[0].evidence_relative_paths =
        vec![PathBuf::from("roles/c74/measurements/semantic-proof.json")];
    assert!(matches!(
        terminal_writer(&role).finish_and_publish(input),
        Err(CheckpointWriterError::SealBinding)
    ));
    assert!(!role.join(SEAL_INPUT_INVENTORY_NAME).exists());
}
