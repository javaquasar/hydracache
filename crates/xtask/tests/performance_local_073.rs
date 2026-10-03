use serde_json::{json, Value as JsonValue};
use std::path::{Path, PathBuf};

const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

fn identity() -> JsonValue {
    json!({
        "os": "windows",
        "arch": "x86_64",
        "logical_cpus": 16,
        "cpu_model_sha256": "1".repeat(64),
        "rustc_vv_sha256": "2".repeat(64),
        "cargo_version_sha256": "3".repeat(64)
    })
}

#[test]
fn generated_context_satisfies_schema_and_semantics() {
    let context =
        xtask::performance_local::build_context_at(SHA, true, identity(), "2026-09-23T10:00:00Z");
    let problems = xtask::performance_local::check_context_at_root(&root(), &context)
        .expect("validate local context");
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn fingerprint_ignores_time_and_source_observations() {
    let first =
        xtask::performance_local::build_context_at(SHA, false, identity(), "2026-09-23T10:00:00Z");
    let second = xtask::performance_local::build_context_at(
        "ffffffffffffffffffffffffffffffffffffffff",
        true,
        identity(),
        "2026-09-24T10:00:00Z",
    );
    assert_eq!(
        first["host_fingerprint_sha256"],
        second["host_fingerprint_sha256"]
    );
}

#[test]
fn fingerprint_changes_when_stable_identity_changes() {
    let first =
        xtask::performance_local::build_context_at(SHA, false, identity(), "2026-09-23T10:00:00Z");
    let mut changed = identity();
    changed["logical_cpus"] = json!(32);
    let second =
        xtask::performance_local::build_context_at(SHA, false, changed, "2026-09-23T10:00:00Z");
    assert_ne!(
        first["host_fingerprint_sha256"],
        second["host_fingerprint_sha256"]
    );
}

#[test]
fn context_never_contains_raw_personal_or_hardware_values() {
    let context =
        xtask::performance_local::build_context_at(SHA, false, identity(), "2026-09-23T10:00:00Z");
    let serialized = serde_json::to_string(&context).expect("serialize context");
    for forbidden in [
        "computer_name",
        "userprofile",
        "home_path\":",
        "serial_number\":",
        "mac_address\":",
    ] {
        assert!(
            !serialized.to_ascii_lowercase().contains(forbidden),
            "context leaked forbidden field {forbidden}: {serialized}"
        );
    }
    assert_eq!(
        context["privacy"]["raw_hardware_values_retained"],
        json!(false)
    );
}

#[test]
fn tampered_fingerprint_is_rejected() {
    let mut context =
        xtask::performance_local::build_context_at(SHA, false, identity(), "2026-09-23T10:00:00Z");
    context["host_fingerprint_sha256"] = json!("f".repeat(64));
    assert!(xtask::performance_local::check_context(&context)
        .iter()
        .any(|problem| problem.contains("does not match identity")));
}
