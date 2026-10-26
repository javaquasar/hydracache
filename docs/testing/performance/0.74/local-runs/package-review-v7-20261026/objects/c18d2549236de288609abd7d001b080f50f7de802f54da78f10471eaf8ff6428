//! Explicit next caller phases preserve old validators and every original assertion.
#[path = "../../../tools/remaining-readers-074/inputs.rs"]
mod current_inputs;
#[path = "../../../tools/authority-runtime-074/gate.rs"]
mod runtime;
#[path = "../../../tools/authority-runtime-074/io.rs"]
mod runtime_io;
#[path = "../../../tools/authority-storage-source-074/contract.rs"]
mod storage_contract;
use current_inputs::phase;
fn raw(path: &str) -> String {
    std::fs::read_to_string(runtime::root().join(path))
        .unwrap()
        .replace("\r\n", "\n")
}
fn assertions(source: &str) -> (Vec<String>, Vec<String>) {
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
    let file = syn::parse_file(source).unwrap();
    let names = file
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Fn(value) if value.attrs.iter().any(|attr| attr.path().is_ident("test")) => {
                Some(value.sig.ident.to_string())
            }
            _ => None,
        })
        .collect();
    let mut visitor = Visitor::default();
    visitor.visit_file(&file);
    (names, visitor.0)
}
#[test]
fn seven_exact_caller_constructions_preserve_all_eighteen_cases_and_assertions() {
    let counts = [2, 4, 4, 1, 7, 0, 0];
    for (index, row) in phase::rules()["rows"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        let path = row["path"].as_str().unwrap();
        let old = runtime::frozen(phase::BASE, path);
        let current = raw(path);
        assert_eq!(phase::current(path, &old).unwrap(), current);
        assert_eq!(phase::previous(path, &current), Some(old.clone()));
        assert_eq!(assertions(&old).0.len(), counts[index]);
        assert_eq!(
            assertions(&old),
            assertions(&current),
            "all original predicates: {path}"
        );
        assert!(std::panic::catch_unwind(|| phase::current(path, &format!("{old}x"))).is_err());
    }
    let current = raw("tools/bootstrap-doc-reader-074/reader.rs");
    let formatted = "remaining_phase::previous(path, &raw)\n            .expect";
    assert_eq!(current.matches(formatted).count(), 1);
    let before = current.replace(formatted, "remaining_phase::previous(path, &raw).expect");
    assert_eq!(
        before
            .parse::<proc_macro2::TokenStream>()
            .unwrap()
            .to_string(),
        current
            .parse::<proc_macro2::TokenStream>()
            .unwrap()
            .to_string()
    );
}
#[test]
fn stale_partial_duplicate_and_unrelated_caller_edits_refuse_after_success() {
    for row in phase::rules()["rows"].as_array().unwrap() {
        let path = row["path"].as_str().unwrap();
        let old = runtime::frozen(phase::BASE, path);
        let current = raw(path);
        assert_eq!(
            phase::previous(path, &current.replace('\n', "\r\n")),
            Some(old.clone())
        );
        let mut mutants = vec![String::new(), old, format!("{current}\n// unknown edit")];
        for edit in row["edits"].as_array().unwrap() {
            let exact = edit["after"].as_str().unwrap();
            let previous = edit["before"].as_str().unwrap();
            mutants.push(current.replacen(exact, previous, 1));
            mutants.push(current.replacen(exact, &format!("{exact}{exact}"), 1));
        }
        for mutant in mutants {
            assert_ne!(mutant, current);
            assert!(phase::previous(path, &mutant).is_none());
        }
    }
    assert!(phase::previous("unknown.rs", "unknown").is_none());
    assert!(phase::current("unknown.rs", "unknown").is_none());
}
#[test]
fn immutable_nine_eighty_one_gate_still_refuses_new_caller_without_explicit_join() {
    for path in [
        "crates/xtask/tests/support/current_tree_input.rs",
        "crates/xtask/tests/support/current_tree_input_rules_074.json",
    ] {
        assert_eq!(raw(path), runtime::frozen(phase::BASE, path));
    }
    assert_eq!(current_inputs::patches().len(), 9);
    assert_eq!(current_inputs::additions().len(), 81);
    for (path, row) in current_inputs::additions() {
        assert!(current_inputs::known_addition(path));
        current_inputs::require_addition(path);
        assert!(current_inputs::accepts_addition(path, &raw(path)));
        let index = format!(
            "{} {} 0\t{path}\n",
            row["mode"].as_str().unwrap(),
            row["blob"].as_str().unwrap()
        );
        assert!(current_inputs::addition_index_matches(path, &index));
    }
    let current = raw(phase::FRESH);
    let old = phase::previous(phase::FRESH, &current).unwrap();
    assert!(current_inputs::old::accepts_patch(phase::FRESH, &old));
    assert!(!current_inputs::old::accepts_patch(phase::FRESH, &current));
    assert!(std::panic::catch_unwind(|| current_inputs::old::read(phase::FRESH)).is_err());
    assert!(current_inputs::accepts_patch(phase::FRESH, &current));
    assert!(!current_inputs::accepts_patch(phase::FRESH, &old));
    assert!(!current_inputs::accepts_patch(
        phase::FRESH,
        &format!("{current}x")
    ));
    assert_eq!(
        current_inputs::read(phase::FRESH),
        runtime::frozen(current_inputs::SOURCE, phase::FRESH)
    );
    for path in current_inputs::patches()
        .keys()
        .filter(|path| path.as_str() != phase::FRESH)
    {
        let body = raw(path);
        assert_eq!(
            current_inputs::accepts_patch(path, &body),
            current_inputs::old::accepts_patch(path, &body)
        );
        assert_eq!(current_inputs::read(path), current_inputs::old::read(path));
    }
    assert!(!current_inputs::accepts_patch("unknown.rs", ""));
}
#[test]
fn selected_kernel_and_compat_join_use_independent_producers_without_extra_reads() {
    for (path, current, previous) in [
        (
            "tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs",
            "958b3135a15c00b9f6aad41522cef9d1f6608d3a",
            "c91068bfa8b297bb075b63457da235aaa9c6232e",
        ),
        (
            "docs/COMPAT.md",
            "a67a5299ab335424e9b11330dc06bd749ea9b859",
            "3c52c0aa34b8d00b2fac44375d5ba8fdf532bb9f",
        ),
    ] {
        let observed = raw(path);
        assert_eq!(observed, runtime::frozen(current, path));
        assert_eq!(
            runtime::successor::project(path, observed.clone()),
            runtime::frozen(previous, path)
        );
        assert!(std::panic::catch_unwind(|| runtime::successor::project(
            path,
            format!("{observed}x")
        ))
        .is_err());
    }
    let old = runtime::frozen(phase::BASE, phase::FRESH);
    let current = raw(phase::FRESH);
    for token in [
        "std::fs::read_to_string",
        "runtime_io::read_to_string",
        "let loader = if path == runtime::status_fields::SOURCE",
        "guard::pid_path::custody_cost::document_scratch::normalize(path, &raw)",
    ] {
        assert_eq!(old.matches(token).count(), current.matches(token).count());
    }
    assert_eq!(
        current
            .matches("runtime::successor::project(path, raw)")
            .count(),
        1
    );
}
