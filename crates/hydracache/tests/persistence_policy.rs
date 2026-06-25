use std::time::Duration;

use hydracache::{
    NamespacePersistenceRule, NamespacePersistenceSettings, PersistenceDurability,
    PersistenceMatcher, PersistencePolicy,
};

#[test]
fn persistence_policy_exact_beats_prefix_beats_default() {
    let policy = PersistencePolicy::try_new([
        NamespacePersistenceRule::new("default", NamespacePersistenceSettings::ram_only()).unwrap(),
        NamespacePersistenceRule::persistent("cache.*").unwrap(),
        NamespacePersistenceRule::new(
            "cache.jwt.pem",
            NamespacePersistenceSettings::persistent()
                .with_durability(PersistenceDurability::Sync)
                .with_snapshot_interval(Duration::from_secs(30)),
        )
        .unwrap(),
    ])
    .unwrap();

    let exact = policy.resolve("cache.jwt.pem");
    assert_eq!(
        exact.matched_by,
        Some(PersistenceMatcher::Exact("cache.jwt.pem".to_owned()))
    );
    assert_eq!(exact.settings.durability, PersistenceDurability::Sync);

    let prefix = policy.resolve("cache.session");
    assert_eq!(
        prefix.matched_by,
        Some(PersistenceMatcher::Prefix("cache.".to_owned()))
    );
    assert!(prefix.persists());

    let default = policy.resolve("other");
    assert_eq!(default.matched_by, Some(PersistenceMatcher::Default));
    assert!(!default.persists());
}

#[test]
fn persistence_policy_unconfigured_namespace_is_ram_only() {
    let policy = PersistencePolicy::ram_only();

    let resolved = policy.resolve("unconfigured");

    assert_eq!(resolved.matched_by, None);
    assert!(!resolved.persists());
}

#[test]
fn persistence_policy_conflicting_rules_fail_loud() {
    let error = PersistencePolicy::try_new([
        NamespacePersistenceRule::persistent("wallet.*").unwrap(),
        NamespacePersistenceRule::ram_only("wallet.*").unwrap(),
    ])
    .unwrap_err();

    assert!(error.to_string().contains("conflicting persistence rules"));
    assert!(error.to_string().contains("wallet.*"));
}

#[test]
fn persistence_policy_wildcard_matches_hazelcast_style_patterns() {
    let policy = PersistencePolicy::try_new([
        NamespacePersistenceRule::persistent("cache.*").unwrap(),
        NamespacePersistenceRule::persistent("wallet.*").unwrap(),
    ])
    .unwrap();

    assert!(policy.resolve("cache.jwt.pem").persists());
    assert!(policy.resolve("wallet.balance").persists());
    assert!(!policy.resolve("walletless.balance").persists());
    assert!(!policy.resolve("common.settings").persists());
}
