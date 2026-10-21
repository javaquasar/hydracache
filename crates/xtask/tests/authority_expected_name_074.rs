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
fn authority_expected_name_exact_view_preserves_baseline_and_refuses_drift() {
    for path in [
        "tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs",
        "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs",
        "docs/testing/gated-test-registry.toml",
    ] {
        let raw = read(path);
        let output = std::process::Command::new("git")
            .args([
                "show",
                &format!("a3ff05f45cc4bae012f9a5f5dff93d2741c7a59d:{path}"),
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        let before = String::from_utf8(output.stdout).unwrap().replace('\r', "");
        let restored = crate::hc074_authority_expected_name_view!(path, &raw);
        assert_eq!(restored, before);
        assert_eq!(
            crate::hc074_authority_expected_name_view!(path, &restored),
            restored
        );
        let marker = if path.ends_with("fixture.rs") {
            "pub(crate) mod expected_name_controls;"
        } else if path.ends_with("alloc_tests.rs") {
            "mod authority_expected_name_alloc;"
        } else {
            "test = \"authority_expected_name_allocation_attribution\""
        };
        assert_eq!(raw.matches(marker).count(), 1);
        for changed in [raw.replacen(marker, "changed", 1), format!("{raw}{raw}")] {
            assert!(
                std::panic::catch_unwind(|| crate::hc074_authority_expected_name_view!(
                    path, &changed
                ))
                .is_err()
            );
        }
    }
    let path = "tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs";
    let raw = read(path);
    // Older adapters deliberately restore this exact historical inventory.
    let old = read("crates/xtask/tests/support/authority_expected_name_old.txt");
    let new = read("crates/xtask/tests/support/authority_expected_name_new.txt");
    let legacy = read("crates/xtask/tests/support/authority_inventory_old.txt");
    let historical = raw
        .replace(new.trim_end_matches('\n'), legacy.trim_end_matches('\n'))
        .replace(old.trim_end_matches('\n'), legacy.trim_end_matches('\n'));
    assert!(
        crate::hc074_authority_expected_name_view!(path, &historical)
            .contains(legacy.trim_end_matches('\n'))
    );
    for changed in [
        raw.replacen("fn inventory(&self)", "fn inventory(&mut self)", 1),
        format!("{raw}\nfn expected_inventory() {{}}"),
    ] {
        assert!(
            std::panic::catch_unwind(|| crate::hc074_authority_expected_name_view!(path, &changed))
                .is_err()
        );
    }
    let unrelated = raw.replacen("fn marker_bytes", "fn changed_marker", 1);
    assert!(
        crate::hc074_authority_expected_name_view!(path, &unrelated).contains("fn changed_marker")
    );
    assert_eq!(guard::normalize("unrelated", "unchanged"), "unchanged");
}
#[test]
fn authority_expected_name_contract_keeps_exact_formatter_and_closed_gates() {
    let contract: toml::Value = toml::from_str(&read(
        "docs/testing/performance/0.74/authority-expected-name-contract.toml",
    ))
    .unwrap();
    for flag in [
        "server_admitted",
        "latency_gain_claimed",
        "fresh_checks_changed",
        "deadlines_changed",
    ] {
        assert_eq!(contract[flag].as_bool(), Some(false));
    }
    let old = read("crates/xtask/tests/support/authority_expected_name_old.txt");
    let new = read("crates/xtask/tests/support/authority_expected_name_new.txt");
    let raw = read("tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs");
    let enabled = contract["candidate_enabled"].as_bool().unwrap();
    let selected = if enabled { &new } else { &old };
    assert_eq!(raw.matches(selected.trim_end_matches('\n')).count(), 1);
    let controls =
        read("tools/long-run-supervisor-074/tests/support/authority_expected_name_controls.rs");
    let compact = |text: &str| text.split_whitespace().collect::<String>();
    let proposal = controls
        .split("pub(crate) fn proposed")
        .nth(1)
        .unwrap()
        .split("#[test]")
        .next()
        .unwrap();
    let implementation = new.split("fn expected_inventory").nth(1).unwrap();
    assert_eq!(compact(proposal), compact(implementation));
    assert!(new.contains("Cursor::new([0_u8; 23])"));
    assert!(new.contains("write!(&mut expected, \"{revision:016x}.{suffix}\")"));
    assert!(new.contains("observed.next().is_none()"));
    assert!(controls.contains("let mut seed = 0x074e_1a9e_u64;"));
}
