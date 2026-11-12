use xtask::imap_storage_probe_075::characterize;

#[test]
fn actual_disk_store_characterization_covers_reopen_fences_and_refusal_paths() {
    let scratch = tempfile::tempdir().unwrap();
    let preserved = scratch.path().join("existing.txt");
    std::fs::write(&preserved, b"preserved").unwrap();
    let first = characterize(scratch.path()).unwrap();
    assert_eq!(first, characterize(scratch.path()).unwrap());
    assert_eq!(first.format_version, 1);
    assert_eq!(first.observations.len(), 8);
    assert_eq!(first.unavailable_imap_guarantees.len(), 6);
    assert_eq!(std::fs::read(preserved).unwrap(), b"preserved");
    assert_eq!(std::fs::read_dir(scratch.path()).unwrap().count(), 1);
    assert!(!first.scope.contains("production"));
}

#[test]
fn storage_probe_requires_an_existing_scratch_directory() {
    let scratch = tempfile::tempdir().unwrap();
    assert!(characterize(&scratch.path().join("missing")).is_err());
    std::fs::write(scratch.path().join("file"), b"file").unwrap();
    assert!(characterize(&scratch.path().join("file")).is_err());
}
