//! `cargo xtask verify` — run the fast, no-network release gates in order and stop
//! on the first failure. This is the single command an agent or developer runs
//! before opening a PR; CI runs the same gates (see `docs/GATES.md`). Network- or
//! time-heavy suites (criterion benchmark *runs*, chaos/soak, Docker) are nightly
//! and intentionally excluded here.

use std::error::Error;
use std::process::Command;

use crate::doc_check;

/// Each gate: a human label, the `cargo` arguments, and an optional extra env var.
type Gate = (
    &'static str,
    &'static [&'static str],
    Option<(&'static str, &'static str)>,
);

const GATES: &[Gate] = &[
    ("format", &["fmt", "--all", "--", "--check"], None),
    (
        "clippy",
        &[
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
    ("dependency bans", &["deny", "check", "bans"], None),
    ("tests", &["test", "--workspace", "--locked"], None),
    (
        "docs",
        &["doc", "--workspace", "--no-deps", "--locked"],
        Some(("RUSTDOCFLAGS", "-D warnings")),
    ),
    (
        "performance budget contract",
        &["test", "-p", "xtask", "--test", "bench_budget", "--locked"],
        None,
    ),
];

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

    for &(label, args, env) in GATES {
        println!("== {label} ==");
        let mut cmd = Command::new("cargo");
        cmd.args(args).current_dir(&root);
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
