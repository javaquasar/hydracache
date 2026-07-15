use std::error::Error;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Instant;

use serde::Serialize;
use sha2::{Digest, Sha256};

pub const PINNED_NIGHTLY: &str = "nightly-2026-07-01";
const TARGET: &str = "x86_64-unknown-linux-gnu";
const REQUIRE_ENV: &str = "HYDRACACHE_REQUIRE_TSAN";
const SUPPRESSIONS_PATH: &str = "docs/testing/tsan-suppressions.txt";
const CI_PREBUILD_COMMAND: &str = "cargo +nightly-2026-07-01 test -Zbuild-std --target x86_64-unknown-linux-gnu --locked --no-run -p hydracache -p hydracache-cluster-raft --test cache_core_concurrency_matrix --test leadership_handoff --test snapshot_delivery_chaos --test tsan_canary";

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum Scope {
    Suites,
    Canary,
}

#[derive(Serialize)]
struct TSanEvidence {
    schema_version: u32,
    release: &'static str,
    source_commit: String,
    toolchain: &'static str,
    target: &'static str,
    scope: Scope,
    expected_race_detected: bool,
    normalized_signature: String,
    suppressions_sha256: String,
    output_sha256: String,
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let (root, scope) = parse_args(args)?;
    structural_check(&root)?;
    if std::env::consts::OS != "linux" {
        return unavailable(format!("ThreadSanitizer requires Linux {TARGET}"));
    }
    if !toolchain_available(&root) {
        return unavailable(format!(
            "pinned toolchain {PINNED_NIGHTLY} with rust-src is unavailable"
        ));
    }
    match scope {
        Scope::Suites => run_suites(&root),
        Scope::Canary => run_canary(&root),
    }
}

pub fn structural_check(root: &Path) -> Result<(), Box<dyn Error>> {
    let fixture =
        fs::read_to_string(root.join("crates/hydracache-cluster-raft/tests/tsan_canary.rs"))?;
    if !fixture.contains("canary_tsan_detects_test_fixture_data_race")
        || !fixture.contains("UnsafeCell")
        || !fixture.contains("#[ignore")
    {
        return Err("TSan race canary is missing or can run in the ordinary test tier".into());
    }
    let workflow = fs::read_to_string(root.join(".github/workflows/ci.yml"))?;
    if !workflow.contains(PINNED_NIGHTLY) || !workflow.contains("thread-sanitizer") {
        return Err("CI is not wired to the pinned ThreadSanitizer lane".into());
    }
    for required in ["Prebuild TSan proof targets", CI_PREBUILD_COMMAND] {
        if !workflow.contains(required) {
            return Err(format!("TSan CI prebuild is missing `{required}`").into());
        }
    }
    let suppressions = fs::read_to_string(root.join(SUPPRESSIONS_PATH))?;
    if !suppressions.contains("Owner: release 0.64 W16/W26")
        || !suppressions.contains("Moka 0.12.15")
        || !suppressions.contains("cannot model memory fences")
    {
        return Err("TSan suppression must retain its reviewed owner and fence rationale".into());
    }
    let active_suppressions = suppressions
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect::<Vec<_>>();
    if active_suppressions != ["race:moka::common::concurrent::arc::MiniArc"] {
        return Err(
            "TSan suppressions must contain only the reviewed Moka MiniArc fence rule".into(),
        );
    }
    Ok(())
}

pub fn canary_output_is_expected_red(success: bool, output: &str) -> bool {
    !success
        && (output.contains("WARNING: ThreadSanitizer: data race")
            || output.contains("ThreadSanitizer: data race"))
}

fn run_suites(root: &Path) -> Result<(), Box<dyn Error>> {
    let tsan_options = tsan_options(root)?;
    for (name, args) in [
        (
            "cache_core_concurrency_matrix",
            vec![
                "-p",
                "hydracache",
                "--test",
                "cache_core_concurrency_matrix",
            ],
        ),
        (
            "leadership_handoff",
            vec![
                "-p",
                "hydracache-cluster-raft",
                "--test",
                "leadership_handoff",
            ],
        ),
        (
            "snapshot_delivery_chaos",
            vec![
                "-p",
                "hydracache-cluster-raft",
                "--test",
                "snapshot_delivery_chaos",
            ],
        ),
    ] {
        println!("tsan-check: START suite={name}");
        io::stdout().flush()?;
        let started = Instant::now();
        let output = cargo_tsan(root, &args, &[("TSAN_OPTIONS", tsan_options.as_str())])?;
        if !output.status.success() {
            return unexpected(name, &output);
        }
        println!(
            "tsan-check: PASS suite={name} elapsed_seconds={:.3}",
            started.elapsed().as_secs_f64()
        );
        io::stdout().flush()?;
    }
    write_artifact(
        root,
        Scope::Suites,
        false,
        "no-data-race",
        b"all suites passed",
    )?;
    println!("tsan-check: Suites OK");
    Ok(())
}

