//! Exact PID preservation reader delta; old assertions and normalizers stay intact.
#[path = "../../../tools/authority-runtime-074/gate.rs"]
mod runtime;
#[path = "../../../tools/authority-storage-source-074/contract.rs"]
mod storage_contract;

const BASE: &str = "54401528c6f71f4c844b47ef1e4a3ff7bd5b57b1";
const FILE: &str = "crates/xtask/tests/pid_path_cost_074.rs";

#[test]
fn pid_path_reader_preserves_all_old_assertions_and_normalizer_order() {
    let old = runtime::frozen(BASE, FILE);
    let hook = "#[path = \"support/pid_path_guard.rs\"]\nmod guard;\n";
    let imports = "#[path = \"../../../tools/authority-runtime-074/gate.rs\"]\nmod runtime;\n#[path = \"../../../tools/authority-runtime-074/io.rs\"]\nmod runtime_io;\n#[path = \"../../../tools/authority-storage-source-074/contract.rs\"]\nmod storage_contract;\n";
    let before = "    let raw = std::fs::read_to_string(\n";
    let after = "    let loader = if path == runtime::status_fields::SOURCE {\n        runtime_io::read_to_string\n    } else {\n        std::fs::read_to_string\n    };\n    let raw = loader(\n";
    assert_eq!(old.matches(hook).count(), 1);
    assert_eq!(old.matches(before).count(), 1);
    let expected = old
        .replacen(hook, &format!("{hook}{imports}"), 1)
        .replacen(before, after, 1);
    let current = std::fs::read_to_string(runtime::root().join(FILE)).unwrap();
    assert!(storage_contract::accepts(&expected, &current));
    for bad in [
        current.replace("path == runtime::status_fields::SOURCE", "true"),
        current.replace(".join(path),", ".join(\"another-file\"),"),
        current.replace(
            "guard::custody_cost::document_scratch::normalize(path, &raw)",
            "raw",
        ),
        current.replace("old(BASE, path)", "String::new()"),
        format!("{current}\nfn unrelated_effect() {{}}"),
    ] {
        assert_ne!(bad, current);
        assert!(!storage_contract::accepts(&expected, &bad));
    }
}
