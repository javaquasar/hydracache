use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

fn read(path: &str) -> String {
    let root = xtask::doc_check::find_repo_root().unwrap();
    fs::read_to_string(root.join(path))
        .unwrap()
        .replace("\r\n", "\n")
}

#[test]
fn relative_eight_runner_keeps_targets_workloads_and_provenance_explicit() {
    let runner = read("scripts/perf/run-relative-eight-cases-telemetry.sh");

    assert!(runner.starts_with("#!/usr/bin/env bash\nset -euo pipefail\n"));
    assert!(runner.contains("targets=hydracache,redis,hazelcast-community"));
    assert!(runner.contains("for op in set get"));
    assert!(runner.contains("for target in hydra redis hazelcast"));
    assert_eq!(runner.matches("'p").count(), 8, "scenario matrix changed");
    assert!(runner.contains("TELEMETRY_INTERVAL_SECONDS-1"));
    assert!(runner.contains("HAZELCAST_IMAGE must include a full sha256 digest"));
    assert!(runner.contains("redis@sha256:"));
    assert!(!runner.contains(":latest"));
    assert!(runner.contains("docker inspect hydracache-relative-redis"));
    assert!(runner.contains("docker inspect hydracache-relative-hazelcast"));
    assert!(runner.contains("source_commit=$(git rev-parse HEAD)"));
    assert!(runner.contains("runner_receipt_sha256="));
    assert!(runner.contains("reference-runtime-irq-delta-guard.sh"));
    assert!(runner.contains("EXPLORATORY_STORAGE_MODE-filesystem"));
    assert!(runner.contains("ram-only diagnostics require output below /dev/shm"));
    assert!(runner.contains("findmnt --noheadings --output FSTYPE"));
    assert!(runner.contains("evidence_class=indicative-exploratory-v1"));
    assert!(runner.contains("capacity_bearing=false"));
    assert!(runner.contains("ship_evidence_eligible=false"));
    assert!(runner.contains("perf-policies/indicative-exploratory-v1.json"));
}

#[test]
fn indicative_policy_and_report_are_non_authoritative_and_non_promotable() {
    let policy: Value = serde_json::from_str(&read(
        "docs/testing/perf-policies/indicative-exploratory-v1.json",
    ))
    .unwrap();
    let renderer = read("scripts/perf/render-exploratory-report.py");
    let documentation = read("docs/testing/PERF_INDICATIVE_0_67_1.md");

    assert_eq!(policy["policy_id"], "indicative-exploratory-v1");
    assert_eq!(policy["evidence_class"], "indicative-exploratory-v1");
    for field in [
        "authoritative",
        "capacity_bearing",
        "qualification_evidence",
        "bootstrap_evidence",
        "ship_evidence_eligible",
    ] {
        assert_eq!(policy[field], false, "indicative policy changed {field}");
        assert!(renderer.contains(&format!("\"{field}\": False")));
    }
    assert_eq!(policy["allowed_storage_modes"][0], "filesystem");
    assert_eq!(policy["allowed_storage_modes"][1], "ram-only");
    assert!(renderer.contains("indicative-receipt.json"));
    assert!(renderer.contains("production sizing guidance"));
    assert!(documentation.contains("never a substitute for"));
    assert!(documentation.contains("It does not"));
    assert!(documentation.contains("relax either IRQ guard"));
}

