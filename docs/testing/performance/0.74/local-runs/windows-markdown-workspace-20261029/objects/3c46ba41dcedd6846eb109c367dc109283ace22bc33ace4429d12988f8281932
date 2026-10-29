//! Read-only server storage ancestry. No journal validation, enrollment or start.
use super::{open_fixed_authority_storage, open_fixed_intent_storage, FixedIntentStorageError};
use serde::Serialize;

pub const STORAGE_PREFLIGHT_SCHEMA: &str = "diagnostic-storage-preflight-074-v1";

#[derive(Debug, Serialize)]
pub struct StoragePreflightReport {
    schema_version: &'static str,
    scope: &'static str,
    storage_ancestry_verified: bool,
    journal_records_verified: bool,
    authority_bound: bool,
    host_enrolled: bool,
    worker_execution_allowed: bool,
    qualification_allowed: bool,
    ship_allowed: bool,
}
fn bracket(
    mut authority: impl FnMut() -> Result<(), FixedIntentStorageError>,
    mut intent: impl FnMut() -> Result<(), FixedIntentStorageError>,
) -> Result<StoragePreflightReport, FixedIntentStorageError> {
    authority()?;
    intent()?;
    authority()?;
    intent()?;
    Ok(StoragePreflightReport {
        schema_version: STORAGE_PREFLIGHT_SCHEMA,
        scope: "storage-ancestry-only",
        storage_ancestry_verified: true,
        journal_records_verified: false,
        authority_bound: false,
        host_enrolled: false,
        worker_execution_allowed: false,
        qualification_allowed: false,
        ship_allowed: false,
    })
}
pub fn inspect_storage() -> Result<StoragePreflightReport, FixedIntentStorageError> {
    let mut authority = open_fixed_authority_storage()?;
    let mut intent = open_fixed_intent_storage()?;
    bracket(|| authority.revalidate(), || intent.revalidate())
}

#[cfg(test)]
mod tests {
    use super::super::{walk_kind, StorageLeaf};
    use super::*;
    use std::cell::RefCell;
    use std::fs;
    use std::os::unix::fs::{symlink, PermissionsExt};

    fn tree() -> (tempfile::TempDir, u32, u32) {
        let temp = tempfile::tempdir_in("/tmp").unwrap();
        for suffix in [
            "",
            "var",
            "var/lib",
            "var/lib/hydracache-performance",
            "var/lib/hydracache-performance/worker-authority-074",
            "var/lib/hydracache-performance/diagnostic-intents-074",
        ] {
            let path = temp.path().join(suffix);
            if !path.exists() {
                fs::create_dir(&path).unwrap();
            }
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let uid = unsafe { libc::geteuid() };
        let gid = unsafe { libc::getegid() };
        assert!(
            uid > 0 && gid > 0,
            "local storage models must remain nonroot"
        );
        (temp, uid, gid)
    }
    #[test]
    fn storage_preflight_modeled_two_leaves_are_readonly_and_neither_is_optional() {
        let (temp, uid, gid) = tree();
        let mut authority = walk_kind(temp.path(), uid, gid, StorageLeaf::Authority).unwrap();
        let mut intent = walk_kind(temp.path(), uid, gid, StorageLeaf::Intent).unwrap();
        bracket(|| authority.revalidate(), || intent.revalidate()).unwrap();
        for kind in [StorageLeaf::Authority, StorageLeaf::Intent] {
            let leaf = temp
                .path()
                .join("var/lib/hydracache-performance")
                .join(kind.name());
            assert_eq!(fs::read_dir(&leaf).unwrap().count(), 0);
            let retained = leaf.with_extension("retained");
            fs::rename(&leaf, &retained).unwrap();
            assert!(walk_kind(temp.path(), uid, gid, kind).is_err());
            assert!(
                !leaf.exists(),
                "preflight must not initialize a missing leaf"
            );
            fs::rename(retained, leaf).unwrap();
        }
        // Directory growth is not a metadata replacement and cannot cause a false refusal.
        fs::create_dir(temp.path().join("var/lib/hydracache-performance/unrelated")).unwrap();
        bracket(|| authority.revalidate(), || intent.revalidate()).unwrap();
    }
    #[test]
    fn storage_preflight_modeled_each_ancestry_substitution_and_gid_drift_latches() {
        for kind in [StorageLeaf::Authority, StorageLeaf::Intent] {
            for level in 0..=4 {
                for mutation in 0..4 {
                    let (temp, uid, gid) = tree();
                    let mut read = walk_kind(temp.path(), uid, gid, kind).unwrap();
                    let component = if level == 0 {
                        &mut read.root
                    } else {
                        &mut read.components[level - 1]
                    };
                    if mutation == 3 {
                        // Model an original GID drift without chown or privilege escalation.
                        component.stamp.gid = component.stamp.gid.checked_add(1).unwrap();
                    } else {
                        let names = ["var", "lib", "hydracache-performance", kind.name()];
                        let path = names
                            .iter()
                            .take(level)
                            .fold(temp.path().to_owned(), |p, n| p.join(n));
                        if mutation == 2 {
                            fs::set_permissions(path, fs::Permissions::from_mode(0o770)).unwrap();
                        } else {
                            let backup = tempfile::tempdir_in("/tmp").unwrap();
                            let retained = backup.path().join("original");
                            fs::rename(&path, &retained).unwrap();
                            if mutation == 0 {
                                symlink(&retained, &path).unwrap();
                            } else {
                                fs::create_dir(&path).unwrap();
                                fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
                                    .unwrap();
                            }
                            assert!(read.revalidate().is_err());
                        }
                    }
                    assert!(read.revalidate().is_err());
                    assert!(read.is_refused());
                    assert!(matches!(
                        read.revalidate(),
                        Err(FixedIntentStorageError::Refused)
                    ));
                }
            }
        }
    }
    #[test]
    fn storage_preflight_order_and_scope_cannot_be_launch_permission() {
        let steps = RefCell::new(Vec::new());
        let report = bracket(
            || {
                steps.borrow_mut().push("authority");
                Ok(())
            },
            || {
                steps.borrow_mut().push("intent");
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(
            *steps.borrow(),
            ["authority", "intent", "authority", "intent"]
        );
        let value = serde_json::to_value(report).unwrap();
        assert_eq!(value["schema_version"], STORAGE_PREFLIGHT_SCHEMA);
        assert_eq!(value["scope"], "storage-ancestry-only");
        assert_eq!(value["storage_ancestry_verified"], true);
        for key in [
            "journal_records_verified",
            "authority_bound",
            "host_enrolled",
            "worker_execution_allowed",
            "qualification_allowed",
            "ship_allowed",
        ] {
            assert_eq!(value[key], false, "{key}");
        }
        assert!(serde_json::to_vec(&value).unwrap().len() < 1024);
    }
    #[test]
    fn storage_preflight_each_failure_stops_without_report_or_later_io() {
        for fail_at in 0..4 {
            let steps = RefCell::new(0);
            let step = || {
                let mut count = steps.borrow_mut();
                let at = *count;
                *count += 1;
                if at == fail_at {
                    Err(FixedIntentStorageError::Invalid)
                } else {
                    Ok(())
                }
            };
            assert!(bracket(step, step).is_err());
            assert_eq!(*steps.borrow(), fail_at + 1);
        }
    }
}
