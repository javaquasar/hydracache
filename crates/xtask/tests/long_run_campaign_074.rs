use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use xtask::long_run_campaign::{raw_file_set_sha256, verify_manifest, RawFile};

use hydracache_long_run_supervisor_074::archive::{create_deterministic_archive, ArchiveLimits};
use hydracache_long_run_supervisor_074::artifact::{
    build_packet, verify_packet, ArtifactError, GuardEvidenceInput, PacketLimits, PacketPlan,
    PacketResult, RoleEvidenceInput,
};
use hydracache_long_run_supervisor_074::Role;

const DOMAIN: &[u8] = b"hydracache-long-run-record-v1";
const GENESIS: &str = "0000000000000000000000000000000000000000000000000000000000000000";

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn decoded(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn record(sequence: u64, previous: &str, role: &str) -> Value {
    let process = json!({
        "boot_id": "boot-a", "pid": 10, "start_ticks": 20,
        "process_group": 10, "cgroup_path": "/hc/a", "cgroup_inode": 30,
        "unit_name": "hc-a.service"
    });
    let payload = json!({
        "campaign_id": "a".repeat(64), "role": role, "phase": "measured",
        "phase_epoch": 1, "monotonic_elapsed_ns": sequence * 1_000,
        "wall_clock_utc": format!("2026-10-04T00:00:0{sequence}Z"),
        "completed": sequence, "failed": 0, "rejected": 0, "timed_out": 0,
        "outstanding": 0, "telemetry_sequence": sequence, "milestone": "progress",
        "surface_counters": {}, "resource_counters": {}, "owner_counters": {},
        "harness": process, "daemon": process
    });
    let payload_sha256 = hex(&Sha256::digest(serde_json::to_vec(&payload).unwrap()));
    let mut digest = Sha256::new();
    digest.update(DOMAIN);
    digest.update([0]);
    digest.update(sequence.to_be_bytes());
    digest.update(decoded(previous));
    digest.update(decoded(&payload_sha256));
    json!({
        "schema_version": 1, "sequence": sequence,
        "previous_record_sha256": previous, "payload": payload,
        "payload_sha256": payload_sha256, "record_sha256": hex(&digest.finalize())
    })
}

fn process() -> Value {
    json!({
        "boot_id": "boot-a", "pid": 10, "start_ticks": 20,
        "process_group": 10, "cgroup_path": "/hc/a", "cgroup_inode": 30,
        "unit_name": "hc-a.service"
    })
}

fn canonical(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}

fn raw_file(root: &Path, relative: &str, bytes: &[u8]) -> RawFile {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, bytes).unwrap();
    RawFile {
        path: PathBuf::from(relative),
        size: bytes.len() as u64,
        sha256: hex(&Sha256::digest(bytes)),
    }
}

fn packet_fixture(directory: &Path) -> PathBuf {
    let first = record(1, GENESIS, "i74");
    let second = record(2, first["record_sha256"].as_str().unwrap(), "i74");
    let journal_bytes = format!(
        "{}\n{}\n",
        serde_json::to_string(&first).unwrap(),
        serde_json::to_string(&second).unwrap()
    )
    .into_bytes();
    let campaign_bytes = canonical(&json!({
        "campaign_id": "a".repeat(64), "release": "0.74"
    }));
    let guard_bytes = canonical(&json!({"guard": "semantic", "passed": true}));
    let mut files = vec![
        raw_file(directory, "raw/campaign-start.json", &campaign_bytes),
        raw_file(directory, "raw/guard-semantic.json", &guard_bytes),
        raw_file(directory, "raw/i74.jsonl", &journal_bytes),
    ];
    files.sort_by(|left, right| left.path.cmp(&right.path));
    let set_sha256 = raw_file_set_sha256(&files).unwrap();
    let raw_manifest = canonical(&json!({
        "schema_version": 1,
        "release": "0.74",
        "campaign_id": "a".repeat(64),
        "file_count": files.len(),
        "total_bytes": files.iter().map(|file| file.size).sum::<u64>(),
        "set_sha256": set_sha256,
        "files": files
    }));
    fs::write(directory.join("raw-manifest.json"), &raw_manifest).unwrap();
    let packet = canonical(&json!({
        "schema_version": 1,
        "release": "0.74",
        "campaign_id": "a".repeat(64),
        "campaign_manifest": "raw/campaign-start.json",
        "campaign_manifest_sha256": hex(&Sha256::digest(&campaign_bytes)),
        "raw_manifest": "raw-manifest.json",
        "raw_manifest_sha256": hex(&Sha256::digest(&raw_manifest)),
        "raw_manifest_set_sha256": set_sha256,
        "result": "incomplete",
        "promotable": false,
        "terminal_reason": "local-fixture",
        "required_final_guards": ["semantic"],
        "guard_results": [{
            "id": "semantic", "passed": true,
            "evidence_sha256": hex(&Sha256::digest(&guard_bytes))
        }],
        "roles": [{
            "id": "i74", "result": "complete", "journal": "raw/i74.jsonl",
            "expected_records": 2,
            "journal_sha256": hex(&Sha256::digest(&journal_bytes)),
            "expected_bytes": journal_bytes.len(),
            "first_record_sha256": first["record_sha256"],
            "expected_head_sha256": second["record_sha256"],
            "allow_incomplete_trailing_bytes": false,
            "harness": process(), "daemon": process()
        }]
    }));
    let path = directory.join("packet.json");
    fs::write(&path, packet).unwrap();
    path
}

