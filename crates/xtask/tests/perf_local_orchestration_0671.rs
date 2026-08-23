use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use hydracache_loadgen::profile::{
    reference_cpu_isolation, MeasurementCore, RunnerAttestationV5, RunnerFingerprint,
    REFERENCE_FINGERPRINT_SCHEMA_VERSION, REFERENCE_HOST_CONTRACT_VERSION,
    REFERENCE_MEASUREMENT_CPUS, REFERENCE_OS_IMAGE, REFERENCE_RUNNER_CLASS,
    REFERENCE_STORAGE_CLASS,
};
use sha2::{Digest, Sha256};
use xtask::perf_bootstrap::{
    build_sample_set, BootstrapArtifactDigest, BootstrapSampleReceipt, FullReferenceEvidence,
};
use xtask::perf_full_dress::{
    build_admission, receipt_from_evidence, validate_admission_receipt, validate_full_dress_receipt,
};

const SHA: &str = "0123456789abcdef0123456789abcdef01234567";
const FINGERPRINT: &str = "55503d33d6592cb062ecfcd289fa67cad93d123ee9657312172615da67a26238";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "hydracache-local-orchestration-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn runner(fingerprint: &str) -> RunnerFingerprint {
    RunnerFingerprint {
        runner_class: REFERENCE_RUNNER_CLASS.to_owned(),
        fingerprint: fingerprint.to_owned(),
        cpu_model: "local-orchestration-fixture".to_owned(),
        logical_cores: 16,
        ram_bytes: 64 * 1024 * 1024 * 1024,
        os: "linux".to_owned(),
        kernel: "fixture".to_owned(),
        cpu_affinity: "1-4".to_owned(),
        cgroup_cpu_quota: "unlimited".to_owned(),
        governor: "performance".to_owned(),
        turbo: "disabled".to_owned(),
        shared_hardware: false,
        calibration_score: 0.01,
        attestation: RunnerAttestationV5 {
            schema_version: REFERENCE_FINGERPRINT_SCHEMA_VERSION,
            contract_version: REFERENCE_HOST_CONTRACT_VERSION.to_owned(),
            virtualization: "none".to_owned(),
            physical_cores: 8,
            measurement_cores: REFERENCE_MEASUREMENT_CPUS
                .into_iter()
                .map(|logical_cpu| MeasurementCore {
                    logical_cpu,
                    package_id: 0,
                    core_id: logical_cpu,
                })
                .collect(),
            cpu_isolation: reference_cpu_isolation(),
            host_digest: "d".repeat(64),
            storage_class: REFERENCE_STORAGE_CLASS.to_owned(),
            storage_identity_digest: "e".repeat(64),
            os_image: REFERENCE_OS_IMAGE.to_owned(),
            toolchain_identity: "rustc-1.94.0".to_owned(),
            prebuild_contract_digest: "a".repeat(64),
        },
    }
}