#[test]
fn indicative_renderer_emits_a_sealed_negative_claim_receipt() {
    let repo = xtask::doc_check::find_repo_root().unwrap();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "hydracache-indicative-render-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("reproduction-command.txt"),
        "targets=hydracache,redis,hazelcast-community\nexploratory_storage_mode=filesystem\n",
    )
    .unwrap();
    fs::write(root.join("hardware-validation.txt"), "guard=passed\n").unwrap();
    fs::write(root.join("telemetry-summary.json"), "{}\n").unwrap();

    let python = if cfg!(windows) { "python" } else { "python3" };
    let status = Command::new(python)
        .current_dir(&repo)
        .arg("scripts/perf/render-exploratory-report.py")
        .arg("--input")
        .arg(&root)
        .arg("--output")
        .arg(root.join("report.md"))
        .arg("--source-root")
        .arg(&repo)
        .arg("--policy")
        .arg("docs/testing/perf-policies/indicative-exploratory-v1.json")
        .status()
        .unwrap();
    assert!(status.success());

    let receipt: Value =
        serde_json::from_str(&fs::read_to_string(root.join("indicative-receipt.json")).unwrap())
            .unwrap();
    assert_eq!(receipt["authoritative"], false);
    assert_eq!(receipt["capacity_bearing"], false);
    assert_eq!(receipt["ship_evidence_eligible"], false);
    assert_eq!(receipt["storage_mode"], "filesystem");
    assert!(receipt["input_sha256"]["hardware_validation"].is_string());
    assert!(receipt["input_sha256"]["reproduction_command"].is_string());
    assert!(receipt["input_sha256"]["telemetry_summary"].is_string());
    let manifest = fs::read_to_string(root.join("artifact-manifest.json")).unwrap();
    assert!(manifest.contains("indicative-receipt.json"));
    let report = fs::read_to_string(root.join("report.md")).unwrap();
    assert!(report.contains("not authoritative"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn telemetry_keeps_raw_and_summary_metrics_distinct() {
    let collector = read("scripts/perf/collect-target-telemetry.py");
    let summary = read("scripts/perf/summarize-telemetry.py");

    for required in [
        "container_cpu_percent",
        "process_cpu_percent",
        "process_cpu_ticks",
        "vmrss_bytes",
        "vmhwm_bytes",
        "effective_cpu_affinity",
        "cgroup_memory_current_bytes",
        "cgroup_memory_peak_bytes",
        "cgroup_memory_limit_bytes",
        "jvm_heap_available",
        "jvm_heap_used_bytes",
        "process_minor_faults",
        "process_major_faults",
        "process_read_bytes",
        "process_write_bytes",
        "psi_memory_some_avg10",
        "psi_cpu_some_avg10",
        "psi_io_some_avg10",
    ] {
        assert!(collector.contains(required), "collector lacks {required}");
        if required != "effective_cpu_affinity"
            && required != "cgroup_memory_limit_bytes"
            && required != "jvm_heap_available"
        {
            assert!(summary.contains(required), "summary lacks {required}");
        }
    }
    assert!(collector.contains("with_suffix(\".csv\")"));
    assert!(collector.contains("json_file.write(json.dumps(row"));
    assert!(summary.contains("\"p50\": percentile(values, 0.50)"));
    assert!(summary.contains("\"p95\": percentile(values, 0.95)"));
    assert!(summary.contains("\"max\": max(values)"));
}

#[test]
fn hazelcast_uses_one_named_distributed_map() {
    let workload = read("scripts/perf/hazelcast-workload.py");

    assert!(workload.contains("client.get_map(\"exploratory-067\")"));
    assert!(workload.contains("cache.set("));
    assert!(workload.contains("cache.get("));
    assert!(workload.contains("cluster_members=[f\"{args.host}:{args.port}\"]"));
}

#[test]
fn memory_leak_runner_never_treats_rejected_hydra_flushall_as_reset() {
    let runner = read("scripts/perf/run-memory-leak-stage.sh");
    let renderer = read("scripts/perf/render-memory-leak-report.py");

    assert!(runner.contains("reset_target()"));
    assert!(runner.contains("HYDRACACHE_DIAGNOSTIC_RESET_ENABLED=true"));
    assert!(runner.contains("/admin/diagnostics/reset"));
    assert!(runner.contains(".embedded_after == 0"));
    assert!(runner.contains(".client.after.store_entries // 0"));
    assert!(runner.contains(".client.after.conditional.session_heartbeats // 0"));
    assert!(runner.contains("' >/dev/null || return"));
    assert!(runner.contains("response=\"$(redis-cli --raw"));
    assert!(runner.contains("[[ \"$response\" == \"OK\" ]]"));
    assert!(runner.contains("[[ \"$remaining\" == \"0\" ]]"));
    assert!(runner.contains("reset-verified"));
    assert!(runner.contains("MEMORY_DIAGNOSTIC_TARGETS-hydra redis hazelcast"));
    assert!(runner.contains("ship_evidence_eligible=false"));
    assert!(runner.contains("source_tree_clean=true"));
    assert!(runner.contains("hydracache_binary_sha256=$(sha256sum"));
    assert!(runner.contains("git status --porcelain"));
    assert!(runner.contains("experiment_status"));
    assert!(!runner.contains("FLUSHALL >/dev/null"));
    assert!(renderer.contains("off-by-default local admin diagnostic reset"));
    assert!(renderer.contains("rejected RESP FLUSHALL is never treated as cleanup"));
}
