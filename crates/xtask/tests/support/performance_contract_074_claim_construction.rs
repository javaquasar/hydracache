//! Exact construction ownership correction, preserving predicates and final owner.
const GUARD: &str = r#"/// Own the successful flock until construction transfers it to the claim.
struct PendingHostClaim(Option<File>);
impl PendingHostClaim {
    fn file(&self) -> &File {
        self.0
            .as_ref()
            .expect("pending claim owns its original file")
    }
    fn into_file(mut self) -> File {
        self.0.take().expect("pending claim transfers exactly once")
    }
}
impl Drop for PendingHostClaim {
    fn drop(&mut self) {
        if let Some(lock) = &self.0 {
            let _ = FileExt::unlock(lock);
        }
    }
}

"#;
const ACQUIRE: &str = r#"        let pending = PendingHostClaim(Some(lock));
        let lock = pending.file();
        #[cfg(all(test, target_os = "linux"))]
        short_fence_tests::after_acquire(lock);
"#;
const GATE: &str =
    "\n#[path = \"support/performance_contract_074_claim_construction.rs\"]\nmod claim_construction;\n";
pub(super) fn restore_host(raw: &str) -> String {
    assert_eq!(raw.matches(GUARD).count(), 1);
    assert_eq!(raw.matches(ACQUIRE).count(), 2);
    let raw = raw.replacen(GUARD, "", 1).replace(ACQUIRE, "");
    assert_eq!(
        raw.matches("            lock: pending.into_file(),")
            .count(),
        2
    );
    let mut raw = raw.replace(
        "            lock: pending.into_file(),",
        "            lock,",
    );
    for (new, old) in [
        ("admission(&root, lock)?", "admission(&root, &lock)?"),
        (
            "admission.present(&root, lock)?",
            "admission.present(&root, &lock)?",
        ),
        (
            "admission.post_present(&root, lock)?",
            "admission.post_present(&root, &lock)?",
        ),
    ] {
        assert_eq!(raw.matches(new).count(), 2);
        raw = raw.replace(new, old);
    }
    raw
}
pub(super) fn restore_gate(raw: &str) -> String {
    assert_eq!(raw.matches(GATE).count(), 1);
    raw.replacen(GATE, "", 1)
}
fn restore_short_contract(raw: &str) -> String {
    let mut raw = raw.to_owned();
    for method in ["restore", "restore_gate"] {
        let added = format!("pub(super) fn {method}(raw: &str) -> String {{\n    let normalized = super::claim_construction::restore_{}(raw);\n    let raw = normalized.as_str();\n", if method == "restore" { "host" } else { "gate" });
        let old = format!("pub(super) fn {method}(raw: &str) -> String {{\n");
        assert_eq!(raw.matches(&added).count(), 1);
        raw = raw.replacen(&added, &old, 1);
    }
    raw
}
#[test]
fn campaign_claim_construction_preserves_admission_markers_and_successful_owner() {
    let read = |p: &str| {
        std::fs::read_to_string(super::root().join(p))
            .unwrap()
            .replace('\r', "")
    };
    let old = |p: &str| {
        let out = std::process::Command::new("git")
            .args(["show", &format!("eaebeb8f:{p}")])
            .current_dir(super::root())
            .output()
            .unwrap();
        assert!(out.status.success());
        String::from_utf8(out.stdout).unwrap().replace('\r', "")
    };
    let path = "tools/long-run-supervisor-074/src/host_execution.rs";
    let raw = read(path);
    assert_eq!(restore_host(&raw), old(path));
    for (from, to) in [
        ("let _ = FileExt::unlock(lock);", "let _ = lock;"),
        ("self.0.take().expect", "self.0.as_ref().expect"),
        (
            "let pending = PendingHostClaim(Some(lock));",
            "let pending = PendingHostClaim(None);",
        ),
        (
            "lock: pending.into_file(),",
            "lock: pending.file().try_clone().unwrap(),",
        ),
    ] {
        assert!(std::panic::catch_unwind(|| restore_host(&raw.replace(from, to))).is_err());
    }
    assert_ne!(
        restore_host(&format!("{raw}\nfn unapproved_tail() {{}}\n")),
        old(path)
    );
    let p = "tools/long-run-supervisor-074/src/host_short_fence_tests.rs";
    let tests = read(p);
    let registration = "#[path = \"host_claim_construction_tests.rs\"]\nmod claim_construction;\n";
    assert_eq!(tests.matches(registration).count(), 1);
    assert_eq!(tests.replacen(registration, "", 1), old(p));
    let p = "crates/xtask/tests/support/performance_contract_074_short_fence.rs";
    assert_eq!(restore_short_contract(&read(p)), old(p));
    let p = "crates/xtask/tests/performance_contract_074.rs";
    assert_eq!(restore_gate(&read(p)), old(p));
    for p in [
        "Cargo.toml",
        "Cargo.lock",
        "docs/COMPAT.md",
        "tools/long-run-supervisor-074/Cargo.toml",
        "docs/testing/gated-test-registry.toml",
        "docs/testing/performance/0.74/qualification-manifest.toml",
        "crates/xtask/tests/support/performance_contract_074_worker_proof.rs",
        "tools/long-run-supervisor-074/src/host_execution_fixed_admission_tests.rs",
        "tools/long-run-supervisor-074/src/diagnostic_fixed_campaign_admission.rs",
        "tools/long-run-supervisor-074/src/diagnostic_fixed_campaign_admission_tests.rs",
        "tools/long-run-supervisor-074/src/diagnostic_output_preparation.rs",
        "tools/long-run-supervisor-074/src/diagnostic_worker_policy.rs",
        "tools/long-run-supervisor-074/src/diagnostic_lease.rs",
        "tools/long-run-supervisor-074/src/diagnostic_ipc.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_fence.rs",
        "tools/long-run-supervisor-074/tests/diagnostic_ipc.rs",
        "tools/long-run-supervisor-074/tests/diagnostic_lease.rs",
        "tools/long-run-supervisor-074/tests/host_execution.rs",
        "tools/long-run-supervisor-074/tests/diagnostic_original_fence.rs",
        "docs/testing/performance/0.74/local-runs/short-fence-20261016/manifest.json",
        "docs/testing/performance/0.74/local-runs/worker-proof-reuse-v2-20261016/manifest.json",
    ] {
        assert_eq!(read(p), old(p), "unrelated input changed: {p}");
    }
    println!("claim_construction_contract original_predicates=true short_owner_unchanged=true same_file_transfer=true marker_retention=true starting=false server=false");
}
