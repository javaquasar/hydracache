#![cfg(target_os = "linux")]

use hydracache_long_run_supervisor_074::manifest::{
    CampaignManifest, ExpectedOutputSchemaSha256s, InstalledBinary, OutputLimits,
    PhaseDurationsSeconds, RoleArgvTemplates,
};
use hydracache_long_run_supervisor_074::spawn::SpawnIntent;
use hydracache_long_run_supervisor_074::systemd_unit::{
    build_transient_unit_spec, expected_command_environment_sha256, inspect_unit,
    stop_unit_and_wait, verify_unit_identity, verify_unit_terminal, UnitError, UnitMismatch,
    UnitProperty, UnitSnapshot, MAXIMUM_ROLE_RUNTIME_SECONDS, UNIT_MEMORY_MAX_BYTES,
    UNIT_NOFILE_LIMIT, UNIT_TASKS_MAX,
};
use hydracache_long_run_supervisor_074::{ProcessIdentity, Role};
use std::path::Path;

fn process(pid: u32) -> ProcessIdentity {
    ProcessIdentity {
        boot_id: "boot-a".to_owned(),
        pid,
        start_ticks: u64::from(pid) * 100,
        process_group: 10,
        cgroup_path: "/system.slice/hydracache-performance-074-a.service".to_owned(),
        cgroup_inode: 50,
        unit_name: "hydracache-performance-074-a.service".to_owned(),
    }
}

fn snapshot() -> UnitSnapshot {
    UnitSnapshot {
        unit_name: "hydracache-performance-074-a.service".to_owned(),
        active_state: "active".to_owned(),
        sub_state: "running".to_owned(),
        main_pid: 100,
        control_group: "/system.slice/hydracache-performance-074-a.service".to_owned(),
        result: "success".to_owned(),
    }
}

fn manifest() -> CampaignManifest {
    let hash = |byte: char| byte.to_string().repeat(64);
    let git = |byte: char| byte.to_string().repeat(40);
    CampaignManifest {
        schema_version: 1,
        repository_id: 10,
        authorization_identity: "protected-performance-074".to_owned(),
        contract_sha256: hash('a'),
        tooling_sha: git('b'),
        i74_source_sha: git('c'),
        c74_source_sha: git('d'),
        i74_tree_sha: git('1'),
        c74_tree_sha: git('2'),
        i74_cargo_lock_sha256: hash('3'),
        c74_cargo_lock_sha256: hash('4'),
        i74_dirty: false,
        c74_dirty: false,
        scenario_sha256: hash('5'),
        workload_sha256: hash('6'),
        offered_load_sha256: hash('7'),
        estimator_sha256: hash('8'),
        thresholds_sha256: hash('9'),
        host_receipt_sha256: hash('a'),
        lease_id: "123e4567-e89b-42d3-a456-426614174000".to_owned(),
        machine_id: "machine-a".to_owned(),
        boot_id: "boot-a".to_owned(),
        mount_identity: hash('b'),
        isolated_cpuset: "1-2".to_owned(),
        housekeeping_cpuset: "0,3".to_owned(),
        seed: 740_074,
        checkpoint_cadence_seconds: 30,
        progress_warning_gap_seconds: 90,
        progress_rejection_gap_seconds: 180,
        diagnostic_grace_seconds: 30,
        product_lease_deadline_unix_seconds: 2_000_000_000,
        maximum_campaign_bytes: 21_474_836_480,
        maximum_campaign_files: 20_000,
        installed_binaries: vec![
            InstalledBinary {
                role: "i74".to_owned(),
                path: "/opt/hydracache-performance/0.74/i74/hydracache".to_owned(),
                sha256: hash('c'),
                size: 1,
                inode: 2,
                device: 3,
                uid: 1_001,
                gid: 1_001,
                mode: 0o555,
            },
            InstalledBinary {
                role: "c74".to_owned(),
                path: "/opt/hydracache-performance/0.74/c74/hydracache".to_owned(),
                sha256: hash('d'),
                size: 1,
                inode: 4,
                device: 3,
                uid: 1_001,
                gid: 1_001,
                mode: 0o555,
            },
        ],
        argv_templates: RoleArgvTemplates {
            i74: vec![
                "/opt/hydracache-performance/0.74/i74/hydracache".to_owned(),
                "--role".to_owned(),
                "i74".to_owned(),
            ],
            c74: vec![
                "/opt/hydracache-performance/0.74/c74/hydracache".to_owned(),
                "--role".to_owned(),
                "c74".to_owned(),
            ],
        },
        command_environment_sha256: hash('e'),
        role_order: vec!["i74".to_owned(), "c74".to_owned()],
        phase_durations_seconds: PhaseDurationsSeconds {
            warmup: 60,
            measured: 300,
            drain: 30,
            durable_companion: 30,
            post_work_idle: 60,
            reconciliation: 30,
        },
        output_limits: OutputLimits {
            stdout_bytes: 1_048_576,
            stderr_bytes: 1_048_576,
            diagnostic_bytes: 1_048_576,
            final_artifact_bytes: 1_073_741_824,
            files: 2_000,
        },
        expected_output_schema_sha256s: ExpectedOutputSchemaSha256s {
            checkpoint: hash('1'),
            measurement: hash('2'),
            reconciliation: hash('3'),
            raw_manifest: hash('4'),
            packet_manifest: hash('5'),
        },
        required_final_guards: vec!["semantic".to_owned()],
        secret_identifiers: vec![],
        release: "0.74".to_owned(),
        campaign_id: "a".repeat(64),
        nonce_sha256: hash('f'),
        dirty: false,
        controller_history: vec![],
        state: "PREPARED".to_owned(),
    }
}

