#![cfg(target_os = "linux")]

use hydracache_long_run_supervisor_074::systemd_unit::{
    inspect_unit, verify_unit_identity, UnitError, UnitMismatch, UnitSnapshot,
};
use hydracache_long_run_supervisor_074::ProcessIdentity;

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
fn dbus_lookup_rejects_arbitrary_unit_names_before_connecting() {
    assert!(matches!(
        inspect_unit("dbus.service"),
        Err(UnitError::UnitName)
    ));
    assert!(matches!(
        inspect_unit("../../hydracache-performance-074-a.service"),
        Err(UnitError::UnitName)
    ));
}
