//! Exact test-only argument recording; no change to any original live operation.
#[path = "performance_contract_074_boot_comparison.rs"]
mod boot_comparison;
const PREFIX: &str = "tools/long-run-supervisor-074/src/";
const HOOKS: &[(&str,&str,&str)] = &[
    ("diagnostic_collector_causal.rs", "#[cfg_attr(test, track_caller)]\n", ""),
    ("diagnostic_collector_causal.rs", "    #[cfg(test)]\n    time_observation::closing::observe(issued, now);\n", ""),
    ("diagnostic_causal_time_observation.rs", "\n#[path = \"diagnostic_closing_age_trace.rs\"]\npub(crate) mod closing;\n", ""),
    ("diagnostic_privileged_root_frame_tests.rs", "#[path = \"diagnostic_closing_validation_fixture.rs\"]\nmod closing;\n", ""),
    ("diagnostic_privileged_root_frame_tests.rs", "            closing::run(|| owned.revalidate()).unwrap();\n            assert_eq!(owned.fixture_origins(), origins);", "            owned.revalidate().unwrap();\n            assert_eq!(owned.fixture_origins(), origins);"),
];
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let mut text = boot_comparison::normalize(path, raw);
    for (file, addition, original) in HOOKS {
        if path == format!("{PREFIX}{file}") {
            assert_eq!(
                text.matches(addition).count(),
                1,
                "closing exact hook: {path}"
            );
            text = text.replacen(addition, original, 1);
        }
    }
    text
}
fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(path: &str) -> String {
    crate::runtime_io::read_to_string(root().join(path))
        .unwrap()
        .replace('\r', "")
}
fn git(args: &[&str]) -> String {
    let result = std::process::Command::new("git")
        .args(args)
        .current_dir(root())
        .output()
        .unwrap();
    assert!(result.status.success());
    String::from_utf8(result.stdout).unwrap().replace('\r', "")
}
#[test]
fn closing_validation_preserves_all_418_previous_runtime_and_fixture_sources() {
    let files = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        "eefcd449",
        "--",
        "tools/long-run-supervisor-074/src",
    ]);
    assert_eq!(files.lines().count(), 418);
    for path in files.lines().chain([
        "docs/COMPAT.md",
        "docs/testing/gated-test-registry.toml",
        "tools/long-run-supervisor-074/tests/owned_root_creator_sigkill.py",
    ]) {
        assert_eq!(
            normalize(path, &read(path)),
            git(&["show", &format!("eefcd449:{path}")]),
            "{path}"
        );
    }
}
#[test]
fn closing_validation_adapter_rejects_missing_duplicate_and_original_age_drift() {
    for (file, addition, _) in HOOKS {
        let path = format!("{PREFIX}{file}");
        let original = read(&path);
        for bad in [
            original.replacen(addition, "", 1),
            format!("{original}{addition}"),
        ] {
            assert!(std::panic::catch_unwind(|| normalize(&path, &bad)).is_err());
        }
    }
    let path = format!("{PREFIX}diagnostic_collector_causal.rs");
    let raw = read(&path);
    for bad in [
        raw.replace("now - issued >=", "now - issued >"),
        raw.replace("now < issued", "now <= issued"),
    ] {
        assert_ne!(
            normalize(&path, &bad),
            git(&["show", &format!("eefcd449:{path}")])
        );
    }
}
#[test]
fn closing_validation_diagnostics_have_no_new_clock_authority_or_execution() {
    let age = read(&format!("{PREFIX}diagnostic_closing_age_trace.rs"));
    for forbidden in [
        "clock_gettime",
        "timerfd",
        "File::",
        "Command::",
        "LOCK_UN",
        "Vec<",
        "unsafe",
    ] {
        assert!(!age.contains(forbidden), "{forbidden}");
    }
    assert!(age.contains("sites: [Site; 3]"));
    let fixture = read(&format!("{PREFIX}diagnostic_closing_validation_fixture.rs"));
    for required in [
        "HC074_CLOSING_DIAGNOSTICS",
        "time::closing::capture",
        "time::capture(call)",
        "trace::kernel_walk::capture",
        "trace::live_walk::capture",
        "schedule_changed",
        "deadline_changed",
    ] {
        assert!(fixture.contains(required), "{required}");
    }
    for forbidden in [
        "timerfd",
        "clock_gettime",
        "sleep(",
        "enroll_once",
        "sendmsg",
        "LOCK_UN",
    ] {
        assert!(!fixture.contains(forbidden), "{forbidden}");
    }
}
