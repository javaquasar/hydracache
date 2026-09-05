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
fn management_architecture_and_baseline_contracts_fail_closed() {
    let architecture =
        fs::read_to_string(root().join("docs/architecture/management-center-v2.md")).unwrap();
    assert!(xtask::management_center::check_architecture_document(&architecture).is_empty());
    let missing_threat = architecture.replace("confused-deputy fan-out and SSRF", "fan-out");
    assert!(
        xtask::management_center::check_architecture_document(&missing_threat)
            .iter()
            .any(|problem| problem.contains("confused-deputy"))
    );

    let baselines =
        fs::read_to_string(root().join("docs/testing/management-center/0.72/baselines.toml"))
            .unwrap();
    assert!(
        xtask::management_center::check_baseline_document(&root(), &baselines, false)
            .unwrap()
            .is_empty()
    );
    let self_baselined = baselines.replace(
        "candidate_may_self_baseline = false",
        "candidate_may_self_baseline = true",
    );
    assert!(
        xtask::management_center::check_baseline_document(&root(), &self_baselined, false)
            .unwrap()
            .iter()
            .any(|problem| problem.contains("candidate_may_self_baseline=false"))
    );
    assert!(
        xtask::management_center::check_baseline_document(&root(), &baselines, true)
            .unwrap()
            .iter()
            .any(|problem| problem.contains("baseline receipt is missing"))
    );
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
fn management_coverage_inventory_rejects_floor_and_module_omission() {
    let coverage =
        fs::read_to_string(root().join("docs/testing/management-center/0.72/coverage.toml"))
            .unwrap();
    let lowered = coverage.replacen(
        "workspace_line_floor_percent = 88.0",
        "workspace_line_floor_percent = 87.9",
        1,
    );
    let problems = xtask::management_center::check_coverage_document(&root(), &lowered).unwrap();
    assert!(problems
        .iter()
        .any(|problem| problem.contains("coverage floor regressed")));

    let omitted = coverage.replacen(
        "path = \"console/src/history.ts\"",
        "path = \"console/not-a-reviewed-module.js\"",
        1,
    );
    let problems = xtask::management_center::check_coverage_document(&root(), &omitted).unwrap();
    assert!(problems.iter().any(|problem| {
        problem.contains("changed management module lacks coverage row: console/src/history.ts")
    }));
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
fn canary_w0_unowned_field_is_rejected() {
    let (claims, taxonomy, canaries, source_map) = documents();
    let unowned = format!(
        "{source_map}\n[[source]]\nfield = \"unowned.test\"\nauthority = \"none\"\nfallback = \"unavailable\"\nroute = \"/management/v1/unowned\"\n"
    );
    let problems = xtask::management_center::check_documents(
        &root(),
        &claims,
        &taxonomy,
        &canaries,
        &unowned,
        false,
    )
    .unwrap();
    assert!(problems
        .iter()
        .any(|problem| problem.contains("source-map route has no claim")));
    if defect("MC72-W0-UNOWNED-FIELD") {
        panic!("HC-CANARY-RED:MC72-W0-UNOWNED-FIELD");
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
fn canary_w1_asset_drift_is_rejected() {
    let package_test = fs::read_to_string(root().join("console/tests/package.test.js")).unwrap();
    assert!(package_test.contains("substituted artifact"));
    assert!(package_test.contains("assert.throws"));
    if defect("MC72-W1-ASSET-DRIFT") {
        panic!("HC-CANARY-RED:MC72-W1-ASSET-DRIFT");
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

#[test]
fn canary_w14_paper_green_release_is_rejected() {
    let result = xtask::release_evidence::run(vec![
        "--root".to_owned(),
        root().display().to_string(),
        "--release".to_owned(),
        "0.72".to_owned(),
        "--require-ship".to_owned(),
    ]);
    let error = result.expect_err("ship admission accepted missing exact-candidate evidence");
    assert!(
        error.to_string().contains("management-center admission"),
        "release evidence bypassed strict management admission: {error}"
    );
    if defect("MC72-W14-PAPER-GREEN") {
        panic!("HC-CANARY-RED:MC72-W14-PAPER-GREEN");
    }
}

#[test]
fn management_w14_requires_every_decoder_fuzz_receipt() {
    let release = fs::read_to_string(root().join("docs/testing/release-evidence/0.72.toml"))
        .expect("0.72 release evidence manifest");
    let registry = fs::read_to_string(root().join("docs/testing/gated-test-registry.toml"))
        .expect("gated test registry");
    let ci = fs::read_to_string(root().join(".github/workflows/ci.yml")).expect("CI workflow");
    for target in ["envelope", "recovery", "placement", "cursor"] {
        let gate = format!("tool.cargo-fuzz.management-{target}-072");
        assert!(release.contains(&gate), "W14 does not require {gate}");
        assert!(
            registry.contains(&format!("id = \"{gate}\"")),
            "{gate} is unregistered"
        );
        assert!(
            ci.contains(&format!("--gate {gate}")),
            "CI does not execute {gate} through evidence-run"
        );
    }
}

#[test]
fn management_w12_requires_dedicated_process_and_linux_resource_receipts() {
    let release = fs::read_to_string(root().join("docs/testing/release-evidence/0.72.toml"))
        .expect("0.72 release evidence manifest");
    let registry = fs::read_to_string(root().join("docs/testing/gated-test-registry.toml"))
        .expect("gated test registry");
    let ci = fs::read_to_string(root().join(".github/workflows/ci.yml")).expect("CI workflow");
    for gate in [
        "env.hydracache-run-management-process-072",
        "env.hydracache-run-management-resource-linux-072",
    ] {
        assert!(release.contains(gate), "W12 does not require {gate}");
        assert!(
            registry.contains(&format!("id = \"{gate}\"")),
            "{gate} is unregistered"
        );
        assert!(
            ci.contains(&format!("--gate {gate}")),
            "CI does not execute {gate} through evidence-run"
        );
    }
    let resource = registry
        .split("[[gate]]")
        .find(|row| row.contains("id = \"env.hydracache-run-management-resource-linux-072\""))
        .expect("resource gate row");
    assert!(resource.contains("platform = \"linux\""));
    assert!(
        resource.contains("three_daemon_fault_recovery_retains_partial_truth_and_resource_bounds")
    );
}
