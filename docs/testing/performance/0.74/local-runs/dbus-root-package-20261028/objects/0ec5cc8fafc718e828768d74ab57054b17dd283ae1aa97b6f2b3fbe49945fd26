//! Source-only composition controls. Never execute a historical body or live host fixture.
#[path = "../../../tools/package-closure-inputs-074/contract.rs"]
mod inputs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn frozen(revision: &str, path: &str) -> String {
    let output = std::process::Command::new("git")
        .args([
            "--no-replace-objects",
            "show",
            &format!("{revision}:{path}"),
        ])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout)
        .unwrap()
        .replace("\r\n", "\n")
}

#[test]
fn exact_current_and_previous_producers_bind_all_six_inverse_rows() {
    for row in &inputs::rules().rows {
        let raw = std::fs::read_to_string(root().join(&row.path)).unwrap();
        let previous = frozen(&row.previous_revision, &row.path);
        assert_eq!(
            raw.replace("\r\n", "\n"),
            frozen(&row.current_revision, &row.path)
        );
        assert_eq!(inputs::project(&row.consumer, &row.path, raw), previous);
    }
}

#[test]
fn fresh_complete_body_is_required_for_each_call_and_seeded_mutants_refuse() {
    let mut seed = 0x074c_105e_u64;
    for row in &inputs::rules().rows {
        let raw = std::fs::read_to_string(root().join(&row.path))
            .unwrap()
            .replace("\r\n", "\n");
        let previous = frozen(&row.previous_revision, &row.path);
        assert_eq!(
            inputs::project(&row.consumer, &row.path, raw.clone()),
            previous
        );
        assert_eq!(
            inputs::project(&row.consumer, &row.path, raw.replace('\n', "\r\n")),
            previous
        );
        for bad in [
            String::new(),
            previous,
            format!("{raw}\nunknown edit"),
            raw[..raw.len() - 1].to_owned(),
        ] {
            assert!(
                std::panic::catch_unwind(|| inputs::project(&row.consumer, &row.path, bad))
                    .is_err()
            );
        }
        for _ in 0..32 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let mut bytes = raw.as_bytes().to_vec();
            let mut at = seed as usize % bytes.len();
            while !bytes[at].is_ascii_alphabetic() {
                at = (at + 1) % bytes.len();
            }
            bytes[at] ^= 0x20;
            let bad = String::from_utf8(bytes).unwrap();
            assert!(
                std::panic::catch_unwind(|| inputs::project(&row.consumer, &row.path, bad))
                    .is_err()
            );
        }
        assert_eq!(
            inputs::project(&row.consumer, &row.path, raw),
            frozen(&row.previous_revision, &row.path)
        );
    }
    assert_eq!(
        inputs::project("cache", "unknown.rs", "unmodified".into()),
        "unmodified"
    );
    assert!(std::panic::catch_unwind(|| inputs::project(
        "unknown-consumer",
        "unknown.rs",
        String::new()
    ))
    .is_err());
}

fn assertion_contract(text: &str) -> (Vec<String>, Vec<String>) {
    use syn::visit::Visit;
    #[derive(Default)]
    struct Assertions(Vec<String>);
    impl<'ast> Visit<'ast> for Assertions {
        fn visit_macro(&mut self, value: &'ast syn::Macro) {
            if value.path.segments.last().is_some_and(|p| {
                matches!(
                    p.ident.to_string().as_str(),
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
    let parsed = syn::parse_file(text).unwrap();
    let names = parsed
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Fn(f) if f.attrs.iter().any(|a| a.path().is_ident("test")) => {
                Some(f.sig.ident.to_string())
            }
            _ => None,
        })
        .collect();
    let mut assertions = Assertions::default();
    assertions.visit_file(&parsed);
    (names, assertions.0)
}

#[test]
fn every_original_case_and_assertion_token_stays_exact() {
    for name in [
        "historical_source_cache_074",
        "pid_path_cost_074",
        "remaining_readers_074",
        "role_census_reader_074",
        "validation_leaf_reader_074",
    ] {
        let path = format!("crates/xtask/tests/{name}.rs");
        let old = frozen("46648903c0eb7ce88d438008fa6465a96cc3c414", &path);
        let raw = std::fs::read_to_string(root().join(&path)).unwrap();
        assert_eq!(
            assertion_contract(&raw),
            assertion_contract(&old),
            "unchanged original assertions: {path}"
        );
    }
    // The document producer advances explicitly; its old predecessor does not.
    let document = std::fs::read_to_string(root().join("docs/COMPAT.md"))
        .unwrap()
        .replace("\r\n", "\n");
    assert_eq!(
        document,
        frozen("d3256fc08b8da2df424d6da5bce99c0e168939b0", "docs/COMPAT.md")
    );
}
