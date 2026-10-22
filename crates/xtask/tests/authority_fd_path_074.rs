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
fn authority_fd_path_exact_view_preserves_baseline_and_refuses_drift() {
    for path in [
        "tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs",
        "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs",
        "docs/testing/gated-test-registry.toml",
    ] {
        let raw = read(path);
        let output = history::command()
            .args([
                "show",
                &format!("d90228c71288a4e45076fb12785cec1ba91ab089:{path}"),
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        let baseline = String::from_utf8(output.stdout).unwrap().replace('\r', "");
        let restored = crate::hc074_authority_fd_path_view!(path, &raw);
        assert_eq!(restored, baseline);
        assert_eq!(
            crate::hc074_authority_fd_path_view!(path, &restored),
            restored
        );
        let marker = if path.ends_with("fixture.rs") {
            "pub(crate) mod fd_path_controls;"
        } else if path.ends_with("alloc_tests.rs") {
            "mod authority_fd_path_alloc;"
        } else {
            "test = \"authority_fd_path_allocation_attribution\""
        };
        for changed in [raw.replacen(marker, "changed", 1), format!("{raw}{raw}")] {
            assert!(
                std::panic::catch_unwind(|| crate::hc074_authority_fd_path_view!(path, &changed))
                    .is_err()
            );
        }
        let unrelated = format!("{raw}\n// unrelated drift\n");
        assert!(
            crate::hc074_authority_fd_path_view!(path, &unrelated).contains("// unrelated drift")
        );
    }
    let path = "tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs";
    let raw = read(path);
    let old = read("crates/xtask/tests/support/authority_fd_path_old.txt");
    let new = read("crates/xtask/tests/support/authority_fd_path_new.txt");
    let candidate = raw.replace(old.trim_end_matches('\n'), new.trim_end_matches('\n'));
    for changed in [
        candidate.replace("[0_u8; 25]", "[0_u8; 24]"),
        candidate.replace("fs::read_dir(path)?", "fs::read_dir(path).unwrap()"),
        format!("{candidate}\n{}", new.trim_end_matches('\n')),
    ] {
        assert!(
            std::panic::catch_unwind(|| crate::hc074_authority_fd_path_view!(path, &changed))
                .is_err()
        );
    }
    assert_eq!(guard::normalize("unrelated", "unchanged"), "unchanged");
}
#[test]
fn authority_fd_path_contract_keeps_fresh_scan_and_exact_formatter() {
    let contract: toml::Value = toml::from_str(&read(
        "docs/testing/performance/0.74/authority-fd-path-contract.toml",
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
    let old = read("crates/xtask/tests/support/authority_fd_path_old.txt");
    let new = read("crates/xtask/tests/support/authority_fd_path_new.txt");
    let raw = read("tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs");
    let selected = if contract["candidate_enabled"].as_bool().unwrap() {
        &new
    } else {
        &old
    };
    assert_eq!(raw.matches(selected.trim_end_matches('\n')).count(), 1);
    let controls =
        read("tools/long-run-supervisor-074/tests/support/authority_fd_path_controls.rs");
    assert!(controls.contains("Cursor::new([0_u8; 25])"));
    assert!(controls.contains("write!(&mut path, \"/proc/self/fd/{fd}\")"));
    assert!(controls.contains("0x074f_d025_u64"));
    assert!(new.contains("fs::read_dir(path)?"));
    assert!(new.contains("self.leaf.as_raw_fd()"));
}
