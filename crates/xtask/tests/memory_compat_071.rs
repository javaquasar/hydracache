use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value as JsonValue;
use toml::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

fn contract() -> Value {
    toml::from_str(
        &fs::read_to_string(root().join("docs/testing/compat/memory-071.toml"))
            .expect("compatibility contract"),
    )
    .expect("compat TOML")
}

#[test]
fn checked_in_upgrade_rollback_and_mixed_version_matrix_is_valid() {
    let problems = xtask::memory_contracts::check_compat(&root(), &contract(), "0.71");
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn missing_rollback_and_wire_generation_are_rejected() {
    let mut value = contract();
    value["row"]
        .as_array_mut()
        .expect("rows")
        .retain(|row| row["id"].as_str() != Some("candidate-to-baseline-compatible-rollback"));
    value["row"][0]["wire_generations"] = Value::Array(vec![Value::String("HC/1".to_owned())]);
    let problems = xtask::memory_contracts::check_compat(&root(), &value, "0.71");
    assert!(problems.iter().any(|problem| problem.contains("rollback")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("HC/1 and HC/2")));
}

#[test]
fn m10_executes_real_binary_compatibility_before_the_long_row() {
    let workflow = fs::read_to_string(root().join(".github/workflows/memory-reference-071.yml"))
        .expect("memory workflow");
    let compat = workflow
        .find("name: Prove real v0.70 candidate upgrade restart and rollback before M10")
        .expect("real compatibility step");
    let long_row = workflow
        .find("name: Run or resume bounded row")
        .expect("long row step");
    assert!(compat < long_row, "S8 must run before the 24-hour workload");
    for marker in [
        "inputs.case_id == 'M10-24h'",
        "scripts/perf/memory_compat_071.sh",
        "compatibility-receipt.json",
        "--candidate-sha \"$HYDRACACHE_MEMORY_SOURCE_SHA\"",
        "--workflow-sha \"$HYDRACACHE_MEMORY_WORKFLOW_SHA\"",
    ] {
        assert!(workflow.contains(marker), "workflow omits {marker}");
    }

    let script = fs::read_to_string(root().join("scripts/perf/memory_compat_071.sh"))
        .expect("compatibility executor");
    for marker in [
        "v0.70.0^{commit}",
        "verify-and-mutate-candidate",
        "hold-after-flush",
        "write-future-and-refuse",
        "memory_compat_process_071",
        "--test protocol --test versioned_codec",
    ] {
        assert!(
            script.contains(marker),
            "compatibility executor omits {marker}"
        );
    }
}

#[test]
fn public_api_gate_covers_every_publishable_package_and_blocks_release() {
    let manifest: JsonValue = serde_json::from_str(
        &fs::read_to_string(root().join("docs/testing/compat/v0.70.0.json"))
            .expect("public API compatibility manifest"),
    )
    .expect("public API JSON");
    assert_eq!(manifest["baseline_tag"], "v0.70.0");
    assert_eq!(manifest["tool"], "cargo-semver-checks");
    assert_eq!(manifest["tool_version"], "0.49.0");
    assert_eq!(
        manifest["profiles"],
        serde_json::json!(["default", "all-features"])
    );

    let output = std::process::Command::new("python")
        .current_dir(root())
        .args([
            "scripts/ci/public_api_compat_071.py",
            "--manifest",
            "docs/testing/compat/v0.70.0.json",
            "--check",
        ])
        .output()
        .expect("run public API inventory check");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let workflow =
        fs::read_to_string(root().join(".github/workflows/ci.yml")).expect("CI workflow");
    for marker in [
        "cargo install cargo-semver-checks --version 0.49.0 --locked",
        "python3 scripts/ci/public_api_compat_071.py --manifest docs/testing/compat/v0.70.0.json --output target/public-api-compat-071/receipt.json",
        "target/public-api-compat-071/**",
    ] {
        assert!(workflow.contains(marker), "CI workflow omits {marker}");
    }
}
