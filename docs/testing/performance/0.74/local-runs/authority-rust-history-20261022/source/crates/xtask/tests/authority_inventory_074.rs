#[path = "support/document_scratch_guard.rs"]
mod guard;
#[path = "support/authority_history_io.rs"]
mod history;
fn read(path: &str) -> String {
    let raw = history::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path),
    )
    .unwrap()
    .replace('\r', "");
    crate::hc074_authority_name_capacity_view!(path, &raw)
}
#[test]
fn authority_inventory_exact_view_restores_baseline_and_rejects_unknown_or_duplicate_hooks() {
    for path in [
        "tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs",
        "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs",
        "docs/testing/gated-test-registry.toml",
    ] {
        let raw = read(path);
        let marker = if path.ends_with("fixture.rs") {
            "pub(crate) mod inventory_controls;"
        } else if path.ends_with(".rs") {
            "mod authority_inventory_alloc;"
        } else {
            "test = \"authority_inventory_allocation_attribution\""
        };
        assert_eq!(raw.matches(marker).count(), 1);
        let output = history::command()
            .args([
                "show",
                &format!("85cec4a666e777c57a6cf05bfa4b4ec2641c9b39:{path}"),
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        let before = String::from_utf8(output.stdout).unwrap().replace('\r', "");
        let restored = crate::hc074_authority_inventory_view!(path, &raw);
        assert_eq!(restored, before);
        assert_eq!(
            crate::hc074_authority_inventory_view!(path, &restored),
            restored
        );
        for changed in [raw.replacen(marker, "changed", 1), format!("{raw}{raw}")] {
            assert!(
                std::panic::catch_unwind(|| crate::hc074_authority_inventory_view!(path, &changed))
                    .is_err()
            );
        }
        if path.ends_with("fixture.rs") {
            for predicate in [
                "!seen.insert(name)",
                "seen.len() > MAX_GENERATIONS * 2",
                "seen.len() % 2 != 0",
                "fs::read_dir",
            ] {
                let changed = raw.replacen(predicate, "unknown", 1);
                assert!(
                    std::panic::catch_unwind(|| crate::hc074_authority_inventory_view!(
                        path, &changed
                    ))
                    .is_err()
                );
            }
        }
    }
    assert_eq!(guard::normalize("unrelated", "unchanged"), "unchanged");
}
#[test]
fn authority_inventory_contract_preserves_observed_prefix_formatter_and_deadlines() {
    let contract: toml::Value = toml::from_str(&read(
        "docs/testing/performance/0.74/authority-expected-inventory-contract.toml",
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
    let old = read("crates/xtask/tests/support/authority_inventory_old.txt");
    let new = read("crates/xtask/tests/support/authority_inventory_new.txt");
    let raw = read("tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs");
    let selected = if contract["candidate_enabled"].as_bool().unwrap() {
        &new
    } else {
        &old
    };
    assert_eq!(raw.matches(selected.trim_end_matches('\n')).count(), 1);
    let prefix = "        let generations = seen.len() / 2;\n";
    assert_eq!(
        old.split(prefix).next().unwrap(),
        new.split(prefix).next().unwrap()
    );
    assert!(new.contains(".eq((1..=generations)"));
    assert!(new.contains("[name(revision, \"commit\"), name(revision, \"json\")]"));
    assert!(raw.contains("format!(\"{revision:016x}.{suffix}\")"));
    let controls =
        read("tools/long-run-supervisor-074/tests/support/authority_inventory_controls.rs");
    let compact = |text: &str| text.split_whitespace().collect::<String>();
    let before_scan = old
        .split("let mut seen")
        .nth(1)
        .unwrap()
        .split(prefix.trim())
        .next()
        .unwrap();
    let frozen_scan = controls
        .split("let mut seen")
        .nth(1)
        .unwrap()
        .split(prefix.trim())
        .next()
        .unwrap()
        .replace("leaf.as_raw_fd()", "self.leaf.as_raw_fd()");
    assert_eq!(compact(before_scan), compact(&frozen_scan));
    let before_suffix = old
        .split("    let expected:")
        .nth(1)
        .unwrap()
        .split("        if seen != expected")
        .next()
        .unwrap();
    let reference_suffix = controls
        .split("    let expected:")
        .nth(1)
        .unwrap()
        .split("    *seen == expected")
        .next()
        .unwrap();
    assert_eq!(compact(before_suffix), compact(reference_suffix));
    let proposed_suffix = controls
        .split("pub(crate) fn proposed")
        .nth(1)
        .unwrap()
        .split("pub(crate) fn observed")
        .next()
        .unwrap();
    let expected_expression = new
        .split(prefix)
        .nth(1)
        .unwrap()
        .strip_prefix("        if !")
        .unwrap()
        .split("\n        {\n")
        .next()
        .unwrap();
    assert!(compact(proposed_suffix).contains(&compact(expected_expression)));
    assert!(controls.contains("let mut seed = 0x0741_1e17_u64;"));
}
