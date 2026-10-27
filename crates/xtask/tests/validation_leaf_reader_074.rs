//! Registry compatibility only; no changed measurement or runtime predicate.
#[path = "../../../tools/package-closure-inputs-074/contract.rs"]
mod closure_inputs;
#[path = "../../../tools/authority-runtime-074/gate.rs"]
mod runtime;
#[path = "../../../tools/authority-runtime-074/io.rs"]
mod runtime_io;
#[path = "../../../tools/authority-storage-source-074/contract.rs"]
mod storage_contract;

const BASE: &str = "a329cac91784d79d9e5b925c22d882cb0d327283";
const FILE: &str = "crates/xtask/tests/validation_leaf_requests_074.rs";
const REGISTRY: &str = "docs/testing/gated-test-registry.toml";

fn expected() -> String {
    let old = runtime::frozen(BASE, FILE);
    let hook = "#[path = \"support/document_scratch_guard.rs\"]\nmod guard;\n";
    let imports = "#[path = \"../../../tools/authority-runtime-074/gate.rs\"]\nmod runtime;\n#[path = \"../../../tools/authority-runtime-074/io.rs\"]\nmod runtime_io;\n#[path = \"../../../tools/authority-storage-source-074/contract.rs\"]\nmod storage_contract;\n";
    let before = "fn read(path: &str) -> String {";
    let after = "fn read(path: &str) -> String {\n    if path == \"docs/testing/gated-test-registry.toml\" {\n        return runtime_io::read_to_string(runtime::root().join(path)).unwrap();\n    }";
    assert_eq!(old.matches(hook).count(), 1);
    assert_eq!(old.matches(before).count(), 1);
    old.replacen(hook, &format!("{hook}{imports}"), 1)
        .replacen(before, after, 1)
}

#[test]
fn leaf_reader_preserves_entire_original_test_and_exact_registry_selection() {
    let raw = std::fs::read_to_string(runtime::root().join(FILE)).unwrap();
    let expected = expected();
    assert!(storage_contract::accepts(&expected, &raw));
    for bad in [
        raw.replace(
            "if path == \"docs/testing/gated-test-registry.toml\"",
            "if true",
        ),
        raw.replace(
            "runtime::root().join(path)",
            "runtime::root().join(\"other\")",
        ),
        raw.replacen("assert_eq!(restored, before);", "", 1),
        raw.replacen("timeout_seconds = 1200", "timeout_seconds = 2400", 1),
        format!("{raw}\nfn unrelated_effect() {{}}"),
    ] {
        assert_ne!(bad, raw);
        assert!(!storage_contract::accepts(&expected, &bad));
    }
}

#[test]
fn leaf_registry_input_checks_current_bytes_before_returning_predecessor() {
    let raw = std::fs::read_to_string(runtime::root().join(REGISTRY)).unwrap();
    let raw = closure_inputs::project("leaf", REGISTRY, raw);
    assert_eq!(raw.replace("\r\n", "\n"), runtime::frozen(BASE, REGISTRY));
    assert!(runtime_io::reviewed_inputs::accepts(REGISTRY, &raw));
    assert_eq!(
        runtime_io::read_to_string(runtime::root().join(REGISTRY)).unwrap(),
        runtime::frozen(runtime_io::reviewed_inputs::PREDECESSOR, REGISTRY)
    );
    for bad in [
        String::new(),
        format!("{raw}\n# unknown registration"),
        raw.replacen("timeout_seconds = 180", "timeout_seconds = 181", 1),
    ] {
        assert_ne!(bad, raw);
        assert!(!runtime_io::reviewed_inputs::accepts(REGISTRY, &bad));
        assert!(
            std::panic::catch_unwind(|| runtime_io::reviewed_inputs::predecessor(REGISTRY, &bad))
                .is_err()
        );
    }
}
