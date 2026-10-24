#[path = "../../../tools/authority-runtime-074/io.rs"]
mod bound_io;
use bound_io::owned_case_view as contract;
#[path = "../../../tools/authority-runtime-074/gate.rs"]
mod runtime;
#[path = "../../../tools/authority-storage-source-074/contract.rs"]
mod storage_contract;
#[test]
fn owned_case_bootstrap_reader_changes_one_read_and_keeps_every_assertion() {
    let path = "crates/xtask/tests/bootstrap_installed_origin_074.rs";
    let old = runtime::frozen(contract::BASE, path);
    let current = std::fs::read_to_string(runtime::root().join(path)).unwrap();
    let imports = "#[path = \"../../../tools/authority-runtime-074/gate.rs\"]\nmod runtime;\n#[path = \"../../../tools/authority-runtime-074/io.rs\"]\nmod runtime_io;\n#[path = \"../../../tools/authority-storage-source-074/contract.rs\"]\nmod storage_contract;\n";
    let before = "fn read(path: &str) -> String {";
    assert_eq!(old.matches(before).count(), 1);
    let after = "fn read(path: &str) -> String {\n    if runtime_io::owned_case_view::PATHS[6..].contains(&path) {\n        return runtime_io::read_to_string(root().join(path)).unwrap();\n    }";
    let expected = format!("{imports}{}", old.replacen(before, after, 1));
    assert!(storage_contract::accepts(&expected, &current));
    assert_ne!(old, current.replace("\r\n", "\n"));
    for changed in [
        current.replace(
            "if runtime_io::owned_case_view::PATHS[6..].contains(&path)",
            "if true",
        ),
        format!("{current}\nfn unrelated_effect() {{}}"),
    ] {
        assert!(!storage_contract::accepts(&expected, &changed));
    }
}
#[test]
fn owned_case_exact_current_additions_preserve_every_old_body_and_reader() {
    contract::require_helpers(&runtime::root());
    for path in contract::PATHS {
        let original = runtime::frozen(contract::BASE, path);
        let current = std::fs::read_to_string(runtime::root().join(path)).unwrap();
        assert!(contract::accepts(path, &original, &current), "{path}");
        for changed in [
            String::new(),
            format!("{current}\nfn unrelated_change() {{}}"),
            current.replacen("std::fs::read_to_string", "panic!", 1),
        ] {
            if changed != current {
                assert!(!contract::accepts(path, &original, &changed));
            }
        }
        assert!(std::panic::catch_unwind(|| contract::candidate(
            path,
            &format!("{original}\n// predecessor drift")
        ))
        .is_err());
    }
    assert!(contract::candidate("unknown.rs", "anything").is_none());
    let path = "tools/long-run-supervisor-074/tests/held_launcher.rs";
    let source = std::fs::read_to_string(runtime::root().join(path)).unwrap();
    let expected = runtime::frozen(contract::BASE, path);
    assert_ne!(source.replace("\r\n", "\n"), expected);
    assert_eq!(
        bound_io::read_to_string(runtime::root().join(path)).unwrap(),
        runtime::frozen(bound_io::reviewed_inputs::PREDECESSOR, path)
    );
}
#[test]
fn owned_case_whole_helper_pins_reject_mutation_before_any_compatibility_view() {
    for (path, _) in contract::HELPERS {
        let raw = std::fs::read_to_string(runtime::root().join(path)).unwrap();
        assert!(contract::helper_matches(path, &raw));
        for changed in [
            String::new(),
            format!("{raw}\n// drift"),
            raw.chars().skip(1).collect(),
        ] {
            assert!(!contract::helper_matches(path, &changed));
        }
    }
    assert!(!contract::helper_matches("unknown", ""));
}
