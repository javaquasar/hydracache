//! Historical IO compatibility, not production admission or a body exclusion.
#[path = "../../../tools/authority-runtime-074/gate.rs"]
mod runtime;
#[path = "../../../tools/authority-runtime-074/io.rs"]
mod runtime_io;
#[path = "../../../tools/authority-storage-source-074/contract.rs"]
mod storage_contract;

const BASE: &str = "b0e7fcac2ef33974df1633ada55457478c624498";
const FILE: &str = "crates/xtask/tests/custody_preparation_cost_074.rs";

#[test]
fn custody_reader_preserves_entire_old_test_except_exact_gateway_io() {
    let old = runtime::frozen(BASE, FILE);
    let hook = "#[path = \"support/custody_cost_guard.rs\"]\nmod guard;\n";
    let imports = "#[path = \"../../../tools/authority-runtime-074/gate.rs\"]\nmod runtime;\n#[path = \"../../../tools/authority-runtime-074/io.rs\"]\nmod runtime_io;\n#[path = \"../../../tools/authority-storage-source-074/contract.rs\"]\nmod storage_contract;\n";
    let before = "    std::fs::read_to_string(root().join(path))";
    let after = "    runtime_io::read_to_string(root().join(path))";
    assert_eq!(old.matches(hook).count(), 1);
    assert_eq!(old.matches(before).count(), 1);
    let expected = old
        .replacen(hook, &format!("{hook}{imports}"), 1)
        .replacen(before, after, 1);
    let current = std::fs::read_to_string(runtime::root().join(FILE)).unwrap();
    assert!(storage_contract::accepts(&expected, &current));
    for bad in [
        current.replace("assert_eq!(paths.lines().count(), 469);", ""),
        current.replace("guard::normalize(path, &read(path))", "String::new()"),
        current.replace(".join(path)", ".join(\"another-file\")"),
        current.replace(
            "git(&[\"show\", &format!(\"{BASE}:{path}\")])",
            "String::new()",
        ),
        format!("{current}\nfn unrelated_effect() {{}}"),
    ] {
        assert_ne!(bad, current);
        assert!(!storage_contract::accepts(&expected, &bad));
    }
}

#[test]
fn custody_launcher_view_requires_exact_reviewed_current_bytes() {
    let path = "tools/long-run-supervisor-074/src/bin/diagnostic_held_launcher.rs";
    let raw = std::fs::read_to_string(runtime::root().join(path)).unwrap();
    assert_eq!(raw.replace("\r\n", "\n"), runtime::frozen(BASE, path));
    assert!(runtime_io::reviewed_inputs::accepts(path, &raw));
    assert_eq!(
        runtime_io::read_to_string(runtime::root().join(path)).unwrap(),
        runtime::frozen(runtime_io::reviewed_inputs::PREDECESSOR, path)
    );
    for bad in [
        String::new(),
        format!("{raw}\nfn unrelated_effect() {{}}"),
        raw.replace("std::process::exit(2)", "std::process::exit(0)"),
    ] {
        assert_ne!(bad, raw);
        assert!(!runtime_io::reviewed_inputs::accepts(path, &bad));
        assert!(
            std::panic::catch_unwind(|| runtime_io::reviewed_inputs::predecessor(path, &bad))
                .is_err()
        );
    }
    assert!(!runtime_io::reviewed_inputs::accepts("unrelated.rs", &raw));
}
