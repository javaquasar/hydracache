//! Whole-body reader controls; historical counts and runtime checks stay fixed.
#[path = "../../../tools/authority-runtime-074/cost_reader.rs"]
mod readers;
#[path = "../../../tools/authority-runtime-074/gate.rs"]
mod runtime;
#[path = "../../../tools/authority-runtime-074/io.rs"]
mod runtime_io;
#[path = "../../../tools/authority-storage-source-074/contract.rs"]
mod storage_contract;

#[test]
fn cost_readers_preserve_every_original_assertion_and_limit() {
    assert_eq!(readers::FILES.len(), 3);
    for path in readers::FILES {
        let raw = std::fs::read_to_string(runtime::root().join(path)).unwrap();
        assert!(readers::accepts(path, &raw), "{path}");
        assert_eq!(
            runtime::frozen(readers::BASE, path),
            runtime::frozen(runtime::BASE, path)
        );
        assert_eq!(
            readers::previous(path, &raw),
            runtime::frozen(runtime::BASE, path)
        );
        for bad in [
            String::new(),
            raw.replacen("assert_eq!", "assert_ne!", 1),
            format!("{raw}\n// unknown source change"),
            format!("{raw}\nfn unrelated_effect() {{}}"),
        ] {
            assert_ne!(bad, raw);
            assert!(!readers::accepts(path, &bad), "{path}");
            assert!(std::panic::catch_unwind(|| readers::previous(path, &bad)).is_err());
        }
    }
    assert!(readers::expected("unrelated.rs").is_none());
    assert!(!readers::accepts("unrelated.rs", "anything"));
    assert!(std::panic::catch_unwind(|| readers::previous("unrelated.rs", "anything")).is_err());
}

#[test]
fn cost_readers_cannot_broaden_sources_change_counts_or_relax_original_limits() {
    let cases = [
        (
            readers::FILES[0],
            vec![
                ("if path == runtime::status_fields::SOURCE", "if true"),
                (
                    "assert_eq!(paths.lines().count(), 473);",
                    "assert_eq!(paths.lines().count(), 472);",
                ),
                (
                    "unchanged measured prototype/reference",
                    "skip measured reference",
                ),
                ("mandatory current test hook", "skip current test hook"),
            ],
        ),
        (
            readers::FILES[1],
            vec![
                ("if path == runtime::status_fields::SOURCE", "if true"),
                ("if args == [\"ls-files\", inventory::SCOPE]", "if true"),
                (
                    "inventory::project(&raw).expect",
                    "Some(raw.clone()).expect",
                ),
                (
                    "assert_eq!(paths.lines().count(), 477);",
                    "assert_eq!(paths.lines().count(), 476);",
                ),
                (
                    "assert_eq!(current.lines().count(), 446);",
                    "assert_eq!(current.lines().count(), 530);",
                ),
            ],
        ),
        (
            readers::FILES[2],
            vec![
                (
                    "if path == \"docs/testing/gated-test-registry.toml\"",
                    "if true",
                ),
                ("timeout_seconds = 1200", "timeout_seconds = 2400"),
                (
                    "const ITERATIONS: usize = 1024;",
                    "const ITERATIONS: usize = 512;",
                ),
                (
                    "assert_eq!(observed.syscw, 0);",
                    "assert_eq!(observed.syscw, 1);",
                ),
            ],
        ),
    ];
    for (path, changes) in cases {
        let raw = std::fs::read_to_string(runtime::root().join(path)).unwrap();
        assert!(readers::accepts(path, &raw));
        for (before, after) in changes {
            assert_eq!(
                raw.matches(before).count(),
                1,
                "required mutation control {path}: {before}"
            );
            let bad = raw.replacen(before, after, 1);
            assert_ne!(bad, raw);
            assert!(!readers::accepts(path, &bad), "{path}");
            assert!(std::panic::catch_unwind(|| readers::previous(path, &bad)).is_err());
        }
    }
}

#[test]
fn cost_reader_parser_inputs_validate_current_runtime_and_pinned_helper_first() {
    for path in [
        runtime::status_fields::SOURCE,
        runtime::status_fields::LEGACY_TEST,
    ] {
        let raw = std::fs::read_to_string(runtime::root().join(path)).unwrap();
        assert!(runtime::accepts(path, &raw));
        assert_eq!(
            runtime_io::read_to_string(runtime::root().join(path)).unwrap(),
            runtime::frozen(runtime::BASE, path)
        );
        for bad in [String::new(), format!("{raw}\nfn unrelated_effect() {{}}")] {
            assert!(!runtime::accepts(path, &bad));
            assert!(std::panic::catch_unwind(|| runtime::view(path, bad)).is_err());
        }
    }
    let helper =
        std::fs::read_to_string(runtime::root().join(runtime::status_fields::HELPER)).unwrap();
    assert!(runtime::status_fields::helper_is_pinned(&helper));
    assert!(!runtime::status_fields::helper_is_pinned(&format!(
        "{helper}\n// unknown helper"
    )));
}
