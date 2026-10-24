//! Construction-only controls; reuse the exact owned pre-exec child fixture.
use super::*;
use std::os::unix::fs::PermissionsExt;

fn fixed() -> (tempfile::TempDir, PathBuf, PathBuf) {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    let temp = tempfile::tempdir_in("/tmp").unwrap();
    fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let mut parent = temp.path().to_owned();
    for name in ["var", "lib", "hydracache-performance"] {
        parent.push(name);
        fs::create_dir(&parent).unwrap();
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let leaf = parent.join("diagnostic-intents-074");
    let root = parent.join("campaigns");
    for path in [&leaf, &root] {
        fs::create_dir(path).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
    fs::write(root.join(HOST_EXECUTION_LOCK_NAME), []).unwrap();
    fs::set_permissions(
        root.join(HOST_EXECUTION_LOCK_NAME),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    (temp, root, leaf)
}
fn marker(root: &Path, id: &str) {
    fs::write(root.join(ACTIVE_CAMPAIGN_NAME), format!("{id}\n")).unwrap();
}
fn released(root: &Path, child: Inherited) {
    // Bypass admission only for the independent kernel-lock probe.
    let next = open_lock(&root.join(HOST_EXECUTION_LOCK_NAME)).unwrap();
    FileExt::try_lock_exclusive(&next)
        .expect("constructor returned but inherited alias retained flock");
    let next = ShortHostFence(next);
    busy(root);
    child.unlock_old();
    busy(root);
    child.finish();
    busy(root);
    drop(next);
    drop(lock_host_root(root).unwrap());
}
fn named_errors(create: bool) {
    for kind in 0..if create { 5 } else { 6 } {
        let (temp, root, leaf) = fixed();
        match kind {
            1 => fs::write(root.join(ACTIVE_DIAGNOSTIC_NAME), b"{}").unwrap(),
            2 => fs::write(root.join(ACTIVE_CAMPAIGN_NAME), b"bad").unwrap(),
            3 => marker(&root, &"b".repeat(64)),
            4 => fs::write(leaf.join("unknown-context"), []).unwrap(),
            _ => {}
        }
        let before = snapshot(&root);
        let (result, child) = with_inherited(|| {
            HostExecutionClaim::acquire_inner_with(&root, &"a".repeat(64), create, |r, l| {
                if kind == 0 {
                    return Err(HostExecutionError::Path);
                }
                DiagnosticAdmission::modeled(temp.path(), r, l)
            })
        });
        match kind {
            3 => assert!(
                matches!(result, Err(HostExecutionError::Conflict { campaign_id }) if campaign_id == "b".repeat(64))
            ),
            4 => assert!(matches!(
                result,
                Err(HostExecutionError::DiagnosticConflict)
            )),
            _ => assert!(matches!(result, Err(HostExecutionError::Path))),
        }
        assert_eq!(snapshot(&root), before);
        released(&root, child);
        assert_eq!(snapshot(&root), before);
        println!(
            "claim_construction named_create={create} kind={kind} release=true preserved=true"
        );
    }
}
#[test]
fn claim_constructor_acquire_errors_release_original() {
    named_errors(true);
}
#[test]
fn claim_constructor_recover_errors_release_original() {
    named_errors(false);
}
#[test]
fn claim_constructor_active_errors_release_original() {
    for kind in 0..4 {
        let (temp, root, leaf) = fixed();
        match kind {
            1 => fs::write(root.join(ACTIVE_DIAGNOSTIC_NAME), b"{}").unwrap(),
            2 => fs::write(root.join(ACTIVE_CAMPAIGN_NAME), b"bad").unwrap(),
            3 => {
                fs::write(leaf.join("unknown-context"), []).unwrap();
                marker(&root, &"a".repeat(64));
            }
            _ => {}
        }
        let before = snapshot(&root);
        let (result, child) = with_inherited(|| {
            HostExecutionClaim::recover_active_with(&root, |r, l| {
                if kind == 0 {
                    return Err(HostExecutionError::Path);
                }
                DiagnosticAdmission::modeled(temp.path(), r, l)
            })
        });
        assert!(matches!(result, Err(HostExecutionError::Path)));
        assert_eq!(snapshot(&root), before);
        released(&root, child);
        assert_eq!(snapshot(&root), before);
        println!("claim_construction active_error={kind} release=true preserved=true");
    }
}
#[test]
fn claim_constructor_active_none_releases_original() {
    for context in [false, true] {
        let (temp, root, leaf) = fixed();
        if context {
            fs::write(leaf.join("unknown-context"), []).unwrap();
        }
        let before = snapshot(&root);
        let (result, child) = with_inherited(|| {
            HostExecutionClaim::recover_active_with(&root, |r, l| {
                DiagnosticAdmission::modeled(temp.path(), r, l)
            })
        });
        assert!(result.unwrap().is_none());
        assert_eq!(snapshot(&root), before);
        released(&root, child);
        assert_eq!(snapshot(&root), before);
        println!("claim_construction active_none_context={context} release=true preserved=true");
    }
}
#[test]
fn claim_constructor_late_refusal_preserves_created_or_recovered_marker() {
    for route in 0..3 {
        let (temp, root, leaf) = fixed();
        let id = "a".repeat(64);
        if route != 0 {
            marker(&root, &id);
        }
        let prior = (route != 0).then(|| snapshot(&root));
        let factory = |r: &Path, l: &File| {
            let mut admission = DiagnosticAdmission::modeled(temp.path(), r, l)?;
            let leaf = leaf.clone();
            admission.checkpoint = Some(Box::new(move |n| {
                if n == 2 {
                    fs::write(leaf.join("late-context"), []).unwrap();
                }
            }));
            Ok(admission)
        };
        let (result, child) = with_inherited(|| {
            if route == 2 {
                HostExecutionClaim::recover_active_with(&root, factory).map(|claim| claim.unwrap())
            } else {
                HostExecutionClaim::acquire_inner_with(&root, &id, route == 0, factory)
            }
        });
        if route == 2 {
            assert!(matches!(result, Err(HostExecutionError::Path)));
        } else {
            assert!(matches!(
                result,
                Err(HostExecutionError::DiagnosticConflict)
            ));
        }
        assert_eq!(
            fs::read(root.join(ACTIVE_CAMPAIGN_NAME)).unwrap(),
            format!("{id}\n").as_bytes()
        );
        let after = snapshot(&root);
        if let Some(prior) = prior {
            assert_eq!(after, prior);
        }
        released(&root, child);
        assert_eq!(snapshot(&root), after);
        assert_eq!(fs::read_dir(&leaf).unwrap().count(), 1);
        println!("claim_construction late_route={route} release=true marker_retained=true");
    }
}
#[test]
fn claim_constructor_success_transfers_same_owner_without_unlock() {
    for route in 0..3 {
        let (temp, root, _) = fixed();
        let id = "a".repeat(64);
        if route != 0 {
            marker(&root, &id);
        }
        let (claim, child) = with_inherited(|| {
            if route == 2 {
                HostExecutionClaim::recover_active_with(&root, |r, l| {
                    DiagnosticAdmission::modeled(temp.path(), r, l)
                })
                .unwrap()
                .unwrap()
            } else {
                HostExecutionClaim::acquire_inner_with(&root, &id, route == 0, |r, l| {
                    DiagnosticAdmission::modeled(temp.path(), r, l)
                })
                .unwrap()
            }
        });
        assert_eq!(claim.campaign_id(), id);
        assert_eq!(
            claim.disposition(),
            if route == 0 {
                ClaimDisposition::Created
            } else {
                ClaimDisposition::Recovered
            }
        );
        busy(&root);
        let contender = HostExecutionClaim::acquire_inner_with(&root, &id, true, |_, _| {
            panic!("busy before factory")
        });
        assert!(matches!(contender, Err(HostExecutionError::Busy)));
        busy(&root);
        let retained = snapshot(&root);
        drop(claim);
        released(&root, child);
        assert_eq!(snapshot(&root), retained);
        println!("claim_construction success_route={route} held_until_claim_drop=true marker_retained=true");
    }
}
