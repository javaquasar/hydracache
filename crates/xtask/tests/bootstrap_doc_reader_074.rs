//! Two document views only, with every preceding caller predicate retained.
#[path = "../../../tools/bootstrap-doc-reader-074/fixture.rs"]
mod docs;
#[path = "../../../tools/bootstrap-doc-reader-074/reader.rs"]
mod proof_reader;
use proof_reader::contract as phase;
use std::path::Path;
mod runtime {
    pub fn root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }
    pub fn frozen(revision: &str, path: &str) -> String {
        assert!(revision.len() == 40 && revision.bytes().all(|byte| byte.is_ascii_hexdigit()));
        let output = std::process::Command::new("git")
            .arg("--no-replace-objects")
            .args(["show", &format!("{revision}:{path}")])
            .current_dir(root())
            .output()
            .unwrap();
        assert!(output.status.success(), "missing fixed predecessor {path}");
        String::from_utf8(output.stdout)
            .unwrap()
            .replace("\r\n", "\n")
    }
}
const BASE: &str = "181c077abdfbae5a8c4dfe67cee6f1c95519d529";
const DOCS: [&str; 2] = ["docs/COMPAT.md", "docs/testing/gated-test-registry.toml"];
fn raw(path: &str) -> String {
    let raw = std::fs::read_to_string(runtime::root().join(path))
        .unwrap()
        .replace("\r\n", "\n");
    docs::assertion_input(path, raw)
}
fn description() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../tools/bootstrap-doc-reader-074/rules.json"
    ))
    .unwrap()
}
fn assertions(source: &str) -> (Vec<String>, Vec<String>) {
    use syn::visit::Visit;
    #[derive(Default)]
    struct Predicates(Vec<String>);
    impl<'ast> Visit<'ast> for Predicates {
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
    let mut visitor = Predicates::default();
    visitor.visit_file(&file);
    (names, visitor.0)
}
#[test]
fn all_eleven_callers_preserve_complete_original_cases_and_predicates() {
    let rules = description();
    let counts = [4, 4, 4, 4, 4, 3, 4, 4, 22, 4, 6];
    for (index, row) in rules["rows"].as_array().unwrap().iter().enumerate() {
        let path = row["path"].as_str().unwrap();
        let old = runtime::frozen(BASE, path);
        let current = raw(path);
        assert_eq!(
            phase::adapt(path, &old).unwrap(),
            current,
            "exact caller {path}"
        );
        let before = assertions(&old);
        assert_eq!(before.0.len(), counts[index]);
        assert_eq!(assertions(&current), before, "unchanged predicates: {path}");
        for changed in [
            String::new(),
            format!("{current}\n// unknown edit"),
            current.replace("assert_eq!", "assert_ne!"),
        ] {
            assert_ne!(changed, current);
            assert_ne!(phase::adapt(path, &old).unwrap(), changed);
        }
        assert!(std::panic::catch_unwind(|| phase::adapt(path, &format!("{old}x"))).is_err());
    }
    assert!(phase::adapt("unknown.rs", "unknown").is_none());
}
#[test]
fn only_the_two_current_documents_reconstruct_the_exact_reviewed_phase() {
    for (index, path) in DOCS.into_iter().enumerate() {
        let current = raw(path);
        let producer = if index == 0 {
            "a67a5299ab335424e9b11330dc06bd749ea9b859"
        } else {
            "958b3135a15c00b9f6aad41522cef9d1f6608d3a"
        };
        assert_eq!(current, runtime::frozen(producer, path));
        let previous = runtime::frozen("3c52c0aa34b8d00b2fac44375d5ba8fdf532bb9f", path);
        assert_eq!(
            docs::read_to_string(runtime::root().join(path)).unwrap(),
            previous
        );
        assert_eq!(
            docs::project(
                &runtime::root(),
                &runtime::root().join(path),
                current.replace('\n', "\r\n")
            ),
            previous
        );
    }
}
#[test]
fn lexical_selection_is_narrow_and_unknown_inputs_remain_byte_identical() {
    let root = runtime::root();
    for path in DOCS {
        assert_eq!(docs::selected(&root, &root.join(path)), Some(path));
    }
    let wrong = root.join("../outside/docs/COMPAT.md");
    for path in [
        wrong,
        root.join("Cargo.lock"),
        root.join("docs/compat.md"),
        Path::new("docs/COMPAT.md").to_path_buf(),
    ] {
        assert_eq!(docs::selected(&root, &path), None);
        let bytes = "arbitrary\r\nbytes".to_owned();
        assert_eq!(docs::project(&root, &path, bytes.clone()), bytes);
    }
    assert_eq!(
        docs::selected(Path::new("relative"), &root.join(DOCS[0])),
        None
    );
    let bytes = raw("tools/long-run-supervisor-074/src/diagnostic_builder.rs");
    assert_eq!(
        docs::project(
            &root,
            &root.join("tools/long-run-supervisor-074/src/diagnostic_builder.rs"),
            bytes.clone()
        ),
        bytes
    );
}
#[test]
fn current_stale_partial_duplicate_and_unrelated_document_mutations_refuse() {
    let description: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tools/source-successor-view-074/rules.json"
    ))
    .unwrap();
    for path in DOCS {
        let current = raw(path);
        let row = description["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["path"] == path)
            .unwrap();
        let mut mutants = vec![
            String::new(),
            runtime::frozen("3c52c0aa34b8d00b2fac44375d5ba8fdf532bb9f", path),
            format!("{current}\nUNREVIEWED"),
        ];
        for edit in row["patches"].as_array().unwrap() {
            let at = edit["offset"].as_u64().unwrap() as usize;
            let exact = edit["current"].as_str().unwrap();
            let old = edit["previous"].as_str().unwrap();
            let mut partial = current.clone();
            partial.replace_range(at..at + exact.len(), old);
            mutants.push(partial);
            let mut duplicate = current.clone();
            duplicate.insert_str(at, exact);
            mutants.push(duplicate);
        }
        let target = runtime::root().join(path);
        assert!(docs::read_to_string(&target).is_ok());
        for mutant in mutants {
            assert_ne!(mutant, current);
            assert!(
                std::panic::catch_unwind(|| docs::project(&runtime::root(), &target, mutant))
                    .is_err()
            );
        }
    }
}
#[test]
fn exactly_one_ordered_observation_preserves_io_errors_without_caching_success() {
    for path in DOCS.into_iter().chain(["Cargo.lock"]) {
        let target = runtime::root().join(path);
        let calls = std::cell::Cell::new(0);
        let result = docs::read_with(&runtime::root(), &target, |name| {
            calls.set(calls.get() + 1);
            assert_eq!(name, target);
            Ok(raw(path))
        });
        assert!(result.is_ok());
        assert_eq!(calls.get(), 1);
        for kind in [
            std::io::ErrorKind::NotFound,
            std::io::ErrorKind::PermissionDenied,
            std::io::ErrorKind::Interrupted,
        ] {
            let result = docs::read_with(&runtime::root(), &target, |_| {
                calls.set(calls.get() + 1);
                Err(std::io::Error::from(kind))
            });
            assert_eq!(result.unwrap_err().kind(), kind);
        }
        assert_eq!(calls.get(), 4);
    }
}
#[test]
fn preceding_owned_case_proof_accepts_only_the_exact_extended_current_reader() {
    let path = "crates/xtask/tests/owned_case_074.rs";
    let old = runtime::frozen(BASE, path);
    let current = raw(path);
    assert_eq!(proof_reader::read(path, current.clone()), old);
    for changed in [
        old,
        String::new(),
        format!("{current}\n// unknown caller edit"),
    ] {
        assert!(std::panic::catch_unwind(|| proof_reader::read(path, changed)).is_err());
    }
    assert_eq!(
        proof_reader::read("unrelated.rs", "unchanged".into()),
        "unchanged"
    );
}
