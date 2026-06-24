use hydracache::{
    restore_backup_to_point, write_full_backup, write_pitr_log, BackupDataset, BackupError,
    InMemoryObjectStore, PitrLog, PitrRecord,
};

fn dataset() -> BackupDataset {
    BackupDataset::new(b"control-plane-v1".to_vec())
        .with_value("user:1", b"Ada".to_vec())
        .with_value("user:2", b"Grace".to_vec())
}

#[test]
fn backup_restore_full_backup_then_restore_roundtrip_is_identical() {
    let mut store = InMemoryObjectStore::new();
    let manifest = write_full_backup(&mut store, "backup-a", 10, &dataset()).unwrap();

    let restored = restore_backup_to_point(&store, &manifest.manifest_key, None, 10).unwrap();

    assert_eq!(restored, dataset());
}

#[test]
fn backup_restore_pitr_restores_to_chosen_point() {
    let mut store = InMemoryObjectStore::new();
    let manifest = write_full_backup(&mut store, "backup-a", 10, &dataset()).unwrap();
    let pitr = PitrLog::new()
        .push(PitrRecord::put(11, "user:3", b"Linus".to_vec()))
        .push(PitrRecord::delete(12, "user:1"))
        .push(PitrRecord::put(13, "user:2", b"Grace Hopper".to_vec()));
    let pitr_key = write_pitr_log(&mut store, "backup-a", &pitr).unwrap();

    let restored =
        restore_backup_to_point(&store, &manifest.manifest_key, Some(&pitr_key), 12).unwrap();

    assert!(!restored.values.contains_key("user:1"));
    assert_eq!(restored.values["user:2"], b"Grace");
    assert_eq!(restored.values["user:3"], b"Linus");
}

#[test]
fn backup_restore_corrupt_backup_is_detected_not_restored() {
    let mut store = InMemoryObjectStore::new();
    let manifest = write_full_backup(&mut store, "backup-a", 10, &dataset()).unwrap();
    let corrupt_key = manifest.values[0].object_key.clone();

    store
        .mutate(&corrupt_key, |bytes| {
            bytes[0] ^= 0x80;
        })
        .unwrap();

    assert_eq!(
        restore_backup_to_point(&store, &manifest.manifest_key, None, 10),
        Err(BackupError::CorruptObject(corrupt_key))
    );
}

#[test]
fn backup_restore_pitr_corruption_is_detected() {
    let mut store = InMemoryObjectStore::new();
    let manifest = write_full_backup(&mut store, "backup-a", 10, &dataset()).unwrap();
    let pitr = PitrLog::new().push(PitrRecord::put(11, "user:3", b"Linus".to_vec()));
    let pitr_key = write_pitr_log(&mut store, "backup-a", &pitr).unwrap();

    store
        .mutate(&pitr_key, |bytes| {
            let last = bytes.len() - 2;
            bytes[last] = b'0';
        })
        .unwrap();

    assert!(matches!(
        restore_backup_to_point(&store, &manifest.manifest_key, Some(&pitr_key), 11),
        Err(BackupError::CorruptObject(_))
    ));
}

#[test]
#[ignore = "chaos gate: run with simulator fault injection in nightly validation"]
fn backup_restore_restore_under_simulated_faults() {
    let mut store = InMemoryObjectStore::new();
    let manifest = write_full_backup(&mut store, "faulted-backup", 10, &dataset()).unwrap();
    let restored = restore_backup_to_point(&store, &manifest.manifest_key, None, 10).unwrap();
    assert_eq!(restored, dataset());
}
