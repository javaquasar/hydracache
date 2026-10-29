//! Tiny executable correctness fixtures. No sealed numerical cohort or claim.
use get_owner_scheduled_controls_074::native::Dataset;
use sha2::{Digest, Sha256};
use std::process::Command;

fn config_file() -> tempfile::NamedTempFile {
    let file = tempfile::NamedTempFile::new().unwrap();
    let config = serde_json::json!({
        "schema_version": 1, "profile_id": "unprofiled-timing-controls-074-v1",
        "surface": "direct", "operation": "get", "seed": 740074,
        "keyspace": 1, "payload_bytes": 1, "dataset_sha256": Dataset::new(1, 1).unwrap().digest(),
        "slots": 1, "pipeline_depth": 0, "warmup_calls": 1,
        "minimum_usable_cpu_ns": 1_000_000_000_u64,
        "minimum_usable_measurement_wall_ns": 1_000_000_000_u64,
        "schedule": { "operations": 1, "offered_rate_per_second": 1,
            "concurrency": 1, "maximum_queued": 0,
            "operation_timeout_ns": 5_000_000_000_u64,
            "drain_timeout_ns": 5_000_000_000_u64, "slo_ns": 5_000_000_000_u64,
            "highest_trackable_ns": 10_000_000_000_u64 }
    });
    std::fs::write(file.path(), serde_json::to_vec(&config).unwrap()).unwrap();
    file
}

#[test]
fn validation_starts_no_fixture_and_bad_binary_seal_never_runs() {
    let file = config_file();
    let exe = env!("CARGO_BIN_EXE_timing-controls-074");
    let validation = Command::new(exe)
        .arg("--validate")
        .arg(file.path())
        .output()
        .unwrap();
    assert!(validation.status.success());
    let value: serde_json::Value = serde_json::from_slice(&validation.stdout).unwrap();
    assert_eq!(value["fixture_started"], false);
    let rejected = Command::new(exe)
        .arg("--run")
        .arg(file.path())
        .args(["0".repeat(64), "b".repeat(40)])
        .output()
        .unwrap();
    assert!(!rejected.status.success());
    assert!(rejected.stdout.is_empty());
    let reason = String::from_utf8(rejected.stderr).unwrap();
    assert!(
        reason.contains(if cfg!(feature = "allocation-diagnostics") {
            "refuses allocation-diagnostics"
        } else {
            "binary seal mismatch"
        })
    );
}

#[test]
fn single_offer_executable_preserves_identity_and_non_admission_or_refuses_profiled_build() {
    let file = config_file();
    let exe = env!("CARGO_BIN_EXE_timing-controls-074");
    let hash = format!("{:x}", Sha256::digest(std::fs::read(exe).unwrap()));
    // Dummy source deliberately has no git verification; the receipt must say so.
    let output = Command::new(exe)
        .arg("--run")
        .arg(file.path())
        .args([hash.clone(), "b".repeat(40)])
        .output()
        .unwrap();
    if cfg!(feature = "allocation-diagnostics") {
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8(output.stderr)
            .unwrap()
            .contains("refuses allocation-diagnostics"));
        return;
    }
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["binary_sha256"], hash);
    assert_eq!(value["source_git_identity_verified_by_binary"], false);
    assert_eq!(value["report"]["shutdown_verified"], true);
    assert_eq!(value["report"]["final_dataset_verified"], true);
    assert_eq!(value["report"]["admission_allowed"], false);
    assert!(value.get("fixed_material").is_none()); // Legacy envelope stays unchanged.
    assert_eq!(
        value["report"]["get_owner_feature"],
        cfg!(feature = "get-owner")
    );
    assert_eq!(value["report"]["observation"]["observed"]["offered"], 1);
    assert_eq!(
        value["compiled_root_lock_sha256"],
        format!(
            "{:x}",
            Sha256::digest(include_bytes!("../../../Cargo.lock"))
        )
    );
    assert_eq!(
        value["compiled_observer_lock_sha256"],
        format!("{:x}", Sha256::digest(include_bytes!("../Cargo.lock")))
    );
}

