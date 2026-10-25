//! Reader routing is a source compatibility proof, never runtime admission.
#[path = "../../../tools/authority-reader-phase-074/main.rs"]
mod main_phase;
#[path = "../../../tools/authority-reader-phase-074/contract.rs"]
mod phase;
#[path = "../../../tools/authority-runtime-074/gate.rs"]
mod runtime;
#[path = "../../../tools/authority-runtime-074/io.rs"]
mod runtime_io;
#[path = "../../../tools/authority-storage-source-074/contract.rs"]
mod storage_contract;
const BASE: &str = "628bc3c113ce12e9ecb10ee4daee6961a6b2ad15";

fn accepts(path: &str, original: &str, current: &str) -> bool {
    phase::expected(path, original)
        .is_some_and(|expected| current.replace("\r\n", "\n") == expected)
}

fn raw(path: &str) -> String {
    std::fs::read_to_string(runtime::root().join(path))
        .unwrap()
        .replace("\r\n", "\n")
}

#[test]
fn five_readers_are_exact_declared_edits_of_independent_fixed_sources() {
    for (path, _) in phase::FILES {
        let old = runtime::frozen(BASE, path);
        let current = raw(path);
        assert_ne!(old, current);
        assert_eq!(phase::expected(path, &old).unwrap(), current, "{path}");
        assert!(accepts(path, &old, &current));
        assert!(accepts(path, &old, &current.replace('\n', "\r\n")));
        assert!(!accepts(path, &old, &old));
        if path == phase::FILES[3].0 {
            assert_eq!(phase::runtime(&old), current);
        }
        assert!(std::panic::catch_unwind(|| phase::expected(path, &format!("{old}x"))).is_err());
    }
    assert!(phase::expected("unrelated.rs", "unknown").is_none());
    assert!(!accepts("unrelated.rs", "unknown", "unknown"));
}

fn assertions(source: &str) -> (Vec<String>, Vec<String>) {
    use syn::visit::Visit;
    #[derive(Default)]
    struct Assertions(Vec<String>);
    impl<'ast> Visit<'ast> for Assertions {
        fn visit_macro(&mut self, node: &'ast syn::Macro) {
            if node.path.segments.last().is_some_and(|name| {
                matches!(
                    name.ident.to_string().as_str(),
                    "assert" | "assert_eq" | "assert_ne"
                )
            }) {
                self.0.push(format!(
                    "{} {}",
                    node.path.segments.last().unwrap().ident,
                    node.tokens
                ));
            }
            syn::visit::visit_macro(self, node);
        }
    }
    let file = syn::parse_file(source).unwrap();
    let names = file
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Fn(function)
                if function
                    .attrs
                    .iter()
                    .any(|attr| attr.path().is_ident("test")) =>
            {
                Some(function.sig.ident.to_string())
            }
            _ => None,
        })
        .collect();
    let mut visitor = Assertions::default();
    visitor.visit_file(&file);
    (names, visitor.0)
}

#[test]
fn every_original_case_and_assertion_survives_the_two_explicit_input_substitutions() {
    for (index, (path, _)) in phase::FILES.into_iter().enumerate() {
        let old = runtime::frozen(BASE, path);
        let current = raw(path)
            .replace(
                "gate::view(&path, view_input(&path))",
                "gate::view(&path, read(&path))",
            )
            .replace(
                "contract::predecessor(path, &entry)",
                "contract::predecessor(path, &current)",
            );
        let before = assertions(&old);
        assert_eq!(before.0.len(), [2, 4, 3, 5, 4][index]);
        assert!(!before.1.is_empty());
        assert_eq!(
            assertions(&current),
            before,
            "every original predicate: {path}"
        );
    }
}

#[test]
fn partial_duplicate_wrong_branch_and_assertion_drift_cannot_enter_old_reader_views() {
    for (path, _) in phase::FILES {
        let old = runtime::frozen(BASE, path);
        let current = raw(path);
        for changed in [
            String::new(),
            format!("{current}{current}"),
            format!("{current}\n// unexpected edit"),
            current.replace("assert_eq!", "assert_ne!"),
            current.chars().skip(1).collect(),
        ] {
            assert_ne!(changed, current);
            assert!(!accepts(path, &old, &changed), "{path}");
        }
        for anchor in [
            "runtime::successor::project",
            "gate::successor::project",
            "if path == gate::FIXTURE",
            "reader_phase::runtime(&expected)",
            "let entry = current.clone();",
        ] {
            if current.contains(anchor) {
                for changed in [
                    current.replacen(anchor, "", 1),
                    current.replacen(anchor, &format!("{anchor}{anchor}"), 1),
                ] {
                    assert!(
                        !accepts(path, &old, &changed),
                        "routing edit {path}: {anchor}"
                    );
                }
            }
        }
    }
}

