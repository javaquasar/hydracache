use std::collections::BTreeSet;
use std::fs;

use serde_json::Value;

fn read(path: &str) -> String {
    let root = xtask::doc_check::find_repo_root().unwrap();
    fs::read_to_string(root.join(path)).unwrap()
}

fn strings<'a>(value: &'a Value, pointer: &str) -> BTreeSet<&'a str> {
    value
        .pointer(pointer)
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item.as_str().unwrap())
        .collect()
}

#[test]
fn ubuntu_reference_profile_is_explicit_versioned_and_safe() {
    let profile: Value = serde_json::from_str(&read(
        "docs/testing/perf-host-profiles/ubuntu-24.04-reference-v1.json",
    ))
    .unwrap();

    assert_eq!(profile["schema_version"], 1);
    assert_eq!(profile["profile_id"], "ubuntu-24.04-reference-v1");
    assert_eq!(profile["operating_system"]["id"], "ubuntu");
    assert_eq!(profile["operating_system"]["version_id"], "24.04");
    assert_eq!(profile["operating_system"]["architecture"], "x86_64");
    assert_eq!(
        profile["operating_system"]["kernel_release_regex"],
        r"^6\.8\.0-[0-9]+-generic$"
    );
    assert_eq!(profile["hardware"]["require_bare_metal"], true);
    assert_eq!(profile["hardware"]["require_cgroup_v2"], true);
    assert_eq!(profile["hardware"]["require_unlimited_cpu_quota"], true);
    assert_eq!(profile["cpu_contract"]["measurement_cpus"], "1-4");
    assert_eq!(profile["cpu_contract"]["housekeeping_cpus"], "0,5-7");
    assert_eq!(profile["cpu_contract"]["smt"], "off");
    assert_eq!(profile["cpu_contract"]["governor"], "performance");

    let protected = strings(&profile, "/service_policy/protected_units");
    let disabled = strings(&profile, "/service_policy/disable_if_present");
    let masked = strings(&profile, "/service_policy/mask_if_present");
    let inactive = strings(&profile, "/service_policy/require_inactive_if_present");
    assert!(protected.contains("ssh.service"));
    assert!(protected.contains("systemd-timesyncd.service"));
    assert!(protected.contains("systemd-journald.service"));
    assert!(masked.contains("irqbalance.service"));
    assert!(disabled.contains("apt-daily.timer"));
    assert!(inactive.contains("docker.service"));
    assert!(inactive.contains("containerd.service"));
    assert!(protected.is_disjoint(&disabled));
    assert!(protected.is_disjoint(&masked));
    assert!(protected.is_disjoint(&inactive));
    assert!(disabled.is_disjoint(&masked));
    assert!(disabled.is_disjoint(&inactive));
    assert!(masked.is_disjoint(&inactive));

    let active_groups = profile["service_policy"]["required_active_groups"]
        .as_array()
        .unwrap();
    assert!(active_groups
        .iter()
        .any(|group| group["id"] == "remote-access"));
    assert!(active_groups
        .iter()
        .any(|group| group["id"] == "time-synchronization"));
    let invalidation = strings(
        &profile,
        "/freeze_contract/invalidate_sample_family_on_change",
    );
    for required in [
        "source_commit",
        "profile_sha256",
        "kernel_release",
        "package_manifest_sha256",
        "cpu_isolation",
        "container_image_digests",
    ] {
        assert!(invalidation.contains(required));
    }
}