#[test]
fn material_modes_are_metadata_only_private_and_exclusive() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("private.json");
    let exe = env!("CARGO_BIN_EXE_timing-controls-074");
    let generated = Command::new(exe)
        .arg("--generate-material")
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    assert!(generated.stderr.is_empty());
    let receipt: serde_json::Value = serde_json::from_slice(&generated.stdout).unwrap();
    assert_eq!(receipt["fixture_started"], false);
    assert_eq!(receipt["admission_allowed"], false);
    assert_eq!(receipt["fixed_material"]["production_enrollment"], false);
    assert!(!String::from_utf8_lossy(&generated.stdout).contains("PRIVATE KEY"));
    let pin = receipt["fixed_material"]["material_sha256"]
        .as_str()
        .unwrap();
    let before = std::fs::read(&path).unwrap();
    let duplicate = Command::new(exe)
        .arg("--generate-material")
        .arg(&path)
        .output()
        .unwrap();
    assert!(!duplicate.status.success());
    assert!(duplicate.stdout.is_empty());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let validated = Command::new(exe)
        .arg("--validate-material")
        .arg(&path)
        .arg(pin)
        .output()
        .unwrap();
    assert!(validated.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&validated.stdout).unwrap(),
        receipt
    );
    let wrong = Command::new(exe)
        .arg("--validate-material")
        .arg(&path)
        .arg("0".repeat(64))
        .output()
        .unwrap();
    assert!(!wrong.status.success());
    assert!(wrong.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&wrong.stderr).contains("PRIVATE KEY"));
}

#[test]
fn fresh_secure_processes_use_one_pin_and_plaintext_refuses_before_fixture() {
    use get_owner_scheduled_controls_074::security::MtlsFixture;
    let dir = tempfile::tempdir().unwrap();
    let material = dir.path().join("private.json");
    let seal = MtlsFixture::new().unwrap().save_new(&material).unwrap();
    let file = config_file();
    let original: serde_json::Value =
        serde_json::from_slice(&std::fs::read(file.path()).unwrap()).unwrap();
    let exe = env!("CARGO_BIN_EXE_timing-controls-074");
    let binary = format!("{:x}", Sha256::digest(std::fs::read(exe).unwrap()));
    for surface in ["direct", "resp2-mtls", "resp3-mtls", "hc2-mtls"] {
        let mut config = original.clone();
        config["surface"] = serde_json::json!(surface);
        config["pipeline_depth"] =
            serde_json::json!(if surface.starts_with("resp") { 1 } else { 0 });
        std::fs::write(file.path(), serde_json::to_vec(&config).unwrap()).unwrap();
        for _ in 0..2 {
            let output = Command::new(exe)
                .arg("--run-with-material")
                .arg(file.path())
                .args([binary.as_str(), &"b".repeat(40)])
                .arg(&material)
                .arg(&seal.material_sha256)
                .output()
                .unwrap();
            if surface == "direct" || cfg!(feature = "allocation-diagnostics") {
                assert!(!output.status.success());
                assert!(output.stdout.is_empty());
                let error = String::from_utf8(output.stderr).unwrap();
                assert!(error.contains(if cfg!(feature = "allocation-diagnostics") {
                    "refuses allocation-diagnostics"
                } else {
                    "fixed material requires an mTLS surface before fixture"
                }));
                continue;
            }
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(
                value["fixed_material"],
                serde_json::to_value(&seal).unwrap()
            );
            assert_eq!(
                value["report"]["transport_security"],
                serde_json::to_value(&seal.transport).unwrap()
            );
            assert_eq!(value["report"]["final_dataset_verified"], true);
            assert_eq!(value["report"]["shutdown_verified"], true);
            assert_eq!(
                value["report"]["secure_fresh_process_material_parity_proven"],
                false
            );
            assert_eq!(value["report"]["admission_allowed"], false);
            assert_eq!(value["report"]["cpu"]["usable_for_ratio"], false);
            assert_eq!(
                value["report"]["get_owner_feature"],
                cfg!(feature = "get-owner")
            );
            assert!(!String::from_utf8_lossy(&output.stdout).contains("PRIVATE KEY"));
        }
    }
    // Correct binary and valid secure config cannot bypass a stale material pin.
    let rejected = Command::new(exe)
        .arg("--run-with-material")
        .arg(file.path())
        .args([binary, "b".repeat(40)])
        .arg(&material)
        .arg("0".repeat(64))
        .output()
        .unwrap();
    assert!(!rejected.status.success());
    assert!(rejected.stdout.is_empty());
}