fn schema(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/testing/performance/0.74/schemas")
        .join(name);
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn built_packet_fixture(
    root: &Path,
    output_name: &str,
) -> hydracache_long_run_supervisor_074::artifact::PacketReceipt {
    let campaign_id = "a".repeat(64);
    let campaign = root.join(&campaign_id);
    fs::create_dir_all(campaign.join("roles/i74/guards")).unwrap();
    let campaign_bytes = canonical(&json!({
        "campaign_id": campaign_id,
        "release": "0.74"
    }));
    fs::write(campaign.join("campaign-start.json"), &campaign_bytes).unwrap();
    let guard_bytes = canonical(&json!({"guard": "semantic", "passed": true}));
    fs::write(campaign.join("roles/i74/guards/semantic.json"), guard_bytes).unwrap();
    let first = record(1, GENESIS, "i74");
    let second = record(2, first["record_sha256"].as_str().unwrap(), "i74");
    fs::write(
        campaign.join("roles/i74/checkpoints.jsonl"),
        format!(
            "{}\n{}\n",
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&second).unwrap()
        ),
    )
    .unwrap();
    let outputs = root.join("outputs");
    fs::create_dir_all(&outputs).unwrap();
    let plan = PacketPlan {
        campaign_id: "a".repeat(64),
        campaign_manifest_sha256: hex(&Sha256::digest(&campaign_bytes)),
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
            PathBuf::from("roles/i74/checkpoints.jsonl"),
            PathBuf::from("campaign-start.json"),
            PathBuf::from("roles/i74/guards/semantic.json"),
        ],
    };
    let limits = PacketLimits {
        maximum_files: 2_000,
        maximum_bytes: 1_073_741_824,
    };
    let receipt = build_packet(&campaign, &outputs.join(output_name), &plan, limits).unwrap();
    assert_eq!(
        verify_packet(&receipt.packet_directory, &plan, limits).unwrap(),
        receipt
    );
    receipt
}

#[test]
fn independent_verifier_accepts_exact_packet_and_rejects_manifest_drift() {
    let directory = tempfile::tempdir().unwrap();
    let manifest = packet_fixture(directory.path());
    let packet_value: Value = serde_json::from_slice(&fs::read(&manifest).unwrap()).unwrap();
    jsonschema::validator_for(&schema("campaign-packet-manifest.schema.json"))
        .unwrap()
        .validate(&packet_value)
        .unwrap();
    let raw_value: Value =
        serde_json::from_slice(&fs::read(directory.path().join("raw-manifest.json")).unwrap())
            .unwrap();
    jsonschema::validator_for(&schema("raw-manifest.schema.json"))
        .unwrap()
        .validate(&raw_value)
        .unwrap();
    assert_eq!(
        xtask::long_run_campaign::verify_manifest(&manifest)
            .unwrap()
            .len(),
        1
    );

    let mut changed: Value = serde_json::from_slice(&fs::read(&manifest).unwrap()).unwrap();
    changed["roles"][0]["expected_records"] = json!(3);
    fs::write(&manifest, canonical(&changed)).unwrap();
    assert!(xtask::long_run_campaign::verify_manifest(&manifest).is_err());
}

