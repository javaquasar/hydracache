use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use hydracache_client_protocol::hibernate::{
    HibernateRegionKind, L2AccessMode, L2ConsistencyLabel, QueryCacheBehavior, QueryCacheMapping,
    RegionMapping, HIBERNATE_CONTRACT_VERSION, HIBERNATE_SUPPORTED_MAJOR,
    HIBERNATE_SUPPORTED_RANGE,
};
use hydracache_client_protocol::{
    ClientRequest, Namespace, ReadConsistency, StructuredKey, WriteConsistency,
};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn user_region(mode: L2AccessMode) -> RegionMapping {
    RegionMapping::new(
        "com.example.User",
        Namespace::new("hibernate:com.example.User").unwrap(),
        HibernateRegionKind::Entity,
        mode,
    )
    .unwrap()
}

fn entity_key(id: &str) -> StructuredKey {
    StructuredKey::new(vec!["entity".to_owned(), id.to_owned()]).unwrap()
}

#[test]
fn hibernate_contract_access_mode_maps_to_consistency_mode() {
    let read_only = L2AccessMode::ReadOnly.consistency_mapping();
    assert_eq!(read_only.label, L2ConsistencyLabel::StrongImmutable);
    assert_eq!(read_only.label.as_str(), "strong-immutable");
    assert_eq!(read_only.read, ReadConsistency::Strong);
    assert_eq!(read_only.write, None);
    assert!(read_only.immutable);
    assert!(!read_only.invalidates_on_write);
    assert!(!read_only.invalidates_on_commit);
    assert!(!read_only.joins_jvm_transaction);

    let nonstrict = L2AccessMode::NonStrictReadWrite.consistency_mapping();
    assert_eq!(nonstrict.label, L2ConsistencyLabel::BestEffortInvalidate);
    assert_eq!(nonstrict.read, ReadConsistency::Eventual);
    assert_eq!(nonstrict.write, Some(WriteConsistency::Local));
    assert!(nonstrict.invalidates_on_write);
    assert!(!nonstrict.invalidates_on_commit);
    assert!(!nonstrict.joins_jvm_transaction);

    for mode in [L2AccessMode::ReadWrite, L2AccessMode::Transactional] {
        let mapping = mode.consistency_mapping();
        assert_eq!(mapping.label, L2ConsistencyLabel::InvalidateOnCommit);
        assert_eq!(mapping.read, ReadConsistency::Session);
        assert_eq!(mapping.write, Some(WriteConsistency::Quorum));
        assert!(!mapping.invalidates_on_write);
        assert!(mapping.invalidates_on_commit);
        assert!(!mapping.joins_jvm_transaction);
    }
}

#[test]
fn hibernate_contract_region_mapping_builds_reviewable_keys_and_context() {
    let mapping = user_region(L2AccessMode::ReadWrite);
    let key = mapping.key(["42"]).unwrap();

    assert_eq!(mapping.ns.as_str(), "hibernate:com.example.User");
    assert_eq!(
        mapping.consistency_mapping().label.as_str(),
        "invalidate-on-commit"
    );
    assert_eq!(
        mapping.client_context().read,
        Some(ReadConsistency::Session)
    );
    assert_eq!(
        mapping.client_context().write,
        Some(WriteConsistency::Quorum)
    );
    assert_eq!(key.segments(), &["entity".to_owned(), "42".to_owned()]);

    match mapping.put(key.clone(), b"user".to_vec(), Some(5_000)) {
        ClientRequest::Put {
            ns,
            key: put_key,
            value,
            ttl_ms,
            dimensions,
        } => {
            assert_eq!(ns, mapping.ns);
            assert_eq!(put_key, key);
            assert_eq!(value, b"user");
            assert_eq!(ttl_ms, Some(5_000));
            assert_eq!(
                dimensions,
                ["hibernate", "entity", "com.example.User"].map(str::to_owned)
            );
        }
        other => panic!("expected put request, got {other:?}"),
    }
}

