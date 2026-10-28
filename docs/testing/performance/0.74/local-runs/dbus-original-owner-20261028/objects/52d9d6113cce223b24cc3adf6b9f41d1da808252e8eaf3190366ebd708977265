#![recursion_limit = "256"]

use hydracache_long_run_supervisor_074::artifact::{build_packet, PacketLimits, PacketResult};
use hydracache_long_run_supervisor_074::manifest::CampaignManifest;
use hydracache_long_run_supervisor_074::seal_artifact::SealArtifactResult;
use hydracache_long_run_supervisor_074::seal_input::{
    resolve_packet_plan, InventoryGuardEvidence, SealInputInventory, SEAL_INPUT_INVENTORY_NAME,
};
use hydracache_long_run_supervisor_074::{
    build_record, CheckpointPayload, Phase, ProcessIdentity, Role, GENESIS_HASH,
};
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn canonical<T: Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(&serde_json::to_value(value).unwrap()).unwrap()
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

fn write_inventory(campaign: &Path, campaign_id: &str, manifest_sha256: &str, role: Role) {
    let name = match role {
        Role::I74 => "i74",
        Role::C74 => "c74",
    };
    let role_root = campaign.join(format!("roles/{name}"));
    fs::create_dir_all(role_root.join("guards")).unwrap();
    let process = ProcessIdentity {
        boot_id: "boot-a".to_owned(),
        pid: 10,
        start_ticks: 20,
        process_group: 10,
        cgroup_path: "/hc/a".to_owned(),
        cgroup_inode: 30,
        unit_name: "hc-a.service".to_owned(),
    };
    let checkpoint = build_record(
        1,
        GENESIS_HASH,
        CheckpointPayload {
            campaign_id: campaign_id.to_owned(),
            role: role.clone(),
            phase: Phase::Terminal,
            phase_epoch: 1,
            monotonic_elapsed_ns: 1_000,
            wall_clock_utc: "2026-10-04T00:00:01Z".to_owned(),
            observed_unix_seconds: 1_001,
            useful_progress_unix_seconds: 1_001,
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
            harness: process.clone(),
            daemon: process,
        },
    )
    .unwrap();
    fs::write(
        role_root.join("checkpoints.jsonl"),
        format!("{}\n", serde_json::to_string(&checkpoint).unwrap()),
    )
    .unwrap();
    fs::write(role_root.join("guards/semantic.json"), b"{}\n").unwrap();
    let inventory = SealInputInventory {
        schema_version: 1,
        release: "0.74".to_owned(),
        campaign_id: campaign_id.to_owned(),
        campaign_manifest_sha256: manifest_sha256.to_owned(),
        role,
        result: PacketResult::Complete,
        terminal_reason: None,
        journal_relative_path: PathBuf::from(format!("roles/{name}/checkpoints.jsonl")),
        guard_evidence: vec![InventoryGuardEvidence {
            id: "semantic".to_owned(),
            passed: true,
            source_relative_path: PathBuf::from(format!("roles/{name}/guards/semantic.json")),
        }],
        raw_files: vec![
            PathBuf::from("campaign-start.json"),
            PathBuf::from(format!("roles/{name}/checkpoints.jsonl")),
            PathBuf::from(format!("roles/{name}/guards/semantic.json")),
            PathBuf::from(format!("roles/{name}/{SEAL_INPUT_INVENTORY_NAME}")),
        ],
    };
    fs::write(
        role_root.join(SEAL_INPUT_INVENTORY_NAME),
        canonical(&inventory),
    )
    .unwrap();
}

fn fixture() -> (
    tempfile::TempDir,
    PathBuf,
    PathBuf,
    CampaignManifest,
    String,
) {
    let temporary = tempfile::tempdir().unwrap();
    let campaign_id = "1".repeat(64);
    let campaign = temporary.path().join(&campaign_id);
    fs::create_dir(&campaign).unwrap();
    let manifest = manifest(&campaign_id);
    let manifest_bytes = canonical(&manifest);
    let manifest_sha256 = hex(&Sha256::digest(&manifest_bytes));
    fs::write(campaign.join("campaign-start.json"), manifest_bytes).unwrap();
    write_inventory(&campaign, &campaign_id, &manifest_sha256, Role::I74);
    let seal_root = temporary.path().join("seals");
    fs::create_dir(&seal_root).unwrap();
    (temporary, campaign, seal_root, manifest, manifest_sha256)
}

fn write_continuation_result(
    campaign: &Path,
    campaign_id: &str,
    packet_directory_name: &str,
    digest: &str,
) {
    let result = SealArtifactResult {
        schema_version: 1,
        intent_sha256: "a".repeat(64),
        campaign_id: campaign_id.to_owned(),
        role: Role::I74,
        packet_directory_name: packet_directory_name.to_owned(),
        packet_manifest_sha256: digest.to_owned(),
        raw_manifest_sha256: "b".repeat(64),
        raw_manifest_set_sha256: "c".repeat(64),
        raw_file_count: 4,
        raw_total_bytes: 100,
        archive_directory_name: "archive".to_owned(),
        archive_sha256: "d".repeat(64),
        archive_bytes: 100,
        archive_input_files: 6,
        archive_input_bytes: 200,
    };
    let bytes = canonical(&result);
    fs::write(campaign.join("i74-seal-result.json"), &bytes).unwrap();
    fs::write(
        campaign.join("i74-seal-result.json.sha256"),
        format!("{}\n", hex(&Sha256::digest(&bytes))),
    )
    .unwrap();
}

