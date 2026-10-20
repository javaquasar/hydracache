use hydracache_cluster_testkit::value_plane_collection_075::{
    BoundedMatch, CollectionBounds, CollectionError, ReferenceCollectionMap, RemovalCause,
    ScanProjection,
};

fn model() -> ReferenceCollectionMap {
    ReferenceCollectionMap::new(CollectionBounds {
        max_key_bytes: 8,
        max_value_bytes: 8,
        max_entries: 8,
        max_page_items: 2,
        max_scan_items: 2,
        max_response_bytes: 16,
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
        .scan(ScanProjection::Entries, first.next_cursor, 2, 0)
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
        Ok(BoundedMatch::Present)
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
