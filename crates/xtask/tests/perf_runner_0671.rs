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
    let receipt_import = read("scripts/perf/import-provisioning-receipt.sh");
    let rootless_docker = read("scripts/perf/rootless-docker.sh");
    let isolation = read("scripts/perf/provision-reference-isolation.sh");

    for script in [
        &audit,
        &service,
        &lifecycle,
        &receipt_import,
        &rootless_docker,
        &isolation,
    ] {
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
        "/proc/self/cgroup",
        "ship_evidence_eligible: false",
        "host_identity_digest",
        "sha256sum",
        "cpu_isolation",
        "housekeeping-only-v1",
        "provision-reference-isolation.sh verify",
        "target/test-evidence/0.67.1/runner-provisioned.json",
    ] {
        assert!(audit.contains(required), "host audit is missing {required}");
    }

    assert!(service.contains("expected label must be exactly hydracache-perf-v1"));
    assert!(service.contains(".repository == \"javaquasar/hydracache\""));
    assert!(service.contains(".service_user == \"github-runner\""));
    assert!(lifecycle.contains("online|offline|status"));
    assert!(!lifecycle.contains("enable "));
    assert!(receipt_import.contains("/var/lib/hydracache-perf/runner-provisioned.json"));
    assert!(receipt_import.contains("stat --format=%U"));
    assert!(receipt_import.contains(".source_commit == $commit"));
    assert!(receipt_import.contains(".host_identity_digest"));
    assert!(receipt_import.contains(".runner_online == false"));
    assert!(receipt_import.contains(".schema_version == 2"));
    assert!(receipt_import.contains(".cpu_isolation.smt_control == \"off\""));
    assert!(
        isolation.contains("isolcpus_argument=\"domain,managed_irq,nohz,1-4\"")
            && isolation.contains("isolcpus=${isolcpus_argument}")
    );
    assert!(isolation.contains("nohz_full=${measurement_cpus}"));
    assert!(isolation.contains("rcu_nocbs=${measurement_cpus}"));
    assert!(isolation.contains("irqaffinity=${housekeeping_cpus}"));
    assert!(isolation.contains("CPUAffinity=0 5 6 7"));
    assert!(isolation.contains("Before `nosmt` takes effect"));
    assert!(isolation.contains("= \"${cpu},${sibling}\""));
    assert!(isolation.contains("= \"$cpu\""));
    assert!(isolation.contains("if test -d \"/sys/devices/system/cpu/cpu${sibling}\"; then"));
    assert!(isolation.contains("IRQ affinity reaches measurement CPUs"));
    assert!(rootless_docker.contains("rootless Docker lifecycle must run as github-runner"));
    assert!(rootless_docker.contains("test ! -S /var/run/docker.sock"));
    assert!(rootless_docker.contains("grep --quiet rootless"));
    assert!(rootless_docker.contains("systemctl --user stop docker.service"));
    assert!(runbook.contains("systemctl start \"user@${runner_uid}.service\""));
    assert!(runbook.contains("rm --force /var/run/docker.sock"));

    for required in [
        "cloud-init",
        "github-runner",
        "hydracache-perf-v1",
        "scripts/perf/audit-reference-host.sh --mode provisioned",
        "scripts/perf/verify-runner-service.sh --expected-label hydracache-perf-v1",
        "docker-ce-rootless-extras",
        "scripts/perf/rootless-docker.sh",
        "scripts/perf/provision-reference-isolation.sh install",
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
    assert!(!audit.contains("read -r cpu_quota cpu_period extra </sys/fs/cgroup/cpu.max"));
    assert!(audit.contains("cgroup_cursor=\"/sys/fs/cgroup${cgroup_path%/}\""));
    assert!(audit.contains("if test -f \"$cgroup_cursor/cpu.max\"; then"));
    assert!(audit.contains("cgroup CPU quota detected at $cgroup_cursor"));
    assert!(audit.contains("test \"$cpu_controller_observed\" = true"));
    assert!(audit.contains("rootful container service must remain inactive"));
    assert!(audit.contains("sudo test -f \"$rootless_unit\""));
    assert!(audit.contains("sudo stat --format=%U \"$rootless_unit\""));
    assert!(audit.contains("/home/github-runner/.config/systemd/user/docker.service"));
    assert!(audit.contains("runner service must be offline"));
}

#[test]
fn provisioning_gate_executes_the_reviewed_script_without_shell_indirection() {
    let registry: xtask::gated_tests::GatedTestRegistry =
        toml::from_str(&read(xtask::gated_tests::REGISTRY_PATH)).unwrap();
    let gate = registry
        .gate
        .iter()
        .find(|gate| gate.id == "tool.perf-runner-provisioned-0671")
        .unwrap();

    assert_eq!(
        gate.command.program,
        "scripts/perf/import-provisioning-receipt.sh"
    );
    assert!(gate.command.args.is_empty());
}
