fn runtime_seed(registry: &mut FixtureAuthorityRegistry) {
    for entry in &mut registry.entries {
        for leaf in [&mut entry.record, &mut entry.marker] {
            leaf.scratch = Some(vec![0xa5; leaf.raw.len()]);
        }
    }
}
fn runtime_empty(registry: &FixtureAuthorityRegistry) -> bool {
    registry
        .entries
        .iter()
        .all(|entry| entry.record.scratch.is_none() && entry.marker.scratch.is_none())
}
fn runtime_bounded(registry: &FixtureAuthorityRegistry) {
    for entry in &registry.entries {
        for (leaf, budget) in [
            (&entry.record, MAX_RECORD_BYTES),
            (&entry.marker, COMMIT_MARKER_BYTES),
        ] {
            if let Some(raw) = &leaf.scratch {
                assert_eq!(raw, &leaf.raw);
                assert_eq!(raw.capacity(), leaf.stamp.len as usize);
                assert!(raw.capacity() <= budget);
            }
        }
    }
}
#[test]
fn authority_runtime_leaf_cold_fresh_storage_and_bounds() {
    for length in [0, 1, 65, 511, MAX_RECORD_BYTES] {
        let (dir, uid, gid) = fixture();
        let parent = File::open(dir.path()).unwrap();
        let bytes: Vec<_> = (0..length).map(|n| (n as u8).wrapping_add(74)).collect();
        let mut file = create_new(&parent, "leaf").unwrap();
        write_seal(&mut file, &bytes).unwrap();
        let mut leaf = Leaf::open(&parent, "leaf".into(), uid, gid, MAX_RECORD_BYTES).unwrap();
        assert!(leaf.scratch.is_none());
        let original = (leaf.file.as_raw_fd(), leaf.stamp.clone(), leaf.raw.clone());
        let mut slot = Some(vec![0xaa; length]);
        let named =
            Leaf::read_reusing(&parent, "leaf", uid, gid, MAX_RECORD_BYTES, &mut slot).unwrap();
        assert!(slot.is_none());
        assert_eq!(
            named.2, bytes,
            "old length and poison are not appended to fresh data"
        );
        assert!(named.1 == original.1);
        assert_eq!(
            named.0.metadata().unwrap().ino(),
            leaf.file.metadata().unwrap().ino()
        );
        for _ in 0..3 {
            leaf.revalidate(&parent, uid, gid, MAX_RECORD_BYTES)
                .unwrap();
            if let Some(raw) = &leaf.scratch {
                assert_eq!(raw, &bytes);
                assert_eq!(raw.capacity(), length);
            }
            assert_eq!(leaf.file.as_raw_fd(), original.0);
            assert!(leaf.stamp == original.1);
            assert_eq!(leaf.raw, original.2);
        }
    }
}
#[test]
fn authority_runtime_leaf_refusals_evict_and_retain_original_objects() {
    for fault in 0..10 {
        let (dir, uid, gid) = fixture();
        let parent = File::open(dir.path()).unwrap();
        let path = dir.path().join("leaf");
        let mut file = create_new(&parent, "leaf").unwrap();
        write_seal(&mut file, &[74; 65]).unwrap();
        let mut leaf = Leaf::open(&parent, "leaf".into(), uid, gid, MAX_RECORD_BYTES).unwrap();
        leaf.scratch = Some(vec![0xbb; 65]);
        let mut check_uid = uid;
        let mut check_gid = gid;
        let mut budget = MAX_RECORD_BYTES;
        match fault {
            0 => fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap(),
            1 => fs::remove_file(&path).unwrap(),
            2 => {
                fs::remove_file(&path).unwrap();
                symlink("missing", &path).unwrap();
            }
            3 => fs::hard_link(&path, dir.path().join("alias")).unwrap(),
            4 => {
                fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
                fs::write(&path, vec![0; MAX_RECORD_BYTES + 1]).unwrap();
                fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
            }
            5 => check_uid = u32::MAX,
            6 => check_gid = u32::MAX,
            7 => budget = 64,
            8 => leaf.raw[0] ^= 1,
            9 => {
                let mut other = create_new(&parent, "other").unwrap();
                write_seal(&mut other, &[74; 65]).unwrap();
                leaf.name = "other".into();
            }
            _ => unreachable!(),
        }
        let original = (leaf.file.as_raw_fd(), leaf.stamp.clone(), leaf.raw.clone());
        assert!(
            leaf.revalidate(&parent, check_uid, check_gid, budget)
                .is_err(),
            "fault {fault}"
        );
        assert!(leaf.scratch.is_none());
        assert_eq!(leaf.file.as_raw_fd(), original.0);
        assert!(leaf.stamp == original.1);
        assert_eq!(leaf.raw, original.2);
    }
}
#[test]
fn authority_runtime_all_five_journal_refusals_evict_every_leaf() {
    for site in 0..5 {
        let (dir, uid, gid) = fixture();
        let p = checked(7);
        let next = checked(8);
        let mut registry = initialize_fixture_registry(dir.path(), uid, gid, &p).unwrap();
        registry.refuse(RefusalReason::Operator).unwrap();
        registry.advance(&next).unwrap();
        registry.inspect(&next.trust, &next.context).unwrap();
        runtime_seed(&mut registry);
        assert!(!runtime_empty(&registry));
        let originals: Vec<_> = registry
            .entries
            .iter()
            .flat_map(|entry| [&entry.record, &entry.marker])
            .map(|leaf| (leaf.file.as_raw_fd(), leaf.stamp.clone(), leaf.raw.clone()))
            .collect();
        match site {
            0 => registry.latch_refusal(),
            1 => {
                fs::remove_file(dir.path().join(name(3, "commit"))).unwrap();
                assert!(registry.inspect(&next.trust, &next.context).is_err());
            }
            2 => {
                let wrong = checked(9);
                assert!(matches!(
                    registry.inspect(&wrong.trust, &wrong.context),
                    Err(AuthorityError::Binding)
                ));
            }
            3 => assert!(matches!(
                registry.advance(&checked(9)),
                Err(AuthorityError::Transition)
            )),
            4 => assert!(matches!(
                registry.change(
                    |record| record.refuse(RefusalReason::Storage),
                    Some(PublicationStage::RecordCreated)
                ),
                Err(AuthorityError::Injected(PublicationStage::RecordCreated))
            )),
            _ => unreachable!(),
        }
        assert!(registry.is_failed());
        assert!(
            runtime_empty(&registry),
            "whole-journal eviction site {site}"
        );
        for (leaf, original) in registry
            .entries
            .iter()
            .flat_map(|entry| [&entry.record, &entry.marker])
            .zip(&originals)
        {
            assert_eq!(leaf.file.as_raw_fd(), original.0);
            assert!(leaf.stamp == original.1);
            assert_eq!(leaf.raw, original.2);
        }
        let count = fs::read_dir(dir.path()).unwrap().count();
        assert!(matches!(
            registry.refuse(RefusalReason::Operator),
            Err(AuthorityError::Refused)
        ));
        assert!(matches!(
            registry.advance(&checked(9)),
            Err(AuthorityError::Refused)
        ));
        assert!(matches!(
            registry.inspect(&next.trust, &next.context),
            Err(AuthorityError::Refused)
        ));
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), count);
        assert!(runtime_empty(&registry));
    }
}
#[test]
fn authority_runtime_reopen_does_not_transfer_scratch() {
    let (dir, uid, gid) = fixture();
    let p = checked(7);
    let mut registry = initialize_fixture_registry(dir.path(), uid, gid, &p).unwrap();
    registry.inspect(&p.trust, &p.context).unwrap();
    let original = registry.entries[0].record.raw.clone();
    for entry in &mut registry.entries {
        entry.record.scratch = Some(vec![0xcc; MAX_RECORD_BYTES + 1]);
        entry.marker.scratch = Some(vec![0xdd; MAX_RECORD_BYTES + 1]);
    }
    drop(registry);
    let mut reopened = open_fixture_registry(dir.path(), uid, gid, &p.trust, &p.context).unwrap();
    runtime_bounded(&reopened); // Required closing revalidation may warm new slots.
    assert_eq!(reopened.inspect(&p.trust, &p.context).unwrap().revision, 1);
    assert_eq!(reopened.entries[0].record.raw, original);
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
}
#[test]
fn authority_runtime_publication_cuts_clear_cache_and_never_repair_tail() {
    for stage in [
        PublicationStage::RecordCreated,
        PublicationStage::RecordSynced,
        PublicationStage::MarkerCreated,
        PublicationStage::DurableCommit,
    ] {
        let (dir, uid, gid) = fixture();
        let p = checked(7);
        let mut registry = initialize_fixture_registry(dir.path(), uid, gid, &p).unwrap();
        runtime_seed(&mut registry);
        let original = registry.entries[0].record.raw.clone();
        assert!(
            matches!(registry.change(|record| record.refuse(RefusalReason::Storage), Some(stage)), Err(AuthorityError::Injected(at)) if at == stage)
        );
        assert!(runtime_empty(&registry));
        assert_eq!(
            fs::read(dir.path().join(name(1, "json"))).unwrap(),
            original
        );
        let count = fs::read_dir(dir.path()).unwrap().count();
        assert!(matches!(
            registry.refuse(RefusalReason::Operator),
            Err(AuthorityError::Refused)
        ));
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), count);
        drop(registry);
        let reopened = open_fixture_registry(dir.path(), uid, gid, &p.trust, &p.context);
        if stage == PublicationStage::DurableCommit {
            assert!(reopened.unwrap().view().unwrap().refused);
        } else {
            assert!(reopened.is_err());
        }
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), count);
    }
}
#[test]
fn authority_runtime_successful_epoch_transitions_keep_bounded_original_history() {
    let (dir, uid, gid) = fixture();
    let initial = checked(7);
    let mut registry = initialize_fixture_registry(dir.path(), uid, gid, &initial).unwrap();
    let original = registry.entries[0].record.raw.clone();
    for epoch in 8..=10 {
        registry.refuse(RefusalReason::Operator).unwrap();
        let p = checked(epoch);
        registry.advance(&p).unwrap();
        assert!(!registry.inspect(&p.trust, &p.context).unwrap().refused);
        runtime_bounded(&registry);
        assert_eq!(registry.entries[0].record.raw, original);
        assert_eq!(registry.entries.len(), 1 + 2 * (epoch as usize - 7));
    }
    assert_eq!(
        fs::read_dir(dir.path()).unwrap().count(),
        registry.entries.len() * 2
    );
}

#[test]
fn authority_runtime_closing_publication_refusal_evicts_new_and_old_entries() {
    let (dir, uid, gid) = fixture();
    let p = checked(7);
    let mut registry = initialize_fixture_registry(dir.path(), uid, gid, &p).unwrap();
    runtime_seed(&mut registry);
    let extra = dir.path().join("unexpected");
    publication_checkpoint::with(
        move |stage| {
            if stage == PublicationStage::DurableCommit {
                fs::write(&extra, b"extra inventory entry").unwrap();
            }
        },
        || assert!(registry.refuse(RefusalReason::Storage).is_err()),
    );
    assert_eq!(
        registry.entries.len(),
        2,
        "new entry retained before closing validation"
    );
    assert_eq!(registry.records.len(), 2);
    assert!(registry.is_failed());
    assert!(runtime_empty(&registry));
    let count = fs::read_dir(dir.path()).unwrap().count();
    assert!(matches!(
        registry.refuse(RefusalReason::Operator),
        Err(AuthorityError::Refused)
    ));
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), count);
}
