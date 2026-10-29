#[path = "support/custody_cost_guard.rs"]
mod guard;
#[path = "../../../tools/authority-runtime-074/gate.rs"]
mod runtime;
#[path = "../../../tools/authority-runtime-074/io.rs"]
mod runtime_io;
#[path = "../../../tools/authority-storage-source-074/contract.rs"]
mod storage_contract;
const BASE: &str = "72efce2401e753b857f7fc03f3300b6aae19bd5d";
fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn git(args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(root())
        .output()
        .unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap().replace('\r', "")
}
fn read(path: &str) -> String {
    runtime_io::read_to_string(root().join(path))
        .unwrap()
        .replace('\r', "")
}
#[test]
fn custody_observation_hooks_reconstruct_all_preceding_469_bodies() {
    let paths = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        BASE,
        "--",
        "tools/long-run-supervisor-074/src",
        "tools/long-run-supervisor-074/tests/support",
    ]);
    assert_eq!(paths.lines().count(), 469);
    for path in paths.lines().chain([
        "Cargo.lock",
        "docs/COMPAT.md",
        "docs/testing/gated-test-registry.toml",
        "docs/testing/performance/0.74/qualification-manifest.toml",
    ]) {
        assert_eq!(
            guard::normalize(path, &read(path)),
            git(&["show", &format!("{BASE}:{path}")]),
            "{path}"
        );
    }
}
#[test]
fn custody_observer_adapter_refuses_missing_duplicate_and_preserves_unrelated_drift() {
    for (path, exact) in guard::HOOKS
        .into_iter()
        .chain([(guard::OBSERVER, guard::NEW)])
    {
        let raw = read(path);
        for bad in [raw.replacen(exact, "", 1), format!("{raw}{exact}")] {
            assert!(std::panic::catch_unwind(|| guard::normalize(path, &bad)).is_err());
        }
    }
    let path = guard::HOOKS[1].0;
    let raw = read(path);
    let changed = raw.replacen("original.inspect_custody()?;", "// dropped live check", 1);
    assert_ne!(raw, changed);
    assert_ne!(
        guard::normalize(path, &changed),
        git(&["show", &format!("{BASE}:{path}")])
    );
}
