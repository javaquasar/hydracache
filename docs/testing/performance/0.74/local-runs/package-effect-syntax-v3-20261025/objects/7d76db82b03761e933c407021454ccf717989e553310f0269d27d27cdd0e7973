//! Source identity and historical input checks cannot authorize a process start.
#[path = "support/current_tree_input.rs"]
mod current_inputs;
#[path = "../../../tools/authority-runtime-074/gate.rs"]
mod runtime;
#[path = "../../../tools/authority-runtime-074/io.rs"]
mod runtime_io;
#[path = "../../../tools/authority-storage-source-074/contract.rs"]
mod storage_contract;
use current_inputs::history_reader;

#[test]
fn current_tree_reader_retains_entire_prior_gate_and_unknown_source_assertions() {
    let expected = history_reader::extend(&runtime::frozen(
        current_inputs::REVIEWED,
        runtime::HISTORY_TEST,
    ));
    let actual = std::fs::read_to_string(runtime::root().join(runtime::HISTORY_TEST))
        .unwrap()
        .replace("\r\n", "\n");
    assert_eq!(expected, actual);
    for bad in [
        actual.replace("current_inputs::known_addition(path)", "true"),
        actual.replace("current_inputs::require_addition(path);", ""),
        actual.replace("assert_eq!(count, 79);", "assert_eq!(count, 78);"),
        actual.replace("assert!(history::safe_path(path));", ""),
        format!("{actual}\nfn unrelated_effect() {{}}"),
    ] {
        assert_ne!(bad, actual);
        assert_ne!(bad, expected);
        assert!(std::panic::catch_unwind(|| history_reader::previous(&bad)).is_err());
    }
}

#[test]
fn runtime_contract_reader_changes_only_the_current_history_input() {
    let path = "crates/xtask/tests/authority_runtime_074.rs";
    let old = runtime::frozen(current_inputs::REVIEWED, path);
    let hook = "mod gate;\n";
    let before = "fn read(path: &str) -> String {\n    std::fs::read_to_string(gate::root().join(path))\n        .unwrap()\n        .replace(\"\\r\\n\", \"\\n\")\n}";
    let after = "fn read(path: &str) -> String {\n    let raw = std::fs::read_to_string(gate::root().join(path))\n        .unwrap()\n        .replace(\"\\r\\n\", \"\\n\");\n    if path == gate::HISTORY_TEST {\n        history_reader::previous(&raw)\n    } else {\n        raw\n    }\n}";
    assert_eq!(old.matches(hook).count(), 1);
    assert_eq!(old.matches(before).count(), 1);
    let expected = old.replacen(hook, "mod gate;\n#[path = \"../../../tools/authority-runtime-074/history_reader.rs\"]\nmod history_reader;\n", 1).replacen(before, after, 1);
    let actual = std::fs::read_to_string(runtime::root().join(path))
        .unwrap()
        .replace("\r\n", "\n");
    assert_eq!(actual, expected);
    for bad in [
        actual.replace("if path == gate::HISTORY_TEST", "if true"),
        actual.replace("assert_eq!(tests, count);", "assert_ne!(tests, count);"),
        format!("{actual}\nfn unknown_effect() {{}}"),
    ] {
        assert_ne!(bad, actual);
        assert_ne!(bad, expected);
    }
}

#[test]
fn all_nine_current_inputs_are_exact_declared_patches_not_assertion_waivers() {
    for path in current_inputs::patches().keys() {
        let raw = std::fs::read_to_string(runtime::root().join(path)).unwrap();
        assert!(current_inputs::accepts_patch(path, &raw), "{path}");
        assert_eq!(
            current_inputs::read(path),
            runtime::frozen(current_inputs::SOURCE, path)
        );
        for bad in [
            String::new(),
            format!("{raw}\nfn unknown_effect() {{}}"),
            raw.replacen("assert", "silenced_assert", 1),
        ] {
            assert_ne!(bad, raw);
            assert!(!current_inputs::accepts_patch(path, &bad));
        }
    }
    assert!(!current_inputs::accepts_patch("unrelated.rs", "anything"));
    assert_eq!(
        current_inputs::read(runtime::FIXTURE),
        runtime::frozen(runtime::BASE, runtime::FIXTURE)
    );
}

#[test]
fn all_eighty_one_declared_sources_require_exact_bytes_index_and_regular_files() {
    for (path, rule) in current_inputs::additions() {
        assert!(current_inputs::known_addition(path));
        current_inputs::require_addition(path);
        let raw = std::fs::read_to_string(runtime::root().join(path)).unwrap();
        assert!(current_inputs::accepts_addition(path, &raw));
        for bad in [
            String::new(),
            format!("{raw}\n// unknown source edit"),
            format!("{raw}\nfn unknown_effect() {{}}"),
        ] {
            assert!(!current_inputs::accepts_addition(path, &bad));
        }
        let index = format!(
            "{} {} 0\t{path}\n",
            rule["mode"].as_str().unwrap(),
            rule["blob"].as_str().unwrap()
        );
        assert!(current_inputs::addition_index_matches(path, &index));
        for bad in [
            index.replacen("100644", "100755", 1),
            index.replace(" 0\t", " 1\t"),
            index.replace(rule["blob"].as_str().unwrap(), &"0".repeat(40)),
            format!("{index}{index}"),
        ] {
            assert!(!current_inputs::addition_index_matches(path, &bad));
        }
    }
    for path in [
        "tools/long-run-supervisor-074/src/new.rs",
        "tools/long-run-supervisor-074/tests/new.rs",
        "../escape.rs",
    ] {
        assert!(!current_inputs::known_addition(path));
        assert!(!current_inputs::accepts_addition(path, "anything"));
        assert!(!current_inputs::addition_index_matches(path, "anything"));
    }
}