fn build_continuation(
    campaign: &Path,
    seal_root: &Path,
    manifest: &CampaignManifest,
    manifest_sha256: &str,
) -> (String, String) {
    let plan =
        resolve_packet_plan(campaign, seal_root, manifest, manifest_sha256, Role::I74).unwrap();
    let packet_directory_name = format!("{}-i74-packet", manifest.campaign_id);
    let receipt = build_packet(
        campaign,
        &seal_root.join(&packet_directory_name),
        &plan,
        PacketLimits {
            maximum_files: 2_000,
            maximum_bytes: 1_073_741_824,
        },
    )
    .unwrap();
    (packet_directory_name, receipt.packet_manifest_sha256)
}

#[test]
fn i74_plan_is_derived_only_from_the_strict_role_inventory() {
    let (_temporary, campaign, seal_root, manifest, digest) = fixture();
    fs::write(campaign.join("unlisted-secret"), b"not packet input").unwrap();

    let plan = resolve_packet_plan(&campaign, &seal_root, &manifest, &digest, Role::I74).unwrap();
    assert_eq!(plan.campaign_id, manifest.campaign_id);
    assert_eq!(plan.continuation_packet_sha256, None);
    assert!(!plan.promotable);
    assert_eq!(plan.roles.len(), 1);
    assert_eq!(plan.roles[0].role, Role::I74);
    assert!(!plan
        .raw_files
        .iter()
        .any(|path| path == Path::new("unlisted-secret")));
    assert!(plan.raw_files.contains(&PathBuf::from(format!(
        "roles/i74/{SEAL_INPUT_INVENTORY_NAME}"
    ))));
}

#[test]
fn c74_plan_combines_explicit_inventories_and_binds_durable_i74_digest() {
    let (_temporary, campaign, seal_root, manifest, digest) = fixture();
    write_inventory(&campaign, &manifest.campaign_id, &digest, Role::C74);
    let (packet_directory_name, continuation_digest) =
        build_continuation(&campaign, &seal_root, &manifest, &digest);
    write_continuation_result(
        &campaign,
        &manifest.campaign_id,
        &packet_directory_name,
        &continuation_digest,
    );

    let plan = resolve_packet_plan(&campaign, &seal_root, &manifest, &digest, Role::C74).unwrap();
    assert_eq!(
        plan.continuation_packet_sha256.as_deref(),
        Some(continuation_digest.as_str())
    );
    assert!(plan.promotable);
    assert_eq!(
        plan.roles
            .iter()
            .map(|input| input.role.clone())
            .collect::<Vec<_>>(),
        [Role::I74, Role::C74]
    );
    assert_eq!(
        plan.guard_evidence[0].source_relative_path,
        PathBuf::from("roles/c74/guards/semantic.json")
    );
    assert!(plan.raw_files.contains(&PathBuf::from(format!(
        "roles/i74/{SEAL_INPUT_INVENTORY_NAME}"
    ))));
    assert!(plan.raw_files.contains(&PathBuf::from(format!(
        "roles/c74/{SEAL_INPUT_INVENTORY_NAME}"
    ))));

    fs::write(campaign.join("roles/i74/guards/semantic.json"), b"drift\n").unwrap();
    assert!(resolve_packet_plan(&campaign, &seal_root, &manifest, &digest, Role::C74).is_err());
}

#[test]
fn inventory_drift_unknown_fields_and_cross_role_paths_fail_closed() {
    let (_temporary, campaign, seal_root, manifest, digest) = fixture();
    let inventory_path = campaign.join(format!("roles/i74/{SEAL_INPUT_INVENTORY_NAME}"));
    let mut value: serde_json::Value =
        serde_json::from_slice(&fs::read(&inventory_path).unwrap()).unwrap();
    value["unknown"] = json!(true);
    fs::write(&inventory_path, canonical(&value)).unwrap();
    assert!(resolve_packet_plan(&campaign, &seal_root, &manifest, &digest, Role::I74).is_err());

    write_inventory(&campaign, &manifest.campaign_id, &digest, Role::I74);
    let mut value: serde_json::Value =
        serde_json::from_slice(&fs::read(&inventory_path).unwrap()).unwrap();
    value["raw_files"]
        .as_array_mut()
        .unwrap()
        .push(json!("roles/c74/foreign.json"));
    fs::write(&inventory_path, canonical(&value)).unwrap();
    fs::create_dir_all(campaign.join("roles/c74")).unwrap();
    fs::write(campaign.join("roles/c74/foreign.json"), b"{}").unwrap();
    assert!(resolve_packet_plan(&campaign, &seal_root, &manifest, &digest, Role::I74).is_err());
}

#[test]
fn final_plan_rejects_missing_or_tampered_continuation_result() {
    let (_temporary, campaign, seal_root, manifest, digest) = fixture();
    write_inventory(&campaign, &manifest.campaign_id, &digest, Role::C74);
    assert!(resolve_packet_plan(&campaign, &seal_root, &manifest, &digest, Role::C74).is_err());

    let (packet_directory_name, continuation_digest) =
        build_continuation(&campaign, &seal_root, &manifest, &digest);
    write_continuation_result(
        &campaign,
        &manifest.campaign_id,
        &packet_directory_name,
        &continuation_digest,
    );
    fs::write(campaign.join("i74-seal-result.json.sha256"), b"bad\n").unwrap();
    assert!(resolve_packet_plan(&campaign, &seal_root, &manifest, &digest, Role::C74).is_err());
}
