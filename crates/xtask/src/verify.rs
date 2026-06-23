//! `cargo xtask verify` — run the fast, no-network release gates in order and stop
//! on the first failure. This is the single command an agent or developer runs
//! before opening a PR; CI runs the same gates (see `docs/GATES.md`). Network- or
//! time-heavy suites (criterion benchmark *runs*, chaos/soak, Docker) are nightly
//! and intentionally excluded here.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::doc_check;

/// A release gate: a human label, the `cargo` arguments, and optional env vars.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Gate {
    label: &'static str,
    args: Vec<&'static str>,
    env: Option<(&'static str, &'static str)>,
}

fn gate(
    label: &'static str,
    args: impl Into<Vec<&'static str>>,
    env: Option<(&'static str, &'static str)>,
) -> Gate {
    Gate {
        label,
        args: args.into(),
        env,
    }
}

fn gates_for_platform(is_windows: bool) -> Vec<Gate> {
    let mut gates = vec![
        gate("format", ["fmt", "--all", "--", "--check"], None),
        gate(
            "clippy",
            [
                "clippy",
                "--workspace",
                "--all-targets",
                "--all-features",
                "--locked",
                "--",
                "-D",
                "warnings",
            ],
            None,
        ),
        gate("dependency bans", ["deny", "check", "bans"], None),
        gate(
            "DST fast budget",
            [
                "test",
                "-p",
                "hydracache-sim",
                "--test",
                "dst_budget",
                "--locked",
            ],
            None,
        ),
    ];

    if is_windows {
        // A running `target/debug/xtask.exe` cannot be overwritten on Windows.
        // Test the rest of the workspace first, then run xtask lib/integration
        // tests without rebuilding the xtask binary target. Serializing the
        // Windows test build also avoids transient linker locks on test EXEs.
        gates.push(gate(
            "tests (workspace excluding xtask)",
            [
                "test",
                "--workspace",
                "--exclude",
                "xtask",
                "--locked",
                "-j",
                "1",
            ],
            None,
        ));
        gates.push(gate(
            "tests (xtask lib/integration)",
            [
                "test", "-p", "xtask", "--lib", "--tests", "--locked", "-j", "1",
            ],
            None,
        ));
    } else {
        gates.push(gate("tests", ["test", "--workspace", "--locked"], None));
    }

    gates.extend([
        gate(
            "docs",
            ["doc", "--workspace", "--no-deps", "--locked"],
            Some(("RUSTDOCFLAGS", "-D warnings")),
        ),
        gate(
            "performance budget contract",
            ["test", "-p", "xtask", "--test", "bench_budget", "--locked"],
            None,
        ),
    ]);

    gates
}

fn windows_verify_target_dir(root: &Path) -> PathBuf {
    root.join("target").join("xtask-verify")
}

pub fn run(_args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let root = doc_check::find_repo_root()?;

    // Cheap, no-network doc-consistency check first.
    println!("== doc-check ==");
    let problems = doc_check::check(&root)?;
    if !problems.is_empty() {
        for problem in &problems {
            eprintln!("doc-check: {problem}");
        }
        return Err(format!("doc-check found {} problem(s)", problems.len()).into());
    }
    println!("doc-check: OK");

    let is_windows = cfg!(windows);
    let windows_target_dir = is_windows.then(|| windows_verify_target_dir(&root));

    for Gate { label, args, env } in gates_for_platform(is_windows) {
        println!("== {label} ==");
        let mut cmd = Command::new("cargo");
        cmd.args(args).current_dir(&root);
        if let Some(target_dir) = &windows_target_dir {
            cmd.env("CARGO_TARGET_DIR", target_dir);
        }
        if let Some((key, value)) = env {
            cmd.env(key, value);
        }
        let status = cmd
            .status()
            .map_err(|err| format!("gate '{label}' could not start: {err}"))?;
        if !status.success() {
            return Err(format!("gate '{label}' failed").into());
        }
    }

    println!("verify: all gates passed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{gates_for_platform, windows_verify_target_dir, Gate};

    fn args_for<'a>(gates: &'a [Gate], label: &str) -> &'a [&'static str] {
        gates
            .iter()
            .find(|gate| gate.label == label)
            .map(|gate| gate.args.as_slice())
            .expect("gate exists")
    }

    #[test]
    fn non_windows_uses_single_workspace_test_gate() {
        let gates = gates_for_platform(false);

        assert_eq!(
            args_for(&gates, "tests"),
            ["test", "--workspace", "--locked"]
        );
        assert!(!gates
            .iter()
            .any(|gate| gate.label == "tests (workspace excluding xtask)"));
    }

    #[test]
    fn windows_test_gates_avoid_rebuilding_running_xtask_binary() {
        let gates = gates_for_platform(true);

        assert_eq!(
            args_for(&gates, "tests (workspace excluding xtask)"),
            [
                "test",
                "--workspace",
                "--exclude",
                "xtask",
                "--locked",
                "-j",
                "1"
            ]
        );
        assert_eq!(
            args_for(&gates, "tests (xtask lib/integration)"),
            ["test", "-p", "xtask", "--lib", "--tests", "--locked", "-j", "1"]
        );
        assert!(!gates
            .iter()
            .any(|gate| gate.label == "tests" && gate.args == ["test", "--workspace", "--locked"]));
    }

    #[test]
    fn windows_verify_target_dir_is_inside_repo_target() {
        let root = Path::new("repo");

        assert_eq!(
            windows_verify_target_dir(root),
            PathBuf::from("repo").join("target").join("xtask-verify")
        );
    }

    #[test]
    fn verify_includes_dst_fast_budget_gate() {
        let gates = gates_for_platform(false);

        assert_eq!(
            args_for(&gates, "DST fast budget"),
            [
                "test",
                "-p",
                "hydracache-sim",
                "--test",
                "dst_budget",
                "--locked"
            ]
        );
    }
}
