use hydracache_cluster_testkit::value_plane_collection_075::{
    BoundedMatch, CollectionBounds, CollectionError, ReferenceCollectionMap, RemovalCause,
    ReplacementEntry, ScanProjection, TimedMutationOptions, TimedMutationOutcome, TimedRejection,
};
use hydracache_cluster_testkit::value_plane_model_075::MutationIdentity;

fn model() -> ReferenceCollectionMap {
    ReferenceCollectionMap::new(CollectionBounds {
        max_key_bytes: 8,
        max_value_bytes: 8,
        max_entries: 8,
        max_page_items: 2,
        max_scan_items: 2,
        max_response_bytes: 64,
        max_bulk_mutation_items: 8,
    })
    .unwrap()
}

#[test]
fn bounded_collection_api_covers_lifecycle_and_views() {
    let mut map = model();
    map.put(b"a".to_vec(), b"one".to_vec(), None, 0).unwrap();
    map.put(b"b".to_vec(), b"two".to_vec(), Some(5), 0).unwrap();
    map.put(b"c".to_vec(), b"three".to_vec(), None, 0).unwrap();

    assert_eq!(map.size(0), Ok(3));
    assert_eq!(map.is_empty(0), Ok(false));
    let first = map.scan(ScanProjection::Keys, None, 2, 0).unwrap();
    assert_eq!(first.items.len(), 2);
    assert!(!first.complete);
    let second = map
        .scan(ScanProjection::Keys, first.next_cursor, 2, 0)
        .unwrap();
    assert_eq!(second.items.len(), 1);
    assert!(second.complete);
    let values = map.scan(ScanProjection::Values, None, 2, 0).unwrap();
    assert!(values.items.iter().all(|item| item.key.is_none()));

    let incomplete = map.contains_value(b"missing", None, 2, 0).unwrap();
    let BoundedMatch::Incomplete(cursor) = incomplete else {
        panic!("expected bounded incomplete scan");
    };
    assert_eq!(cursor.offset(), 2);
    assert_eq!(
        map.contains_value(b"three", Some(cursor), 2, 0),
        Err(CollectionError::CursorMismatch)
    );
    assert_eq!(
        map.contains_value(b"missing", Some(cursor), 2, 0),
        Ok(BoundedMatch::Absent)
    );

    assert_eq!(map.evict(b"a", 0), Ok(true));
    assert_eq!(map.evict(b"a", 0), Ok(false));
    let evicted = map.evict_all(0).unwrap();
    assert_eq!(evicted.cause, RemovalCause::Evict);
    assert_eq!(evicted.removed, 2);
    assert_eq!(map.clear(0).unwrap().removed, 0);

    map.put(b"z".to_vec(), b"last".to_vec(), None, 0).unwrap();
    let destroyed = map.destroy(0).unwrap();
    assert_eq!(destroyed.cause, RemovalCause::Destroy);
    assert_eq!(destroyed.removed, 1);
    assert!(map.is_destroyed());
    assert_eq!(map.size(0), Err(CollectionError::MapDestroyed));
}

#[test]
fn extended_point_api_preserves_versions_ttl_and_defaults() {
    let mut map = model();
    assert_eq!(
        map.get_or_default(b"a", b"fallback", 0),
        Ok(b"fallback".to_vec())
    );
    assert_eq!(map.size(0), Ok(0));
    map.set(Vec::new(), Vec::new(), None, 0).unwrap();
    assert_eq!(map.get_or_default(b"", b"fallback", 0), Ok(Vec::new()));
    assert_eq!(map.contains_entry(b"", b"", 0), Ok(true));
    assert_eq!(map.contains_entry(b"", b"other", 0), Ok(false));
    assert_eq!(map.contains_entry(b"missing", b"", 0), Ok(false));

    map.set(b"a".to_vec(), b"one".to_vec(), Some(5), 0).unwrap();
    let first = map.get_entry_view(b"a", 1).unwrap().unwrap();
    assert_eq!(first.value, b"one");
    assert_eq!(first.remaining_ttl, Some(4));
    map.set(b"a".to_vec(), b"two".to_vec(), None, 1).unwrap();
    let second = map.get_entry_view(b"a", 4).unwrap().unwrap();
    assert!(second.version > first.version);
    assert_eq!(second.remaining_ttl, None);
    map.set(b"b".to_vec(), b"ttl".to_vec(), Some(5), 0).unwrap();
    assert_eq!(map.get_entry_view(b"b", 5), Ok(None));
    assert_eq!(
        map.get_or_default(b"b", b"default", 5),
        Ok(b"default".to_vec())
    );
    assert!(matches!(
        map.get_entry_view(b"oversized", 5),
        Err(CollectionError::BoundExceeded {
            bound: "key_bytes",
            ..
        })
    ));
    assert!(matches!(
        map.get_or_default(b"a", b"oversized", 5),
        Err(CollectionError::BoundExceeded {
            bound: "value_bytes",
            ..
        })
    ));
    assert!(matches!(
        map.contains_entry(b"a", b"oversized", 5),
        Err(CollectionError::BoundExceeded {
            bound: "value_bytes",
            ..
        })
    ));
}

