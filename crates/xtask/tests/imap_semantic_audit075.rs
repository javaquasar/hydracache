use std::path::Path;
use xtask::imap_semantic_audit075::{check_at_root, check_sources, SemanticSources};

const LEDGER: &str = include_str!("../../../docs/integrations/imap-compatibility-0.75.md");
const OPERATIONS: &str = include_str!("../../../docs/testing/imap/0.75/operation-contract.json");
const BORROWED: &str = include_str!("../../../docs/integrations/hazelcast_borrowed_suite.json");
const ADAPTER: &str = include_str!("../../../tests/java-imap-benchmark/src/main/java/io/hydracache/imap/semantic/UnifiedIMapServiceSemanticAdapter.java");
const TESTS: &str = include_str!("../../../tests/java-imap-benchmark/src/test/java/io/hydracache/imap/semantic/UnifiedIMapServiceSemanticAdapterTest.java");
const SIGNATURES: &str = include_str!("../../../tests/java-imap-benchmark/src/test/java/io/hydracache/imap/semantic/IMapSemanticCompatibilityTest.java");
const JAVA: &str = include_str!("../../../tests/java-imap-benchmark/src/main/java/io/hydracache/imap/semantic/UnifiedIMapService.java");
const RUST: &str = include_str!("../../hydracache-cluster-testkit/src/value_plane_service_075.rs");
fn sources() -> SemanticSources<'static> {
    SemanticSources {
        ledger: LEDGER,
        operations: OPERATIONS,
        borrowed: BORROWED,
        adapter: ADAPTER,
        adapter_tests: TESTS,
        signature_tests: SIGNATURES,
        java_service: JAVA,
        rust_service: RUST,
    }
}

#[test]
fn provisional_ledger_maps_every_core_operation_helper_and_retained_map_expectation() {
    assert!(check_sources(&sources()).unwrap().is_empty());
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    assert!(check_at_root(root, "0.75").unwrap().is_empty());
    assert!(!check_at_root(root, "0.74").unwrap().is_empty());
}

#[test]
fn semantic_guard_rejects_missing_rows_fake_owners_filter_drift_and_abi_overclaim() {
    for target in [
        "row",
        "helper",
        "production",
        "abi",
        "owner",
        "shadow",
        "signature",
        "borrowed",
        "filter",
    ] {
        let mut s = sources();
        let changed;
        match target {
            "row" => {
                changed = LEDGER.replace("| core:get |", "| core:unregistered |");
                s.ledger = &changed;
            }
            "helper" => {
                changed = LEDGER.replace(
                    "| helper:safe_outcome_retirement |",
                    "| helper:unregistered |",
                );
                s.ledger = &changed;
            }
            "production" => {
                changed = LEDGER.replace(
                    "Expanded production capability: disabled.",
                    "Expanded production capability: enabled.",
                );
                s.ledger = &changed;
            }
            "abi" => {
                changed = LEDGER.replace("not a final binary ABI", "a final frozen binary ABI");
                s.ledger = &changed;
            }
            "owner" => {
                changed = ADAPTER.replace("service.execute(command)", "fake.execute(command)");
                s.adapter = &changed;
            }
            "shadow" => {
                changed = format!("{ADAPTER}\nTreeMap<BytesValue, BytesValue> values;");
                s.adapter = &changed;
            }
            "signature" => {
                changed = SIGNATURES.replace("getGenericParameterTypes", "getParameterTypes");
                s.signature_tests = &changed;
            }
            "borrowed" => {
                changed =
                    BORROWED.replace("a9ce2a02ac17f88fcd38869ac698e56e613dc40c", &"b".repeat(40));
                s.borrowed = &changed;
            }
            _ => {
                changed = RUST.replace(
                    "pub exact_key: Option<Vec<u8>>",
                    "pub removed_exact_filter: Option<Vec<u8>>",
                );
                s.rust_service = &changed;
            }
        }
        assert!(!check_sources(&s).unwrap().is_empty(), "{target}");
    }
    let mut s = sources();
    let duplicate = format!("{LEDGER}\n| core:get | duplicate | reference only |\n");
    s.ledger = &duplicate;
    assert!(check_sources(&s).is_err());
    let mut s = sources();
    s.operations = "{";
    assert!(check_sources(&s).is_err());
    let huge = "x".repeat(512 * 1024 + 1);
    let mut s = sources();
    s.adapter = &huge;
    assert!(check_sources(&s).is_err());
    assert!(xtask::imap_semantic_audit075::run(vec!["--release".into(), "0.74".into()]).is_err());
}