fn spec() -> hydracache_long_run_supervisor_074::systemd_unit::TransientUnitSpec {
    let mut manifest = manifest();
    let campaign =
        Path::new("/var/lib/hydracache-performance/campaigns").join(&manifest.campaign_id);
    manifest.command_environment_sha256 =
        expected_command_environment_sha256(&manifest, &campaign).unwrap();
    let intent = SpawnIntent::new(
        manifest.campaign_id.clone(),
        "123e4567-e89b-42d3-a456-426614174000".to_owned(),
        "1".repeat(64),
        "2".repeat(64),
        manifest.nonce_sha256.clone(),
        Role::I74,
    )
    .unwrap();
    build_transient_unit_spec(&manifest, &campaign, &intent).unwrap()
}

fn property<'a>(
    spec: &'a hydracache_long_run_supervisor_074::systemd_unit::TransientUnitSpec,
    name: &str,
) -> &'a UnitProperty {
    &spec
        .properties
        .iter()
        .find(|(candidate, _)| *candidate == name)
        .unwrap()
        .1
}

#[test]
fn exact_active_service_snapshot_binds_both_processes() {
    let harness = process(100);
    let daemon = process(101);
    assert!(verify_unit_identity(&harness, &daemon, &snapshot()).is_ok());
}

#[test]
fn every_unit_drift_is_reported_together() {
    let harness = process(100);
    let daemon = process(101);
    let drifted = UnitSnapshot {
        unit_name: "outside.service".to_owned(),
        active_state: "deactivating".to_owned(),
        sub_state: "stop-sigterm".to_owned(),
        main_pid: 999,
        control_group: "/system.slice/replacement.service".to_owned(),
        result: "signal".to_owned(),
    };
    let UnitError::Mismatch(mismatches) =
        verify_unit_identity(&harness, &daemon, &drifted).unwrap_err()
    else {
        panic!("expected aggregate mismatch");
    };
    for mismatch in [
        UnitMismatch::UnitName,
        UnitMismatch::ActiveState,
        UnitMismatch::SubState,
        UnitMismatch::MainPid,
        UnitMismatch::ControlGroup,
        UnitMismatch::Result,
    ] {
        assert!(mismatches.contains(&mismatch), "missing {mismatch:?}");
    }
}

#[test]
fn retained_successful_exit_is_the_only_terminal_unit_shape() {
    let harness = process(100);
    let daemon = process(101);
    let terminal = UnitSnapshot {
        unit_name: harness.unit_name.clone(),
        active_state: "active".to_owned(),
        sub_state: "exited".to_owned(),
        main_pid: 0,
        control_group: harness.cgroup_path.clone(),
        result: "success".to_owned(),
    };
    assert!(verify_unit_terminal(&harness, &daemon, &terminal).is_ok());

    let UnitError::Mismatch(mismatches) =
        verify_unit_terminal(&harness, &daemon, &snapshot()).unwrap_err()
    else {
        panic!("expected running unit to fail terminal admission");
    };
    assert!(mismatches.contains(&UnitMismatch::SubState));
    assert!(mismatches.contains(&UnitMismatch::MainPid));
}

#[test]
fn dbus_lookup_rejects_arbitrary_unit_names_before_connecting() {
    assert!(matches!(
        inspect_unit("dbus.service"),
        Err(UnitError::UnitName)
    ));
    assert!(matches!(
        inspect_unit("../../hydracache-performance-074-a.service"),
        Err(UnitError::UnitName)
    ));
    assert!(matches!(
        stop_unit_and_wait("dbus.service", 30),
        Err(UnitError::UnitName)
    ));
    assert!(matches!(
        stop_unit_and_wait("hydracache-performance-074-a.service", 0),
        Err(UnitError::Policy)
    ));
}