#[test]
fn current_fixture_entry_keeps_fresh_current_bytes_separate_from_old_body_checks() {
    let current = raw(runtime::FIXTURE);
    let previous = runtime::successor::project(runtime::FIXTURE, current.clone());
    assert!(runtime::accepts(runtime::FIXTURE, &previous));
    assert!(!runtime::accepts(runtime::FIXTURE, &current));
    assert_eq!(
        runtime::view(runtime::FIXTURE, current.clone()),
        runtime::frozen(runtime::BASE, runtime::FIXTURE)
    );
    for changed in [
        previous,
        String::new(),
        format!("{current}\n// unknown current edit"),
        current.chars().skip(1).collect(),
    ] {
        assert!(std::panic::catch_unwind(|| runtime::view(runtime::FIXTURE, changed)).is_err());
    }
    assert_eq!(
        runtime::view(runtime::FIXTURE, raw(runtime::FIXTURE)),
        runtime::frozen(runtime::BASE, runtime::FIXTURE)
    );
}

#[test]
fn all_eighteen_entry_inputs_and_current_mutants_retain_original_phase_checks() {
    for path in runtime_io::reviewed_inputs::PATHS {
        let mut entry = raw(path);
        if runtime_io::owned_case_view::PATHS.contains(&path) {
            runtime_io::owned_case_view::require_helpers(&runtime::root());
            let old = runtime::frozen(runtime_io::owned_case_view::BASE, path);
            assert!(runtime_io::owned_case_view::accepts(path, &old, &entry));
            entry = old;
        }
        let previous = runtime::successor::project(path, entry.clone());
        assert!(runtime_io::reviewed_inputs::accepts(path, &previous));
        assert_eq!(
            runtime_io::reviewed_inputs::predecessor(path, &entry),
            runtime::frozen(runtime_io::reviewed_inputs::PREDECESSOR, path)
        );
        assert_eq!(
            runtime_io::read_to_string(runtime::root().join(path)).unwrap(),
            runtime::frozen(runtime_io::reviewed_inputs::PREDECESSOR, path)
        );
        for changed in [
            String::new(),
            format!("{entry}\nUNREVIEWED_CHANGE\n"),
            entry.chars().skip(1).collect(),
        ] {
            assert!(
                std::panic::catch_unwind(|| runtime_io::reviewed_inputs::predecessor(
                    path, &changed
                ))
                .is_err(),
                "latest input {path}"
            );
        }
    }
}

#[test]
fn terminal_main_text_matches_fixed_producers_without_compiling_the_old_runtime() {
    let current = raw(main_phase::SOURCE);
    assert_eq!(
        current,
        runtime::frozen(
            "a67a5299ab335424e9b11330dc06bd749ea9b859",
            main_phase::SOURCE
        )
    );
    let previous = runtime::frozen(
        "37db65f68e67ed7c0ee0f1cc84fed1c30bf9822a",
        main_phase::SOURCE,
    );
    assert_eq!(main_phase::project(current.clone()), previous);
    assert!(runtime::accepts(main_phase::SOURCE, &previous));
    assert!(!runtime::accepts(main_phase::SOURCE, &current));
    assert_eq!(main_phase::project(current.replace('\n', "\r\n")), previous);
    let rules: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tools/authority-reader-phase-074/main-rules.json"
    ))
    .unwrap();
    let current = raw(main_phase::SOURCE);
    let mut mutants = vec![
        previous,
        String::new(),
        format!("{current}\n// unknown body"),
    ];
    for edit in rules["rows"][0]["patches"].as_array().unwrap() {
        let at = edit["offset"].as_u64().unwrap() as usize;
        let exact = edit["current"].as_str().unwrap();
        let prior = edit["previous"].as_str().unwrap();
        let mut partial = current.clone();
        partial.replace_range(at..at + exact.len(), prior);
        mutants.push(partial);
        let mut duplicate = current.clone();
        duplicate.insert_str(at, if exact.is_empty() { prior } else { exact });
        mutants.push(duplicate);
    }
    for changed in mutants {
        assert_ne!(changed, current);
        assert!(std::panic::catch_unwind(|| main_phase::project(changed)).is_err());
    }
}
