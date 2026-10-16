//! Exact private lifetime correction; no relaxation of admission or old tests.
const GUARD: &str = r#"/// Own only the successful short transaction's original open description.
/// No clone/extraction or release receipt; durable markers are never removed.
pub(crate) struct ShortHostFence(File);
#[cfg(all(test, target_os = "linux"))]
impl ShortHostFence {
    pub(crate) fn original_for_test(&self) -> &File {
        &self.0
    }
}
impl Drop for ShortHostFence {
    fn drop(&mut self) {
        // A pre-exec fork alias must not extend this completed transaction.
        let _ = FileExt::unlock(&self.0);
    }
}

"#;
const NEW_ACQUIRE: &str = r#"    let lock = ShortHostFence(lock);
    #[cfg(all(test, target_os = "linux"))]
    short_fence_tests::after_acquire(&lock.0);
    if admission(&root, &lock.0)?.fixed_present(&root, &lock.0)? {"#;
const REGISTRATION: &str = r#"
#[cfg(all(test, target_os = "linux"))]
#[path = "host_short_fence_tests.rs"]
mod short_fence_tests;
"#;
pub(super) fn restore(raw: &str) -> String {
    let normalized = super::claim_construction::restore_host(raw);
    let raw = normalized.as_str();
    for s in [GUARD, NEW_ACQUIRE, REGISTRATION] {
        assert_eq!(raw.matches(s).count(), 1);
    }
    raw.replacen(GUARD, "", 1)
        .replacen(REGISTRATION, "", 1)
        .replacen(NEW_ACQUIRE, "    if admission(&root, &lock)?.fixed_present(&root, &lock)? {", 1)
        .replacen(
            "pub(crate) fn lock_host_root(\n    campaign_root: &Path,\n) -> Result<(PathBuf, ShortHostFence), HostExecutionError>",
            "pub(crate) fn lock_host_root(campaign_root: &Path) -> Result<(PathBuf, File), HostExecutionError>", 1)
        .replacen(") -> Result<(PathBuf, ShortHostFence), HostExecutionError>", ") -> Result<(PathBuf, File), HostExecutionError>", 1)
}
pub(super) fn restore_gate(raw: &str) -> String {
    let normalized = super::claim_construction::restore_gate(raw);
    let raw = normalized.as_str();
    let registration =
        "\n#[path = \"support/performance_contract_074_short_fence.rs\"]\nmod short_fence;\n";
    let legacy = "    let host = short_fence::restore(&host.replace('\\r', \"\"));\n";
    assert_eq!(raw.matches(registration).count(), 1);
    assert_eq!(raw.matches(legacy).count(), 1);
    raw.replacen(registration, "", 1).replacen(legacy, "", 1)
}
#[test]
fn short_fence_owned_unlock_preserves_original_admission_and_test_assertions() {
    let read = |p: &str| {
        std::fs::read_to_string(super::root().join(p))
            .unwrap()
            .replace('\r', "")
    };
    let old = |p: &str| {
        let out = std::process::Command::new("git")
            .args(["show", &format!("166c3d1a:{p}")])
            .current_dir(super::root())
            .output()
            .unwrap();
        assert!(out.status.success());
        String::from_utf8(out.stdout).unwrap().replace('\r', "")
    };
    let p = "tools/long-run-supervisor-074/src/host_execution.rs";
    let raw = read(p);
    assert_eq!(restore(&raw), old(p));
    assert!(std::panic::catch_unwind(|| restore(
        &raw.replace("let _ = FileExt::unlock(&self.0);", "let _ = ();")
    ))
    .is_err());
    assert!(std::panic::catch_unwind(|| restore(
        &raw.replace("let lock = ShortHostFence(lock);", "let lock = lock;")
    ))
    .is_err());
    assert_ne!(
        restore(&format!("{raw}\nfn unapproved_tail() {{}}\n")),
        old(p)
    );
    let p = "tools/long-run-supervisor-074/src/host_execution_fixed_admission_tests.rs";
    let current = read(p);
    assert_eq!(current.matches("&lock.0").count(), 9);
    assert_eq!(current.replace("&lock.0", "&lock"), old(p));
    let p = "tools/long-run-supervisor-074/src/diagnostic_output_preparation.rs";
    let current = read(p);
    assert_eq!(
        current
            .matches("FileExt::unlock(fence.original_for_test()).unwrap();")
            .count(),
        1
    );
    assert_eq!(
        current.replace(
            "FileExt::unlock(fence.original_for_test()).unwrap();",
            "FileExt::unlock(&fence).unwrap();"
        ),
        old(p)
    );
    let p = "tools/long-run-supervisor-074/src/diagnostic_fixed_campaign_admission_tests.rs";
    let current = read(p);
    let restored = current
        .replace("fn fixture() -> (\n    tempfile::TempDir,\n    PathBuf,\n    crate::host_execution::ShortHostFence,\n)", "fn fixture() -> (tempfile::TempDir, PathBuf, File)")
        .replace("lock.original_for_test()", "&lock")
        .replace("let mut read =\n        FixedCampaignAdmission::modeled(temp.path(), &root, &lock).unwrap();", "let mut read = FixedCampaignAdmission::modeled(temp.path(), &root, &lock).unwrap();")
        .replace("let mut read =\n            FixedCampaignAdmission::modeled(temp.path(), &root, &lock).unwrap();", "let mut read = FixedCampaignAdmission::modeled(temp.path(), &root, &lock).unwrap();")
        .replace("assert!(\n            FixedCampaignAdmission::modeled(temp.path(), argument, &lock)\n                .is_err()\n        );", "assert!(FixedCampaignAdmission::modeled(temp.path(), argument, &lock).is_err());");
    assert_eq!(restored, old(p));
    let p = "crates/xtask/tests/performance_contract_074.rs";
    let current = read(p);
    assert_eq!(restore_gate(&current), old(p));
    let p = "crates/xtask/tests/support/performance_contract_074_worker_proof.rs";
    assert_eq!(
        read(p).replace(
            "let current_gate = super::short_fence::restore_gate(&read(gate));",
            "let current_gate = read(gate);"
        ),
        old(p)
    );
    for p in [
        "Cargo.lock",
        "Cargo.toml",
        "docs/COMPAT.md",
        "docs/testing/performance/0.74/qualification-manifest.toml",
        "tools/long-run-supervisor-074/src/diagnostic_ipc.rs",
        "tools/long-run-supervisor-074/src/diagnostic_lease.rs",
        "tools/long-run-supervisor-074/src/diagnostic_worker_policy.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_fence.rs",
        "tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs",
        "tools/long-run-supervisor-074/tests/diagnostic_ipc.rs",
        "tools/long-run-supervisor-074/tests/diagnostic_lease.rs",
        "docs/testing/performance/0.74/local-runs/worker-proof-reuse-v2-20261016/manifest.json",
    ] {
        assert_eq!(read(p), old(p), "unrelated guard/input changed: {p}");
    }
    println!("short_fence_contract original_admission=true old_assertions=true long_guards_unchanged=true server=false");
}
