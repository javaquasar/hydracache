use hydracache_long_run_supervisor_074::host_execution::{
    ClaimDisposition, HostExecutionClaim, HostExecutionError, ACTIVE_CAMPAIGN_NAME,
    HOST_EXECUTION_LOCK_NAME,
};
use std::fs;

#[test]
fn one_campaign_claim_survives_restart_and_blocks_every_other_campaign() {
    let temporary = tempfile::tempdir().unwrap();
    let first_id = "a".repeat(64);
    let second_id = "b".repeat(64);
    let first = HostExecutionClaim::acquire(temporary.path(), &first_id).unwrap();
    assert_eq!(first.disposition(), ClaimDisposition::Created);
    assert_eq!(first.campaign_id(), first_id);
    assert_eq!(
        first.campaign_root(),
        temporary.path().canonicalize().unwrap()
    );
    assert!(matches!(
        HostExecutionClaim::acquire(temporary.path(), &first_id),
        Err(HostExecutionError::Busy)
    ));
    drop(first);

    let recovered = HostExecutionClaim::acquire(temporary.path(), &first_id).unwrap();
    assert_eq!(recovered.disposition(), ClaimDisposition::Recovered);
    drop(recovered);
    assert!(matches!(
        HostExecutionClaim::acquire(temporary.path(), &second_id),
        Err(HostExecutionError::Conflict { campaign_id }) if campaign_id == first_id
    ));
    assert_eq!(
        fs::read_to_string(temporary.path().join(ACTIVE_CAMPAIGN_NAME)).unwrap(),
        format!("{first_id}\n")
    );
}

#[test]
fn unsafe_marker_lock_and_campaign_identity_fail_closed() {
    let temporary = tempfile::tempdir().unwrap();
    assert!(matches!(
        HostExecutionClaim::acquire(temporary.path(), "../escape"),
        Err(HostExecutionError::Path)
    ));
    fs::write(temporary.path().join(ACTIVE_CAMPAIGN_NAME), b"invalid\n").unwrap();
    assert!(matches!(
        HostExecutionClaim::acquire(temporary.path(), &"a".repeat(64)),
        Err(HostExecutionError::Path)
    ));

    let other = tempfile::tempdir().unwrap();
    fs::create_dir(other.path().join(HOST_EXECUTION_LOCK_NAME)).unwrap();
    assert!(matches!(
        HostExecutionClaim::acquire(other.path(), &"a".repeat(64)),
        Err(HostExecutionError::Path)
    ));
}

#[cfg(unix)]
#[test]
fn symlink_and_hardlink_markers_are_rejected() {
    use std::os::unix::fs::symlink;

    let hardlink_root = tempfile::tempdir().unwrap();
    let marker = hardlink_root.path().join(ACTIVE_CAMPAIGN_NAME);
    fs::write(&marker, format!("{}\n", "a".repeat(64))).unwrap();
    fs::hard_link(&marker, hardlink_root.path().join("alias")).unwrap();
    assert!(matches!(
        HostExecutionClaim::acquire(hardlink_root.path(), &"a".repeat(64)),
        Err(HostExecutionError::Path)
    ));

    let symlink_root = tempfile::tempdir().unwrap();
    let target = symlink_root.path().join("target");
    fs::write(&target, format!("{}\n", "a".repeat(64))).unwrap();
    symlink(&target, symlink_root.path().join(ACTIVE_CAMPAIGN_NAME)).unwrap();
    assert!(matches!(
        HostExecutionClaim::acquire(symlink_root.path(), &"a".repeat(64)),
        Err(HostExecutionError::Path)
    ));
}
