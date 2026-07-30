use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use hydracache_loadgen::profile::{
    reference_cpu_isolation, MeasurementCore, RunnerAttestationV5, RunnerFingerprint,
    REFERENCE_FINGERPRINT_SCHEMA_VERSION, REFERENCE_HOST_CONTRACT_VERSION,
    REFERENCE_MEASUREMENT_CPUS, REFERENCE_OS_IMAGE, REFERENCE_RUNNER_CLASS,
    REFERENCE_STORAGE_CLASS,
};
use xtask::perf_bootstrap::{
    bootstrap_context_problems, build_sample_set, BootstrapArtifactDigest, BootstrapSampleReceipt,
};
use xtask::perf_qualification::QualificationContext;

const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

fn context() -> QualificationContext {
    QualificationContext {
        github_actions: "true".to_owned(),
        event_name: "workflow_dispatch".to_owned(),
        git_ref: "refs/heads/main".to_owned(),
        repository: "javaquasar/hydracache".to_owned(),
        head_repository: None,
        workflow_ref: "javaquasar/hydracache/.github/workflows/ci.yml@refs/heads/main".to_owned(),
        performance_mode: "bootstrap".to_owned(),
        candidate_release: "0.67.1".to_owned(),
        runner_class: REFERENCE_RUNNER_CLASS.to_owned(),
        github_sha: SHA.to_owned(),
        git_head: SHA.to_owned(),
        github_run_id: "100".to_owned(),
        clean_worktree: true,
    }
}

fn temp_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "hydracache-bootstrap-{label}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

fn sample(run: u64, fingerprint: &str) -> BootstrapSampleReceipt {
    BootstrapSampleReceipt {
        schema_version: 1,
        release: "0.67.1".to_owned(),
        profile: "reference-v1".to_owned(),
        source_commit: SHA.to_owned(),
        github_run_id: run.to_string(),
        runner_fingerprint: fingerprint.to_owned(),
        observed_runner: RunnerFingerprint {
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
        },
        prebuild_contract_digest: "a".repeat(64),
        scenario_contract_set_digest: "b".repeat(64),
        evidence_files: vec![BootstrapArtifactDigest {
            path: "target/test-evidence/0.67/local.json".to_owned(),
            sha256: "f".repeat(64),
        }],
        passed: true,
        bootstrap_eligible: true,
        ship_evidence_eligible: false,
    }
}

fn write_samples(directory: &Path, fingerprints: &[String]) {
    for (index, fingerprint) in fingerprints.iter().enumerate() {
        fs::write(
            directory.join(format!("sample-{index}.json")),
            serde_json::to_vec_pretty(&sample(index as u64 + 1, fingerprint)).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn bootstrap_context_is_manual_trusted_main_and_distinct_from_qualification() {
    assert!(bootstrap_context_problems(&context()).is_empty());
    let mut qualify = context();
    qualify.performance_mode = "qualify".to_owned();
    assert!(!bootstrap_context_problems(&qualify).is_empty());
}

#[test]
fn committed_acquisition_manifest_is_empty_and_non_promotable_before_collection() {
    let manifest = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/testing/perf-bootstrap/0.67.1/sample-set.toml"),
    )
    .unwrap();
    let value: toml::Value = toml::from_str(&manifest).unwrap();
    assert_eq!(value["status"].as_str(), Some("awaiting-dedicated-host"));
    assert_eq!(value["minimum_samples"].as_integer(), Some(5));
    assert_eq!(value["sample_receipts"].as_array().unwrap().len(), 0);
    assert_eq!(value["bootstrap_eligible"].as_bool(), Some(false));
    assert_eq!(value["ship_evidence_eligible"].as_bool(), Some(false));
}
#[test]
fn bootstrap_workflow_collects_full_reference_families_without_ship_promotion() {
    let workflow = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.github/workflows/ci.yml"),
    )
    .unwrap();
    let _: serde_yaml::Value = serde_yaml::from_str(&workflow).unwrap();
    for required in [
        "release-0671-performance-bootstrap:",
        "inputs.performance_0671_mode == 'bootstrap'",
        "Run bootstrap core reference evidence",
        "Start isolated rootless Docker for Redis comparison",
        "Run bootstrap RESP reference evidence",
        "Stop isolated rootless Docker after Redis comparison",
        "Run bootstrap control-plane reference evidence",
        "Retain non-ship bootstrap sample",
        "Prepare tmpfs reference evidence",
        "Materialize tmpfs reference evidence",
        "group: release-067-performance-reference-v1",
    ] {
        assert!(
            workflow.contains(required),
            "bootstrap workflow lost {required}"
        );
    }
    let bootstrap_job = workflow
        .split("  release-0671-performance-bootstrap:")
        .nth(1)
        .unwrap()
        .split("  raft-loom:")
        .next()
        .unwrap();
    assert!(!bootstrap_job.contains("Check 0.67 performance budgets"));
    assert!(!bootstrap_job.contains("--require-ship"));
    assert!(!bootstrap_job.contains("taskset --cpu-list 1-4 cargo run"));
    let materialize = bootstrap_job
        .find("Materialize tmpfs reference evidence")
        .unwrap();
    let retain = bootstrap_job
        .find("Retain non-ship bootstrap sample")
        .unwrap();
    assert!(materialize < retain);
    assert!(bootstrap_job.contains("scripts/perf/reference-evidence-tmpfs.sh materialize"));
}
#[test]
fn sample_set_requires_five_unique_same_fingerprint_and_contract_receipts() {
    let valid = temp_dir("valid");
    write_samples(&valid, &vec!["c".repeat(64); 5]);
    let set = build_sample_set(&valid).unwrap();
    assert_eq!(set.samples.len(), 5);
    assert!(set.bootstrap_eligible);
    assert!(!set.ship_evidence_eligible);

    let short = temp_dir("short");
    write_samples(&short, &vec!["c".repeat(64); 4]);
    assert!(build_sample_set(&short).is_err());

    let mixed = temp_dir("mixed");
    write_samples(
        &mixed,
        &[
            "c".repeat(64),
            "c".repeat(64),
            "c".repeat(64),
            "c".repeat(64),
            "9".repeat(64),
        ],
    );
    assert!(build_sample_set(&mixed).is_err());

    fs::remove_dir_all(valid).unwrap();
    fs::remove_dir_all(short).unwrap();
    fs::remove_dir_all(mixed).unwrap();
}

#[test]
fn canary_bootstrap_accepts_a_mixed_fingerprint_sample_set() {
    let directory = temp_dir("canary-mixed");
    write_samples(
        &directory,
        &[
            "c".repeat(64),
            "c".repeat(64),
            "c".repeat(64),
            "c".repeat(64),
            "9".repeat(64),
        ],
    );
    let accepted = build_sample_set(&directory).is_ok();
    fs::remove_dir_all(directory).unwrap();

    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W4") {
        assert!(
            accepted,
            "HC-CANARY-RED:W4 mixed-fingerprint bootstrap samples were accepted"
        );
    } else {
        assert!(!accepted);
    }
}
