use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use xtask::imap_extended_receipt_075::Artifact;
use xtask::imap_pre074_075::*;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}
fn contract() -> IntegrationContract {
    serde_json::from_str(include_str!(
        "../../../docs/testing/imap/0.75/pre074-integration-contract.json"
    ))
    .unwrap()
}
fn graph() -> Value {
    json!({"packages":[
        {"name":"hydracache-server","dependencies":[{"name":"hydracache-cluster-testkit","kind":"dev"}]},
        {"name":"hydracache-client-hc2","dependencies":[]},
        {"name":"hydracache-client-transport-axum","dependencies":[]},
        {"name":"hydracache-cluster-testkit","dependencies":[]},
        {"name":"intermediate","dependencies":[{"name":"hydracache-cluster-testkit","kind":null}]}
    ]})
}
fn java() -> JavaProbe {
    serde_json::from_value(json!({"schema":"imap075-packaged-hazelcast-probe-v1","profile":"exclusive_eternal_binary_no_tls_no_backups",
        "artifact_sha256":HAZELCAST_SHA256,"steps":[
            {"operation":"absent","kind":"absent","value":null},{"operation":"put","kind":"inserted","value":null},
            {"operation":"present","kind":"present","value":"0080ff"},{"operation":"mismatch","kind":"mismatch","value":"0080ff"},
            {"operation":"replace","kind":"replaced","value":"0080ff"},{"operation":"remove","kind":"removed","value":"09"}],
        "native_events":["added","updated","removed"],"snapshot_digest":xtask::imap_qualification_075::digest(&[]),
        "typed_port_kind":"inserted","typed_port_value":"0080ff","operation_count":40,
        "diagnostics":{"scope":"single_jvm_member_client_diagnostic","operation":"native_get_absent","warmup_operations":8,"samples_ns":[1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1],"heap_used_bytes":1024,"heap_committed_bytes":2048,"non_heap_used_bytes":1024,"threads":32,"gc_collections":0}})).unwrap()
}

#[test]
fn checked_in_contract_keeps_operational_evidence_and_predecessor_boundaries_explicit() {
    validate_contract(&root(), &contract()).unwrap();
    check_contract(&root()).unwrap();
    for field in 0..9 {
        let mut changed = contract();
        match field {
            0 => changed.production_enabled = true,
            1 => changed.release = "0.74".into(),
            2 => changed.scope = "production".into(),
            3 => changed.predecessor_required = "none".into(),
            4 => {
                changed.blocked_until_074.pop();
            }
            5 => {
                changed.production_observations_required.pop();
            }
            6 => changed.profiles.clear(),
            7 => changed.tests[0] = "../outside".into(),
            _ => changed.tests[0] = changed.tests[1].clone(),
        }
        assert!(validate_contract(&root(), &changed).is_err(), "{field}");
    }
}

#[test]
fn dev_edges_are_allowed_but_normal_build_optional_and_target_paths_cannot_reach_testkit() {
    validate_production_dependencies(&graph()).unwrap();
    for kind in [Value::Null, json!("build")] {
        let mut value = graph();
        value["packages"][0]["dependencies"] =
            json!([{"name":"intermediate","kind":kind,"optional":true,"target":"cfg(windows)"}]);
        assert!(validate_production_dependencies(&value).is_err());
    }
    for malformed in [
        json!({}),
        json!({"packages":[]}),
        json!({"packages":[{"name":"same"},{"name":"same"}]}),
    ] {
        assert!(validate_production_dependencies(&malformed).is_err());
    }
    let mut value = graph();
    value["packages"][0]["dependencies"][0]["kind"] = json!("future-kind");
    assert!(validate_production_dependencies(&value).is_err());
    let mut value = graph();
    value["packages"][1]["dependencies"] = json!([{"name":"hydracache-client-hc2","kind":null}]);
    validate_production_dependencies(&value).unwrap();
}

#[test]
fn native_java_verifier_checks_raw_results_events_profile_and_artifact_not_a_pass_flag() {
    validate_java_probe(&java()).unwrap();
    for field in 0..9 {
        let mut changed = java();
        match field {
            0 => changed.schema = "fixture".into(),
            1 => changed.profile = "S2".into(),
            2 => changed.artifact_sha256 = "a".repeat(64),
            3 => {
                changed.steps.pop();
            }
            4 => changed.steps[3].kind = "replaced".into(),
            5 => changed.steps[2].value = Some("2a".into()),
            6 => {
                changed.native_events.pop();
            }
            7 => changed.snapshot_digest = "a".repeat(64),
            _ => changed.operation_count = 39,
        }
        assert!(validate_java_probe(&changed).is_err(), "{field}");
    }
    let mut value = serde_json::to_value(java()).unwrap();
    value["passed"] = json!(true);
    assert!(serde_json::from_value::<JavaProbe>(value).is_err());
    for field in 0..4 {
        let mut changed = java();
        match field {
            0 => changed.diagnostics.scope = "S2".into(),
            1 => changed.diagnostics.samples_ns.clear(),
            2 => changed.diagnostics.heap_used_bytes = 4096,
            _ => changed.diagnostics.threads = 0,
        }
        assert!(validate_java_probe(&changed).is_err());
    }
}