fn evidence(run_id: u64) -> FullReferenceEvidence {
    FullReferenceEvidence {
        source_commit: SHA.to_owned(),
        github_run_id: run_id.to_string(),
        runner_fingerprint: FINGERPRINT.to_owned(),
        observed_runner: runner(FINGERPRINT),
        runner_provisioning_sha256: "6".repeat(64),
        prebuild_contract_digest: "a".repeat(64),
        scenario_contract_set_digest: "b".repeat(64),
        evidence_files: vec![BootstrapArtifactDigest {
            path: "target/test-evidence/0.67/local-orchestration.json".to_owned(),
            sha256: "f".repeat(64),
        }],
    }
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn write_json(path: &Path, value: &impl serde::Serialize) -> Vec<u8> {
    let bytes = serde_json::to_vec_pretty(value).unwrap();
    fs::write(path, &bytes).unwrap();
    bytes
}

fn sample(
    index: u32,
    run_id: u64,
    admission_sha256: &str,
    predecessor: Option<(String, String)>,
) -> BootstrapSampleReceipt {
    BootstrapSampleReceipt {
        schema_version: 2,
        release: "0.67.1".to_owned(),
        profile: "reference-v1".to_owned(),
        source_commit: SHA.to_owned(),
        github_run_id: run_id.to_string(),
        observed_at: format!("2026-08-{index:02}T12:00:00Z"),
        runner_fingerprint: FINGERPRINT.to_owned(),
        observed_runner: runner(FINGERPRINT),
        runner_provisioning_sha256: "6".repeat(64),
        prebuild_contract_digest: "a".repeat(64),
        scenario_contract_set_digest: "b".repeat(64),
        sample_index: index,
        admission_sha256: admission_sha256.to_owned(),
        predecessor_github_run_id: predecessor.as_ref().map(|value| value.0.clone()),
        predecessor_receipt_sha256: predecessor.map(|value| value.1),
        evidence_files: vec![BootstrapArtifactDigest {
            path: format!("target/test-evidence/0.67/sample-{index}.json"),
            sha256: "f".repeat(64),
        }],
        passed: true,
        bootstrap_eligible: true,
        ship_evidence_eligible: false,
    }
}

fn write_chain(directory: &Path, admission_sha256: &str) {
    let mut predecessor = None;
    for index in 1..=5 {
        let receipt = sample(index, 200 + u64::from(index), admission_sha256, predecessor);
        let bytes = write_json(&directory.join(format!("sample-{index}.json")), &receipt);
        predecessor = Some((receipt.github_run_id, digest(&bytes)));
    }
}

#[test]
fn complete_full_dress_admission_and_bootstrap_chain_is_non_ship_evidence() {
    let scratch = Scratch::new("complete");
    let first_path = scratch.0.join("full-dress-1.json");
    let second_path = scratch.0.join("full-dress-2.json");
    let first = receipt_from_evidence(evidence(101));
    let second = receipt_from_evidence(evidence(102));
    validate_full_dress_receipt(&first).unwrap();
    validate_full_dress_receipt(&second).unwrap();
    write_json(&first_path, &first);
    write_json(&second_path, &second);

    let admission = build_admission(&[first_path, second_path]).unwrap();
    validate_admission_receipt(&admission).unwrap();
    assert!(admission.bootstrap_admission_eligible);
    assert!(!admission.bootstrap_eligible);
    assert!(!admission.ship_evidence_eligible);
    let admission_bytes = serde_json::to_vec_pretty(&admission).unwrap();

    let samples = scratch.0.join("samples");
    fs::create_dir(&samples).unwrap();
    write_chain(&samples, &digest(&admission_bytes));
    let sample_set = build_sample_set(&samples).unwrap();
    assert_eq!(sample_set.samples.len(), 5);
    assert_eq!(sample_set.runner_fingerprint, FINGERPRINT);
    assert_eq!(
        sample_set.full_dress_admission_sha256,
        digest(&admission_bytes)
    );
    assert!(sample_set.bootstrap_eligible);
    assert!(!sample_set.ship_evidence_eligible);
}

#[test]
fn state_machine_rejects_truncation_unknown_fields_identity_drift_and_reordering() {
    let scratch = Scratch::new("mutations");
    let first_path = scratch.0.join("first.json");
    let second_path = scratch.0.join("second.json");
    write_json(&first_path, &receipt_from_evidence(evidence(101)));
    write_json(&second_path, &receipt_from_evidence(evidence(102)));

    fs::write(&second_path, b"{\"schema_version\":1").unwrap();
    assert!(build_admission(&[first_path.clone(), second_path.clone()]).is_err());

    let mut unknown = serde_json::to_value(receipt_from_evidence(evidence(102))).unwrap();
    unknown["unexpected"] = serde_json::json!(true);
    fs::write(&second_path, serde_json::to_vec_pretty(&unknown).unwrap()).unwrap();
    assert!(build_admission(&[first_path.clone(), second_path.clone()]).is_err());

    let mut drifted = receipt_from_evidence(evidence(102));
    drifted.runner_provisioning_sha256 = "7".repeat(64);
    write_json(&second_path, &drifted);
    assert!(build_admission(&[first_path.clone(), second_path.clone()]).is_err());

    write_json(&second_path, &receipt_from_evidence(evidence(102)));
    let admission = build_admission(&[first_path, second_path]).unwrap();
    let admission_sha256 = digest(&serde_json::to_vec_pretty(&admission).unwrap());
    let samples = scratch.0.join("samples");
    fs::create_dir(&samples).unwrap();
    write_chain(&samples, &admission_sha256);

    let fourth_path = samples.join("sample-4.json");
    let mut fourth: BootstrapSampleReceipt =
        serde_json::from_slice(&fs::read(&fourth_path).unwrap()).unwrap();
    fourth.predecessor_receipt_sha256 = Some("7".repeat(64));
    write_json(&fourth_path, &fourth);
    assert!(build_sample_set(&samples).is_err());

    fs::remove_dir_all(&samples).unwrap();
    fs::create_dir(&samples).unwrap();
    write_chain(&samples, &admission_sha256);
    let fifth_path = samples.join("sample-5.json");
    let mut fifth: serde_json::Value =
        serde_json::from_slice(&fs::read(&fifth_path).unwrap()).unwrap();
    fifth["unexpected"] = serde_json::json!("must-fail-closed");
    fs::write(&fifth_path, serde_json::to_vec_pretty(&fifth).unwrap()).unwrap();
    assert!(build_sample_set(&samples).is_err());

    fs::remove_file(fifth_path).unwrap();
    assert!(build_sample_set(&samples).is_err());
}

#[test]
fn local_harness_is_pinned_non_promotable_and_covers_all_six_scenarios() {
    let harness =
        fs::read_to_string(root().join("scripts/perf/local-orchestration-preflight.ps1")).unwrap();
    let dockerfile =
        fs::read_to_string(root().join("scripts/perf/local-orchestration/Dockerfile")).unwrap();
    for required in [
        "state_machine",
        "systemd_lifecycle",
        "fault_injection",
        "offline_replay",
        "static_analysis",
        "cleanup_recovery",
        "bootstrap_eligible = $false",
        "ship_evidence_eligible = $false",
        "--network\", \"none",
        "foreign-checkout-identity",
        "offline-empty-cargo-cache",
        "actual-memory-only-smoke.sh",
        "cargo build --locked -p hydracache-loadgen -p hydracache-server",
        "$containerGitDir = \"/git\"",
        "$containerGitDir = \"/git/worktrees/$worktreeName\"",
        "Git directory is neither the primary worktree nor a linked worktree",
    ] {
        assert!(harness.contains(required), "local harness lost {required}");
    }
    for digest in [
        "561618e2c15bf2397621dd04f96926663a3b5616c189cf7e38db7e82f5c538ea",
        "365468470075493dc4583f47387001854321c5a8583ea9604b297e67f01c5a4f",
        "3aaec283e6e593bde528077d60280ac1589887067a39273348860837c9346d7e",
        "8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8",
    ] {
        assert!(
            harness.contains(digest) || dockerfile.contains(digest),
            "local harness lost pinned identity {digest}"
        );
    }
    assert!(!harness.contains(":latest"));
    assert!(!dockerfile.contains(":latest"));
    assert!(
        harness.contains("source=$cargoTargetVolume,target=/cargo-target"),
        "Cargo target volume must be mounted outside the read-only checkout"
    );
    assert_eq!(
        harness.matches("CARGO_TARGET_DIR=/cargo-target").count(),
        4,
        "every Cargo execution path must use the external target mount"
    );
    assert!(
        !harness.contains("target=/repo/target"),
        "a clean read-only checkout has no target mountpoint"
    );
    assert!(
        harness.contains("source=$cargoTargetVolume,target=/cargo-target,readonly"),
        "the actual-binary smoke must consume the prebuilt target read-only"
    );
}
