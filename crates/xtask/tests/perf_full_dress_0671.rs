use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use hydracache_loadgen::profile::{
    reference_cpu_isolation, MeasurementCore, RunnerAttestationV5, RunnerFingerprint,
    REFERENCE_FINGERPRINT_SCHEMA_VERSION, REFERENCE_HOST_CONTRACT_VERSION,
    REFERENCE_MEASUREMENT_CPUS, REFERENCE_OS_IMAGE, REFERENCE_RUNNER_CLASS,
    REFERENCE_STORAGE_CLASS,
};
use xtask::perf_bootstrap::{BootstrapArtifactDigest, FullReferenceEvidence};
use xtask::perf_full_dress::{
    build_admission, full_dress_context_problems, receipt_from_evidence, validate_admission_receipt,
};
use xtask::perf_qualification::QualificationContext;

const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn context() -> QualificationContext {
    QualificationContext {
        github_actions: "true".to_owned(),
        event_name: "workflow_dispatch".to_owned(),
        git_ref: "refs/heads/main".to_owned(),
        repository: "javaquasar/hydracache".to_owned(),
        head_repository: None,
        workflow_ref: "javaquasar/hydracache/.github/workflows/ci.yml@refs/heads/main".to_owned(),
        performance_mode: "full-dress".to_owned(),
        candidate_release: "0.67.1".to_owned(),
        runner_class: REFERENCE_RUNNER_CLASS.to_owned(),
        github_sha: SHA.to_owned(),
        git_head: SHA.to_owned(),
        github_run_id: "101".to_owned(),
        clean_worktree: true,
    }
}

fn runner(fingerprint: &str) -> RunnerFingerprint {
    RunnerFingerprint {
        runner_class: REFERENCE_RUNNER_CLASS.to_owned(),
        fingerprint: fingerprint.to_owned(),
        cpu_model: "fixture".to_owned(),
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

fn evidence(run_id: u64, fingerprint: &str) -> FullReferenceEvidence {
    FullReferenceEvidence {
        source_commit: SHA.to_owned(),
        github_run_id: run_id.to_string(),
        runner_fingerprint: fingerprint.to_owned(),
        observed_runner: runner(fingerprint),
        runner_provisioning_sha256: "6".repeat(64),
        prebuild_contract_digest: "a".repeat(64),
        scenario_contract_set_digest: "b".repeat(64),
        evidence_files: vec![BootstrapArtifactDigest {
            path: "target/test-evidence/0.67/local.json".to_owned(),
            sha256: "f".repeat(64),
        }],
    }
}

fn temp_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "hydracache-full-dress-{label}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn full_dress_context_is_manual_trusted_main_and_distinct_from_other_modes() {
    assert!(full_dress_context_problems(&context()).is_empty());
    for mode in ["qualify", "bootstrap", "off"] {
        let mut wrong = context();
        wrong.performance_mode = mode.to_owned();
        assert!(!full_dress_context_problems(&wrong).is_empty());
    }
}

#[test]
fn admission_requires_two_unique_identical_non_promotable_full_dress_runs() {
    let directory = temp_dir("admission");
    let first = directory.join("first.json");
    let second = directory.join("second.json");
    fs::write(
        &first,
        serde_json::to_vec_pretty(&receipt_from_evidence(evidence(101, &"c".repeat(64)))).unwrap(),
    )
    .unwrap();
    fs::write(
        &second,
        serde_json::to_vec_pretty(&receipt_from_evidence(evidence(102, &"c".repeat(64)))).unwrap(),
    )
    .unwrap();

    let admission = build_admission(&[first.clone(), second.clone()]).unwrap();
    validate_admission_receipt(&admission).unwrap();
    assert!(admission.bootstrap_admission_eligible);
    assert!(!admission.bootstrap_eligible);
    assert!(!admission.ship_evidence_eligible);
    assert!(build_admission(std::slice::from_ref(&first)).is_err());
    assert!(build_admission(&[first.clone(), first.clone()]).is_err());

    let mut provisioning_drift = receipt_from_evidence(evidence(102, &"c".repeat(64)));
    provisioning_drift.runner_provisioning_sha256 = "7".repeat(64);
    fs::write(
        &second,
        serde_json::to_vec_pretty(&provisioning_drift).unwrap(),
    )
    .unwrap();
    assert!(build_admission(&[first.clone(), second.clone()]).is_err());

    fs::write(
        &second,
        serde_json::to_vec_pretty(&receipt_from_evidence(evidence(102, &"9".repeat(64)))).unwrap(),
    )
    .unwrap();
    assert!(build_admission(&[first, second]).is_err());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn canary_full_dress_admits_mixed_runner_receipts() {
    let directory = temp_dir("canary-mixed");
    let first = directory.join("first.json");
    let second = directory.join("second.json");
    fs::write(
        &first,
        serde_json::to_vec_pretty(&receipt_from_evidence(evidence(101, &"c".repeat(64)))).unwrap(),
    )
    .unwrap();
    fs::write(
        &second,
        serde_json::to_vec_pretty(&receipt_from_evidence(evidence(102, &"9".repeat(64)))).unwrap(),
    )
    .unwrap();
    let accepted = build_admission(&[first, second]).is_ok();
    fs::remove_dir_all(directory).unwrap();

    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W3") {
        assert!(
            accepted,
            "HC-CANARY-RED:W3 mixed-identity full-dress receipts admitted bootstrap"
        );
    } else {
        assert!(!accepted);
    }
}

#[test]
fn full_dress_and_bootstrap_execute_the_same_reference_families_in_order() {
    let workflow = fs::read_to_string(root().join(".github/workflows/ci.yml")).unwrap();
    let _: serde_yaml::Value = serde_yaml::from_str(&workflow).unwrap();
    let full_dress = workflow
        .split("  release-0671-performance-full-dress:")
        .nth(1)
        .unwrap()
        .split("  release-0671-performance-bootstrap:")
        .next()
        .unwrap();
    let bootstrap = workflow
        .split("  release-0671-performance-bootstrap:")
        .nth(1)
        .unwrap()
        .split("  raft-loom:")
        .next()
        .unwrap();
    let commands = [
        "--gate env.hydracache-run-067-perf-core",
        "scripts/perf/rootless-docker.sh start",
        "--gate env.hydracache-run-067-perf-resp",
        "scripts/perf/rootless-docker.sh stop",
        "--gate env.hydracache-run-067-perf-control-plane",
    ];
    for job in [full_dress, bootstrap] {
        let positions = commands
            .iter()
            .map(|command| job.find(command).unwrap())
            .collect::<Vec<_>>();
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    }
    for required in [
        "inputs.performance_0671_mode == 'full-dress'",
        "performance-0671-full-dress-receipt",
        "performance-0671-full-dress-admission",
        "Admit bootstrap after the second identical full-dress run",
    ] {
        assert!(
            full_dress.contains(required),
            "full-dress job lost {required}"
        );
    }
}