#[test]
fn hibernate_contract_evict_region_clears_namespace() {
    let mapping = user_region(L2AccessMode::NonStrictReadWrite);
    let other = RegionMapping::from_region(
        "com.example.Order",
        HibernateRegionKind::Entity,
        L2AccessMode::ReadWrite,
    )
    .unwrap();

    let key_42 = entity_key("42");
    let key_43 = entity_key("43");
    let mut store = BTreeMap::from([
        ((mapping.ns.clone(), key_42.clone()), b"user-42".to_vec()),
        ((mapping.ns.clone(), key_43.clone()), b"user-43".to_vec()),
        ((other.ns.clone(), entity_key("9")), b"order-9".to_vec()),
    ]);

    match mapping.evict_region() {
        ClientRequest::EvictRegion { ns } => {
            store.retain(|(entry_ns, _), _| entry_ns != &ns);
        }
        other => panic!("expected region eviction, got {other:?}"),
    }

    assert!(!store.contains_key(&(mapping.ns.clone(), key_42)));
    assert!(!store.contains_key(&(mapping.ns.clone(), key_43)));
    assert_eq!(store.len(), 1);
    assert!(store.keys().all(|(ns, _)| ns == &other.ns));
}

#[test]
fn hibernate_contract_query_region_uses_timestamp_bulk_invalidation() {
    let query_region = RegionMapping::from_region(
        "default-query-results",
        HibernateRegionKind::Query,
        L2AccessMode::NonStrictReadWrite,
    )
    .unwrap();
    let timestamps_region = RegionMapping::from_region(
        "default-update-timestamps",
        HibernateRegionKind::Timestamps,
        L2AccessMode::ReadWrite,
    )
    .unwrap();

    assert_eq!(
        query_region.kind.query_cache_behavior(),
        QueryCacheBehavior::TimestampBulkInvalidation
    );
    assert_eq!(
        timestamps_region.kind.query_cache_behavior(),
        QueryCacheBehavior::TimestampBulkInvalidation
    );
    assert_eq!(
        HibernateRegionKind::Entity.query_cache_behavior(),
        QueryCacheBehavior::NotQueryCache
    );

    let query_cache = QueryCacheMapping::new(query_region.clone(), timestamps_region.clone())
        .expect("query cache mapping");
    let [query_evict, timestamps_evict] = query_cache.bulk_update_evictions();
    assert_eq!(query_evict, query_region.evict_region());
    assert_eq!(timestamps_evict, timestamps_region.evict_region());
}

#[test]
fn hibernate_contract_query_mapping_rejects_wrong_region_kind() {
    let entity_region = RegionMapping::from_region(
        "entity",
        HibernateRegionKind::Entity,
        L2AccessMode::ReadWrite,
    )
    .unwrap();
    let timestamps_region = RegionMapping::from_region(
        "ts",
        HibernateRegionKind::Timestamps,
        L2AccessMode::ReadWrite,
    )
    .unwrap();

    let err = QueryCacheMapping::new(entity_region, timestamps_region).unwrap_err();
    assert_eq!(
        err.to_string(),
        "invalid client protocol field: query_region_kind"
    );
}

#[test]
fn hibernate_contract_supported_matrix_and_docs_are_registered() {
    assert_eq!(HIBERNATE_CONTRACT_VERSION, 1);
    assert_eq!(HIBERNATE_SUPPORTED_MAJOR, 6);
    assert_eq!(HIBERNATE_SUPPORTED_RANGE, "Hibernate ORM 6.x");

    let root = repo_root();
    let integration =
        fs::read_to_string(root.join("docs/integrations/hibernate.md")).expect("hibernate docs");
    let adr = fs::read_to_string(root.join("docs/adr/0006-why-not-clone-hibernate-hikaricp.md"))
        .expect("hibernate ADR");
    let compat = fs::read_to_string(root.join("docs/COMPAT.md")).expect("compat");

    for needle in [
        "Hibernate ORM 6.x",
        "read-only -> strong-immutable",
        "nonstrict-read-write -> best-effort-invalidate",
        "read-write / transactional -> invalidate-on-commit",
        "HydraCache does not join the JVM transaction",
        "query cache",
    ] {
        assert!(
            integration.contains(needle),
            "integration docs missing {needle}"
        );
    }

    assert!(
        adr.contains("provider"),
        "ADR should choose provider approach"
    );
    assert!(
        compat.contains("Hibernate L2 provider contract"),
        "COMPAT should register W2 contract"
    );
    assert!(
        compat.contains("0006-why-not-clone-hibernate-hikaricp.md"),
        "COMPAT should link ADR 0006"
    );
}
