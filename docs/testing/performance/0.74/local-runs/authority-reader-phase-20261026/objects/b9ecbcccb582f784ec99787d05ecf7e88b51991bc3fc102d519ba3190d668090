//! Exact historical test input only; never a runtime or authorization adapter.
#[path = "../../../tools/authority-runtime-074/gate.rs"]
mod runtime;
#[path = "../../../tools/authority-runtime-074/io.rs"]
mod runtime_io;
#[path = "../../../tools/authority-storage-source-074/contract.rs"]
mod storage_contract;

const BASE: &str = "1dc9f08b5812ffdf02d87f29e687c0c726bc962a";
const FILE: &str = "crates/xtask/tests/authority_key_hex_074.rs";
const REGISTRY: &str = "docs/testing/gated-test-registry.toml";

fn expected_reader() -> String {
    let old = runtime::frozen(BASE, FILE);
    assert_eq!(
        old,
        runtime::frozen("cd6c69312816f4129871e8a90570f0bda3a91603", FILE)
    );
    let guard = "#[path = \"support/document_scratch_guard.rs\"]\nmod guard;\n";
    let imports = "#[path = \"../../../tools/authority-runtime-074/gate.rs\"]\nmod runtime;\n#[path = \"../../../tools/authority-runtime-074/io.rs\"]\nmod runtime_io;\n#[path = \"../../../tools/authority-storage-source-074/contract.rs\"]\nmod storage_contract;\n";
    let before = "fn read(path: &str) -> String {";
    let after = "fn read(path: &str) -> String {\n    if path == \"docs/testing/gated-test-registry.toml\" {\n        return runtime_io::read_to_string(\n            std::path::Path::new(env!(\"CARGO_MANIFEST_DIR\"))\n                .join(\"../..\")\n                .join(path),\n        )\n        .unwrap();\n    }";
    assert_eq!(old.matches(guard).count(), 1);
    assert_eq!(old.matches(before).count(), 1);
    old.replacen(guard, &format!("{guard}{imports}"), 1)
        .replacen(before, after, 1)
}

#[test]
fn hex_reader_changes_only_registry_input_and_preserves_every_old_assertion() {
    let current = std::fs::read_to_string(runtime::root().join(FILE)).unwrap();
    let expected = expected_reader();
    assert!(storage_contract::accepts(&expected, &current));
    for bad in [
        current.replace(
            "if path == \"docs/testing/gated-test-registry.toml\"",
            "if true",
        ),
        current.replace(".join(path),", ".join(\"another-file\"),"),
        format!("{current}\nfn unrelated_effect() {{}}"),
        current.replacen(
            "assert_eq!(restored, before);",
            "assert_ne!(restored, before);",
            1,
        ),
    ] {
        assert_ne!(bad, current);
        assert!(!storage_contract::accepts(&expected, &bad));
    }
}

#[test]
fn hex_registry_view_requires_exact_current_bytes_before_the_old_view() {
    let raw = std::fs::read_to_string(runtime::root().join(REGISTRY)).unwrap();
    assert_eq!(raw.replace("\r\n", "\n"), runtime::frozen(BASE, REGISTRY));
    assert!(runtime_io::reviewed_inputs::accepts(REGISTRY, &raw));
    assert_eq!(
        runtime_io::read_to_string(runtime::root().join(REGISTRY)).unwrap(),
        runtime::frozen(runtime_io::reviewed_inputs::PREDECESSOR, REGISTRY)
    );
    for bad in [
        String::new(),
        format!("{raw}\n# unreviewed gate"),
        raw.replacen("schema_version = 1", "schema_version = 2", 1),
    ] {
        assert!(!runtime_io::reviewed_inputs::accepts(REGISTRY, &bad));
        assert!(
            std::panic::catch_unwind(|| runtime_io::reviewed_inputs::predecessor(REGISTRY, &bad))
                .is_err()
        );
    }
}
