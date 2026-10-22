#[path = "support/document_scratch_guard.rs"]
mod guard;
#[path = "support/authority_history_io.rs"]
mod history;
fn read(path: &str) -> String {
    history::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path),
    )
    .unwrap()
    .replace('\r', "")
}
#[test]
fn authority_name_capacity_exact_view_preserves_every_baseline_byte_and_old_assertion() {
    for path in [
        "tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs",
        "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs",
        "docs/testing/gated-test-registry.toml",
        "crates/xtask/tests/authority_inventory_074.rs",
    ] {
        let raw = read(path);
        let output = history::command()
            .args([
                "show",
                &format!("36f9f61ff0989340cefae2107f726b48c650c7a8:{path}"),
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        let before = String::from_utf8(output.stdout).unwrap().replace('\r', "");
        let restored = crate::hc074_authority_name_capacity_view!(path, &raw);
        assert_eq!(restored, before);
        assert_eq!(
            crate::hc074_authority_name_capacity_view!(path, &restored),
            restored
        );
        let marker = if path.ends_with("fixture.rs") {
            "pub(crate) mod name_capacity_controls;"
        } else if path.ends_with("alloc_tests.rs") {
            "mod authority_name_capacity_alloc;"
        } else if path.ends_with(".toml") {
            "test = \"authority_name_capacity_allocation_attribution\""
        } else {
            "hc074_authority_name_capacity_view!"
        };
        assert_eq!(raw.matches(marker).count(), 1);
        for changed in [raw.replacen(marker, "changed", 1), format!("{raw}{raw}")] {
            assert!(
                std::panic::catch_unwind(|| crate::hc074_authority_name_capacity_view!(
                    path, &changed
                ))
                .is_err()
            );
        }
    }
    let path = "tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs";
    let raw = read(path);
    let changed = raw.replacen("fn name(revision: usize", "fn name(revision: u16", 1);
    assert!(
        std::panic::catch_unwind(|| crate::hc074_authority_name_capacity_view!(path, &changed))
            .is_err()
    );
    let unrelated = raw.replacen("fn marker_bytes", "fn changed_marker", 1);
    assert!(
        crate::hc074_authority_name_capacity_view!(path, &unrelated).contains("fn changed_marker")
    );
    assert_eq!(guard::normalize("unrelated", "unchanged"), "unchanged");
}
#[test]
fn authority_name_capacity_contract_keeps_formatter_and_all_admission_gates_closed() {
    let contract: toml::Value = toml::from_str(&read(
        "docs/testing/performance/0.74/authority-name-capacity-contract.toml",
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
    let old = read("crates/xtask/tests/support/authority_name_capacity_old.txt");
    let new = read("crates/xtask/tests/support/authority_name_capacity_new.txt");
    let raw = read("tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs");
    let selected = if contract["candidate_enabled"].as_bool().unwrap() {
        &new
    } else {
        &old
    };
    assert_eq!(raw.matches(selected.trim_end_matches('\n')).count(), 1);
    let controls =
        read("tools/long-run-supervisor-074/tests/support/authority_name_capacity_controls.rs");
    let compact = |text: &str| text.split_whitespace().collect::<String>();
    let reference = controls
        .split("fn reference")
        .nth(1)
        .unwrap()
        .split("fn proposed")
        .next()
        .unwrap();
    let proposed = controls
        .split("fn proposed")
        .nth(1)
        .unwrap()
        .split("pub(crate) fn render")
        .next()
        .unwrap();
    assert_eq!(compact(reference), compact(&old.replacen("fn name", "", 1)));
    assert_eq!(compact(proposed), compact(&new.replacen("fn name", "", 1)));
    assert!(new.contains("String::with_capacity(17 + suffix.len())"));
    assert!(new.contains("write!(&mut output, \"{revision:016x}.{suffix}\")"));
    assert!(controls.contains("let mut seed = 0x074c_a9ac_u64;"));
}