fn run_canary(root: &Path) -> Result<(), Box<dyn Error>> {
    let tsan_options = tsan_options(root)?;
    println!("tsan-check: START canary=isolated_data_race");
    io::stdout().flush()?;
    let started = Instant::now();
    let output = cargo_tsan(
        root,
        &[
            "-p",
            "hydracache-cluster-raft",
            "--test",
            "tsan_canary",
            "--",
            "--ignored",
            "--exact",
            "canary_tsan_detects_test_fixture_data_race",
            "--nocapture",
        ],
        &[("TSAN_OPTIONS", tsan_options.as_str())],
    )?;
    let text = combined(&output);
    if !canary_output_is_expected_red(output.status.success(), &text) {
        return unexpected("canary", &output);
    }
    println!(
        "tsan-check: PASS canary=isolated_data_race elapsed_seconds={:.3}",
        started.elapsed().as_secs_f64()
    );
    io::stdout().flush()?;
    write_artifact(
        root,
        Scope::Canary,
        true,
        "ThreadSanitizer: data race",
        text.as_bytes(),
    )?;
    println!("tsan-check: Canary expected-red OK");
    Ok(())
}

fn tsan_options(root: &Path) -> Result<String, Box<dyn Error>> {
    let suppressions = root.join(SUPPRESSIONS_PATH).canonicalize()?;
    Ok(format!(
        "halt_on_error=1:exitcode=66:suppressions={}",
        suppressions.display()
    ))
}

fn cargo_tsan(root: &Path, args: &[&str], envs: &[(&str, &str)]) -> Result<Output, Box<dyn Error>> {
    let mut command = Command::new("cargo");
    command
        .arg(format!("+{PINNED_NIGHTLY}"))
        .args(["test", "-Zbuild-std", "--target", TARGET, "--locked"])
        .args(args)
        .env("RUSTFLAGS", "-Zsanitizer=thread")
        .env("RUSTDOCFLAGS", "-Zsanitizer=thread")
        .envs(envs.iter().copied())
        .current_dir(root);
    Ok(command.output()?)
}

fn toolchain_available(root: &Path) -> bool {
    Command::new("rustup")
        .args(["run", PINNED_NIGHTLY, "rustc", "--version"])
        .current_dir(root)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn unavailable(message: String) -> Result<(), Box<dyn Error>> {
    if std::env::var(REQUIRE_ENV).as_deref() == Ok("1") {
        Err(message.into())
    } else {
        println!("tsan-check: SKIP {message}; set {REQUIRE_ENV}=1 to fail");
        Ok(())
    }
}

fn unexpected(kind: &str, output: &Output) -> Result<(), Box<dyn Error>> {
    let text = combined(output);
    eprintln!("{text}");
    Err(format!("TSan {kind} produced an unexpected result").into())
}

fn combined(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn write_artifact(
    root: &Path,
    scope: Scope,
    expected_race_detected: bool,
    signature: &str,
    output: &[u8],
) -> Result<(), Box<dyn Error>> {
    let artifact = TSanEvidence {
        schema_version: 2,
        release: "0.64",
        source_commit: git_commit(root),
        toolchain: PINNED_NIGHTLY,
        target: TARGET,
        scope,
        expected_race_detected,
        normalized_signature: signature.to_owned(),
        suppressions_sha256: sha256_hex(&fs::read(root.join(SUPPRESSIONS_PATH))?),
        output_sha256: sha256_hex(output),
    };
    let name = match scope {
        Scope::Suites => "tsan-suites.json",
        Scope::Canary => "tsan-canary.json",
    };
    let path = root.join("target/test-evidence/0.64").join(name);
    fs::create_dir_all(path.parent().expect("artifact parent"))?;
    fs::write(path, serde_json::to_vec_pretty(&artifact)?)?;
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
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

fn parse_args(args: Vec<String>) -> Result<(PathBuf, Scope), Box<dyn Error>> {
    let mut root = crate::doc_check::find_repo_root()?;
    let mut scope = None;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => root = PathBuf::from(args.next().ok_or("--root requires a path")?),
            "--scope" => {
                scope = Some(match args.next().as_deref() {
                    Some("suites") => Scope::Suites,
                    Some("canary") => Scope::Canary,
                    value => return Err(format!("invalid TSan scope: {value:?}").into()),
                })
            }
            other => return Err(format!("unknown tsan-check argument: {other}").into()),
        }
    }
    Ok((root, scope.ok_or("tsan-check requires --scope")?))
}