fn timed(now: u64, deadline: u64) -> TimedMutationOptions {
    TimedMutationOptions {
        now,
        deadline,
        cancelled: false,
        lose_response: false,
    }
}

#[test]
fn timed_mutations_retain_identity_and_report_certainty() {
    let mut map = model();
    let id = MutationIdentity::new("caller", 1);
    assert_eq!(
        map.try_put(
            id.clone(),
            b"a".to_vec(),
            b"one".to_vec(),
            Some(8),
            timed(2, 2)
        ),
        Ok(TimedMutationOutcome::NotApplied(TimedRejection::Deadline))
    );
    assert_eq!(
        map.try_put(
            id.clone(),
            b"a".to_vec(),
            b"one".to_vec(),
            Some(8),
            TimedMutationOptions {
                cancelled: true,
                ..timed(2, 10)
            }
        ),
        Ok(TimedMutationOutcome::NotApplied(TimedRejection::Cancelled))
    );
    assert_eq!(map.size(2), Ok(0));
    assert_eq!(
        map.try_put(
            id.clone(),
            b"a".to_vec(),
            b"one".to_vec(),
            Some(8),
            TimedMutationOptions {
                lose_response: true,
                ..timed(2, 10)
            }
        ),
        Ok(TimedMutationOutcome::OutcomeUnknown)
    );
    let revision = map.revision();
    let resolved = map
        .try_put(
            id.clone(),
            b"a".to_vec(),
            b"one".to_vec(),
            Some(8),
            timed(20, 10),
        )
        .unwrap();
    assert_eq!(
        resolved,
        TimedMutationOutcome::Applied {
            changed: true,
            version: Some(revision)
        }
    );
    assert_eq!(map.revision(), revision);
    assert_eq!(
        map.try_remove(id, b"a", timed(3, 10)),
        Err(CollectionError::MutationIdentityConflict)
    );
    assert_eq!(
        map.try_put(
            MutationIdentity::new("", 1),
            b"a".to_vec(),
            Vec::new(),
            None,
            timed(3, 10)
        ),
        Err(CollectionError::InvalidMutationIdentity)
    );

    let remove_id = MutationIdentity::new("caller", 2);
    assert_eq!(
        map.try_remove(
            remove_id.clone(),
            b"a",
            TimedMutationOptions {
                lose_response: true,
                ..timed(3, 10)
            }
        ),
        Ok(TimedMutationOutcome::OutcomeUnknown)
    );
    let revision = map.revision();
    assert_eq!(
        map.try_remove(remove_id, b"a", timed(4, 10)),
        Ok(TimedMutationOutcome::Applied {
            changed: true,
            version: Some(revision)
        })
    );
    assert_eq!(
        map.try_remove(MutationIdentity::new("caller", 3), b"a", timed(4, 10)),
        Ok(TimedMutationOutcome::Applied {
            changed: false,
            version: None
        })
    );
    assert_eq!(map.size(4), Ok(0));
    assert_eq!(map.retained_mutation_count(), 3);
    map.destroy(4).unwrap();
    assert_eq!(map.retained_mutation_count(), 0);
    assert_eq!(
        map.try_remove(MutationIdentity::new("caller", 4), b"a", timed(4, 10)),
        Err(CollectionError::MapDestroyed)
    );

    let mut bounded = ReferenceCollectionMap::new(CollectionBounds {
        max_entries: 1,
        ..CollectionBounds::default()
    })
    .unwrap();
    bounded
        .try_remove(MutationIdentity::new("caller", 1), b"absent", timed(0, 1))
        .unwrap();
    assert!(matches!(
        bounded.try_remove(MutationIdentity::new("caller", 2), b"absent", timed(0, 1)),
        Err(CollectionError::BoundExceeded {
            bound: "retained_mutations",
            ..
        })
    ));
}

