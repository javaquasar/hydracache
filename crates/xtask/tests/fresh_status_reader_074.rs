//! Test input compatibility only; no live validation or runtime substitution.
#[path = "../../../tools/authority-runtime-074/gate.rs"]
mod runtime;
#[path = "../../../tools/authority-runtime-074/io.rs"]
mod runtime_io;
#[path = "../../../tools/authority-storage-source-074/contract.rs"]
mod storage_contract;

const BASE: &str = "f4ca503c5d640d55d2118937aa5c7128fc5b3a62";
const FILE: &str = "crates/xtask/tests/fresh_kernel_cost_074.rs";

fn expected_reader() -> String {
    let old = runtime::frozen(BASE, FILE);
    let hook = "#[path = \"support/fresh_status_guard.rs\"]\nmod guard;\n";
    let imports = "#[path = \"../../../tools/authority-runtime-074/gate.rs\"]\nmod runtime;\n#[path = \"../../../tools/authority-runtime-074/io.rs\"]\nmod runtime_io;\n#[path = \"../../../tools/authority-storage-source-074/contract.rs\"]\nmod storage_contract;\n";
    let before = "    let raw = std::fs::read_to_string(\n";
    let after = "    let loader = if path == runtime::status_fields::SOURCE {\n        runtime_io::read_to_string\n    } else {\n        std::fs::read_to_string\n    };\n    let raw = loader(\n";
    assert_eq!(old.matches(hook).count(), 1);
    assert_eq!(old.matches(before).count(), 1);
    old.replacen(hook, &format!("{hook}{imports}"), 1)
        .replacen(before, after, 1)
}

#[test]
fn fresh_status_reader_changes_only_admission_input_before_original_normalizers() {
    let current = std::fs::read_to_string(runtime::root().join(FILE)).unwrap();
    let expected = expected_reader();
    assert!(storage_contract::accepts(&expected, &current));
    for bad in [
        current.replace("path == runtime::status_fields::SOURCE", "true"),
        current.replace(".join(path),", ".join(\"another-file\"),"),
        current.replace(
            "guard::pid_path::custody_cost::document_scratch::normalize(path, &raw)",
            "raw",
        ),
        current.replace("old(path), \"{path}\"", "String::new(), \"{path}\""),
        format!("{current}\nfn unrelated_effect() {{}}"),
    ] {
        assert_ne!(bad, current);
        assert!(!storage_contract::accepts(&expected, &bad));
    }
}

#[test]
fn fresh_status_admission_requires_exact_source_and_helper_before_old_input() {
    let path = runtime::status_fields::SOURCE;
    let raw = std::fs::read_to_string(runtime::root().join(path)).unwrap();
    assert_eq!(raw.replace("\r\n", "\n"), runtime::frozen(BASE, path));
    assert!(runtime::accepts(path, &raw));
    assert_eq!(
        runtime_io::read_to_string(runtime::root().join(path)).unwrap(),
        runtime::frozen(runtime::BASE, path)
    );
    for bad in [
        String::new(),
        format!("{raw}\nfn unrelated_effect() {{}}"),
        raw.replace("origin.revalidate()?;", "/* removed live check */"),
        raw.replace("60_000_000_000", "600_000_000_000"),
    ] {
        assert_ne!(bad, raw);
        assert!(!runtime::accepts(path, &bad));
        assert!(std::panic::catch_unwind(|| runtime::view(path, bad)).is_err());
    }
    assert!(!runtime::accepts("unrelated.rs", &raw));
    let helper =
        std::fs::read_to_string(runtime::root().join(runtime::status_fields::HELPER)).unwrap();
    assert!(runtime::status_fields::helper_is_pinned(&helper));
    assert!(!runtime::status_fields::helper_is_pinned(&format!(
        "{helper}\n// unreviewed helper"
    )));
}
