#![cfg(target_os = "linux")]

use hydracache_long_run_supervisor_074::process_identity::{
    inspect_process, verify_process_cpuset, verify_process_identity, IdentityMismatch,
    ProcessIdentityError,
};
use hydracache_long_run_supervisor_074::ProcessIdentity;

#[test]
fn current_process_snapshot_has_stable_kernel_identity_fields() {
    let snapshot = inspect_process(std::process::id()).unwrap();
    assert_eq!(snapshot.pid, std::process::id());
    assert!(!snapshot.boot_id.is_empty());
    assert!(snapshot.start_ticks > 0);
    assert!(snapshot.process_group > 0);
    assert!(snapshot.cgroup_path.starts_with('/'));
    assert!(snapshot.cgroup_inode > 0);
    assert!(!snapshot.cpus_allowed_list.is_empty());
}

#[test]
fn cpuset_is_bound_to_the_live_process_status() {
    let snapshot = inspect_process(std::process::id()).unwrap();
    let expected = ProcessIdentity {
        boot_id: snapshot.boot_id.clone(),
        pid: snapshot.pid,
        start_ticks: snapshot.start_ticks,
        process_group: snapshot.process_group,
        cgroup_path: snapshot.cgroup_path.clone(),
        cgroup_inode: snapshot.cgroup_inode,
        unit_name: snapshot
            .cgroup_path
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .to_owned(),
    };
    assert!(verify_process_cpuset(&expected, &snapshot.cpus_allowed_list).is_ok());
    assert!(matches!(
        verify_process_cpuset(&expected, "999999"),
        Err(ProcessIdentityError::Mismatch(mismatches))
            if mismatches == vec![IdentityMismatch::CpuSet]
    ));
}

#[test]
fn verifier_reports_every_changed_kernel_identity_field() {
    let snapshot = inspect_process(std::process::id()).unwrap();
    let expected = ProcessIdentity {
        boot_id: "wrong-boot".to_owned(),
        pid: snapshot.pid,
        start_ticks: snapshot.start_ticks + 1,
        process_group: snapshot.process_group + 1,
        cgroup_path: "/wrong.service".to_owned(),
        cgroup_inode: snapshot.cgroup_inode + 1,
        unit_name: "wrong.service".to_owned(),
    };
    let ProcessIdentityError::Mismatch(mismatches) =
        verify_process_identity(&expected).unwrap_err()
    else {
        panic!("expected an identity mismatch");
    };
    for mismatch in [
        IdentityMismatch::BootId,
        IdentityMismatch::StartTicks,
        IdentityMismatch::ProcessGroup,
        IdentityMismatch::CgroupPath,
        IdentityMismatch::CgroupInode,
        IdentityMismatch::UnitName,
    ] {
        assert!(mismatches.contains(&mismatch), "missing {mismatch:?}");
    }
}
