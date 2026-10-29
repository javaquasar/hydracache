//! Exact current/terminal source edges; no old runtime compilation or admission.
#[path = "../../../tools/authority-reader-phase-074/current_main.rs"]
mod current;
#[path = "../../../tools/authority-runtime-074/gate.rs"]
mod runtime;
#[path = "../../../tools/authority-storage-source-074/contract.rs"]
mod storage_contract;
#[path = "../../../tools/authority-reader-phase-074/main.rs"]
mod terminal;

const STEP: &str = "5c7ef1c893a91d57a566e0a484a218f87c3df48f";
const LEGACY: &str = "37db65f68e67ed7c0ee0f1cc84fed1c30bf9822a";

fn read(path: &str) -> String {
    std::fs::read_to_string(runtime::root().join(path))
        .unwrap()
        .replace("\r\n", "\n")
}

fn frozen(revision: &str) -> String {
    runtime::frozen(revision, current::SOURCE)
}

#[test]
fn one_reader_edge_matches_independent_producer_and_terminal_predecessor() {
    let raw = read(current::SOURCE);
    assert_eq!(raw, frozen(current::PRODUCER));
    let old = frozen(current::PREVIOUS);
    assert_ne!(raw, old);
    assert_eq!(current::project(raw.clone()), old);
    assert_eq!(current::project(raw.replace('\n', "\r\n")), old);
    assert_eq!(terminal::project(old), frozen(LEGACY));
}

#[test]
fn stale_partial_duplicate_and_unknown_source_edits_refuse() {
    let raw = read(current::SOURCE);
    let edge =
        "    let raw_lock = runtime_io::read_to_string(root().join(\"Cargo.lock\")).unwrap();";
    assert_eq!(raw.matches(edge).count(), 1);
    for bad in [
        String::new(),
        frozen(current::PREVIOUS),
        frozen(LEGACY),
        raw.replacen(edge, "", 1),
        raw.replacen(edge, &format!("{edge}{edge}"), 1),
        raw.replacen("assert_eq!", "assert_ne!", 1),
        format!("{raw}\n// unreviewed"),
        raw.replace('\n', "\r"),
    ] {
        assert_ne!(bad, raw);
        assert!(std::panic::catch_unwind(|| current::project(bad)).is_err());
    }
}

#[test]
fn security_and_terminal_routes_reach_the_same_complete_legacy_body() {
    let raw = read(current::SOURCE);
    let security = runtime::successor::project(current::SOURCE, raw.clone());
    let terminal = terminal::project(current::project(raw));
    assert_eq!(security, terminal);
    assert_eq!(security, frozen(LEGACY));
    // The security predecessor is a constructed phase, not the raw source at BASE.
    assert_ne!(security, frozen(runtime::BASE));
    assert!(runtime::accepts(current::SOURCE, &security));
}

#[test]
fn seeded_single_byte_corpus_cannot_reuse_a_successful_approval() {
    let raw = read(current::SOURCE);
    let mut seed = 0x0740_2026_1027_u64;
    for case in 0..24 {
        assert_eq!(current::project(raw.clone()), frozen(current::PREVIOUS));
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut at = seed as usize % raw.len();
        if !raw.as_bytes()[at].is_ascii() {
            at = 0;
        }
        let mut bad = raw.clone().into_bytes();
        bad[at] ^= 1;
        let bad = String::from_utf8(bad).unwrap();
        assert_ne!(bad, raw, "seed={seed:#x} case={case}");
        assert!(std::panic::catch_unwind(|| current::project(bad)).is_err());
    }
}

#[test]
fn gate_requires_fresh_current_input_not_either_historical_representation() {
    let raw = read(current::SOURCE);
    for _ in 0..2 {
        assert_eq!(
            runtime::view(current::SOURCE, raw.clone()),
            frozen(runtime::BASE)
        );
    }
    for bad in [
        frozen(current::PREVIOUS),
        frozen(LEGACY),
        frozen(runtime::BASE),
        format!("{raw}\nfn unexpected_effect() {{}}"),
    ] {
        assert!(std::panic::catch_unwind(|| runtime::view(current::SOURCE, bad)).is_err());
    }
    assert_eq!(
        runtime::view("unrelated.rs", "unmodified".into()),
        "unmodified"
    );
}

#[test]
fn the_complete_original_gate_and_io_bodies_are_unchanged() {
    for path in [
        "tools/authority-runtime-074/gate.rs",
        "tools/authority-runtime-074/io.rs",
    ] {
        assert_eq!(read(path), runtime::frozen(STEP, path));
    }
}

fn assertions(source: &str) -> (Vec<String>, Vec<String>) {
    use syn::visit::Visit;
    #[derive(Default)]
    struct Visitor(Vec<String>);
    impl<'ast> Visit<'ast> for Visitor {
        fn visit_macro(&mut self, node: &'ast syn::Macro) {
            let name = node.path.segments.last().unwrap().ident.to_string();
            if matches!(name.as_str(), "assert" | "assert_eq" | "assert_ne") {
                self.0.push(format!("{name} {}", node.tokens));
            }
            syn::visit::visit_macro(self, node);
        }
    }
    let source = syn::parse_file(source).unwrap();
    let tests = source
        .items
        .iter()
        .filter_map(|node| match node {
            syn::Item::Fn(f) if f.attrs.iter().any(|a| a.path().is_ident("test")) => {
                Some(f.sig.ident.to_string())
            }
            _ => None,
        })
        .collect();
    let mut visitor = Visitor::default();
    visitor.visit_file(&source);
    (tests, visitor.0)
}

#[test]
fn all_old_assertions_cases_and_fixed_reader_inputs_remain_exact() {
    for (path, count) in [
        ("crates/xtask/tests/authority_runtime_074.rs", 5),
        ("crates/xtask/tests/authority_reader_phase_074.rs", 6),
    ] {
        let old = assertions(&runtime::frozen(STEP, path));
        assert_eq!(old.0.len(), count);
        assert!(!old.1.is_empty());
        assert_eq!(assertions(&read(path)), old);
    }
    let path = "tools/authority-reader-phase-074/contract.rs";
    let old = runtime::frozen(STEP, path);
    let now = read(path);
    // Includes all five independent predecessor hashes and both old read templates.
    assert_eq!(
        old.split_once("fn digest(").unwrap().0,
        now.split_once("fn digest(").unwrap().0
    );
}

#[test]
fn old_terminal_security_and_product_artifacts_remain_unchanged() {
    for path in [
        "tools/authority-reader-phase-074/main.rs",
        "tools/authority-reader-phase-074/main-rules.json",
        "tools/security-input-view-074/contract.rs",
        "tools/security-input-view-074/rules.json",
        "tools/source-successor-view-074/contract.rs",
        "Cargo.lock",
        "Cargo.toml",
        "docs/testing/performance/0.74/qualification-manifest.toml",
        current::SOURCE,
    ] {
        assert_eq!(read(path), runtime::frozen(STEP, path), "{path}");
    }
}
