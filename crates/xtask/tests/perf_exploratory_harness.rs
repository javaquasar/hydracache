use std::fs;

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
