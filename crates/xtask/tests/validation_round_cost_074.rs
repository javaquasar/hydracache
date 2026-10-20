#[path = "support/document_scratch_guard.rs"]
mod guard;
fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn git(args: &[&str]) -> String {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root())
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().replace('\r', "")
}
#[test]
fn round_observer_recovers_all_477_prior_bodies_without_new_runtime_path() {
    let paths = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        "8ecb405b4068236970bcf72563a72ee3e7ca9c2e",
        "--",
        "tools/long-run-supervisor-074/src",
        "tools/long-run-supervisor-074/tests/support",
    ]);
    assert_eq!(paths.lines().count(), 477);
    for path in paths.lines() {
        let raw = std::fs::read_to_string(root().join(path))
            .unwrap()
            .replace('\r', "");
        let restored = crate::hc074_round_cost_view!(path, &raw);
        assert_eq!(
            crate::hc074_round_cost_view!(path, &restored),
            restored,
            "idempotent: {path}"
        );
        assert_eq!(
            restored,
            git(&[
                "show",
                &format!("8ecb405b4068236970bcf72563a72ee3e7ca9c2e:{path}")
            ]),
            "{path}"
        );
    }
    let current = git(&["ls-files", "tools/long-run-supervisor-074/src"]);
    assert_eq!(current.lines().count(), 446);
    assert_eq!(guard::normalize("unrelated", "unchanged"), "unchanged");
}
#[test]
fn round_observer_rejects_missing_duplicate_and_changed_hooks_and_preserves_guards() {
    for (file, marker) in [
        (
            "src/diagnostic_validation_trace.rs",
            "pub(crate) mod round_cost;",
        ),
        (
            "src/diagnostic_kernel_walk_trace.rs",
            "frame.borrow().report.clone()",
        ),
        ("src/diagnostic_live_walk_trace.rs", "frame.borrow().stats"),
        ("src/diagnostic_validation_frame.rs", "let step_scope ="),
        ("tests/support/consuming_held_cost.rs", "let round_scope ="),
    ] {
        let path = format!("tools/long-run-supervisor-074/{file}");
        let raw = std::fs::read_to_string(root().join(&path))
            .unwrap()
            .replace('\r', "");
        assert_eq!(raw.matches(marker).count(), 1, "required current observer");
        for changed in [raw.replacen(marker, "missing", 1), format!("{raw}{raw}")] {
            assert!(std::panic::catch_unwind(|| crate::hc074_round_cost_view!(
                path.as_str(),
                &changed
            ))
            .is_err());
        }
        let old = crate::hc074_round_cost_view!(path.as_str(), &raw);
        let changed = raw.replacen("use ", "removed ", 1);
        assert_ne!(crate::hc074_round_cost_view!(path.as_str(), &changed), old);
    }
}
