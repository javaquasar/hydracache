#[path = "support/document_scratch_guard.rs"]
mod guard;
fn read(path: &str) -> String {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path),
    )
    .unwrap()
    .replace('\r', "")
}
#[test]
fn authority_leaf_name_restores_previous_sources_and_refuses_unknown_implementations() {
    for path in [
        "tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs",
        "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs",
        "docs/testing/gated-test-registry.toml",
    ] {
        let raw = read(path);
        let marker = if path.ends_with("fixture.rs") {
            "pub(crate) mod leaf_name_controls;"
        } else if path.ends_with(".rs") {
            "mod authority_leaf_name_alloc;"
        } else {
            "test = \"authority_leaf_name_allocation_attribution\""
        };
        assert_eq!(raw.matches(marker).count(), 1);
        let output = std::process::Command::new("git")
            .args([
                "show",
                &format!("8a605558e8e773df2c0e6f09bfa23364ada158f1:{path}"),
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        let before = String::from_utf8(output.stdout).unwrap().replace('\r', "");
        let restored = crate::hc074_authority_leaf_name_view!(path, &raw);
        assert_eq!(restored, before);
        assert_eq!(
            crate::hc074_authority_leaf_name_view!(path, &restored),
            restored
        );
        for changed in [raw.replacen(marker, "changed", 1), format!("{raw}{raw}")] {
            assert!(
                std::panic::catch_unwind(|| crate::hc074_authority_leaf_name_view!(path, &changed))
                    .is_err()
            );
        }
    }
    assert_eq!(guard::normalize("unrelated", "unchanged"), "unchanged");
}
#[test]
fn authority_leaf_name_contract_keeps_full_read_reference_and_original_drop_scope() {
    let contract: toml::Value = toml::from_str(&read(
        "docs/testing/performance/0.74/authority-leaf-name-contract.toml",
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
    let root = read("tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs");
    let old = read("crates/xtask/tests/support/authority_leaf_name_old.txt");
    let new = read("crates/xtask/tests/support/authority_leaf_name_new.txt");
    let selected = if contract["candidate_enabled"].as_bool().unwrap() {
        &new
    } else {
        &old
    };
    assert_eq!(root.matches(selected.trim_end_matches('\n')).count(), 1);
    let controls =
        read("tools/long-run-supervisor-074/tests/support/authority_leaf_name_controls.rs");
    let reference = controls
        .split("fn reference_open")
        .nth(1)
        .unwrap()
        .split("fn proposed_read")
        .next()
        .unwrap();
    let previous = old
        .split("fn open")
        .nth(1)
        .unwrap()
        .split("fn revalidate")
        .next()
        .unwrap()
        .replace("Result<Self,", "Result<Leaf,")
        .replace("Ok(Self {", "Ok(Leaf {");
    let compact = |text: &str| text.split_whitespace().collect::<String>();
    assert_eq!(compact(reference), compact(&previous));
    let proposed = controls
        .split("let mut file = open_at(dir, name, false)?;")
        .nth(1)
        .unwrap()
        .split("Ok((file, stamp, raw))")
        .next()
        .unwrap();
    let candidate = new
        .split("let mut file = open_at(dir, name, false)?;")
        .nth(1)
        .unwrap()
        .split("Ok((file, stamp, raw))")
        .next()
        .unwrap();
    assert_eq!(compact(proposed), compact(candidate));
    assert!(new.contains("let named = Self::read(dir, &self.name, uid, gid, budget)?;"));
    assert!(new.contains("if named.1 != self.stamp || named.2 != self.raw"));
    assert!(controls.contains("let mut seed = 0x0741_eaf5_u64;"));
    assert!(controls.contains("for fault in 0..12"));
}
