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
        return runtime_io::read_to_string(runtime::root().join(path)).unwrap();
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
fn validation_leaf_requests_only_add_exact_test_hook_and_registered_manual_gate() {
    for path in [
        "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs",
        "docs/testing/gated-test-registry.toml",
    ] {
        let current = read(path);
        let output = std::process::Command::new("git")
            .args([
                "show",
                &format!("45b218eba1b3907acdbeab395f4dcf811b3c1291:{path}"),
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        let before = String::from_utf8(output.stdout).unwrap().replace('\r', "");
        let restored = crate::hc074_leaf_requests_view!(path, &current);
        assert_eq!(restored, before);
        assert_eq!(crate::hc074_leaf_requests_view!(path, &restored), restored);
        assert_ne!(current, before);
        let marker = if path.ends_with(".rs") {
            "mod validation_leaf_requests;"
        } else {
            "required_features = [\"fixed-leaf-alloc-probe\"]\nrequired_env = []\nrequired_tools = [\"cargo\"]\ntimeout_seconds = 1200\nowner_release = \"0.74.0\"\nship_mandatory = false\nartifacts = []\n[gate.ci]"
        };
        for changed in [
            current.replacen(marker, "changed", 1),
            format!("{current}{current}"),
        ] {
            assert!(
                std::panic::catch_unwind(|| crate::hc074_leaf_requests_view!(path, &changed))
                    .is_err()
            );
        }
    }
    assert_eq!(guard::normalize("unrelated", "unchanged"), "unchanged");
}
#[test]
fn validation_leaf_requests_keep_measurement_scopes_and_limits_explicit() {
    let raw = read("tools/long-run-supervisor-074/tests/support/validation_leaf_requests.rs");
    for required in [
        "const ITERATIONS: usize = 1024;",
        "run(32);",
        "for sample in 0..3",
        "run(64)",
        "assert_eq!(trace.nodes.map(|s| s.calls), expected);",
        "\"all_syscalls_observed\":false",
        "\"authority_fixed_ancestry\":false",
        "let ((), counts) = capture(|| run(ITERATIONS));",
        "assert_eq!(observed.syscw, 0);",
        "let empty = Io::read(&io).since(before);",
    ] {
        assert!(raw.contains(required), "{required}");
    }
    assert!(!raw.contains("Command::new"));
    assert!(!raw.contains("Instant::now"));
    assert!(!raw.contains("open_fixed_registry"));
}
