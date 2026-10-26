//! Enumerate the complete current gate frontier, never stop at its first drift.
#[path = "../../../tools/authority-rust-history-074/contract.rs"]
mod contract;
#[path = "support/current_tree_input.rs"]
mod current_inputs;
#[path = "../../../tools/authority-runtime-074/gate.rs"]
mod runtime;
#[path = "../../../tools/authority-runtime-074/io.rs"]
mod runtime_io;
#[path = "../../../tools/authority-storage-source-074/contract.rs"]
mod storage_contract;
use std::collections::BTreeSet;

fn git(args: &[&str]) -> String {
    let result = std::process::Command::new("git")
        .arg("--no-replace-objects")
        .args(args)
        .current_dir(runtime::root())
        .output()
        .unwrap();
    assert!(result.status.success());
    String::from_utf8(result.stdout).unwrap()
}
fn protected(path: &str, previous: bool) -> bool {
    (previous && (path.starts_with("tools/") || path.starts_with("crates/")))
        || path.starts_with("tools/long-run-supervisor-074/src/")
        || path.starts_with("tools/long-run-supervisor-074/tests/")
        || matches!(
            path,
            "Cargo.toml"
                | "Cargo.lock"
                | "docs/COMPAT.md"
                | "docs/plans/releases.toml"
                | "docs/testing/gated-test-registry.toml"
        )
        || (previous
            && path.starts_with("docs/testing/performance/0.74/")
            && path.ends_with(".toml"))
}
#[test]
fn current_tree_input_frontier_is_fully_accounted_without_path_exclusions() {
    let scope = [
        "tools",
        "crates",
        "Cargo.toml",
        "Cargo.lock",
        "docs/COMPAT.md",
        "docs/plans/releases.toml",
        "docs/testing/gated-test-registry.toml",
        "docs/testing/performance/0.74",
    ];
    let mut args = vec!["ls-tree", "-r", "--name-only", contract::SOURCE, "--"];
    args.extend(scope);
    let previous: BTreeSet<_> = git(&args).lines().map(str::to_owned).collect();
    let mut args = vec!["diff", "--name-only", contract::SOURCE, "--"];
    args.extend(scope);
    let changed = git(&args);
    let mut refused = Vec::new();
    for path in changed
        .lines()
        .filter(|p| protected(p, previous.contains(*p)))
    {
        if !previous.contains(path) {
            if frontier::known_addition(path) {
                frontier::require_addition(path);
            } else {
                refused.push(format!("new runtime: {path}"));
            }
            continue;
        }
        let metadata = std::fs::symlink_metadata(runtime::root().join(path)).unwrap();
        assert!(metadata.is_file() && !metadata.file_type().is_symlink());
        let current = std::panic::catch_unwind(|| frontier::read(path));
        let Ok(current) = current else {
            refused.push(format!("input refused: {path}"));
            continue;
        };
        let old = runtime::frozen(contract::SOURCE, path);
        let guard = contract::GUARDS
            .iter()
            .find(|(name, _, _)| path == format!("crates/xtask/tests/{name}.rs"));
        let accepted = if let Some((_, reads, commands)) = guard {
            contract::accepts(&old, &current, *reads, *commands)
        } else {
            current == old
        };
        if !accepted {
            refused.push(format!("body: {path}"));
        }
    }
    for refusal in &refused {
        eprintln!("CURRENT_TREE_FRONTIER {refusal}");
    }
    assert!(
        refused.is_empty(),
        "{} current-tree inputs remain",
        refused.len()
    );
}

#[path = "../../../tools/current-frontier-074/reader.rs"]
mod frontier;
