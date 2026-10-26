//! Fixed source accounting cannot revive a rejected runtime or authorize execution.
#[path = "support/current_tree_input.rs"]
mod current_inputs;
#[path = "../../../tools/current-frontier-074/reader.rs"]
mod frontier;
#[path = "../../../tools/authority-runtime-074/gate.rs"]
mod runtime;
#[path = "../../../tools/authority-runtime-074/io.rs"]
mod runtime_io;
#[path = "../../../tools/authority-storage-source-074/contract.rs"]
mod storage_contract;
use frontier::{additions, contract};
const BASE: &str = "bf352a1dfbe3040af9ce058a5d42efe55202db62";
fn raw(path: &str) -> String {
    std::fs::read_to_string(runtime::root().join(path))
        .unwrap()
        .replace("\r\n", "\n")
}
#[test]
fn twelve_current_sources_reconstruct_exact_independent_predecessors() {
    for row in contract::rules()["rows"].as_array().unwrap() {
        let path = row["path"].as_str().unwrap();
        let current = runtime::frozen(row["revision"].as_str().unwrap(), path);
        let previous = runtime::frozen(row["previous_revision"].as_str().unwrap(), path);
        assert_eq!(raw(path), current);
        assert_ne!(current, previous);
        assert_eq!(contract::project(path, current.clone()), previous);
        assert_eq!(
            contract::project(path, current.replace('\n', "\r\n")),
            previous
        );
        assert!(contract::known(path));
    }
}
#[test]
fn stale_partial_duplicate_and_unrelated_current_source_edits_refuse() {
    for row in contract::rules()["rows"].as_array().unwrap() {
        let path = row["path"].as_str().unwrap();
        let current = raw(path);
        let mut mutants = vec![
            String::new(),
            format!("{current}\n// unreviewed"),
            runtime::frozen(row["previous_revision"].as_str().unwrap(), path),
        ];
        for edit in row["patches"].as_array().unwrap() {
            let at = edit["offset"].as_u64().unwrap() as usize;
            let exact = edit["current"].as_str().unwrap();
            let old = edit["previous"].as_str().unwrap();
            let mut partial = current.clone();
            partial.replace_range(at..at + exact.len(), old);
            mutants.push(partial);
            let mut duplicate = current.clone();
            duplicate.insert_str(at, if exact.is_empty() { old } else { exact });
            mutants.push(duplicate);
        }
        assert!(std::panic::catch_unwind(|| contract::project(path, current)).is_ok());
        for mutant in mutants {
            assert_ne!(mutant, raw(path));
            assert!(std::panic::catch_unwind(|| contract::project(path, mutant)).is_err());
        }
    }
}
#[test]
fn all_fourteen_additions_require_independent_bodies_and_exact_index_entries() {
    assert_eq!(additions::rows().len(), 14);
    for (path, row) in additions::rows() {
        assert!(!current_inputs::known_addition(path));
        assert!(frontier::known_addition(path));
        assert!(additions::accepts(path, &raw(path)));
        assert_eq!(raw(path), runtime::frozen(additions::REVISION, path));
        frontier::require_addition(path);
        let index = format!(
            "{} {} 0\t{path}\n",
            row["mode"].as_str().unwrap(),
            row["blob"].as_str().unwrap()
        );
        assert!(additions::index_matches(path, &index));
        for bad in [
            index.replace("100644", "100755"),
            index.replace(" 0\t", " 1\t"),
            index.replace(row["blob"].as_str().unwrap(), &"0".repeat(40)),
            format!("{index}{index}"),
            index.replace(path, "other.rs"),
        ] {
            assert!(!additions::index_matches(path, &bad));
        }
        for bad in [String::new(), format!("{}\n// unknown edit", raw(path))] {
            assert!(!additions::accepts(path, &bad));
        }
    }
}
#[test]
fn successful_addition_validation_does_not_cache_body_or_index_observations() {
    let root = tempfile::tempdir().unwrap();
    let (path, row) = additions::rows().iter().next().unwrap();
    let file = root.path().join(path);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, runtime::frozen(additions::REVISION, path)).unwrap();
    let index = format!(
        "{} {} 0\t{path}\n",
        row["mode"].as_str().unwrap(),
        row["blob"].as_str().unwrap()
    );
    let calls = std::cell::Cell::new(0);
    let observer = || {
        calls.set(calls.get() + 1);
        index.clone()
    };
    additions::require_with(root.path(), path, observer);
    assert_eq!(calls.get(), 1);
    std::fs::write(&file, "unreviewed body").unwrap();
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| additions::require_with(
            root.path(),
            path,
            observer
        )))
        .is_err()
    );
    assert_eq!(calls.get(), 2);
    std::fs::write(&file, runtime::frozen(additions::REVISION, path)).unwrap();
    assert!(
        std::panic::catch_unwind(|| additions::require_with(root.path(), path, || format!(
            "{index}{index}"
        )))
        .is_err()
    );
    std::fs::remove_file(&file).unwrap();
    std::fs::create_dir(&file).unwrap();
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| additions::require_with(
            root.path(),
            path,
            observer
        )))
        .is_err()
    );
    assert_eq!(
        calls.get(),
        2,
        "nonregular input refuses before index observation"
    );
}
#[test]
fn main_retains_old_full_body_gate_and_all_twelve_current_reads_reach_source() {
    for row in contract::rules()["rows"].as_array().unwrap() {
        let path = row["path"].as_str().unwrap();
        assert!(!runtime_io::owned_case_view::PATHS.contains(&path));
        assert!(!runtime_io::reviewed_inputs::PATHS.contains(&path));
        if path == runtime::performance::MAIN {
            let previous = contract::project(path, raw(path));
            assert_eq!(previous, runtime::expected(path).unwrap());
            assert!(runtime::accepts(path, &previous));
            assert!(!runtime::accepts(path, &raw(path)));
        } else {
            assert!(runtime::expected(path).is_none());
        }
        assert_eq!(
            frontier::read(path),
            runtime::frozen(current_inputs::SOURCE, path)
        );
    }
}
fn predicates(source: &str) -> Vec<String> {
    use syn::visit::Visit;
    #[derive(Default)]
    struct Visitor(Vec<String>);
    impl<'ast> Visit<'ast> for Visitor {
        fn visit_macro(&mut self, value: &'ast syn::Macro) {
            if value.path.segments.last().is_some_and(|part| {
                matches!(
                    part.ident.to_string().as_str(),
                    "assert" | "assert_eq" | "assert_ne"
                )
            }) {
                self.0.push(format!(
                    "{} {}",
                    value.path.segments.last().unwrap().ident,
                    value.tokens
                ));
            }
            syn::visit::visit_macro(self, value);
        }
    }
    let mut visitor = Visitor::default();
    visitor.visit_file(&syn::parse_file(source).unwrap());
    visitor.0
}
#[test]
fn audit_changes_only_three_input_joins_and_preserves_every_original_predicate() {
    let rule = &contract::rules()["audit"];
    let path = rule["path"].as_str().unwrap();
    let old = runtime::frozen(BASE, path);
    assert_eq!(
        contract::digest(old.as_bytes()),
        rule["before_sha256"].as_str().unwrap()
    );
    let mut expected = old.clone();
    for edit in rule["edits"].as_array().unwrap() {
        let before = edit["before"].as_str().unwrap();
        assert_eq!(
            expected.matches(before).count() as u64,
            edit["count"].as_u64().unwrap()
        );
        expected = expected.replace(before, edit["after"].as_str().unwrap());
    }
    expected.push_str(rule["append"].as_str().unwrap());
    assert_eq!(
        contract::digest(expected.as_bytes()),
        rule["after_sha256"].as_str().unwrap()
    );
    assert_eq!(expected, raw(path));
    assert_eq!(predicates(&old), predicates(&expected));
    for path in [
        "crates/xtask/tests/support/current_tree_input.rs",
        "crates/xtask/tests/support/current_tree_input_rules_074.json",
    ] {
        assert_eq!(raw(path), runtime::frozen(BASE, path));
    }
    assert_eq!(current_inputs::patches().len(), 9);
    assert_eq!(current_inputs::additions().len(), 81);
}
#[test]
fn unknown_paths_are_not_approved_or_transformed_and_legacy_routes_stay_intact() {
    for path in [
        "unknown.rs",
        "../escape.rs",
        "tools/long-run-supervisor-074/src/unreviewed.rs",
    ] {
        assert!(!contract::known(path));
        assert_eq!(
            contract::project(path, "unchanged\r\n".into()),
            "unchanged\r\n"
        );
        assert!(!frontier::known_addition(path));
        assert!(!additions::accepts(path, ""));
        assert!(!additions::index_matches(path, ""));
    }
    for path in current_inputs::patches().keys() {
        assert!(!contract::known(path));
        assert_eq!(frontier::read(path), current_inputs::read(path));
    }
    for path in current_inputs::additions().keys() {
        assert!(!additions::known(path));
        assert!(frontier::known_addition(path));
    }
}
