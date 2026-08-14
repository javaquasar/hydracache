use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const WORK_ITEMS: [(&str, &str, &str); 13] = [
    ("W0", "docs/adr/0019-hc2-client-transport.md", "gRPC"),
    (
        "W1",
        "crates/hydracache-client-hc2/proto/hc2_contract.proto",
        "ClientPlaneAlpha",
    ),
    ("W2", "crates/hydracache-server/src/hc2.rs", "Connection"),
    (
        "W3",
        "crates/hydracache-client-hc2/src/client.rs",
        "idempotency",
    ),
    (
        "W4",
        "crates/hydracache-client-hc2/src/client.rs",
        "Subscription",
    ),
    (
        "W5",
        "crates/hydracache-client-hc2/src/types.rs",
        "TopologySnapshot",
    ),
    (
        "W6",
        "crates/hydracache-client-hc2/src/client.rs",
        "FencedSession",
    ),
    (
        "W7",
        "crates/hydracache-client-hc2/src/types.rs",
        "CacheValue",
    ),
    (
        "W8",
        "sdks/java/hydracache-hazelcast-facade/pom.xml",
        "hydracache-hazelcast-facade",
    ),
    (
        "W9",
        "sdks/python/hydracache-client-hc2/pyproject.toml",
        "hydracache-client-hc2",
    ),
    (
        "W10",
        "crates/hydracache-client-plane-spike/tests/observability_contract.rs",
        "privacy",
    ),
    (
        "W11",
        "crates/hydracache-client-plane-spike/tests/fault_proxy.rs",
        "same_seed",
    ),
    (
        "W12",
        "docs/testing/gated-test-registry.toml",
        "tool.hc2-hosted-admission-068",
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
            problems.push(format!("{work_item} closure evidence was removed"));
            continue;
        }
        match fs::read_to_string(root.join(source)) {
            Ok(text) if text.contains(marker) => {}
            Ok(_) => problems.push(format!(
                "{work_item} marker {marker:?} is absent from {source}"
            )),
            Err(error) => problems.push(format!(
                "{work_item} source {source} cannot be read: {error}"
            )),
        }
    }
    if let Err(error) = xtask::client_conformance::validate_manifest_at_root(&root) {
        problems.push(format!(
            "cross-SDK conformance manifest is invalid: {error}"
        ));
    }
    let workflow =
        fs::read_to_string(root.join(".github/workflows/hc2-client-plane.yml")).unwrap_or_default();
    for command in [
        "client-schema-check",
        "client-conformance --all-sdks",
        "client-package-check",
        "evidence-run --release 0.68 --gate tool.hc2-hosted-admission-068",
    ] {
        if !workflow.contains(command) {
            problems.push(format!("HC/2 workflow omits {command}"));
        }
    }
    let rust_manifest = fs::read_to_string(root.join("crates/hydracache-client-hc2/Cargo.toml"))
        .unwrap_or_default();
    if rust_manifest
        .lines()
        .any(|line| line.trim() == "publish = false")
    {
        problems.push("Rust HC/2 client must remain publishable for crates.io".to_owned());
    }
    let release_readiness =
        fs::read_to_string(root.join("scripts/verify-release-readiness.ps1")).unwrap_or_default();
    if !release_readiness.contains("hydracache-client-hc2") {
        problems.push("Rust HC/2 client is missing from the crates.io release order".to_owned());
    }
    let package_script =
        fs::read_to_string(root.join("scripts/package-publishable.ps1")).unwrap_or_default();
    if !package_script.contains("\"hydracache-client-hc2\"") {
        problems.push("publishable Rust HC/2 client package validation is missing".to_owned());
    }
    let ci = fs::read_to_string(root.join(".github/workflows/ci.yml")).unwrap_or_default();
    let history_checkpoint = ci.find("- name: Restore and verify full release history");
    let workspace_test = ci
        .find("- name: Test\n")
        .or_else(|| ci.find("- name: Run exact-candidate fast evidence\n"));
    for command in [
        "git rev-parse --is-shallow-repository",
        "git fetch --prune --unshallow origin",
        "+refs/heads/*:refs/remotes/origin/*",
        "+refs/tags/*:refs/tags/*",
        "client-plane-compat-check --manifest-only",
    ] {
        if !ci.contains(command) {
            problems.push(format!(
                "Rust CI history checkpoint omits required command {command:?}"
            ));
        }
    }
    if !matches!((history_checkpoint, workspace_test), (Some(checkpoint), Some(test)) if checkpoint < test)
    {
        problems.push(
            "Rust CI must restore and verify full release history before workspace Nextest"
                .to_owned(),
        );
    }
    let java = fs::read_to_string(root.join("sdks/java/pom.xml")).unwrap_or_default();
    if !java.contains("0.68.0-alpha.1-SNAPSHOT") {
        problems.push("Java HC/2 clients must retain preview SNAPSHOT coordinates".to_owned());
    }
    let python = fs::read_to_string(root.join("sdks/python/hydracache-client-hc2/pyproject.toml"))
        .unwrap_or_default();
    if !python.contains("version = \"0.68.0a1\"") {
        problems.push("Python HC/2 client must retain its source-only alpha version".to_owned());
    }
    problems
}

#[test]
fn release_068_client_plane_closure_is_fail_closed() {
    let problems = closure_problems(None);
    assert!(problems.is_empty(), "0.68 closure problems: {problems:#?}");
}

#[test]
fn canary_release_068_client_plane_accepts_missing_work_item_evidence() {
    let Ok(defect) = env::var("HYDRACACHE_CANARY_DEFECT") else {
        return;
    };
    assert!(WORK_ITEMS.iter().any(|(item, _, _)| *item == defect));
    let problems = closure_problems(Some(&defect));
    assert!(
        problems.is_empty(),
        "HC-CANARY-RED:{defect}: release closure correctly rejected missing evidence: {problems:#?}"
    );
}
