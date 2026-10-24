use std::fs;
use std::path::PathBuf;

fn repository_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

#[test]
fn service_is_detached_confined_and_exposes_no_workflow_shell() {
    let service = fs::read_to_string(repository_path(
        "scripts/perf/long-run-supervisor-074/hydracache-performance-supervisor-074.service",
    ))
    .unwrap();
    for required in [
        "Type=notify",
        "User=root",
        "Restart=on-failure",
        "IOAccounting=yes",
        "NoNewPrivileges=yes",
        "ProtectSystem=strict",
        "ProtectHome=yes",
        "ProtectControlGroups=yes",
        "RestrictAddressFamilies=AF_UNIX",
        "KillMode=process",
        "RuntimeDirectoryMode=0711",
        "ReadWritePaths=/var/lib/hydracache-performance /run/hydracache-perf",
    ] {
        assert!(service.contains(required), "missing {required}");
    }
    assert!(!service.contains("StateDirectory="));
    assert!(!service.contains("StateDirectoryMode="));
    for forbidden in ["/bin/sh", "sudo", "systemd-run", "EnvironmentFile="] {
        assert!(!service.contains(forbidden), "forbidden {forbidden}");
    }
}

#[test]
fn accounts_directories_and_unresolved_key_fail_closed() {
    let sysusers = fs::read_to_string(repository_path(
        "scripts/perf/long-run-supervisor-074/hydracache-performance-074.sysusers.conf",
    ))
    .unwrap();
    assert!(sysusers.contains("u hydracache-perf"));
    assert!(sysusers.contains("g hydracache-perf-client"));
    assert!(sysusers.contains("/usr/sbin/nologin"));

    let tmpfiles = fs::read_to_string(repository_path(
        "scripts/perf/long-run-supervisor-074/hydracache-performance-074.tmpfiles.conf",
    ))
    .unwrap();
    assert!(tmpfiles
        .contains("/var/lib/hydracache-performance           0750 root           hydracache-perf"));
    assert!(tmpfiles.contains("/var/lib/hydracache-performance/campaigns 0750"));
    assert!(tmpfiles.contains("/var/lib/hydracache-performance/staging   0750"));
    assert!(
        tmpfiles.contains("/var/lib/hydracache-performance/worker-authority-074 0700 root root -")
    );
    assert!(tmpfiles
        .contains("/var/lib/hydracache-performance/diagnostic-intents-074 0700 root root -"));
    assert!(tmpfiles.contains("/run/hydracache-perf                       0711"));

    let template = fs::read_to_string(repository_path(
        "scripts/perf/long-run-supervisor-074/supervisor-074.toml.example",
    ))
    .unwrap();
    assert!(template.contains("UNRESOLVED_PRODUCTION_ED25519_PUBLIC_KEY"));
    assert!(template.contains("expected_repository_id = 0"));
}

#[test]
fn sudo_boundary_exposes_only_the_fixed_signed_bundle_installer() {
    let sudoers = fs::read_to_string(repository_path(
        "scripts/perf/long-run-supervisor-074/hydracache-performance-074.sudoers",
    ))
    .unwrap();
    assert!(sudoers.contains(
        "github-runner ALL=(root) NOPASSWD: /usr/local/sbin/hydracache-provision-host-074"
    ));
    for forbidden in [
        "/bin/sh",
        "/bin/bash",
        "systemctl",
        "ALL=(ALL) NOPASSWD: ALL",
    ] {
        assert!(!sudoers.contains(forbidden), "forbidden {forbidden}");
    }
}

#[test]
fn role_overhead_does_not_expand_the_fixed_root_entrypoint() {
    let entrypoint = fs::read_to_string(repository_path(
        "scripts/perf/long-run-supervisor-074/provision-host-074.sh",
    ))
    .unwrap();
    assert!(!entrypoint.contains("--role-overhead-smoke"));
    assert!(!entrypoint.contains("role-overhead-fixture"));
}

#[test]
fn private_storage_is_checked_before_installation_without_metadata_repair() {
    let source = fs::read_to_string(repository_path(
        "scripts/perf/long-run-supervisor-074/provision-host-074.sh",
    ))
    .unwrap();
    let guard = source
        .find("check_diagnostic_storage\nalready_active=false")
        .unwrap();
    assert!(guard < source.find("systemd-sysusers \"$bundle_dir/").unwrap());
    assert!(guard < source.find("atomic_install \"$bundle_dir/").unwrap());
    assert!(guard < source.find("systemd-tmpfiles --create").unwrap());
    for required in [
        "[[ -d \"$path\" && ! -L \"$path\" ]]",
        "700:0:0",
        "worker-authority-074 diagnostic-intents-074",
        "unsafe existing diagnostic storage",
        "8#$mode & 07022",
    ] {
        assert!(source.contains(required), "missing {required}");
    }
    let function = source
        .split("check_diagnostic_storage() {")
        .nth(1)
        .unwrap()
        .split("\n}\n")
        .next()
        .unwrap();
    for forbidden in ["chmod", "chown", "mkdir", "rm ", "mv ", "systemctl"] {
        assert!(
            !function.contains(forbidden),
            "guard mutates via {forbidden}"
        );
    }
}
