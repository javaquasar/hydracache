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
    let runtime_irq_guard = read("scripts/perf/reference-runtime-irq-guard.sh");
    let evidence_tmpfs = read("scripts/perf/reference-evidence-tmpfs.sh");
    let measurement = read("scripts/perf/run-reference-measurement.sh");
    let prebuild = read("crates/xtask/src/perf.rs");
    let host_attestation = read("crates/xtask/src/host_attestation.rs");
    let workflow = read(".github/workflows/ci.yml");

    for script in [
        &audit,
        &service,
        &lifecycle,
        &receipt_import,
        &rootless_docker,
        &isolation,
        &runtime_irq_guard,
        &evidence_tmpfs,
        &measurement,
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
    assert!(receipt_import.contains(".schema_version == 4"));
    assert!(receipt_import.contains(".cpu_isolation.smt_control == \"off\""));
    assert!(
        receipt_import.contains(".cpu_isolation.measurement_idle_policy == \"latency-cap-us-v1\"")
    );
    assert!(receipt_import.contains(".cpu_isolation.measurement_max_idle_latency_us == 1"));
    assert!(
        receipt_import.contains(".cpu_isolation.housekeeping_idle_policy == \"latency-cap-us-v1\"")
    );
    assert!(receipt_import.contains(".cpu_isolation.housekeeping_max_idle_latency_us == 1"));
    assert!(
        isolation.contains("isolcpus_argument=\"domain,managed_irq,nohz,1-4\"")
            && isolation.contains("isolcpus=${isolcpus_argument}")
    );
    assert!(isolation.contains("nohz_full=${measurement_cpus}"));
    assert!(isolation.contains("rcu_nocbs=${measurement_cpus}"));
    assert!(isolation.contains("irqaffinity=${housekeeping_cpus}"));
    assert!(isolation.contains("CPUAffinity=0 5 6 7"));
    assert!(isolation.contains("hydracache-perf-idle-policy.service"));
    assert!(isolation.contains("systemctl enable \"$idle_policy_unit\""));
    assert!(isolation.contains("systemctl restart \"$idle_policy_unit\""));
    assert!(!isolation.contains("systemctl enable --now \"$idle_policy_unit\""));
    assert!(isolation.contains("measurement_max_idle_latency_us=1"));
    assert!(isolation.contains("housekeeping_max_idle_latency_us=1"));
    assert!(isolation.contains("for cpu in 0 1 2 3 4 5 6 7"));
    assert!(isolation.contains("latency > maximum_idle_latency_us"));
    assert!(isolation.contains("printf '1' >\"\\$state/disable\""));
    assert!(isolation.contains("Before `nosmt` takes effect"));
    assert!(isolation.contains("= \"${cpu},${sibling}\""));
    assert!(isolation.contains("= \"$cpu\""));
    assert!(isolation.contains("if test -d \"/sys/devices/system/cpu/cpu${sibling}\"; then"));
    assert!(isolation.contains("IFS=' ' read -r -a kernel_arguments </proc/cmdline"));
    assert!(isolation.contains("normalize_cpu_list"));
    assert!(isolation.contains("expected_housekeeping_cpus"));
    assert!(isolation.contains("docker_cpu_affinity"));
    assert!(isolation.contains("dormant_unmapped_nvme_irq"));
    assert!(isolation.contains("test -z \"$(cat \"$cpu_list_path\")\""));
    assert!(isolation.contains("test \"$interrupt_total\" = 0"));
    assert!(isolation.contains("dormant-unmapped-nvme="));
    assert!(isolation.contains("test -n \"$affinity\" || continue"));
    assert!(isolation.contains("IRQ affinity reaches measurement CPUs"));
    assert!(rootless_docker.contains("rootless Docker lifecycle must run as github-runner"));
    assert!(rootless_docker.contains("test ! -S /var/run/docker.sock"));
    assert!(rootless_docker.contains("grep --quiet rootless"));
    assert!(rootless_docker.contains("systemctl --user stop docker.service"));
    assert!(runbook.contains("systemctl start \"user@${runner_uid}.service\""));
    assert!(runbook.contains("rm --force /var/run/docker.sock"));

    assert!(runtime_irq_guard.contains("dormant_unmapped_nvme_irq"));
    assert!(runtime_irq_guard.contains("runtime IRQ guard failed phase=${phase}"));
    assert!(runtime_irq_guard.contains("per_cpu_counts=${counts}"));
    assert!(runtime_irq_guard.contains("measurement=1-4"));
    assert!(evidence_tmpfs.contains("/dev/shm/hydracache-reference-evidence-v1"));
    assert!(evidence_tmpfs.contains("findmnt --noheadings --output FSTYPE"));
    assert!(evidence_tmpfs.contains("ln --symbolic"));
    assert!(evidence_tmpfs.contains("source-commit"));
    assert!(evidence_tmpfs.contains("materialize_one"));
    assert!(prebuild.contains("exact_tmpfs_publication_contract"));
    assert!(prebuild.contains("REFERENCE_EVIDENCE_067_TMPFS"));
    assert!(prebuild.contains("REFERENCE_EVIDENCE_SOURCE_COMMIT"));
    assert!(prebuild.contains("/usr/bin/findmnt"));
    assert!(prebuild.contains("GITHUB_ACTIONS"));
    assert!(prebuild.contains("GITHUB_SHA"));
    assert!(prebuild.contains("Some(\"qualify\" | \"bootstrap\")"));
    assert!(prebuild.contains("filesystem_type == \"tmpfs\""));
    assert!(measurement
        .contains("reference measurement orchestration must remain on housekeeping CPUs 0,5-7"));
    assert!(measurement.contains("scripts/perf/reference-evidence-tmpfs.sh verify"));
    assert!(measurement.contains("reference-runtime-irq-guard.sh \"${mode}-pre\""));
    assert!(measurement.contains("reference-runtime-irq-guard.sh \"${mode}-post\""));
    assert!(measurement
        .contains("HYDRACACHE_MEASUREMENT_IO_POLICY=\"tmpfs-housekeeping-orchestration-v1\""));
    assert!(measurement.contains("taskset --cpu-list 1-4 \"${command_argv[@]}\""));
    assert!(measurement.contains("warm_file docs/plans/releases.toml"));
    assert!(measurement.contains("warm_file docs/testing/perf-profiles/reference-v1.toml"));
    assert!(!measurement.contains("docs/testing/perf-profiles/0.67"));
    assert!(measurement.contains("warm_file /etc/os-release"));
    assert!(measurement.contains("warm_file /var/lib/hydracache-perf/runner-provisioned.json"));
    for command in ["findmnt", "lsblk", "systemd-detect-virt"] {
        assert!(
            measurement.contains(&format!("warm_command {command}")),
            "measurement wrapper does not warm {command}"
        );
    }
    assert!(host_attestation
        .contains("let quiet = probe_output(\"systemd-detect-virt\", &[\"--quiet\"])?;"));
    assert!(!host_attestation.contains("Command::new(\"systemd-detect-virt\")"));
    assert!(host_attestation.contains("\"--raw\","));
    assert!(host_attestation.contains("parse_root_storage_identity(text)?"));
    let bootstrap_job = workflow
        .split_once("  release-0671-performance-bootstrap:\n")
        .unwrap()
        .1
        .split_once("\n  msrv:\n")
        .unwrap()
        .0;
    let bootstrap_tmpfs_prepare = bootstrap_job
        .find("      - name: Prepare tmpfs reference evidence\n")
        .unwrap();
    let bootstrap_receipt_import = bootstrap_job
        .find("      - name: Import offline runner provisioning proof\n")
        .unwrap();
    assert!(
        bootstrap_tmpfs_prepare < bootstrap_receipt_import,
        "bootstrap must prepare tmpfs before the receipt import materializes 0.67.1 evidence"
    );
    assert!(measurement.contains("docker pull --platform linux/amd64"));
    assert!(measurement.contains("--cpuset-cpus 0"));
    assert!(!measurement.contains("taskset --cpu-list 1-4 cargo"));
    assert_eq!(measurement.matches("smoke-v1").count(), 2);
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
        "scripts/perf/run-reference-measurement.sh",
        "scripts/perf/reference-runtime-irq-guard.sh",
        "scripts/perf/reference-evidence-tmpfs.sh",
        "/dev/shm/hydracache-reference-evidence-v1",
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