#[test]
fn explicit_replacements_are_bounded_and_ordered() {
    let mut map = model();
    map.set(b"a".to_vec(), b"one".to_vec(), Some(5), 0).unwrap();
    map.set(b"b".to_vec(), b"two".to_vec(), Some(2), 0).unwrap();
    let replacements = [
        ReplacementEntry {
            key: b"missing".to_vec(),
            value: Vec::new(),
        },
        ReplacementEntry {
            key: b"a".to_vec(),
            value: b"new".to_vec(),
        },
        ReplacementEntry {
            key: b"b".to_vec(),
            value: b"expired".to_vec(),
        },
    ];
    let receipts = map.replace_all_detailed(&replacements, 2).unwrap();
    assert_eq!(
        receipts
            .iter()
            .map(|receipt| receipt.input_index)
            .collect::<Vec<_>>(),
        [0, 1, 2]
    );
    assert_eq!(
        receipts
            .iter()
            .map(|receipt| receipt.replaced)
            .collect::<Vec<_>>(),
        [false, true, false]
    );
    assert_eq!(receipts[1].key, b"a");
    assert!(receipts[1].version.is_some());
    assert_eq!(
        map.get_entry_view(b"a", 2).unwrap().unwrap().remaining_ttl,
        Some(3)
    );
    assert_eq!(map.get_or_default(b"missing", b"no", 2), Ok(b"no".to_vec()));
    assert!(matches!(
        map.replace_all_detailed(&[], 2),
        Err(CollectionError::BoundExceeded {
            bound: "bulk_mutation_items",
            actual: 0,
            ..
        })
    ));
    let revision = map.revision();
    let duplicate = [replacements[1].clone(), replacements[1].clone()];
    assert_eq!(
        map.replace_all_detailed(&duplicate, 2),
        Err(CollectionError::DuplicateKey)
    );
    let invalid = [
        replacements[1].clone(),
        ReplacementEntry {
            key: b"b".to_vec(),
            value: b"oversized".to_vec(),
        },
    ];
    assert!(matches!(
        map.replace_all_detailed(&invalid, 2),
        Err(CollectionError::BoundExceeded {
            bound: "value_bytes",
            ..
        })
    ));
    assert_eq!(map.revision(), revision);
    assert_eq!(map.get_or_default(b"a", b"", 2), Ok(b"new".to_vec()));
    let oversized_count = vec![replacements[1].clone(); 9];
    assert!(matches!(
        map.replace_all_detailed(&oversized_count, 2),
        Err(CollectionError::BoundExceeded {
            bound: "bulk_mutation_items",
            ..
        })
    ));
    let mut response_bounded = ReferenceCollectionMap::new(CollectionBounds {
        max_response_bytes: 1,
        ..CollectionBounds::default()
    })
    .unwrap();
    assert!(matches!(
        response_bounded.replace_all_detailed(
            &[ReplacementEntry {
                key: b"aa".to_vec(),
                value: Vec::new()
            }],
            0
        ),
        Err(CollectionError::BoundExceeded {
            bound: "response_bytes",
            ..
        })
    ));
}

