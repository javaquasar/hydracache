use std::fs;

fn read(path: &str) -> String {
    let root = xtask::doc_check::find_repo_root().unwrap();
    fs::read_to_string(root.join(path)).unwrap()
}

#[test]
fn runner_runbook_and_helpers_are_fail_closed_and_secret_free() {
    let runbook = read("docs/testing/PERF_RUNNER_0_67_1.md");
    let audit = read("scripts/perf/audit-reference-host.sh");
    let service = read("scripts/perf/verify-runner-service.sh");
    let lifecycle = read("scripts/perf/runner-service.sh");

    for script in [&audit, &service, &lifecycle] {
        assert!(script.starts_with("#!/usr/bin/env bash\nset -euo pipefail\n"));
        assert!(!script.contains('\r'));
        assert!(!script.contains("PRIVATE KEY"));
        assert!(!script.contains("RUNNER_TOKEN="));
        assert!(!script.contains("REMOVE_TOKEN="));
    }

    for required in [
        "systemd-detect-virt --quiet",
        "distinct_measurement_cores",
        "taskset --cpu-list 1-4",
        "storage_transport: \"nvme\"",
        "cgroup_cpu_quota: \"unlimited\"",
        "ship_evidence_eligible: false",
        "target/test-evidence/0.67.1/runner-provisioned.json",
    ] {
        assert!(audit.contains(required), "host audit is missing {required}");
    }

    assert!(service.contains("expected label must be exactly hydracache-perf-v1"));
    assert!(service.contains(".repository == \"javaquasar/hydracache\""));
    assert!(service.contains(".service_user == \"github-runner\""));
    assert!(lifecycle.contains("online|offline|status"));
    assert!(!lifecycle.contains("enable "));

    for required in [
        "cloud-init",
        "github-runner",
        "hydracache-perf-v1",
        "scripts/perf/audit-reference-host.sh --mode provisioned",
        "scripts/perf/verify-runner-service.sh --expected-label hydracache-perf-v1",
        "Do **not** dispatch",
    ] {
        assert!(
            runbook.contains(required),
            "runner runbook is missing {required}"
        );
    }
}

#[test]
fn runner_contract_has_exact_offline_lifecycle_and_public_labels() {
    let audit = read("scripts/perf/audit-reference-host.sh");
    let service = read("scripts/perf/verify-runner-service.sh");

    let expected_labels =
        ".labels == [\"self-hosted\", \"linux\", \"x64\", \"hydracache-perf-v1\"]";
    assert!(audit.contains(expected_labels));
    assert!(service.contains(".labels == [\"self-hosted\", \"linux\", \"x64\", $expected]"));
    assert!(audit.contains("runner_online: false"));
    assert!(audit.contains("runner service must be offline"));
}
