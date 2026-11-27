//! Offline durable-state check for rollback to the published 0.74.0 daemon.

use std::fs::{self, File};
use std::io;
use std::path::Path;

use fs2::FileExt;
use hydracache_cluster_raft::{B74IncompatibleMetadata, SledRaftLogStore};
use serde::Serialize;

use crate::grid_host::{IMAP_HC2_DIR, RAFT_LOG_DIR};
use crate::imap_value_journal::ValueJournal;

/// The durable IMap compatibility decision for one offline member volume.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum B74RollbackCheck {
    Compatible,
    Blocked {
        reason: B74RollbackReason,
        #[serde(skip_serializing_if = "Option::is_none")]
        raft_index: Option<u64>,
    },
}

/// Durable record that makes rollback unsafe for the published predecessor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum B74RollbackReason {
    RaftLog,
    RaftSnapshot,
    ValueJournal,
}

/// Check an initialized, stopped member volume. Sled opens only scratch copies.
pub fn check_b74_rollback(storage: &Path) -> io::Result<B74RollbackCheck> {
    let storage = storage.canonicalize()?;
    let temp_root = std::env::temp_dir().canonicalize()?;
    if temp_root.starts_with(&storage) {
        return Err(io::Error::other(
            "rollback scratch directory must be outside the member volume",
        ));
    }
    let scratch = tempfile::Builder::new()
        .prefix("hydracache-b74-")
        .tempdir_in(temp_root)?;

    // Keep the same exclusive database lock used by Sled until both checks finish.
    let raft_path = storage.join(RAFT_LOG_DIR);
    let mut raft_db = lock_store(&raft_path)?;
    let raft_copy = scratch.path().join(RAFT_LOG_DIR);
    copy_directory(&raft_path, &raft_copy, Some(&mut raft_db))?;
    let raft = SledRaftLogStore::open(raft_copy).map_err(io::Error::other)?;
    if let Some(incompatible) = raft.b74_incompatible_metadata().map_err(io::Error::other)? {
        let (reason, index) = match incompatible {
            B74IncompatibleMetadata::Log { index } => (B74RollbackReason::RaftLog, index),
            B74IncompatibleMetadata::Snapshot { index } => (B74RollbackReason::RaftSnapshot, index),
        };
        return Ok(B74RollbackCheck::Blocked {
            reason,
            raft_index: Some(index),
        });
    }
    let journal_path = storage.join(IMAP_HC2_DIR);
    match fs::symlink_metadata(&journal_path) {
        Ok(_) => {
            let mut journal_db = lock_store(&journal_path)?;
            let journal_copy = scratch.path().join(IMAP_HC2_DIR);
            copy_directory(&journal_path, &journal_copy, Some(&mut journal_db))?;
            let journal = ValueJournal::open(&journal_copy).map_err(io::Error::other)?;
            if journal.has_value_state().map_err(io::Error::other)? {
                return Ok(B74RollbackCheck::Blocked {
                    reason: B74RollbackReason::ValueJournal,
                    raft_index: None,
                });
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    Ok(B74RollbackCheck::Compatible)
}

fn lock_store(path: &Path) -> io::Result<File> {
    let db = path.join("db");
    if !fs::symlink_metadata(path)?.is_dir() || !fs::symlink_metadata(&db)?.is_file() {
        return Err(io::Error::other(
            "expected a regular Sled directory and database file",
        ));
    }
    let db = File::open(db)?;
    FileExt::try_lock_exclusive(&db).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("cannot lock member database for offline inspection: {error}"),
        )
    })?;
    Ok(db)
}

fn copy_directory(
    source: &Path,
    target: &Path,
    mut locked_db: Option<&mut File>,
) -> io::Result<()> {
    fs::create_dir(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let name = entry.file_name();
        let destination = target.join(&name);
        if kind.is_dir() {
            copy_directory(&entry.path(), &destination, None)?;
        } else if kind.is_file() {
            let mut output = File::create(destination)?;
            if let Some(db) = locked_db.as_deref_mut().filter(|_| name == "db") {
                io::copy(db, &mut output)?;
            } else {
                io::copy(&mut File::open(entry.path())?, &mut output)?;
            }
        } else {
            return Err(io::Error::other(
                "Sled store contains a symbolic link or special file",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydracache_cluster_raft::{RaftMetadataRuntime, RaftMetadataRuntimeConfig};

    #[test]
    fn initialized_volume_with_only_a_scope_manifest_is_compatible() {
        let volume = tempfile::tempdir().unwrap();
        let raft_dir = volume.path().join(RAFT_LOG_DIR);
        drop(
            RaftMetadataRuntime::sled_with_config(
                RaftMetadataRuntimeConfig::single_node("test", 1),
                &raft_dir,
            )
            .unwrap(),
        );
        assert_eq!(
            check_b74_rollback(volume.path()).unwrap(),
            B74RollbackCheck::Compatible
        );
        let journal_dir = volume.path().join(IMAP_HC2_DIR);
        let journal = ValueJournal::open(&journal_dir).unwrap();
        journal.bind_scope("tenant", "hc2", 1).unwrap();
        assert!(
            check_b74_rollback(volume.path()).is_err(),
            "live journal holds the Sled lock"
        );
        drop(journal);
        assert_eq!(
            check_b74_rollback(volume.path()).unwrap(),
            B74RollbackCheck::Compatible
        );

        let db = sled::open(journal_dir).unwrap();
        db.insert(b"pending-value-state", b"fixture state").unwrap();
        db.flush().unwrap();
        drop(db);
        assert_eq!(
            check_b74_rollback(volume.path()).unwrap(),
            B74RollbackCheck::Blocked {
                reason: B74RollbackReason::ValueJournal,
                raft_index: None,
            }
        );
    }

    #[test]
    fn missing_or_uninitialized_database_is_not_compatible_evidence() {
        let volume = tempfile::tempdir().unwrap();
        assert!(check_b74_rollback(volume.path()).is_err());
        let raft = volume.path().join(RAFT_LOG_DIR);
        drop(sled::open(&raft).unwrap());
        assert!(check_b74_rollback(volume.path()).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn refuses_to_copy_links_outside_the_volume() {
        let volume = tempfile::tempdir().unwrap();
        let raft_dir = volume.path().join(RAFT_LOG_DIR);
        drop(
            RaftMetadataRuntime::sled_with_config(
                RaftMetadataRuntimeConfig::single_node("test", 1),
                &raft_dir,
            )
            .unwrap(),
        );
        std::os::unix::fs::symlink("../outside", raft_dir.join("unexpected-link")).unwrap();
        assert!(check_b74_rollback(volume.path())
            .unwrap_err()
            .to_string()
            .contains("symbolic link"));
    }
}