#[test]
fn cursor_iterators_are_lazy_and_fail_on_stale_revision() {
    let mut map = model();
    map.set(b"a".to_vec(), b"one".to_vec(), None, 0).unwrap();
    map.set(b"b".to_vec(), b"two".to_vec(), None, 0).unwrap();
    map.set(b"c".to_vec(), b"three".to_vec(), None, 0).unwrap();
    for projection in [
        ScanProjection::Keys,
        ScanProjection::Values,
        ScanProjection::Entries,
    ] {
        let mut iterator = map.iterator(projection, 2, 0).unwrap();
        let mut items = Vec::new();
        while let Some(item) = iterator.next(&mut map, 0) {
            items.push(item.unwrap());
        }
        assert_eq!(items.len(), 3);
        assert!(iterator.next(&mut map, 0).is_none());
    }
    let page = map.scan(ScanProjection::Keys, None, 1, 0).unwrap();
    assert_eq!(
        map.scan(ScanProjection::Values, page.next_cursor, 1, 0),
        Err(CollectionError::CursorMismatch)
    );
    let mut other = model();
    assert_eq!(
        other.scan(ScanProjection::Keys, page.next_cursor, 1, 0),
        Err(CollectionError::CursorMismatch)
    );
    let mut iterator = map.iterator(ScanProjection::Keys, 2, 0).unwrap();
    assert!(iterator.next(&mut map, 0).unwrap().is_ok());
    map.set(b"d".to_vec(), b"four".to_vec(), None, 0).unwrap();
    assert!(matches!(
        iterator.next(&mut map, 0),
        Some(Err(CollectionError::CursorStale { .. }))
    ));
    assert!(iterator.next(&mut map, 0).is_none());
    let mut iterator = map.iterator(ScanProjection::Keys, 1, 0).unwrap();
    assert_eq!(
        iterator.next(&mut other, 0),
        Some(Err(CollectionError::CursorMismatch))
    );
    assert!(map.iterator(ScanProjection::Keys, 0, 0).is_err());
    assert!(map.iterator(ScanProjection::Keys, 3, 0).is_err());
    let mut expiring = model();
    expiring
        .set(b"a".to_vec(), b"one".to_vec(), Some(2), 0)
        .unwrap();
    let mut iterator = expiring.iterator(ScanProjection::Keys, 1, 0).unwrap();
    assert!(matches!(
        iterator.next(&mut expiring, 2),
        Some(Err(CollectionError::CursorStale { .. }))
    ));
    let mut empty = model();
    assert!(empty
        .iterator(ScanProjection::Entries, 1, 0)
        .unwrap()
        .next(&mut empty, 0)
        .is_none());
    let mut large = ReferenceCollectionMap::new(CollectionBounds {
        max_response_bytes: 1,
        ..CollectionBounds::default()
    })
    .unwrap();
    large
        .set(b"a".to_vec(), b"large".to_vec(), None, 0)
        .unwrap();
    let mut iterator = large.iterator(ScanProjection::Entries, 1, 0).unwrap();
    assert!(matches!(
        iterator.next(&mut large, 0),
        Some(Err(CollectionError::BoundExceeded {
            bound: "response_bytes",
            ..
        }))
    ));
    assert!(matches!(
        large.get_entry_view(b"a", 0),
        Err(CollectionError::BoundExceeded {
            bound: "response_bytes",
            ..
        })
    ));
    assert!(matches!(
        large.get_or_default(b"absent", b"fallback", 0),
        Err(CollectionError::BoundExceeded {
            bound: "response_bytes",
            ..
        })
    ));
}

#[test]
fn bulk_retry_retains_receipts_and_shares_point_identity_domain() {
    let mut map = model();
    map.set(b"a".to_vec(), b"old".to_vec(), None, 0).unwrap();
    let entries = [ReplacementEntry {
        key: b"a".to_vec(),
        value: b"new".to_vec(),
    }];
    let identity = MutationIdentity::new("caller", 1);
    let first = map
        .replace_all_detailed_idempotent(identity.clone(), &entries, 0)
        .unwrap();
    map.set(b"a".to_vec(), b"later".to_vec(), None, 0).unwrap();
    let revision = map.revision();
    assert_eq!(
        map.replace_all_detailed_idempotent(identity.clone(), &entries, 0),
        Ok(first)
    );
    assert_eq!(map.revision(), revision);
    assert_eq!(map.get_or_default(b"a", b"", 0), Ok(b"later".to_vec()));
    let conflicting = [ReplacementEntry {
        key: b"a".to_vec(),
        value: b"other".to_vec(),
    }];
    assert_eq!(
        map.replace_all_detailed_idempotent(identity.clone(), &conflicting, 0),
        Err(CollectionError::MutationIdentityConflict)
    );
    assert_eq!(
        map.try_remove(identity, b"a", timed(0, 1)),
        Err(CollectionError::MutationIdentityConflict)
    );
    let point_identity = MutationIdentity::new("caller", 2);
    map.try_put(
        point_identity.clone(),
        b"a".to_vec(),
        b"point".to_vec(),
        None,
        timed(0, 1),
    )
    .unwrap();
    assert_eq!(
        map.replace_all_detailed_idempotent(point_identity, &entries, 0),
        Err(CollectionError::MutationIdentityConflict)
    );
    assert_eq!(
        map.replace_all_detailed_idempotent(MutationIdentity::new("", 1), &entries, 0),
        Err(CollectionError::InvalidMutationIdentity)
    );
    assert!(matches!(
        map.try_remove(MutationIdentity::new("x".repeat(129), 1), b"a", timed(0, 1)),
        Err(CollectionError::BoundExceeded {
            bound: "client_identity_bytes",
            ..
        })
    ));
    let mut bounded = ReferenceCollectionMap::new(CollectionBounds {
        max_entries: 1,
        ..CollectionBounds::default()
    })
    .unwrap();
    bounded
        .replace_all_detailed_idempotent(MutationIdentity::new("caller", 1), &entries, 0)
        .unwrap();
    assert!(matches!(
        bounded.replace_all_detailed_idempotent(MutationIdentity::new("caller", 2), &entries, 0),
        Err(CollectionError::BoundExceeded {
            bound: "retained_mutations",
            ..
        })
    ));
}