#[test]
fn host_tuning_is_allowlisted_reversible_and_fail_closed() {
    let tuning = read("scripts/perf/reference-host-tuning.sh");
    let checker = read("scripts/perf/check-reference-host-freeze.sh");
    let wrapper = read("scripts/perf/prepare-reference-host.sh");
    let playbook = read("docs/testing/PERF_RUNNER_NEXT_RENTAL_PLAYBOOK.md");
    let early_irq = read("scripts/perf/reference-host-irq-layout-preflight.sh");
    let systemd_smoke = read("scripts/perf/local-orchestration/systemd-smoke.sh");
    for script in [&tuning, &checker, &wrapper] {
        assert!(script.starts_with("#!/usr/bin/env bash\nset -euo pipefail\n"));
        assert!(!script.contains('\r'));
        assert!(!script.contains("PRIVATE KEY"));
        assert!(!script.contains("RUNNER_TOKEN="));
    }

    for mode in ["plan", "apply", "verify", "freeze", "restore"] {
        assert!(tuning.contains(mode), "host tuning lacks {mode}");
    }
    assert!(tuning.contains("exact_allowlist_only: true"));
    assert!(tuning.contains("service policy attempts to mutate protected unit"));
    assert!(tuning.contains("sort | uniq -d | wc --lines"));
    assert!(!tuning.contains("uniq --duplicates"));
    assert!(tuning.contains("systemctl disable --now \"$unit\""));
    assert!(tuning.contains("systemctl mask --now \"$unit\""));
    assert!(tuning.contains("restore requires plan.json and applied.json"));
    assert!(tuning.contains("rootless Docker socket must be absent"));
    assert!(tuning.contains("runner service must be offline"));
    assert!(tuning.contains("provision-reference-isolation.sh\" verify"));
    assert!(tuning.contains("audit-reference-host.sh\" --mode provisioned"));
    assert!(tuning.contains("dpkg-query --show"));
    assert!(tuning.contains("systemd-unit-files.tsv"));
    assert!(tuning.contains("systemd-active-state.tsv"));
    assert!(tuning.contains("sysctls.tsv"));
    assert!(tuning.contains("read_frozen_kernel_tunable"));
    assert!(tuning.contains("/sys/kernel/debug/sched/migration_cost_ns"));
    assert!(tuning.contains("CONFIG_SCHED_DEBUG=y"));
    assert!(tuning.contains("sample_family_frozen: true"));
    assert!(tuning.contains(r#"printf "%.0f\n", $2 * 1024"#));
    assert!(!tuning.contains(r#"printf "%.0f\\n", $2 * 1024"#));
    assert!(!tuning.contains("systemctl disable --now --all"));
    assert!(!tuning.contains("systemctl mask --now '*'"));

    assert!(checker.contains("frozen host drift detected"));
    assert!(checker.contains("reference-host-tuning.sh\" verify"));
    assert!(checker.contains("package_manifest_sha256"));
    assert!(checker.contains("systemd_unit_files_sha256"));
    assert!(checker.contains("systemd_active_state_sha256"));
    assert!(checker.contains("sysctl_manifest_sha256"));
    assert!(checker.contains("read_frozen_kernel_tunable"));
    assert!(checker.contains("/sys/kernel/debug/sched/migration_cost_ns"));
    assert!(checker.contains("CONFIG_SCHED_DEBUG=y"));
    assert!(wrapper.contains("REBOOT_REQUIRED=true"));
    assert!(wrapper.contains("SAMPLE_FAMILY_FROZEN=true"));
    assert!(wrapper.contains("check-reference-host-freeze.sh"));
    assert!(wrapper.contains("deliberately does not reboot"));
    assert!(wrapper.contains("irq-layout-preflight"));
    assert!(wrapper.contains("EARLY_IRQ_LAYOUT_ELIGIBLE=true"));
    assert!(early_irq.starts_with("#!/usr/bin/env bash\nset -euo pipefail\n"));
    assert!(early_irq.contains("reference-runtime-irq-guard.sh\" early-layout"));
    assert!(early_irq.contains("mutates_irq_affinity: false"));
    assert!(early_irq.contains("qualification_evidence: false"));
    assert!(early_irq.contains("ship_evidence_eligible: false"));
    assert!(!early_irq.contains("smp_affinity_list"));
    assert!(!early_irq.contains("irqbalance --banirq"));
    assert!(systemd_smoke.contains("kernel.sched_migration_cost_ns"));
    assert!(systemd_smoke.contains("/sys/kernel/debug/sched/migration_cost_ns"));
    assert!(systemd_smoke.contains("CONFIG_SCHED_DEBUG=y"));
    for required in [
        "Ubuntu Server 24.04 LTS",
        "check-frozen",
        "delete/release",
        "five **serialized successful** bootstrap samples",
        "Failed, cancelled, unstable",
        "Do not run `restore` between qualification and bootstrap samples",
        "reference-rental-readiness.py",
        "irq-layout-preflight",
    ] {
        assert!(
            playbook.contains(required),
            "next-rental playbook is missing {required}"
        );
    }
}

#[test]
fn rental_procurement_gate_names_owners_and_remains_non_evidence() {
    let policy: Value =
        serde_json::from_str(&read("docs/testing/perf-procurement/0.67.1.json")).unwrap();
    let admission = read("scripts/perf/reference-rental-readiness.py");

    assert_eq!(policy["release"], "0.67.1");
    assert_eq!(policy["required_resource_kind"], "hourly-bare-metal");
    assert_eq!(
        policy["provider_resource_must_be_deleted_to_stop_billing"],
        true
    );
    assert_eq!(policy["qualification_evidence"], false);
    assert_eq!(policy["bootstrap_evidence"], false);
    assert_eq!(policy["ship_evidence_eligible"], false);
    for owner in ["rental_operator", "billing_owner", "deletion_owner"] {
        assert!(policy["required_owner_fields"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == owner));
    }
    for required in [
        "--approve",
        "main SHA differs from the locally fetched origin/main",
        "rental admission must be created from exact origin/main",
        "rental admission requires a clean exact-main checkout",
        "main CI run is not a successful exact-main run",
        "\"gh\"",
        "\"run\"",
        "\"view\"",
        "refusing to overwrite rental admission",
        "contains_provider_credentials",
        "ship_evidence_eligible",
    ] {
        assert!(
            admission.contains(required),
            "rental admission lacks {required}"
        );
    }
    for forbidden in ["api_key", "project_id", "public_ip", "runner_token"] {
        assert!(!admission.contains(&format!("\"{forbidden}\":")));
    }
}

#[test]
fn reference_campaign_controller_is_serial_resumable_and_provider_safe() {
    let controller = read("scripts/perf/reference_campaign.py");
    let wrapper = read("scripts/perf/reference-campaign.sh");
    let burn_in = read("scripts/perf/reference-host-irq-burn-in.sh");
    let importer = read("scripts/perf/import-reference-campaign-admission.sh");
    let workflow = read(".github/workflows/ci.yml");
    let guide = read("docs/testing/PERF_RUNNER_0_67_1_CAMPAIGN_AUTOMATION.md");

    assert!(wrapper.starts_with("#!/usr/bin/env bash\nset -euo pipefail\n"));
    assert!(burn_in.starts_with("#!/usr/bin/env bash\nset -euo pipefail\n"));
    assert_eq!(
        controller.lines().next(),
        Some("#!/usr/bin/env python3"),
        "campaign controller must be a Python 3 script"
    );
    assert!(importer.starts_with("#!/usr/bin/env bash\nset -euo pipefail\n"));
    for required in [
        "reboot-required",
        "host-admission-failed",
        "assert_no_foreign_reference_runs",
        "arm_runner_watchdog",
        "check-frozen",
        "reference-runtime-irq-guard.sh",
        "original-artifacts",
        "archive_sha256",
        "bootstrap-samples/sample-",
        "perf-bootstrap",
        "sample-set",
        "SAFE_TO_DELETE_SERVER=true",
        "publish_host_admission",
        "validate_host_admission_artifact",
    ] {
        assert!(
            controller.contains(required),
            "campaign controller lacks {required}"
        );
    }
    for required in [
        "duration_seconds=900",
        "test \"$duration_seconds\" -ge 600",
        "dd if=\"$device\" of=/dev/null",
        "taskset --cpu-list \"$cpu\"",
        "post-stimulus",
        "post-idle",
        "qualification_evidence: false",
        "ship_evidence_eligible: false",
    ] {
        assert!(burn_in.contains(required), "IRQ burn-in lacks {required}");
    }
    assert!(!controller.contains("provider delete"));
    assert!(!controller.contains("server delete"));
    for required in [
        "reference-campaign-host-admission.tar.gz",
        "reference-campaign-admission.json",
        "source_commit == $source_commit",
        "host_admission_bundle_sha256 == $bundle_sha256",
        "ship_evidence_eligible == false",
        "cp --update=none --preserve=mode,timestamps",
    ] {
        assert!(
            importer.contains(required),
            "campaign admission importer lacks {required}"
        );
    }
    assert!(workflow.contains("performance_0671_campaign:"));
    assert!(workflow.contains("0.67.1 Frozen Candidate Reference Evidence"));
    assert!(workflow.contains("CI dispatch {0}"));
    assert!(workflow.contains(":qualification$"));
    assert!(workflow.contains(":full-dress-[12]$"));
    assert!(workflow.contains(":bootstrap-[1-5]$"));
    assert_eq!(
        workflow
            .matches("run: scripts/perf/import-reference-campaign-admission.sh")
            .count(),
        4
    );
    for required in [
        "не создаёт и не удаляет сервер",
        "qualification → full-dress-1 → full-dress-2",
        "исходных ZIP",
        "после первого отказа",
        "SAFE_TO_DELETE_SERVER=true",
    ] {
        assert!(
            guide.contains(required),
            "campaign automation guide lacks {required}"
        );
    }
}

#[test]
fn reference_irq_burn_in_uses_ubuntu_iputils_supported_ping_options() {
    let burn_in = read("scripts/perf/reference-host-irq-burn-in.sh");

    assert!(
        burn_in.contains("ping -4 -n -q -c 32 -i 0.02 -w 10 \"$network_target\""),
        "IRQ burn-in must use the short options supported by Ubuntu 24.04 iputils ping"
    );
    for unsupported in ["--numeric", "--quiet", "--count", "--interval", "--wait"] {
        assert!(
            !burn_in.contains(unsupported),
            "IRQ burn-in must not use unsupported ping option {unsupported}"
        );
    }
}