#[test]
fn supervisor_builder_produces_a_deterministic_independently_verified_packet() {
    let first_root = tempfile::tempdir().unwrap();
    let second_root = tempfile::tempdir().unwrap();
    let first = built_packet_fixture(first_root.path(), "packet");
    let second = built_packet_fixture(second_root.path(), "packet");

    assert_eq!(first.packet_manifest_sha256, second.packet_manifest_sha256);
    assert_eq!(first.raw_manifest_sha256, second.raw_manifest_sha256);
    assert_eq!(
        first.raw_manifest_set_sha256,
        second.raw_manifest_set_sha256
    );
    assert_eq!(first.raw_file_count, 3);
    assert_eq!(
        verify_manifest(&first.packet_manifest_path).unwrap().len(),
        1
    );
    assert_eq!(
        verify_manifest(&second.packet_manifest_path).unwrap().len(),
        1
    );

    let archive_limits = ArchiveLimits {
        maximum_files: 10,
        maximum_uncompressed_bytes: 1_073_741_824,
        maximum_archive_bytes: 1_073_741_824,
    };
    let first_archive = create_deterministic_archive(
        &first.packet_directory,
        &first_root.path().join("archive"),
        archive_limits,
    )
    .unwrap();
    let second_archive = create_deterministic_archive(
        &second.packet_directory,
        &second_root.path().join("archive"),
        archive_limits,
    )
    .unwrap();
    assert_eq!(first_archive.archive_sha256, second_archive.archive_sha256);
}

#[test]
fn supervisor_builder_produces_a_promotable_two_role_packet() {
    let root = tempfile::tempdir().unwrap();
    let continuation = built_packet_fixture(root.path(), "continuation");
    assert_eq!(
        verify_manifest(&continuation.packet_manifest_path)
            .unwrap()
            .len(),
        1
    );

    let campaign = root.path().join("a".repeat(64));
    fs::create_dir_all(campaign.join("roles/c74")).unwrap();
    let first = record(1, GENESIS, "c74");
    let second = record(2, first["record_sha256"].as_str().unwrap(), "c74");
    fs::write(
        campaign.join("roles/c74/checkpoints.jsonl"),
        format!(
            "{}\n{}\n",
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&second).unwrap()
        ),
    )
    .unwrap();
    let campaign_bytes = fs::read(campaign.join("campaign-start.json")).unwrap();
    let final_packet = build_packet(
        &campaign,
        &root.path().join("outputs/final"),
        &PacketPlan {
            campaign_id: "a".repeat(64),
            campaign_manifest_sha256: hex(&Sha256::digest(&campaign_bytes)),
            result: PacketResult::Complete,
            promotable: true,
            terminal_reason: None,
            required_final_guards: vec!["semantic".to_owned()],
            guard_evidence: vec![GuardEvidenceInput {
                id: "semantic".to_owned(),
                passed: true,
                source_relative_path: PathBuf::from("roles/i74/guards/semantic.json"),
            }],
            roles: vec![
                RoleEvidenceInput {
                    role: Role::C74,
                    result: PacketResult::Complete,
                    journal_relative_path: PathBuf::from("roles/c74/checkpoints.jsonl"),
                },
                RoleEvidenceInput {
                    role: Role::I74,
                    result: PacketResult::Complete,
                    journal_relative_path: PathBuf::from("roles/i74/checkpoints.jsonl"),
                },
            ],
            raw_files: vec![
                PathBuf::from("campaign-start.json"),
                PathBuf::from("roles/i74/guards/semantic.json"),
                PathBuf::from("roles/i74/checkpoints.jsonl"),
                PathBuf::from("roles/c74/checkpoints.jsonl"),
            ],
        },
        PacketLimits {
            maximum_files: 2_000,
            maximum_bytes: 1_073_741_824,
        },
    )
    .unwrap();
    let reports = verify_manifest(&final_packet.packet_manifest_path).unwrap();
    assert_eq!(reports.len(), 2);
    assert_eq!(
        reports
            .iter()
            .map(|report| report.role.as_str())
            .collect::<Vec<_>>(),
        ["c74", "i74"]
    );
}