#[test]
fn explicit_replacement_payload_and_item_limits_reject_before_cloning_or_apply() {
    let mut map = ReferenceCollectionMap::new(CollectionBounds::default()).unwrap();
    let too_many = (0_u16..257)
        .map(|index| ReplacementEntry {
            key: index.to_be_bytes().to_vec(),
            value: Vec::new(),
        })
        .collect::<Vec<_>>();
    assert!(matches!(
        map.replace_all_detailed_idempotent(MutationIdentity::new("caller", 1), &too_many, 0),
        Err(CollectionError::BoundExceeded {
            bound: "bulk_mutation_items",
            limit: 256,
            ..
        })
    ));
    let too_large = (0_u8..3)
        .map(|index| ReplacementEntry {
            key: vec![index],
            value: vec![0; 4 * 1_024 * 1_024],
        })
        .collect::<Vec<_>>();
    assert!(matches!(
        map.replace_all_detailed_idempotent(MutationIdentity::new("caller", 1), &too_large, 0),
        Err(CollectionError::BoundExceeded {
            bound: "request_bytes",
            ..
        })
    ));
    assert_eq!(map.size(0), Ok(0));
    assert_eq!(map.retained_mutation_count(), 0);
}

#[test]
fn read_only_recovery_resolves_prior_outcomes_and_rejects_payload_conflicts() {
    let mut map = model();
    let put_id = MutationIdentity::new("caller", 1);
    let remove_id = MutationIdentity::new("caller", 2);
    let bulk_id = MutationIdentity::new("caller", 3);
    let replacements = [ReplacementEntry {
        key: b"a".to_vec(),
        value: b"bulk".to_vec(),
    }];
    assert_eq!(
        map.recover_timed_put(&put_id, b"a", b"one", Some(5)),
        Ok(None)
    );
    assert_eq!(map.recover_timed_remove(&remove_id, b"a"), Ok(None));
    assert_eq!(map.recover_replacements(&bulk_id, &replacements), Ok(None));
    map.try_put(
        put_id.clone(),
        b"a".to_vec(),
        b"one".to_vec(),
        Some(5),
        TimedMutationOptions {
            lose_response: true,
            ..timed(0, 1)
        },
    )
    .unwrap();
    let put_outcome = map
        .recover_timed_put(&put_id, b"a", b"one", Some(5))
        .unwrap()
        .unwrap();
    assert!(matches!(
        put_outcome,
        TimedMutationOutcome::Applied { changed: true, .. }
    ));
    let bulk_receipts = map
        .replace_all_detailed_idempotent(bulk_id.clone(), &replacements, 0)
        .unwrap();
    let removed = map
        .try_remove(remove_id.clone(), b"a", timed(0, 1))
        .unwrap();
    assert_eq!(map.size(100), Ok(0));
    let revision = map.revision();
    assert_eq!(
        map.recover_timed_put(&put_id, b"a", b"one", Some(5)),
        Ok(Some(put_outcome))
    );
    assert_eq!(
        map.recover_timed_remove(&remove_id, b"a"),
        Ok(Some(removed))
    );
    assert_eq!(
        map.recover_replacements(&bulk_id, &replacements),
        Ok(Some(bulk_receipts))
    );
    assert_eq!(map.revision(), revision);
    assert_eq!(
        map.recover_timed_put(&put_id, b"a", b"other", Some(5)),
        Err(CollectionError::MutationIdentityConflict)
    );
    assert_eq!(
        map.recover_timed_put(&put_id, b"a", b"one", None),
        Err(CollectionError::MutationIdentityConflict)
    );
    assert_eq!(
        map.recover_timed_remove(&remove_id, b"b"),
        Err(CollectionError::MutationIdentityConflict)
    );
    assert_eq!(
        map.recover_timed_remove(&put_id, b"a"),
        Err(CollectionError::MutationIdentityConflict)
    );
    assert_eq!(
        map.recover_timed_put(&bulk_id, b"a", b"bulk", None),
        Err(CollectionError::MutationIdentityConflict)
    );
    assert_eq!(
        map.recover_replacements(&put_id, &replacements),
        Err(CollectionError::MutationIdentityConflict)
    );
    assert_eq!(
        map.recover_replacements(
            &bulk_id,
            &[ReplacementEntry {
                key: b"a".to_vec(),
                value: b"other".to_vec()
            }]
        ),
        Err(CollectionError::MutationIdentityConflict)
    );
    assert_eq!(
        map.try_remove(MutationIdentity::new("  ", 1), b"a", timed(0, 1)),
        Err(CollectionError::InvalidMutationIdentity)
    );
    assert!(matches!(
        map.try_remove(
            MutationIdentity::new("caller", 4),
            b"oversized",
            timed(0, 1)
        ),
        Err(CollectionError::BoundExceeded {
            bound: "key_bytes",
            ..
        })
    ));
    assert!(matches!(
        map.recover_timed_remove(&MutationIdentity::new("я".repeat(65), 1), b"a"),
        Err(CollectionError::BoundExceeded {
            bound: "client_identity_bytes",
            ..
        })
    ));
    map.destroy(100).unwrap();
    assert_eq!(
        map.recover_timed_remove(&remove_id, b"a"),
        Err(CollectionError::MapDestroyed)
    );
}

