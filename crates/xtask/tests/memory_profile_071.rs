use std::path::PathBuf;
use std::process::Command;

use serde_json::json;

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn loadgen_declares_the_exact_phase_order_and_epoch_fields() {
    let source = std::fs::read_to_string(
        repository_root().join("crates/hydracache-loadgen/src/memory_efficiency.rs"),
    )
    .expect("memory efficiency source");
    let mut cursor = 0;
    for phase in [
        "MemoryPhase::Cold",
        "MemoryPhase::Fill",
        "MemoryPhase::Steady",
        "MemoryPhase::ExpireOrDelete",
        "MemoryPhase::Reset",
        "MemoryPhase::Refill",
        "MemoryPhase::PostIdle",
        "MemoryPhase::Shutdown",
    ] {
        let offset = source[cursor..].find(phase).expect("mandatory phase");
        cursor += offset + phase.len();
    }
    for field in [
        "sequence",
        "epoch",
        "monotonic_ns",
        "owner_snapshot_digest",
        "telemetry_checkpoint",
        "provider_mark",
    ] {
        assert!(source.contains(field), "missing timeline field {field}");
    }
}

#[test]
fn every_admitted_allocator_has_the_complete_provider_protocol() {
    let root = repository_root().join("scripts/perf/memory-providers");
    let common = std::fs::read_to_string(root.join("provider_common.py")).expect("provider");
    for command in ["probe", "start", "mark", "snapshot", "stop", "normalize"] {
        assert!(
            common.contains(&format!("add_parser(\"{command}\")")),
            "missing provider command {command}"
        );
    }
    for provider in ["system", "jemalloc", "mimalloc"] {
        let wrapper =
            std::fs::read_to_string(root.join(format!("{provider}.py"))).expect("provider wrapper");
        assert!(wrapper.contains(&format!("main(\"{provider}\")")));
    }
}

#[test]
fn provider_fixture_proves_ordering_classification_and_redaction() {
    let script = repository_root().join("scripts/perf/memory-providers/test_provider_contract.py");
    let output = Command::new("python")
        .arg(script)
        .output()
        .expect("python is required by the 0.71 provider contract gate");
    assert!(
        output.status.success(),
        "provider contract failed:\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn profile_output_is_excluded_from_production_sizing() {
    let source = std::fs::read_to_string(
        repository_root().join("crates/hydracache-loadgen/src/memory_efficiency.rs"),
    )
    .expect("memory efficiency source");
    assert!(source.contains("promotable: false"));
    assert!(source.contains("workload_epoch_acknowledged"));
}

#[test]
fn host_fingerprint_excludes_volatile_admission_observations() {
    let first = json!({
        "platform": "linux",
        "hardware_model": "stable-cpu",
        "kernel": "stable-kernel",
        "tools": {"cargo": "cargo 1.88.0"},
        "competing_load": "0.01 0.02 0.03",
        "available_memory": "MemAvailable: 1000 kB",
        "temperature": "42000",
        "major_faults_and_throttling": "Pid: 10"
    });
    let mut second = first.clone();
    second["competing_load"] = json!("4.00 3.00 2.00");
    second["available_memory"] = json!("MemAvailable: 900 kB");
    second["temperature"] = json!("51000");
    second["major_faults_and_throttling"] = json!("Pid: 99");

    let first_identity = xtask::memory_contracts::host_identity_probes(&first);
    let second_identity = xtask::memory_contracts::host_identity_probes(&second);
    assert_eq!(
        xtask::memory_contracts::canonical_json_digest(&first_identity),
        xtask::memory_contracts::canonical_json_digest(&second_identity)
    );

    second["kernel"] = json!("changed-kernel");
    assert_ne!(
        xtask::memory_contracts::canonical_json_digest(&first_identity),
        xtask::memory_contracts::canonical_json_digest(
            &xtask::memory_contracts::host_identity_probes(&second)
        )
    );
}
