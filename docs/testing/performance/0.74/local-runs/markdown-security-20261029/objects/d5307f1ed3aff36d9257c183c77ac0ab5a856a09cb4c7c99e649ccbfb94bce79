//! Exact historical test IO; unknown current source must never be hidden.
#[path = "support/document_scratch_inputs.rs"]
mod inputs;
#[path = "../../../tools/authority-runtime-074/gate.rs"]
mod runtime;
#[path = "../../../tools/authority-runtime-074/io.rs"]
mod runtime_io;
#[path = "../../../tools/authority-storage-source-074/contract.rs"]
mod storage_contract;

const BASE: &str = "edd2c8953f8dd5572f96d436c17259dfc49f3313";
const FILE: &str = "crates/xtask/tests/document_scratch_cost_074.rs";

fn replace(raw: &str, old: &str, new: &str) -> String {
    assert_eq!(raw.matches(old).count(), 1, "one exact reader delta");
    raw.replacen(old, new, 1)
}

#[test]
fn scratch_reader_preserves_every_original_assertion_and_both_old_counts() {
    let old = runtime::frozen(BASE, FILE);
    let hook = "#[path = \"support/document_scratch_guard.rs\"]\nmod guard;\n";
    let imports = "#[path = \"support/document_scratch_inputs.rs\"]\nmod inputs;\n#[path = \"../../../tools/authority-runtime-074/gate.rs\"]\nmod runtime;\n#[path = \"../../../tools/authority-runtime-074/io.rs\"]\nmod runtime_io;\n#[path = \"../../../tools/authority-storage-source-074/contract.rs\"]\nmod storage_contract;\n";
    let before = "    std::fs::read_to_string(root().join(path))\n        .unwrap()\n        .replace('\\r', \"\")";
    let after = "    let raw = inputs::read(path);\n    if path == guard::ORIGIN {\n        crate::hc074_pidfd_info_path_view!(path, &raw)\n    } else {\n        raw\n    }";
    let expected = replace(
        &replace(&old, hook, &format!("{hook}{imports}")),
        before,
        after,
    );
    let current = std::fs::read_to_string(runtime::root().join(FILE)).unwrap();
    assert!(storage_contract::accepts(&expected, &current));
    for bad in [
        current.replace("if path == guard::ORIGIN", "if true"),
        current.replace("assert_eq!(paths.lines().count(), 473);", ""),
        current.replace("guard::normalize(path, &read(path))", "String::new()"),
        current.replace("current.replacen(&expected, original, 1)", "old.clone()"),
        format!("{current}\nfn unrelated_effect() {{}}"),
    ] {
        assert_ne!(bad, current);
        assert!(!storage_contract::accepts(&expected, &bad));
    }
}

#[test]
fn scratch_five_reader_inputs_differ_only_by_known_io_not_old_assertions() {
    assert_eq!(inputs::PATHS.len(), 5);
    for path in inputs::PATHS {
        let old = runtime::frozen(inputs::PREDECESSOR, path);
        let expected = if path.contains("performance_contract_074_") {
            replace(
                &old,
                "std::fs::read_to_string",
                "crate::runtime_io::read_to_string",
            )
        } else if path.ends_with("held_preparation_cost_074.rs") {
            let old = replace(&old, "#[path = \"support/projection_prefix_guard.rs\"]", "#[path = \"support/authority_history_io.rs\"]\nmod history;\n#[path = \"support/projection_prefix_guard.rs\"]");
            let old = replace(&old, "std::fs::read_to_string", "history::read_to_string");
            replace(
                &old,
                "std::process::Command::new(\"git\")",
                "history::command()",
            )
        } else {
            let guard = if path.ends_with("fresh_kernel_cost_074.rs") {
                "#[path = \"support/fresh_status_guard.rs\"]\nmod guard;\n"
            } else {
                "#[path = \"support/pid_path_guard.rs\"]\nmod guard;\n"
            };
            let imports = "#[path = \"../../../tools/authority-runtime-074/gate.rs\"]\nmod runtime;\n#[path = \"../../../tools/authority-runtime-074/io.rs\"]\nmod runtime_io;\n#[path = \"../../../tools/authority-storage-source-074/contract.rs\"]\nmod storage_contract;\n";
            let old = replace(&old, guard, &format!("{guard}{imports}"));
            replace(&old, "    let raw = std::fs::read_to_string(\n", "    let loader = if path == runtime::status_fields::SOURCE {\n        runtime_io::read_to_string\n    } else {\n        std::fs::read_to_string\n    };\n    let raw = loader(\n")
        };
        assert_eq!(expected, runtime::frozen(inputs::REVIEWED, path));
        let current = std::fs::read_to_string(runtime::root().join(path)).unwrap();
        assert!(inputs::accepts(path, &current));
        assert_eq!(inputs::read(path), old);
        for bad in [
            String::new(),
            format!("{current}\n// unreviewed edit"),
            current.replacen("assert_eq!", "assert_ne!", 1),
        ] {
            assert_ne!(bad, current);
            assert!(!inputs::accepts(path, &bad));
            assert!(std::panic::catch_unwind(|| inputs::predecessor(path, &bad)).is_err());
        }
    }
    assert!(!inputs::accepts("unrelated.rs", "anything"));
    assert!(std::panic::catch_unwind(|| inputs::predecessor("unrelated.rs", "anything")).is_err());
    assert_eq!(
        inputs::read(runtime::status_fields::SOURCE),
        runtime::frozen(runtime::BASE, runtime::status_fields::SOURCE)
    );
}
