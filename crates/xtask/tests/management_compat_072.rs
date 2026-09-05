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

    let rehearsal = fs::read_to_string(root.join("scripts/rehearse-publication.ps1")).unwrap();
    for marker in [
        "cargo @arguments",
        "--list",
        "artifact_set_sha256",
        "different candidate",
        "valid topological prefix",
        "Injected publication rehearsal interruption",
        "-Resume",
        "status = \"complete\"",
    ] {
        assert!(
            rehearsal.contains(marker),
            "missing rehearsal guard {marker}"
        );
    }

    let readiness = fs::read_to_string(root.join("scripts/verify-release-readiness.ps1")).unwrap();
    let packager = fs::read_to_string(root.join("scripts/package-publishable.ps1")).unwrap();
    assert!(packager.contains("[switch]$NoVerify"));
    assert!(packager.contains("$args += \"--no-verify\""));
    let packages = [
        "hydracache-core",
        "hydracache-macros",
        "hydracache",
        "hydracache-client-protocol",
        "hydracache-observability",
        "hydracache-client-transport-axum",
        "hydracache-client",
        "hydracache-client-hc2",
        "hydracache-cluster-chitchat",
        "hydracache-cluster-transport-axum",
        "hydracache-cluster-raft",
        "hydracache-cluster",
        "hydracache-actuator-axum",
        "hydracache-redis-compat",
        "hydracache-server",
        "hydracache-db",
        "hydracache-sql-lint",
        "hydracache-cdc-postgres",
        "hydracache-diesel",
        "hydracache-seaorm",
        "hydracache-sqlx",
        "hydracache-transport-nats",
        "hydracache-transport-redis",
    ];
    for package in packages {
        assert!(readiness.contains(&format!("\"{package}\"")));
        assert!(rehearsal.contains(&format!("\"{package}\"")));
    }
}

#[test]
fn real_mixed_binary_proof_is_release_blocking_and_cannot_use_the_065_fallback() {
    let root = root();
    let release = fs::read_to_string(root.join("docs/testing/release-evidence/0.72.toml")).unwrap();
    let gates = fs::read_to_string(root.join("docs/testing/gated-test-registry.toml")).unwrap();
    let workflow = fs::read_to_string(root.join(".github/workflows/ci.yml")).unwrap();
    let source =
        fs::read_to_string(root.join("crates/hydracache-server/tests/management_mixed_072.rs"))
            .unwrap();
    let gate = "env.hydracache-run-management-mixed-072";
    assert!(release.contains(gate));
    let row = gates
        .split("[[gate]]")
        .find(|row| row.contains(&format!("id = \"{gate}\"")))
        .expect("0.72 mixed gate");
    assert!(row.contains("ship_mandatory = true"));
    assert!(row.contains("management-mixed-071-072.json"));
    assert!(workflow.contains(&format!("--gate {gate}")));
    assert!(workflow.contains("git rev-parse --verify refs/tags/v0.71.0"));
    assert!(source.contains("const PREVIOUS_TAG: &str = \"v0.71.0\""));
    assert!(!source.contains("PREVIOUS_DAEMON_DEV_COMMIT"));
    assert!(!source.contains("v0.65.0"));
}
