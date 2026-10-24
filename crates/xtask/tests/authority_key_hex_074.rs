#[path = "support/document_scratch_guard.rs"]
mod guard;
#[path = "../../../tools/authority-runtime-074/gate.rs"]
mod runtime;
#[path = "../../../tools/authority-runtime-074/io.rs"]
mod runtime_io;
#[path = "../../../tools/authority-storage-source-074/contract.rs"]
mod storage_contract;
fn read(path: &str) -> String {
    if path == "docs/testing/gated-test-registry.toml" {
        return runtime_io::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(path),
        )
        .unwrap();
    }
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path),
    )
    .unwrap()
    .replace('\r', "")
}
#[test]
fn authority_hex_restores_preceding_bodies_and_rejects_unknown_encoder() {
    for path in [
        "tools/long-run-supervisor-074/src/diagnostic_authority_registry.rs",
        "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs",
        "docs/testing/gated-test-registry.toml",
    ] {
        let raw = read(path);
        let required = if path.ends_with("registry.rs") {
            "pub(crate) mod hex_controls;"
        } else if path.ends_with(".rs") {
            "mod authority_hex_alloc;"
        } else {
            "# BEGIN AUTHORITY KEY HEX GATE"
        };
        assert_eq!(raw.matches(required).count(), 1);
        let output = std::process::Command::new("git")
            .args([
                "show",
                &format!("b6810311b5d43c318d6c01960359e417e5d147cd:{path}"),
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        let before = String::from_utf8(output.stdout).unwrap().replace('\r', "");
        let restored = crate::hc074_authority_hex_view!(path, &raw);
        assert_eq!(restored, before);
        assert_eq!(crate::hc074_authority_hex_view!(path, &restored), restored);
        assert_ne!(raw, before);
        let marker = if path.ends_with("registry.rs") {
            "pub(crate) mod hex_controls;"
        } else if path.ends_with(".rs") {
            "mod authority_hex_alloc;"
        } else {
            "test = \"authority_hex_allocation_attribution\""
        };
        for changed in [raw.replacen(marker, "changed", 1), format!("{raw}{raw}")] {
            assert!(
                std::panic::catch_unwind(|| crate::hc074_authority_hex_view!(path, &changed))
                    .is_err()
            );
        }
        if path.ends_with("registry.rs") {
            assert!(
                std::panic::catch_unwind(|| crate::hc074_authority_hex_view!(
                    path,
                    &raw.replacen("fn key_hex(", "fn changed(", 1)
                ))
                .is_err()
            );
        }
    }
    assert_eq!(guard::normalize("unrelated", "unchanged"), "unchanged");
}
#[test]
fn authority_hex_current_candidate_is_exact_and_reference_is_frozen() {
    let contract: toml::Value = toml::from_str(&read(
        "docs/testing/performance/0.74/authority-key-hex-contract.toml",
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
    let raw = read("tools/long-run-supervisor-074/src/diagnostic_authority_registry.rs");
    let old = "    trust\n        .key\n        .as_bytes()\n        .iter()\n        .map(|b| format!(\"{b:02x}\"))\n        .collect()";
    let body = if contract["candidate_enabled"].as_bool().unwrap() {
        "    crate::hex(trust.key.as_bytes())"
    } else {
        old
    };
    assert_eq!(
        raw.matches(&format!(
            "fn key_hex(trust: &WorkerPolicyTrust) -> String {{\n{body}\n}}"
        ))
        .count(),
        1
    );
    let reference = read("tools/long-run-supervisor-074/tests/support/authority_hex_controls.rs");
    assert!(reference.contains(&format!(
        "fn reference(trust: &WorkerPolicyTrust) -> String {{\n{old}\n}}"
    )));
    assert!(reference.contains("let mut seed = 0x074a_17e5_u64;"));
    assert!(reference.contains("for case in 0..13"));
}
