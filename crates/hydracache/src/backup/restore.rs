use super::full::{read_manifest, restore_dataset_from_manifest};
use super::{BackupDataset, BackupError, ObjectStore, PitrLog};

/// Restore a full backup and replay PITR records up to `target_sequence`.
pub fn restore_backup_to_point<S>(
    store: &S,
    manifest_key: &str,
    pitr_log_key: Option<&str>,
    target_sequence: u64,
) -> Result<BackupDataset, BackupError>
where
    S: ObjectStore,
{
    let manifest = read_manifest(store, manifest_key)?;
    let mut dataset = restore_dataset_from_manifest(store, &manifest)?;
    if let Some(pitr_log_key) = pitr_log_key {
        let log = PitrLog::decode(&store.get(pitr_log_key)?)?;
        for record in log.records {
            if record.sequence > target_sequence {
                break;
            }
            match record.value {
                Some(value) => {
                    dataset.values.insert(record.key, value);
                }
                None => {
                    dataset.values.remove(&record.key);
                }
            }
        }
    }
    Ok(dataset)
}
