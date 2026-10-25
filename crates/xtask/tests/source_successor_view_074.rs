//! Source-preservation compatibility only, not runtime or release admission.
#[path = "../../../tools/authority-runtime-074/gate.rs"]
mod runtime;
#[path = "../../../tools/authority-runtime-074/io.rs"]
mod runtime_io;
#[path = "../../../tools/authority-storage-source-074/contract.rs"]
mod storage_contract;

const CASES: [(&str, &str); 5] = [
    ("docs/COMPAT.md", "3c52c0aa34b8d00b2fac44375d5ba8fdf532bb9f"),
    (
        "docs/testing/gated-test-registry.toml",
        "3c52c0aa34b8d00b2fac44375d5ba8fdf532bb9f",
    ),
    (
        "tools/long-run-supervisor-074/src/diagnostic_authority_fixed.rs",
        "c91068bfa8b297bb075b63457da235aaa9c6232e",
    ),
    (
        "tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs",
        "c91068bfa8b297bb075b63457da235aaa9c6232e",
    ),
    (
        "tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs",
        "c91068bfa8b297bb075b63457da235aaa9c6232e",
    ),
];

#[test]
fn exact_successor_inputs_reach_unchanged_historical_chain() {
    for (path, revision) in CASES {
        let old = runtime::frozen(revision, path);
        let expected = if runtime_io::reviewed_inputs::PATHS.contains(&path) {
            assert!(runtime_io::reviewed_inputs::accepts(path, &old));
            runtime::frozen(runtime_io::reviewed_inputs::PREDECESSOR, path)
        } else if runtime::expected(path).is_some() {
            assert!(runtime::accepts(path, &old));
            runtime::frozen(runtime::BASE, path)
        } else {
            old
        };
        assert_eq!(
            runtime_io::read_to_string(runtime::root().join(path)).unwrap(),
            expected,
            "{path}"
        );
    }
}

fn raw(path: &str) -> String {
    std::fs::read_to_string(runtime::root().join(path))
        .unwrap()
        .replace("\r\n", "\n")
}

#[test]
fn whole_body_projection_matches_independent_fixed_producers() {
    assert_eq!(runtime::successor::PATHS, CASES.map(|(path, _)| path));
    for (index, (path, previous)) in CASES.into_iter().enumerate() {
        let current = raw(path);
        let producer = if index == 0 {
            "a67a5299ab335424e9b11330dc06bd749ea9b859"
        } else {
            "958b3135a15c00b9f6aad41522cef9d1f6608d3a"
        };
        assert_eq!(current, runtime::frozen(producer, path));
        let old = runtime::frozen(previous, path);
        assert_eq!(runtime::successor::project(path, current.clone()), old);
        assert_eq!(
            runtime::successor::project(path, current.replace('\n', "\r\n")),
            old
        );
    }
}

#[test]
fn stale_partial_duplicate_and_unrelated_edits_refuse_before_projection() {
    let rules: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tools/source-successor-view-074/rules.json"
    ))
    .unwrap();
    for (path, previous) in CASES {
        let current = raw(path);
        let row = rules["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["path"] == path)
            .unwrap();
        let mut bad = vec![
            String::new(),
            runtime::frozen(previous, path),
            format!("{current}\n// unreviewed side effect\n"),
        ];
        for patch in row["patches"].as_array().unwrap() {
            let at = patch["offset"].as_u64().unwrap() as usize;
            let exact = patch["current"].as_str().unwrap();
            let old = patch["previous"].as_str().unwrap();
            let mut missing = current.clone();
            missing.replace_range(at..at + exact.len(), old);
            bad.push(missing);
            let mut duplicate = current.clone();
            duplicate.insert_str(at, if exact.is_empty() { old } else { exact });
            bad.push(duplicate);
        }
        for bad in bad {
            assert_ne!(bad, current, "real mutation {path}");
            assert!(
                std::panic::catch_unwind(|| runtime::successor::project(path, bad)).is_err(),
                "{path}"
            );
        }
        // No successful observation is cached as current state.
        assert!(std::panic::catch_unwind(|| runtime::successor::project(
            path,
            format!("{current}x")
        ))
        .is_err());
    }
}

#[test]
fn wrong_path_cannot_borrow_another_files_approval_and_unknowns_stay_unchanged() {
    for (path, _) in CASES {
        for (other, _) in CASES {
            if path != other {
                let bytes = raw(other);
                assert!(
                    std::panic::catch_unwind(|| runtime::successor::project(path, bytes)).is_err()
                );
            }
        }
    }
    for path in ["Cargo.lock", "../docs/COMPAT.md", "unrelated.rs"] {
        let bytes = "arbitrary\r\nuntouched input".to_owned();
        assert_eq!(runtime::successor::project(path, bytes.clone()), bytes);
    }
}

#[test]
fn old_gates_and_assertions_survive_exact_entry_edits() {
    const BASE: &str = "87366d7490c7962c3ec256cbc8b88460ab57073f";
    let gate = "tools/authority-runtime-074/gate.rs";
    let old = runtime::frozen(BASE, gate);
    let after = old.replacen("use std::path::{Path, PathBuf};", "use std::path::{Path, PathBuf};\n#[path = \"../source-successor-view-074/contract.rs\"]\npub mod successor;", 1)
        .replacen("pub fn view(path: &str, raw: String) -> String {", "pub fn view(path: &str, raw: String) -> String {\n    let raw = successor::project(path, raw);", 1);
    assert_eq!(raw(gate), after);
    let path = "tools/historical-input-view-074/contract.rs";
    let old = runtime::frozen(BASE, path);
    let after = old.replacen("pub fn predecessor(path: &str, raw: &str) -> String {", "pub fn predecessor(path: &str, raw: &str) -> String {\n    let raw = crate::runtime::successor::project(path, raw.to_owned());", 1)
        .replacen("accepts(path, raw),", "accepts(path, &raw),", 1);
    assert_eq!(raw(path), after);
    for path in [
        "tools/authority-runtime-074/io.rs",
        "tools/owned-case-074/contract.rs",
        "crates/xtask/tests/diagnostic_artifact_fixture_074.rs",
        "crates/xtask/tests/artifact_inventory_reader_074.rs",
    ] {
        assert_eq!(raw(path), runtime::frozen(BASE, path), "unchanged {path}");
    }
}
