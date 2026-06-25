use std::collections::BTreeMap;
use std::path::PathBuf;

use hydracache::{
    NamespaceMetricLabels, PersistenceConfig, PersistenceConfigErrorKind, PersistenceDurability,
    PersistenceDurabilityConfig, PersistenceNamespaceConfig, PersistenceRecoveryConfig,
    PersistenceRegionSelectorConfig, RecoveryMode, OTHER_NAMESPACE_METRIC_LABEL,
};

#[test]
fn persistence_config_config_roundtrips_to_policy() {
    let mut namespaces = BTreeMap::new();
    namespaces.insert(
        "cache.jwt.pem".to_owned(),
        PersistenceNamespaceConfig {
            persist: true,
            durability: PersistenceDurabilityConfig::Sync,
            snapshot_interval_secs: Some(15),
            regions: PersistenceRegionSelectorConfig::HomeRegionOnly,
            ..PersistenceNamespaceConfig::default()
        },
    );
    let config = PersistenceConfig {
        storage_dir: Some(PathBuf::from("target/hydracache-persistence")),
        snapshot_interval_default_secs: Some(30),
        recovery: PersistenceRecoveryConfig {
            mode: RecoveryMode::FullRecoveryOnly,
            validation_timeout_secs: 10,
            data_load_timeout_secs: 20,
            auto_remove_stale_data: true,
        },
        namespaces,
    };

    let encoded = serde_json::to_string(&config).unwrap();
    let decoded: PersistenceConfig = serde_json::from_str(&encoded).unwrap();
    let policy = decoded.to_policy().unwrap();

    let resolved = policy.resolve("cache.jwt.pem");
    assert!(resolved.persists());
    assert_eq!(resolved.settings.durability, PersistenceDurability::Sync);
    assert_eq!(decoded.to_recovery_policy().data_load_timeout.as_secs(), 20);
}

#[test]
fn persistence_config_persistence_without_storage_dir_refuses_to_start() {
    let mut namespaces = BTreeMap::new();
    namespaces.insert(
        "cache.jwt.pem".to_owned(),
        PersistenceNamespaceConfig::persistent(),
    );
    let config = PersistenceConfig {
        namespaces,
        ..PersistenceConfig::default()
    };

    let error = config.validate_startup(true).unwrap_err();

    assert_eq!(error.kind(), PersistenceConfigErrorKind::MissingStorageDir);
    assert!(error.to_string().contains("no storage_dir"));
}

#[test]
fn persistence_config_namespace_metric_labels_are_bounded() {
    let mut namespaces = BTreeMap::new();
    namespaces.insert(
        "cache.jwt.pem".to_owned(),
        PersistenceNamespaceConfig::persistent(),
    );
    namespaces.insert(
        "cache.*".to_owned(),
        PersistenceNamespaceConfig::persistent(),
    );
    let policy = PersistenceConfig {
        storage_dir: Some(PathBuf::from("target/hydracache-persistence-labels")),
        namespaces,
        ..PersistenceConfig::default()
    }
    .to_policy()
    .unwrap();
    let labels = NamespaceMetricLabels::from_policy(&policy);

    assert_eq!(labels.label_for("cache.jwt.pem"), "cache.jwt.pem");
    assert_eq!(
        labels.label_for("cache.user.42"),
        OTHER_NAMESPACE_METRIC_LABEL
    );
    assert!(labels.registered_labels().contains("cache.jwt.pem"));
    assert!(labels
        .registered_labels()
        .contains(OTHER_NAMESPACE_METRIC_LABEL));
    assert!(!labels.registered_labels().contains("cache.user.42"));
}

#[test]
fn persistence_config_hazelcast_example_translates() {
    let mut namespaces = BTreeMap::new();
    namespaces.insert(
        "cache.jwt.pem".to_owned(),
        PersistenceNamespaceConfig {
            persist: true,
            durability: PersistenceDurabilityConfig::Sync,
            snapshot_interval_secs: Some(30),
            regions: PersistenceRegionSelectorConfig::Only(vec!["eu".to_owned()]),
            ..PersistenceNamespaceConfig::default()
        },
    );
    namespaces.insert("cache.*".to_owned(), PersistenceNamespaceConfig::ram_only());
    let config = PersistenceConfig {
        storage_dir: Some(PathBuf::from("target/hydracache-hot-restart")),
        namespaces,
        ..PersistenceConfig::default()
    };

    let policy = config.to_policy().unwrap();

    assert!(policy.resolve("cache.jwt.pem").persists());
    assert!(!policy.resolve("cache.session").persists());
    assert_eq!(
        policy.resolve("cache.jwt.pem").settings.durability,
        PersistenceDurability::Sync
    );
}
