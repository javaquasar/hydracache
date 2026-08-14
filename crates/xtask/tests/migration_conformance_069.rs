use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const WORK_ITEMS: [(&str, &str, &str); 6] = [
    (
        "W0",
        "crates/xtask/src/migration_conformance.rs",
        "validate_at_root",
    ),
    (
        "W1",
        "sdks/java/hydracache-hazelcast-facade/src/test/java/io/hydracache/hazelcast/BorrowedHazelcastExpectationsTest.java",
        "borrowedHazelcastSuiteOutcomesMatchTheManifestExactly",
    ),
    (
        "W2",
        "crates/hydracache/tests/borrowed_cache_semantics.rs",
        "borrowed_cache_semantics_rows_all_execute_and_match_manifest",
    ),
    (
        "W3",
        "docs/testing/compat/legacy-clients.toml",
        "hc1-v0.63.0",
    ),
    (
        "W4",
        "crates/hydracache-db/tests/cached_vs_direct_differential.rs",
        "cached_reads_match_direct_queries_per_consistency_mode_under_concurrent_writes",
    ),
    (
        "W5",
        "docs/testing/release-evidence/0.69.toml",
        "dynamic_canary_work_items",
    ),
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}

fn closure_problems(omitted: Option<&str>) -> Vec<String> {
    let root = root();
    let mut problems = Vec::new();
    for (work_item, source, marker) in WORK_ITEMS {
        if omitted == Some(work_item) {
            problems.push(format!(
                "HC-CANARY-RED:{work_item} closure evidence was removed"
            ));
            continue;
        }
        match fs::read_to_string(root.join(source)) {
            Ok(text) if text.contains(marker) => {}
            Ok(_) => problems.push(format!(
                "HC-CANARY-RED:{work_item} marker {marker:?} is absent from {source}"
            )),
            Err(error) => problems.push(format!(
                "HC-CANARY-RED:{work_item} source {source} cannot be read: {error}"
            )),
        }
    }
    if let Err(error) = xtask::migration_conformance::validate_at_root(&root) {
        problems.push(format!("0.69 manifest contract is invalid: {error}"));
    }
    problems
}

#[test]
fn release_069_migration_conformance_closure_is_fail_closed() {
    let problems = closure_problems(None);
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn release_069_daemon_readiness_contract_is_versioned_across_consumers() {
    let root = root();
    for source in [
        "crates/hydracache-client-plane-spike/src/fixture_receipt.rs",
        "sdks/java/hydracache-client-hc2/src/test/java/io/hydracache/client/hc2/DaemonProcess.java",
        "sdks/java/hydracache-hazelcast-facade/src/test/java/io/hydracache/hazelcast/FacadeDaemonProcess.java",
        "tests/rust-hc2-compat-consumer/tests/production_daemon.rs",
        "tests/java-hc2-consumer/src/test/java/io/hydracache/consumer/ExternalConsumerTest.java",
    ] {
        let text = fs::read_to_string(root.join(source)).unwrap();
        assert!(
            text.contains("READY_DAEMON_V1"),
            "{source} is not bound to the versioned daemon readiness contract"
        );
        assert!(
            !text.contains("assertEquals(\"READY_DAEMON\""),
            "{source} still accepts the unversioned readiness discriminator"
        );
    }
    let rust_consumer =
        fs::read_to_string(root.join("crates/hydracache-client-hc2/tests/grpc_process.rs"))
            .unwrap();
    assert!(rust_consumer.contains("ProductionDaemonReadyReceipt::parse"));
    assert!(!rust_consumer.contains("fields.len() == 6"));
}

#[test]
fn release_069_w1_canary_builds_its_maven_reactor_dependencies() {
    let registry =
        fs::read_to_string(root().join("docs/testing/canary-registry-0.69.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&registry).unwrap();
    let w1 = parsed["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["w_item"] == "W1")
        .unwrap();
    for command in [&w1["guard_command"], &w1["canary_command"]] {
        let args = command["args"].as_array().unwrap();
        assert!(args.iter().any(|arg| arg == "-am"));
        assert!(args
            .iter()
            .any(|arg| arg == "-Dsurefire.failIfNoSpecifiedTests=false"));
    }
}

#[test]
fn release_069_ci_admission_is_independent_fail_loud_and_sha_bound() {
    let workflow = fs::read_to_string(root().join(".github/workflows/ci.yml")).unwrap();
    for required in [
        "migration-conformance-fast-evidence-069:",
        "name: Migration Conformance Fast Evidence 0.69",
        "migration-conformance-admission-069:",
        "if: always()",
        "ci-admission-status",
        "HYDRACACHE_EVIDENCE_HEAD_SHA",
        "HYDRACACHE_EVIDENCE_BASE_SHA",
        "HYDRACACHE_EVIDENCE_TESTED_SHA",
        "postgres:18-alpine@sha256:9a8afca54e7861fd90fab5fdf4c42477a6b1cb7d293595148e674e0a3181de15",
        "ignored.hydracache-db.cached-vs-direct-postgres-soak-069",
    ] {
        assert!(workflow.contains(required), "CI lost {required}");
    }
}

#[test]
fn canary_release_069_accepts_missing_work_item_evidence() {
    let omitted = env::var("HYDRACACHE_CANARY_DEFECT").unwrap_or_default();
    if omitted.is_empty() {
        return;
    }
    assert!(WORK_ITEMS.iter().any(|(item, _, _)| *item == omitted));
    let problems = closure_problems(Some(&omitted));
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn canary_borrowed_manifest_accepts_an_unpinned_commit() {
    if env::var("HYDRACACHE_CANARY_DEFECT").as_deref() != Ok("W0_BAD_PIN") {
        return;
    }
    let root = root();
    let path = "docs/integrations/hazelcast_borrowed_suite.json";
    let text = fs::read_to_string(root.join(path))
        .unwrap()
        .replace("a9ce2a02ac17f88fcd38869ac698e56e613dc40c", "a9ce2a02ac17");
    let result = xtask::migration_conformance::validate_borrowed_text_at_root(&root, path, &text);
    assert!(
        result.is_ok(),
        "HC-CANARY-RED:W0 an abbreviated source commit was accepted: {result:?}"
    );
}

#[test]
fn canary_legacy_matrix_marks_an_unbuilt_tag_green() {
    if env::var("HYDRACACHE_CANARY_DEFECT").as_deref() != Ok("W3_FALSE_GREEN") {
        return;
    }
    let expected = vec![
        "hc1-v0.62.0".to_owned(),
        "hc1-v0.62.1".to_owned(),
        "hc1-v0.63.0".to_owned(),
    ];
    let executed = expected[..2].iter().cloned().collect();
    let result = xtask::migration_conformance::validate_legacy_execution(&expected, &executed);
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn canary_release_069_accepts_an_unwired_postgres_gate() {
    if env::var("HYDRACACHE_CANARY_DEFECT").as_deref() != Ok("W5_UNWIRE_POSTGRES") {
        return;
    }
    let workflow = fs::read_to_string(root().join(".github/workflows/ci.yml"))
        .unwrap()
        .replace("      - migration-conformance-postgres-069\n", "");
    assert!(
        workflow.contains("      - migration-conformance-postgres-069\n"),
        "HC-CANARY-RED:W5 hc2-linux-required accepted a missing PostgreSQL dependency"
    );
}
