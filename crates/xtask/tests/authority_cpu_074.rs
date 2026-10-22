#[path = "support/document_scratch_guard.rs"]
mod guard;
#[path = "support/authority_history_io.rs"]
mod history;

#[test]
fn authority_cpu_only_adds_exact_test_hooks_and_preserves_runtime() {
    for path in [
        "tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs",
        "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs",
        "docs/testing/gated-test-registry.toml",
    ] {
        let raw = history::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(path),
        )
        .unwrap()
        .replace('\r', "");
        let output = history::command()
            .args([
                "show",
                &format!("3d227add18d68bea62686ea85f5c225c51ba3f1b:{path}"),
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        let baseline = String::from_utf8(output.stdout).unwrap().replace('\r', "");
        let restored = crate::hc074_authority_cpu_view!(path, &raw);
        assert_eq!(restored, baseline);
        assert_eq!(crate::hc074_authority_cpu_view!(path, &restored), restored);
        assert!(
            crate::hc074_authority_cpu_view!(path, &format!("{raw}\n// unrelated drift\n"))
                .contains("// unrelated drift")
        );
        if path.ends_with("fixture.rs") {
            continue;
        }
        let marker = if path.ends_with("alloc_tests.rs") {
            "mod authority_cpu;"
        } else {
            "test = \"authority_cpu_actual_phase_attribution\""
        };
        for changed in [raw.replacen(marker, "changed", 1), format!("{raw}{raw}")] {
            assert!(
                std::panic::catch_unwind(|| crate::hc074_authority_cpu_view!(path, &changed))
                    .is_err()
            );
        }
    }
    assert_eq!(guard::normalize("unrelated", "unchanged"), "unchanged");
}
