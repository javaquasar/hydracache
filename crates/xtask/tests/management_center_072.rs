use std::fs;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn defect(id: &str) -> bool {
    std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok(id)
}

fn documents() -> (String, String, String, String) {
    let root = root();
    (
        fs::read_to_string(root.join("docs/testing/management-center/0.72/claims.toml")).unwrap(),
        fs::read_to_string(root.join("docs/testing/management-center/0.72/failure-taxonomy.toml"))
            .unwrap(),
        fs::read_to_string(root.join("docs/testing/canaries/0.72-management-center.toml")).unwrap(),
        fs::read_to_string(root.join("docs/testing/management-center/0.72/source-map.toml"))
            .unwrap(),
    )
}

#[test]
fn management_registry_covers_every_route_and_failure_class() {
    let problems = xtask::management_center::check(&root(), false).unwrap();
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn management_registry_rejects_missing_or_tampered_proof() {
    let (claims, taxonomy, canaries, source_map) = documents();
    let tampered = claims.replacen("MC72-W2-LEAK-OR-OVERSIZE", "MC72-W2-NONEXISTENT-CANARY", 1);
    let problems = xtask::management_center::check_documents(
        &root(),
        &tampered,
        &taxonomy,
        &canaries,
        &source_map,
        false,
    )
    .unwrap();
    assert!(problems
        .iter()
        .any(|problem| problem.contains("unknown canary")));

    let ship_problems = xtask::management_center::check(&root(), true).unwrap();
    assert!(ship_problems
        .iter()
        .any(|problem| problem.contains("lacks exact-candidate evidence")));
    assert!(ship_problems
        .iter()
        .any(|problem| problem.contains("bounded-resource-pressure is not covered")));
}

#[test]
fn canary_w0_missing_baseline_is_rejected() {
    let bounds =
        fs::read_to_string(root().join("docs/testing/management-center/0.72/bounds.toml")).unwrap();
    assert!(bounds.contains("max_response_bytes"));
    if defect("MC72-W0-BASELINE-DRIFT") {
        panic!("HC-CANARY-RED:MC72-W0-BASELINE-DRIFT");
    }
}

#[test]
fn canary_w1_false_source_mapping_is_rejected() {
    let source =
        fs::read_to_string(root().join("docs/testing/management-center/0.72/source-map.toml"))
            .unwrap();
    assert!(source.contains("authority ="));
    assert!(source.contains("fallback"));
    if defect("MC72-W1-FALSE-SOURCE") {
        panic!("HC-CANARY-RED:MC72-W1-FALSE-SOURCE");
    }
}

#[test]
fn canary_w13_mixed_artifact_is_rejected() {
    let package_test = fs::read_to_string(root().join("console/tests/package.test.js")).unwrap();
    assert!(package_test.contains("substituted artifact"));
    assert!(package_test.contains("assert.throws"));
    if defect("MC72-W13-MIXED-ARTIFACT") {
        panic!("HC-CANARY-RED:MC72-W13-MIXED-ARTIFACT");
    }
}

#[test]
fn canary_w14_tampered_evidence_is_rejected() {
    let (claims, taxonomy, canaries, source_map) = documents();
    let tampered = claims.replacen("status = \"implemented\"", "status = \"deferred\"", 1);
    let problems = xtask::management_center::check_documents(
        &root(),
        &tampered,
        &taxonomy,
        &canaries,
        &source_map,
        false,
    )
    .unwrap();
    assert!(problems
        .iter()
        .any(|problem| problem.contains("still Deferred")));
    if defect("MC72-W14-EVIDENCE-TAMPER") {
        panic!("HC-CANARY-RED:MC72-W14-EVIDENCE-TAMPER");
    }
}
