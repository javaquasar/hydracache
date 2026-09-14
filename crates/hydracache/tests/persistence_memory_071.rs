use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use hydracache::{
    NamespaceMetricLabels, PersistenceConfig, PersistenceConfigErrorKind,
    PersistenceNamespaceConfig, OTHER_NAMESPACE_METRIC_LABEL,
};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

#[test]
fn persistence_without_storage_is_rejected_before_allocating_runtime_state() {
    let mut namespaces = BTreeMap::new();
    namespaces.insert(
        "cache.durable".to_owned(),
        PersistenceNamespaceConfig::persistent(),
    );
    let config = PersistenceConfig {
        namespaces,
        ..PersistenceConfig::default()
    };
    let error = config
        .validate_startup(true)
        .expect_err("storage is required");
    assert_eq!(error.kind(), PersistenceConfigErrorKind::MissingStorageDir);
}

#[test]
fn persistence_metric_labels_are_bounded_and_do_not_embed_dynamic_keys() {
    let mut namespaces = BTreeMap::new();
    namespaces.insert(
        "cache.durable".to_owned(),
        PersistenceNamespaceConfig::persistent(),
    );
    let policy = PersistenceConfig {
        storage_dir: Some(PathBuf::from("target/memory-071-persistence")),
        namespaces,
        ..PersistenceConfig::default()
    }
    .to_policy()
    .expect("policy");
    let labels = NamespaceMetricLabels::from_policy(&policy);
    assert_eq!(
        labels.label_for("cache.dynamic.secret"),
        OTHER_NAMESPACE_METRIC_LABEL
    );
    assert!(!labels.registered_labels().contains("cache.dynamic.secret"));
}

#[test]
fn persistence_optimization_is_deferred_until_anon_file_split_is_qualified() {
    let policy =
        std::fs::read_to_string(root().join("docs/testing/memory/0.71/release-policy.toml"))
            .expect("policy");
    let row = policy
        .split("[[optional_work]]")
        .find(|row| row.contains("id = \"W9-W11\""))
        .expect("W9-W11 row");
    assert!(row.contains("disposition = \"deferred\""));
    assert!(row.contains("persistence"));
}
