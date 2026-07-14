use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde::Serialize;
use sha2::{Digest, Sha256};

pub const PINNED_NIGHTLY: &str = "nightly-2026-07-01";
const REQUIRE_ENV: &str = "HYDRACACHE_REQUIRE_MIRI";

#[derive(Serialize)]
struct MiriEvidence {
    schema_version: u32,
    release: &'static str,
    source_commit: String,
    toolchain: &'static str,
    target_os: &'static str,
    suites: Vec<&'static str>,
    normalized_result: &'static str,
    output_sha256: String,
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let root = parse_args(args)?;
    structural_check(&root)?;
    if !toolchain_available(&root) {
        return unavailable(format!(
            "pinned toolchain {PINNED_NIGHTLY} with Miri is unavailable"
        ));
    }
    let commands = [
        vec![
            "test",
            "-p",
            "hydracache-cluster-raft",
            "--test",
            "snapshot_immutability",
            "--locked",
            "miri_snapshot_store_returns_deep_cloned_export",
        ],
        vec![
            "test",
            "-p",
            "hydracache-cluster-raft",
            "--features",
            "test-failpoints",
            "--test",
            "snapshot_apply",
            "--locked",
            "miri_snapshot_apply_rejects_inconsistent_indexes_without_tokio_runtime",
        ],
    ];
    let mut output_bytes = Vec::new();
    for args in commands {
        let output = cargo_miri(&root, &args)?;
        output_bytes.extend_from_slice(&output.stdout);
        output_bytes.extend_from_slice(&output.stderr);
        if !output.status.success() {
            return unexpected(&output);
        }
    }
    write_artifact(&root, &output_bytes)?;
    println!("miri-check: OK (2 scoped snapshot proofs)");
    Ok(())
}

pub fn structural_check(root: &Path) -> Result<(), Box<dyn Error>> {
    let immutability = fs::read_to_string(
        root.join("crates/hydracache-cluster-raft/tests/snapshot_immutability.rs"),
    )?;
    let apply =
        fs::read_to_string(root.join("crates/hydracache-cluster-raft/tests/snapshot_apply.rs"))?;
    if !immutability.contains("miri_snapshot_store_returns_deep_cloned_export")
        || !apply.contains("miri_snapshot_apply_rejects_inconsistent_indexes_without_tokio_runtime")
    {
        return Err("Miri-safe snapshot proof functions are missing".into());
    }
    let workflow = fs::read_to_string(root.join(".github/workflows/ci.yml"))?;
    for required in [
        PINNED_NIGHTLY,
        "Run commit-bound Miri snapshot proofs",
        "evidence-run --release 0.64 --gate tool.miri.snapshot-safety",
        "target/test-evidence/0.64/miri-snapshot-safety.json",
    ] {
        if !workflow.contains(required) {
            return Err(format!("Miri CI wiring is missing `{required}`").into());
        }
    }
    Ok(())
}

fn cargo_miri(root: &Path, args: &[&str]) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new("cargo")
        .arg(format!("+{PINNED_NIGHTLY}"))
        .arg("miri")
        .args(args)
        .env("CARGO_BUILD_JOBS", "2")
        .current_dir(root)
        .output()?)
}

fn toolchain_available(root: &Path) -> bool {
    let toolchains = Command::new("rustup")
        .args(["toolchain", "list"])
        .current_dir(root)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .is_some_and(|output| output.lines().any(|line| line.starts_with(PINNED_NIGHTLY)));
    if !toolchains {
        return false;
    }
    Command::new("rustup")
        .args([
            "component",
            "list",
            "--toolchain",
            PINNED_NIGHTLY,
            "--installed",
        ])
        .current_dir(root)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .is_some_and(|output| output.lines().any(|line| line.starts_with("miri-")))
}

fn unavailable(message: String) -> Result<(), Box<dyn Error>> {
    if std::env::var(REQUIRE_ENV).as_deref() == Ok("1") {
        Err(message.into())
    } else {
        println!("miri-check: SKIP {message}; set {REQUIRE_ENV}=1 to fail");
        Ok(())
    }
}

fn unexpected(output: &Output) -> Result<(), Box<dyn Error>> {
    eprint!("{}", String::from_utf8_lossy(&output.stdout));
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    Err("Miri snapshot proof failed".into())
}

fn write_artifact(root: &Path, output: &[u8]) -> Result<(), Box<dyn Error>> {
    let evidence = MiriEvidence {
        schema_version: 1,
        release: "0.64",
        source_commit: git_commit(root),
        toolchain: PINNED_NIGHTLY,
        target_os: std::env::consts::OS,
        suites: vec![
            "snapshot_immutability::miri_snapshot_store_returns_deep_cloned_export",
            "snapshot_apply::miri_snapshot_apply_rejects_inconsistent_indexes_without_tokio_runtime",
        ],
        normalized_result: "no-undefined-behavior",
        output_sha256: Sha256::digest(output)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    };
    let path = root.join("target/test-evidence/0.64/miri-snapshot-safety.json");
    fs::create_dir_all(path.parent().expect("Miri artifact parent"))?;
    fs::write(path, serde_json::to_vec_pretty(&evidence)?)?;
    Ok(())
}

fn git_commit(root: &Path) -> String {
    Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_owned())
        .unwrap_or_else(|| "unknown".to_owned())
}

fn parse_args(args: Vec<String>) -> Result<PathBuf, Box<dyn Error>> {
    let mut root = crate::doc_check::find_repo_root()?;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => root = PathBuf::from(args.next().ok_or("--root requires a path")?),
            other => return Err(format!("unknown miri-check argument: {other}").into()),
        }
    }
    Ok(root)
}