#[test]
fn transient_policy_has_exact_argv_clean_environment_and_resource_bounds() {
    let spec = spec();
    let names = spec
        .properties
        .iter()
        .map(|(name, _)| *name)
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(names.len(), spec.properties.len());
    assert_eq!(
        property(&spec, "CPUAffinity"),
        &UnitProperty::Bytes(vec![15])
    );
    assert_eq!(
        property(&spec, "RuntimeMaxUSec"),
        &UnitProperty::Unsigned(540_000_000)
    );
    assert_eq!(
        property(&spec, "MemoryMax"),
        &UnitProperty::Unsigned(UNIT_MEMORY_MAX_BYTES)
    );
    assert_eq!(
        property(&spec, "LimitNOFILE"),
        &UnitProperty::Unsigned(UNIT_NOFILE_LIMIT)
    );
    assert_eq!(
        property(&spec, "TasksMax"),
        &UnitProperty::Unsigned(UNIT_TASKS_MAX)
    );
    assert_eq!(
        property(&spec, "RemainAfterExit"),
        &UnitProperty::Boolean(true)
    );
    assert_eq!(
        property(&spec, "IOAccounting"),
        &UnitProperty::Boolean(true)
    );
    assert_eq!(
        property(&spec, "ProtectHome"),
        &UnitProperty::Text("yes".to_owned())
    );
    assert_eq!(
        property(&spec, "StandardOutputFileToAppend"),
        &UnitProperty::Text(
            "/var/lib/hydracache-performance/campaigns/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/roles/i74/stdout.log"
                .to_owned()
        )
    );
    assert_eq!(
        property(&spec, "StandardErrorFileToAppend"),
        &UnitProperty::Text(
            "/var/lib/hydracache-performance/campaigns/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/roles/i74/stderr.log"
                .to_owned()
        )
    );
    assert!(!names.contains("StandardOutput"));
    assert!(!names.contains("StandardError"));
    let UnitProperty::Strings(environment) = property(&spec, "Environment") else {
        panic!("environment property must be a string array");
    };
    assert!(environment.iter().any(|item| item == "HYDRACACHE_ROLE=i74"));
    assert!(environment
        .iter()
        .any(|item| item == "HYDRACACHE_ISOLATED_CPUSET=1-2"));
    assert!(environment
        .iter()
        .any(|item| item == "HYDRACACHE_HOUSEKEEPING_CPUSET=0,3"));
    assert!(environment.iter().all(|item| {
        !item.starts_with("GITHUB_")
            && !item.starts_with("RUNNER_")
            && !item.starts_with("CI=")
            && !item.starts_with("SSH_AUTH_SOCK=")
    }));
    let UnitProperty::Commands(commands) = property(&spec, "ExecStart") else {
        panic!("ExecStart property must be an argv command array");
    };
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].path, commands[0].argv[0]);
    assert_eq!(commands[0].argv[1..], ["--role", "i74"]);
    assert!(!commands[0].ignore_failure);
}

#[test]
fn transient_policy_fails_closed_on_environment_cpuset_and_runtime_drift() {
    let campaign = Path::new("/var/lib/hydracache-performance/campaigns").join("a".repeat(64));
    let intent = SpawnIntent::new(
        "a".repeat(64),
        "123e4567-e89b-42d3-a456-426614174000".to_owned(),
        "1".repeat(64),
        "2".repeat(64),
        "f".repeat(64),
        Role::I74,
    )
    .unwrap();

    let mut environment_drift = manifest();
    assert!(matches!(
        build_transient_unit_spec(&environment_drift, &campaign, &intent),
        Err(UnitError::Policy)
    ));
    environment_drift.command_environment_sha256 =
        expected_command_environment_sha256(&environment_drift, &campaign).unwrap();

    let mut cpuset_drift = environment_drift.clone();
    cpuset_drift.isolated_cpuset = "1-2,2".to_owned();
    assert!(matches!(
        build_transient_unit_spec(&cpuset_drift, &campaign, &intent),
        Err(UnitError::Policy)
    ));

    let mut runtime_drift = environment_drift;
    runtime_drift.phase_durations_seconds.measured = MAXIMUM_ROLE_RUNTIME_SECONDS;
    assert!(matches!(
        build_transient_unit_spec(&runtime_drift, &campaign, &intent),
        Err(UnitError::Policy)
    ));
}
