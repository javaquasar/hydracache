#[path = "support/document_scratch_guard.rs"]
mod guard;
fn read(path: &str) -> String {
    let raw = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path),
    )
    .unwrap()
    .replace('\r', "");
    crate::hc074_authority_fd_path_view!(path, &raw)
}
#[test]
fn authority_breakdown_restores_every_runtime_byte_and_rejects_hook_drift() {
    for path in [
        "tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs",
        "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs",
        "docs/testing/gated-test-registry.toml",
    ] {
        let raw = read(path);
        let output = std::process::Command::new("git")
            .args([
                "show",
                &format!("ad5d8ad07707cbe5f76d9f01b6d05cf41129adb6:{path}"),
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        let before = String::from_utf8(output.stdout).unwrap().replace('\r', "");
        let restored = crate::hc074_authority_breakdown_view!(path, &raw);
        assert_eq!(restored, before);
        assert_eq!(
            crate::hc074_authority_breakdown_view!(path, &restored),
            restored
        );
        let marker = if path.ends_with("fixture.rs") {
            "pub(crate) mod breakdown_controls;"
        } else if path.ends_with("alloc_tests.rs") {
            "mod authority_breakdown_alloc;"
        } else {
            "test = \"authority_breakdown_allocation_attribution\""
        };
        assert_eq!(raw.matches(marker).count(), 1);
        for changed in [raw.replacen(marker, "changed", 1), format!("{raw}{raw}")] {
            assert!(
                std::panic::catch_unwind(|| crate::hc074_authority_breakdown_view!(path, &changed))
                    .is_err()
            );
        }
        let unrelated = format!("{raw}\n// unrelated drift\n");
        assert!(
            crate::hc074_authority_breakdown_view!(path, &unrelated).contains("// unrelated drift")
        );
    }
    assert_eq!(guard::normalize("unrelated", "unchanged"), "unchanged");
}
#[test]
fn authority_breakdown_contract_preserves_scan_reader_and_non_admission_scope() {
    let contract: toml::Value = toml::from_str(&read(
        "docs/testing/performance/0.74/authority-breakdown-contract.toml",
    ))
    .unwrap();
    for flag in [
        "runtime_changed",
        "candidate_enabled",
        "allocation_reduction_claimed",
        "server_admitted",
        "latency_gain_claimed",
        "fresh_checks_changed",
        "deadlines_changed",
    ] {
        assert_eq!(contract[flag].as_bool(), Some(false));
    }
    let actual = read("tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs");
    let controls =
        read("tools/long-run-supervisor-074/tests/support/authority_breakdown_controls.rs");
    let compact = |text: &str| text.split_whitespace().collect::<String>();
    let original_scan = actual
        .split("fn inventory(&self)")
        .nth(1)
        .unwrap()
        .split("let mut seen")
        .nth(1)
        .unwrap()
        .split("let generations")
        .next()
        .unwrap()
        .replace("self.leaf", "directory.leaf");
    let frozen_scan = controls
        .split("fn scan(")
        .nth(1)
        .unwrap()
        .split("let mut seen")
        .nth(1)
        .unwrap()
        .split("Ok(seen)")
        .next()
        .unwrap();
    assert_eq!(compact(&original_scan), compact(frozen_scan));
    let frozen = read("tools/long-run-supervisor-074/tests/support/validation_leaf_requests.rs");
    let probe = read("tools/long-run-supervisor-074/tests/support/authority_breakdown_alloc.rs");
    let reader = |source: &str| {
        source
            .split("struct Io {")
            .nth(1)
            .unwrap()
            .split("struct Journal")
            .next()
            .unwrap()
            .to_owned()
    };
    let probe_reader = probe
        .split("struct Io {")
        .nth(1)
        .unwrap()
        .split("#[test]")
        .next()
        .unwrap();
    assert_eq!(compact(&reader(&frozen)), compact(probe_reader));
    for actual_call in [
        "self.registry.inspect(",
        "self.registry.directory.revalidate()?",
        "self.registry.directory.inventory()?",
        "entry.record.revalidate(",
        "entry.marker.revalidate(",
        "record.bind(",
        "Directory::expected_inventory(",
    ] {
        assert!(compact(&controls).contains(actual_call));
    }
    assert!(controls.contains("fixture.run(Phase::Expected).unwrap()"));
    assert!(controls.contains("Err(AuthorityError::Refused)"));
}