#[test]
fn standalone_consumer_cannot_resolve_new_versions_sources_or_checksums() {
    let seed="version=4\n[[package]]\nname='library'\nversion='1.2.3'\nsource='registry+fixture'\nchecksum='accepted'\n";
    let resolved =
        format!("{seed}\n[[package]]\nname='imap-contract-consumer-075'\nversion='0.0.0'\n");
    validate_consumer_lock(seed, &resolved).unwrap();
    for changed in [
        resolved.replace("1.2.3", "1.2.4"),
        resolved.replace("registry+fixture", "registry+foreign"),
        resolved.replace("accepted", "changed"),
        resolved.replace("version=4", "version=5"),
    ] {
        assert!(validate_consumer_lock(seed, &changed).is_err());
    }
    assert!(validate_consumer_lock(seed, "version=4").is_err());
}

#[test]
fn preparation_header_cannot_be_promoted_to_production_or_mixed_source_evidence() {
    let a = Artifact {
        path: "target/fixture".into(),
        bytes: 1,
        sha256: "a".repeat(64),
    };
    let receipt = PreparationReceipt {
        schema: "hydracache.imap.pre074-preparation.v4".into(),
        release: "0.75".into(),
        scope: "pre074_preparation".into(),
        source_sha: "a".repeat(40),
        production_enabled: false,
        tooling: a.clone(),
        runtime: PreparationRuntime {
            rust_fingerprint: "rustc 1.94.0 fixture".into(),
            java_fingerprint: "openjdk 17 fixture".into(),
            java_launcher_sha256: "a".repeat(64),
        },
        reproduced_packages: ["java-imap-benchmark", "java-imap-hazelcast-075"]
            .into_iter()
            .map(
                |component| xtask::imap_package_reproduction_075::JavaPackageProof {
                    component: component.into(),
                    sha256: "a".repeat(64),
                },
            )
            .collect(),
        inputs: vec![a.clone(); 11],
        daemon: a.clone(),
        storage: a.clone(),
        java: a.clone(),
        native_extended: a.clone(),
        native_histories: a.clone(),
        native_recovery: a.clone(),
        rust: a.clone(),
        workspace_metadata: a,
        blocked_until_074: contract().blocked_until_074,
    };
    validate_receipt_header(&receipt, &"a".repeat(40)).unwrap();
    let schema: Value = serde_json::from_str(include_str!(
        "../../../docs/testing/imap/0.75/schemas/pre074-preparation.schema.json"
    ))
    .unwrap();
    jsonschema::validator_for(&schema)
        .unwrap()
        .validate(&serde_json::to_value(&receipt).unwrap())
        .unwrap();
    for field in 0..11 {
        let mut changed = receipt.clone();
        match field {
            0 => changed.production_enabled = true,
            1 => changed.scope = "production".into(),
            2 => changed.source_sha = "b".repeat(40),
            3 => changed.schema = "future".into(),
            4 => changed.release = "0.74".into(),
            5 => {
                changed.inputs.pop();
            }
            6 => changed.blocked_until_074.clear(),
            7 => changed.source_sha = "invalid".into(),
            8 => changed.runtime.rust_fingerprint.clear(),
            9 => changed.runtime.java_fingerprint.clear(),
            _ => changed.runtime.java_launcher_sha256 = "invalid".into(),
        }
        assert!(validate_receipt_header(&changed, &"a".repeat(40)).is_err());
    }
    let mut value = serde_json::to_value(receipt).unwrap();
    value["fixture"] = json!(true);
    assert!(serde_json::from_value::<PreparationReceipt>(value).is_err());
}

#[test]
fn preparation_cli_refuses_ambiguous_or_weakened_modes_before_execution() {
    for mode in ["tools", "produce", "verify", "unknown"] {
        for args in [
            vec![],
            vec!["--release", "0.74"],
            vec!["--release", "0.75", "--skip"],
            vec![
                "--release",
                "0.75",
                "--output",
                "target/a",
                "--receipt",
                "target/b",
            ],
        ] {
            assert!(run(args.into_iter().map(String::from).collect(), mode).is_err());
        }
    }
}
