use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn compatibility_registry_is_complete_and_version_locked() {
    let root = root();
    let registry =
        fs::read_to_string(root.join("docs/testing/management-center/0.72/compatibility.toml"))
            .unwrap();
    let document: toml::Value = toml::from_str(&registry).unwrap();
    assert_eq!(document["schema_version"].as_integer(), Some(1));
    assert_eq!(document["release"].as_str(), Some("0.72.0"));
    assert_eq!(document["previous_release"].as_str(), Some("0.71.0"));
    assert_eq!(
        document["previous_release_artifact_required"].as_bool(),
        Some(true)
    );
    assert_eq!(
        document["legacy_overview_path"].as_str(),
        Some("/cluster/overview")
    );

    let ids = document["artifact"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["id"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        ids,
        BTreeSet::from([
            "management-v1-consensus-progress-v1",
            "management-v1-durable-recovery-v1",
            "management-v1-formation-v1",
            "management-v1-health-catalogue-v1",
            "management-v1-placement-trace-v1",
        ])
    );
    assert!(document["artifact"].as_array().unwrap().iter().all(|row| {
        row["schema_version"].as_integer() == Some(1)
            && row["persisted"].as_bool() == Some(false)
            && row["reader_versions"].as_array().is_some_and(|versions| {
                versions.len() == 1 && versions[0].as_str() == Some("0.72.0")
            })
    }));
    assert_eq!(document["mixed_scenario"].as_array().unwrap().len(), 5);

    let workspace = fs::read_to_string(root.join("Cargo.toml"))
        .unwrap()
        .replace("\r\n", "\n");
    assert!(workspace.contains("[workspace.package]\nversion = \"0.72.0\""));
    assert!(!workspace.contains("version = \"0.70.0\""));
}

#[test]
fn compatibility_guards_are_wired_to_product_and_package_tests() {
    let root = root();
    let aggregation = fs::read_to_string(
        root.join("crates/hydracache-server/tests/management_aggregation_072.rs"),
    )
    .unwrap();
    for marker in [
        "real_http_transport_probes_capability_before_posting_to_an_old_member",
        "pre_072_peer_is_partial_without_sending_an_unsupported_rpc",
        "disabling_management_v1_preserves_legacy_cluster_overview",
    ] {
        assert!(
            aggregation.contains(marker),
            "missing compatibility test {marker}"
        );
    }
    let consumer = fs::read_to_string(root.join("tests/post-publish-consumer/src/lib.rs")).unwrap();
    assert!(consumer.contains("published_management_v1_contract_smoke_test"));
    assert!(consumer.contains("ManagementObservationSource::Unknown"));

    let package = fs::read_to_string(root.join("console/package.json")).unwrap();
    assert!(package.contains("package-management-center.mjs package"));
    assert!(package.contains("package-management-center.mjs verify"));
}
