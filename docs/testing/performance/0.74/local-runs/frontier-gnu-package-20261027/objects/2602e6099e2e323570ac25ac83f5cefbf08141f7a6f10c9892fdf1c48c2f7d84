//! Exact test-reader edits only; no production enrollment or execution permit.
pub const BASE: &str = "a329cac91784d79d9e5b925c22d882cb0d327283";
pub const FILES: [&str; 3] = [
    "crates/xtask/tests/remaining_preparation_cost_074.rs",
    "crates/xtask/tests/validation_round_cost_074.rs",
    "crates/xtask/tests/validation_leaf_requests_074.rs",
];
const IMPORTS: &str = "#[path = \"../../../tools/authority-runtime-074/gate.rs\"]\nmod runtime;\n#[path = \"../../../tools/authority-runtime-074/io.rs\"]\nmod runtime_io;\n#[path = \"../../../tools/authority-storage-source-074/contract.rs\"]\nmod storage_contract;\n";
fn edit(raw: &str, before: &str, after: &str) -> String {
    assert_eq!(raw.matches(before).count(), 1, "one exact cost reader edit");
    raw.replacen(before, after, 1)
}
pub fn expected(path: &str) -> Option<String> {
    let index = FILES.iter().position(|file| *file == path)?;
    let old = crate::runtime::frozen(BASE, path);
    let hook = "#[path = \"support/document_scratch_guard.rs\"]\nmod guard;\n";
    let mut current = edit(&old, hook, &format!("{hook}{IMPORTS}"));
    current = match index {
        0 => edit(&current,
            "    let read = |path: &str| {\n        std::fs::read_to_string(root().join(path))\n            .unwrap()\n            .replace('\\r', \"\")\n    };",
            "    let read = |path: &str| {\n        let raw = runtime_io::read_to_string(root().join(path))\n            .unwrap()\n            .replace('\\r', \"\");\n        if path == runtime::status_fields::SOURCE {\n            crate::hc074_projection_groups_view!(path, &raw)\n        } else {\n            raw\n        }\n    };"),
        1 => {
            let current = edit(&current, IMPORTS, &format!("#[path = \"support/artifact_inventory_input.rs\"]\nmod inventory;\n{IMPORTS}"));
            let current = edit(&current,
                "        let raw = std::fs::read_to_string(root().join(path))\n            .unwrap()\n            .replace('\\r', \"\");",
                "        let raw = runtime_io::read_to_string(root().join(path))\n            .unwrap()\n            .replace('\\r', \"\");\n        let raw = if path == runtime::status_fields::SOURCE {\n            crate::hc074_projection_groups_view!(path, &raw)\n        } else {\n            raw\n        };" );
            edit(&current,
                "    String::from_utf8(output.stdout).unwrap().replace('\\r', \"\")\n}",
                "    let raw = String::from_utf8(output.stdout).unwrap().replace('\\r', \"\");\n    if args == [\"ls-files\", inventory::SCOPE] {\n        inventory::project(&raw).expect(\"current inventory refused before historical phase\")\n    } else {\n        raw\n    }\n}")
        }
        2 => edit(&current,
            "fn read(path: &str) -> String {",
            "fn read(path: &str) -> String {\n    if path == \"docs/testing/gated-test-registry.toml\" {\n        return runtime_io::read_to_string(runtime::root().join(path)).unwrap();\n    }"),
        _ => unreachable!(),
    };
    Some(current)
}
pub fn accepts(path: &str, raw: &str) -> bool {
    expected(path).is_some_and(|expected| raw.replace("\r\n", "\n") == expected)
}
pub fn previous(path: &str, raw: &str) -> String {
    assert!(accepts(path, raw), "unreviewed cost test reader: {path}");
    crate::runtime::frozen(crate::runtime::BASE, path)
}
