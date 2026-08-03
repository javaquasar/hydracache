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
    assert!(tuning.contains("sample_family_frozen: true"));
    assert!(!tuning.contains("systemctl disable --now --all"));
    assert!(!tuning.contains("systemctl mask --now '*'"));

    assert!(checker.contains("frozen host drift detected"));
    assert!(checker.contains("reference-host-tuning.sh\" verify"));
    assert!(checker.contains("package_manifest_sha256"));
    assert!(checker.contains("systemd_unit_files_sha256"));
    assert!(checker.contains("systemd_active_state_sha256"));
    assert!(checker.contains("sysctl_manifest_sha256"));
    assert!(wrapper.contains("REBOOT_REQUIRED=true"));
    assert!(wrapper.contains("SAMPLE_FAMILY_FROZEN=true"));
    assert!(wrapper.contains("check-reference-host-freeze.sh"));
    assert!(wrapper.contains("deliberately does not reboot"));
    for required in [
        "Ubuntu Server 24.04 LTS",
        "check-frozen",
        "delete/release",
        "five **serialized successful** bootstrap samples",
        "Failed, cancelled, unstable",
        "Do not run `restore` between qualification and bootstrap samples",
    ] {
        assert!(
            playbook.contains(required),
            "next-rental playbook is missing {required}"
        );
    }
}