#[test]
fn expiry_and_mutations_invalidate_collection_cursors() {
    let mut map = model();
    map.put(b"a".to_vec(), b"one".to_vec(), None, 0).unwrap();
    map.put(b"b".to_vec(), b"two".to_vec(), Some(2), 0).unwrap();
    map.put(b"c".to_vec(), b"three".to_vec(), None, 0).unwrap();
    let cursor = map
        .scan(ScanProjection::Keys, None, 2, 0)
        .unwrap()
        .next_cursor
        .unwrap();
    let cursor_revision = cursor.revision();
    assert_eq!(map.size(2), Ok(2));
    assert!(map.revision() > cursor_revision);
    assert!(matches!(
        map.scan(ScanProjection::Keys, Some(cursor), 2, 2),
        Err(CollectionError::CursorStale { .. })
    ));
}

#[test]
fn collection_bounds_fail_closed_before_visible_mutation() {
    assert_eq!(
        ReferenceCollectionMap::new(CollectionBounds {
            max_page_items: 0,
            ..CollectionBounds::default()
        })
        .unwrap_err(),
        CollectionError::InvalidBound("page_items")
    );

    let mut map = ReferenceCollectionMap::new(CollectionBounds {
        max_key_bytes: 1,
        max_value_bytes: 3,
        max_entries: 2,
        max_page_items: 1,
        max_scan_items: 1,
        max_response_bytes: 2,
        max_bulk_mutation_items: 1,
    })
    .unwrap();
    assert!(matches!(
        map.put(b"aa".to_vec(), b"v".to_vec(), None, 0),
        Err(CollectionError::BoundExceeded {
            bound: "key_bytes",
            ..
        })
    ));
    assert_eq!(
        map.put(b"a".to_vec(), b"v".to_vec(), Some(0), 0),
        Err(CollectionError::InvalidExpiry)
    );
    map.put(b"a".to_vec(), b"one".to_vec(), None, 0).unwrap();
    map.put(b"b".to_vec(), b"two".to_vec(), None, 0).unwrap();
    assert!(matches!(
        map.clear(0),
        Err(CollectionError::BoundExceeded {
            bound: "bulk_mutation_items",
            ..
        })
    ));
    assert_eq!(map.size(0), Ok(2));
    assert!(matches!(
        map.scan(ScanProjection::Entries, None, 1, 0),
        Err(CollectionError::BoundExceeded {
            bound: "response_bytes",
            ..
        })
    ));
    assert!(matches!(
        map.contains_value(b"one", None, 0, 0),
        Err(CollectionError::BoundExceeded {
            bound: "scan_items",
            ..
        })
    ));
}
