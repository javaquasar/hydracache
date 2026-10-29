use super::*;
fn ready(storage: &mut CollectorReservationStorage<'_>) {
    storage.begin_starting().unwrap();
    for at in STAGES {
        storage.starting_step(at, b"starting\n").unwrap();
    }
    storage.acknowledge_starting().unwrap();
}
#[test]
fn original_collector_custody_requires_acknowledged_starting_and_sticks() {
    with_storage(|storage, _| {
        assert!(matches!(
            storage.retain_original_collector_custody(),
            Err(StorageAttemptError::Refused)
        ));
        assert!(storage.is_refused());
        assert!(matches!(
            storage.retain_original_collector_custody(),
            Err(StorageAttemptError::Refused)
        ));
    });
}
#[test]
fn original_collector_custody_checks_storage_before_acquisition_and_never_retries() {
    with_storage(|storage, leaf| {
        ready(storage);
        std::fs::set_permissions(
            leaf.join(STARTING_NAME),
            std::fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        assert!(storage.retain_original_collector_custody().is_err());
        assert!(storage.is_refused());
        assert!(!storage.original.empty.fence.custody_retained);
        assert!(matches!(
            storage.retain_original_collector_custody(),
            Err(StorageAttemptError::Refused)
        ));
    });
}
#[test]
fn original_collector_custody_has_one_owner_and_latches_repeat_refusal() {
    with_storage(|storage, _| {
        ready(storage);
        let custody = storage.retain_original_collector_custody().unwrap();
        custody.inspect(storage).unwrap();
        storage.revalidate().unwrap();
        assert!(storage.retain_original_collector_custody().is_err());
        assert!(storage.is_refused());
        assert!(matches!(
            custody.inspect(storage),
            Err(StorageAttemptError::Refused)
        ));
        drop(custody);
    });
}
#[test]
fn original_collector_custody_rejects_foreign_original_and_descriptor_drift() {
    for kind in 0..3 {
        with_storage(|storage, _| {
            ready(storage);
            let mut custody = storage.retain_original_collector_custody().unwrap();
            custody.inspect(storage).unwrap();
            if kind == 0 {
                with_storage(|other, _| {
                    ready(other);
                    assert!(matches!(
                        custody.inspect(other),
                        Err(StorageAttemptError::Document)
                    ));
                });
                custody.inspect(storage).unwrap();
            } else {
                custody.tamper_for_test(kind == 1);
                assert!(matches!(
                    custody.inspect(storage),
                    Err(StorageAttemptError::Document)
                ));
            }
        });
    }
}
