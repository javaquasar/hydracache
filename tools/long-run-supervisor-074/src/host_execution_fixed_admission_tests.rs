use super::*;
use std::os::unix::fs::{symlink, PermissionsExt};

fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
    assert_ne!(unsafe { libc::geteuid() }, 0, "models remain nonroot");
    let temp = tempfile::tempdir_in("/tmp").unwrap();
    let mut parent = temp.path().to_owned();
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).unwrap();
    for name in ["var", "lib", "hydracache-performance"] {
        parent.push(name);
        fs::create_dir(&parent).unwrap();
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let leaf = parent.join("diagnostic-intents-074");
    fs::create_dir(&leaf).unwrap();
    fs::set_permissions(&leaf, fs::Permissions::from_mode(0o700)).unwrap();
    let campaigns = parent.join("campaigns");
    fs::create_dir(&campaigns).unwrap();
    fs::set_permissions(&campaigns, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(campaigns.join(HOST_EXECUTION_LOCK_NAME), []).unwrap();
    fs::set_permissions(
        campaigns.join(HOST_EXECUTION_LOCK_NAME),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    (temp, campaigns, leaf)
}
fn model(
    origin: &Path,
    root: &Path,
    lock: &File,
) -> Result<DiagnosticAdmission, HostExecutionError> {
    DiagnosticAdmission::modeled(origin, root, lock)
}
fn acquire(origin: &Path, root: &Path) -> Result<HostExecutionClaim, HostExecutionError> {
    HostExecutionClaim::acquire_inner_with(root, &"a".repeat(64), true, |r, l| model(origin, r, l))
}

fn reserved_bytes() -> Vec<u8> {
    use crate::diagnostic_lease::{DiagnosticIdentity, DiagnosticStage, DiagnosticState};
    let state = DiagnosticState {
        identity: DiagnosticIdentity {
            lease_id: "a".repeat(64),
            boot_id: "11111111-2222-3333-4444-555555555555".into(),
            binary_sha256: "b".repeat(64),
            build_provenance_sha256: "c".repeat(64),
        },
        revision: 1,
        stage: DiagnosticStage::Reserved,
        completed_cells: 0,
        reserved_monotonic_ns: 1,
        last_observed_monotonic_ns: 2,
        controller_monotonic_ns: 2,
        cell_started_monotonic_ns: None,
        cgroup_inode: None,
        reason: None,
        cleanup_confirmed: true,
        promotable: false,
        admission_allowed: false,
    };
    let envelope = serde_json::json!({"schema_version":1, "state":state,
        "state_sha256":crate::sha256_hex(&crate::canonical_json(&state).unwrap())});
    let mut bytes = crate::canonical_json(&envelope).unwrap();
    bytes.push(b'\n');
    assert_eq!(
        crate::diagnostic_lease::decode_active(&bytes).unwrap(),
        state
    );
    bytes
}

#[test]
fn fixed_admission_context_after_fence_drop_blocks_acquire_and_both_recoveries() {
    let (temp, root, leaf) = fixture();
    let (_, fence) = lock_host_root(&root).unwrap();
    fs::write(leaf.join("active-diagnostic.json"), reserved_bytes()).unwrap();
    drop(fence);
    assert!(matches!(
        acquire(temp.path(), &root),
        Err(HostExecutionError::DiagnosticConflict)
    ));
    assert!(!root.join(ACTIVE_CAMPAIGN_NAME).exists());
    assert!(
        HostExecutionClaim::recover_active_with(&root, |r, l| model(temp.path(), r, l))
            .unwrap()
            .is_none()
    );
    fs::write(
        root.join(ACTIVE_CAMPAIGN_NAME),
        format!("{}\n", "a".repeat(64)),
    )
    .unwrap();
    assert!(matches!(
        HostExecutionClaim::acquire_inner_with(&root, &"a".repeat(64), false, |r, l| model(
            temp.path(),
            r,
            l
        )),
        Err(HostExecutionError::DiagnosticConflict)
    ));
    assert!(matches!(
        HostExecutionClaim::recover_active_with(&root, |r, l| model(temp.path(), r, l)),
        Err(HostExecutionError::Path)
    ));
    assert!(matches!(
        lock_host_root_with(&root, |r, l| model(temp.path(), r, l)),
        Err(HostExecutionError::DiagnosticConflict)
    ));
    assert!(root.join(ACTIVE_CAMPAIGN_NAME).exists());
}

#[test]
fn fixed_admission_empty_storage_preserves_legacy_create_and_recovery() {
    let (temp, root, leaf) = fixture();
    // Legacy ownership/modes are not silently changed or made root authority.
    fs::set_permissions(&root, fs::Permissions::from_mode(0o750)).unwrap();
    fs::set_permissions(
        root.join(HOST_EXECUTION_LOCK_NAME),
        fs::Permissions::from_mode(0o660),
    )
    .unwrap();
    let claim = acquire(temp.path(), &root).unwrap();
    assert_eq!(claim.disposition(), ClaimDisposition::Created);
    drop(claim);
    let recovered = HostExecutionClaim::recover_active_with(&root, |r, l| model(temp.path(), r, l))
        .unwrap()
        .unwrap();
    assert_eq!(recovered.campaign_id(), "a".repeat(64));
    drop(recovered);
    assert_eq!(
        acquire(temp.path(), &root).unwrap().disposition(),
        ClaimDisposition::Recovered
    );
    assert_eq!(fs::read_dir(&leaf).unwrap().count(), 0);
    assert_eq!(
        fs::metadata(&root).unwrap().permissions().mode() & 0o7777,
        0o750
    );
}

#[test]
fn fixed_admission_all_known_unknown_and_nonregular_contexts_block_without_decode() {
    for name in [
        "active-diagnostic.json",
        ".active-diagnostic.pending",
        ".diagnostic-requests.pending",
        "diagnostic-requests-v1.json",
        "terminal.json",
        "diagnostic-intent-cell-1.commit",
        "future-format",
    ] {
        for kind in 0..4 {
            let (temp, root, leaf) = fixture();
            let path = leaf.join(name);
            match kind {
                0 => fs::write(&path, []).unwrap(),
                1 => fs::create_dir(&path).unwrap(),
                2 => symlink("missing", &path).unwrap(),
                _ => {
                    let name = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
                    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
                }
            }
            assert!(matches!(
                acquire(temp.path(), &root),
                Err(HostExecutionError::DiagnosticConflict)
            ));
            assert!(!root.join(ACTIVE_CAMPAIGN_NAME).exists());
            assert!(fs::symlink_metadata(&path).is_ok());
        }
    }
}

#[test]
fn fixed_admission_missing_unsafe_or_replaced_storage_is_not_absence() {
    for kind in 0..3 {
        let (temp, root, leaf) = fixture();
        match kind {
            0 => fs::remove_dir(&leaf).unwrap(),
            1 => fs::set_permissions(&leaf, fs::Permissions::from_mode(0o750)).unwrap(),
            _ => {
                fs::rename(&leaf, leaf.with_extension("saved")).unwrap();
                symlink(leaf.with_extension("saved"), &leaf).unwrap();
            }
        }
        assert!(matches!(
            acquire(temp.path(), &root),
            Err(HostExecutionError::Path)
        ));
        assert!(!root.join(ACTIVE_CAMPAIGN_NAME).exists());
        if kind == 0 {
            assert!(!leaf.exists());
        }
    }
}

#[test]
fn fixed_admission_repeated_scans_and_original_refusal_never_refresh() {
    let (temp, root, leaf) = fixture();
    let (_, lock) = lock_host_root(&root).unwrap();
    let mut admission = model(temp.path(), &root, &lock.0).unwrap();
    assert!(!admission.present(&root, &lock.0).unwrap());
    assert!(!admission.present(&root, &lock.0).unwrap());
    fs::write(leaf.join("future"), []).unwrap();
    assert!(admission.present(&root, &lock.0).unwrap());
    fs::remove_file(leaf.join("future")).unwrap();
    fs::set_permissions(&leaf, fs::Permissions::from_mode(0o750)).unwrap();
    assert!(admission.present(&root, &lock.0).is_err());
    fs::set_permissions(&leaf, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(admission.present(&root, &lock.0).is_err());
}

#[test]
fn fixed_admission_seeded_late_context_and_object_failures_retain_created_marker() {
    let seed = 0x0747_u64;
    eprintln!("seed={seed:#x}");
    for kind in 0..4 {
        let (temp, root, leaf) = fixture();
        let changed_root = root.clone();
        let changed_leaf = leaf.clone();
        let result =
            HostExecutionClaim::acquire_inner_with(&root, &"a".repeat(64), true, |r, l| {
                let mut a = model(temp.path(), r, l)?;
                a.checkpoint = Some(Box::new(move |check| {
                    if check != 2 {
                        return;
                    }
                    match kind {
                        0 => fs::write(changed_leaf.join(format!("late-{seed}")), []).unwrap(),
                        1 => {
                            let name = changed_root.join(HOST_EXECUTION_LOCK_NAME);
                            fs::rename(&name, changed_root.join("displaced-lock")).unwrap();
                            fs::write(&name, []).unwrap();
                        }
                        2 => {
                            fs::rename(&changed_leaf, changed_leaf.with_extension("saved"))
                                .unwrap();
                            fs::create_dir(&changed_leaf).unwrap();
                            fs::set_permissions(&changed_leaf, fs::Permissions::from_mode(0o700))
                                .unwrap();
                        }
                        _ => fs::set_permissions(&changed_root, fs::Permissions::from_mode(0o755))
                            .unwrap(),
                    }
                }));
                Ok(a)
            });
        assert!(result.is_err());
        assert_eq!(
            fs::read(root.join(ACTIVE_CAMPAIGN_NAME)).unwrap(),
            format!("{}\n", "a".repeat(64)).as_bytes()
        );
        assert!(matches!(
            HostExecutionClaim::acquire_inner_with(&root, &"b".repeat(64), true, |r, l| model(
                temp.path(),
                r,
                l
            )),
            Err(HostExecutionError::DiagnosticConflict
                | HostExecutionError::Path
                | HostExecutionError::Conflict { .. })
        ));
    }
}

#[test]
fn fixed_admission_recovery_late_failure_never_removes_existing_marker() {
    let (temp, root, leaf) = fixture();
    fs::write(
        root.join(ACTIVE_CAMPAIGN_NAME),
        format!("{}\n", "a".repeat(64)),
    )
    .unwrap();
    let before = fs::read(root.join(ACTIVE_CAMPAIGN_NAME)).unwrap();
    let result = HostExecutionClaim::recover_active_with(&root, |r, l| {
        let mut a = model(temp.path(), r, l)?;
        a.checkpoint = Some(Box::new(move |check| {
            if check == 2 {
                fs::write(leaf.join("late"), []).unwrap();
            }
        }));
        Ok(a)
    });
    assert!(matches!(result, Err(HostExecutionError::Path)));
    assert_eq!(fs::read(root.join(ACTIVE_CAMPAIGN_NAME)).unwrap(), before);
}

#[test]
fn fixed_admission_busy_precedes_factory_and_aliases_use_canonical_dispatch() {
    let (temp, root, _) = fixture();
    let claim = acquire(temp.path(), &root).unwrap();
    let result = HostExecutionClaim::acquire_inner_with(&root, &"a".repeat(64), true, |_, _| {
        panic!("factory cannot run without original flock")
    });
    assert!(matches!(result, Err(HostExecutionError::Busy)));
    drop(claim);
    let alias = temp.path().join("alias");
    symlink(&root, &alias).unwrap();
    assert!(matches!(
        HostExecutionClaim::acquire_inner_with(&alias, &"a".repeat(64), true, |_, _| panic!(
            "symlink root must refuse before factory"
        )),
        Err(HostExecutionError::Path)
    ));
    let claim =
        HostExecutionClaim::acquire_inner_with(&root.join("."), &"a".repeat(64), true, |r, l| {
            assert_eq!(r, root.canonicalize().unwrap());
            model(temp.path(), r, l)
        })
        .unwrap();
    assert_eq!(claim.disposition(), ClaimDisposition::Recovered);
}

#[test]
fn fixed_admission_production_nonroot_refuses_and_fixture_route_stays_distinct() {
    let (temp, root, _) = fixture();
    let (_, lock) = lock_host_root(&root).unwrap();
    assert!(
        DiagnosticAdmission::open(Path::new(crate::config::PRODUCTION_CAMPAIGN_ROOT), &lock.0)
            .is_err()
    );
    assert!(!DiagnosticAdmission::open(&root, &lock.0)
        .unwrap()
        .present(&root, &lock.0)
        .unwrap());
    drop(lock);
    assert!(HostExecutionClaim::acquire(&root, &"a".repeat(64)).is_ok());
    assert!(temp.path().exists());
}