#[test]
fn supervisor_builder_rejects_unsafe_sources_before_publishing_a_packet() {
    let root = tempfile::tempdir().unwrap();
    let campaign_id = "a".repeat(64);
    let campaign = root.path().join(&campaign_id);
    fs::create_dir(&campaign).unwrap();
    let campaign_bytes = canonical(&json!({
        "campaign_id": campaign_id,
        "release": "0.74"
    }));
    fs::write(campaign.join("campaign-start.json"), &campaign_bytes).unwrap();
    let plan = PacketPlan {
        campaign_id: "a".repeat(64),
        campaign_manifest_sha256: hex(&Sha256::digest(&campaign_bytes)),
        result: PacketResult::Complete,
        promotable: false,
        terminal_reason: None,
        required_final_guards: vec!["semantic".to_owned()],
        guard_evidence: vec![GuardEvidenceInput {
            id: "semantic".to_owned(),
            passed: true,
            source_relative_path: PathBuf::from("campaign-start.json"),
        }],
        roles: vec![RoleEvidenceInput {
            role: Role::I74,
            result: PacketResult::Complete,
            journal_relative_path: PathBuf::from("../escape.jsonl"),
        }],
        raw_files: vec![
            PathBuf::from("campaign-start.json"),
            PathBuf::from("../escape.jsonl"),
        ],
    };
    assert!(matches!(
        build_packet(
            &campaign,
            &root.path().join("packet"),
            &plan,
            PacketLimits {
                maximum_files: 10,
                maximum_bytes: 1_024,
            },
        ),
        Err(ArtifactError::Path)
    ));
    assert!(!root.path().join("packet").exists());
}

#[test]
fn independent_verifier_rejects_hash_identity_and_traversal_failures() {
    let record = record(1, GENESIS, "i74");
    let mut bytes = serde_json::to_vec(&record).unwrap();
    bytes.push(b'\n');
    assert!(xtask::long_run_campaign::verify_journal_bytes(&bytes).is_ok());

    let mut corrupt: Value = record;
    corrupt["payload"]["completed"] = json!(99);
    let mut corrupt_bytes = serde_json::to_vec(&corrupt).unwrap();
    corrupt_bytes.push(b'\n');
    assert!(xtask::long_run_campaign::verify_journal_bytes(&corrupt_bytes).is_err());

    let directory = tempfile::tempdir().unwrap();
    let manifest = packet_fixture(directory.path());
    let mut packet: Value = serde_json::from_slice(&fs::read(&manifest).unwrap()).unwrap();
    packet["roles"][0]["journal"] = json!("../escape.jsonl");
    fs::write(&manifest, canonical(&packet)).unwrap();
    assert!(xtask::long_run_campaign::verify_manifest(&manifest).is_err());
}

#[test]
fn independent_verifier_rejects_raw_file_and_guard_tampering() {
    let directory = tempfile::tempdir().unwrap();
    let manifest = packet_fixture(directory.path());
    fs::write(
        directory.path().join("raw/guard-semantic.json"),
        b"tampered",
    )
    .unwrap();
    assert!(xtask::long_run_campaign::verify_manifest(&manifest).is_err());

    let directory = tempfile::tempdir().unwrap();
    let manifest = packet_fixture(directory.path());
    let mut packet: Value = serde_json::from_slice(&fs::read(&manifest).unwrap()).unwrap();
    packet["guard_results"][0]["id"] = json!("unregistered");
    fs::write(&manifest, canonical(&packet)).unwrap();
    assert!(xtask::long_run_campaign::verify_manifest(&manifest).is_err());

    let directory = tempfile::tempdir().unwrap();
    let manifest = packet_fixture(directory.path());
    fs::write(directory.path().join("raw/unlisted.json"), b"{}").unwrap();
    assert!(xtask::long_run_campaign::verify_manifest(&manifest).is_err());

    let directory = tempfile::tempdir().unwrap();
    let manifest = packet_fixture(directory.path());
    fs::hard_link(
        directory.path().join("raw/guard-semantic.json"),
        directory.path().join("raw/hard-link.json"),
    )
    .unwrap();
    assert!(xtask::long_run_campaign::verify_manifest(&manifest).is_err());

    let directory = tempfile::tempdir().unwrap();
    let manifest = packet_fixture(directory.path());
    let packet: Value = serde_json::from_slice(&fs::read(&manifest).unwrap()).unwrap();
    fs::write(&manifest, serde_json::to_vec_pretty(&packet).unwrap()).unwrap();
    assert!(xtask::long_run_campaign::verify_manifest(&manifest).is_err());
}
