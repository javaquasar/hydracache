use serde_json::{json, Value as JsonValue};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

const RELEASE: &str = "0.73";
const PROFILE: &str = "local-screening-073-v1";
const ENVIRONMENT_CLASS: &str = "local_screening";
const CONTEXT_SCHEMA: &str = "docs/testing/performance/0.73/local-screening-context-v1.schema.json";

pub fn run_context(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let options = Options::parse(args)?;
    if options.release != RELEASE {
        return Err(format!("unsupported local performance release {}", options.release).into());
    }

    let source_sha = command_text(&options.root, "git", &["rev-parse", "HEAD"])?;
    let dirty_worktree = !command_text(
        &options.root,
        "git",
        &["status", "--porcelain", "--untracked-files=normal"],
    )?
    .is_empty();
    let identity = observed_identity(&options.root)?;
    let context = build_context(&source_sha, dirty_worktree, identity)?;
    let problems = check_context_at_root(&options.root, &context)?;
    if !problems.is_empty() {
        return Err(format!(
            "performance-local-context refused invalid output:\n- {}",
            problems.join("\n- ")
        )
        .into());
    }

    let output = options.output.unwrap_or_else(|| {
        options
            .root
            .join("target/performance-evidence/0.73/local/context.json")
    });
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output, serde_json::to_vec_pretty(&context)?)?;
    println!(
        "performance-local-context: OK ({}, non-promotable)",
        output.display()
    );
    Ok(())
}

pub fn build_context(
    source_sha: &str,
    dirty_worktree: bool,
    identity: JsonValue,
) -> Result<JsonValue, Box<dyn Error>> {
    let generated_at = OffsetDateTime::now_utc().format(&Rfc3339)?;
    Ok(build_context_at(
        source_sha,
        dirty_worktree,
        identity,
        &generated_at,
    ))
}

pub fn build_context_at(
    source_sha: &str,
    dirty_worktree: bool,
    identity: JsonValue,
    generated_at_utc: &str,
) -> JsonValue {
    let fingerprint = digest_json(&identity);
    json!({
        "schema_version": 1,
        "release": RELEASE,
        "profile_id": PROFILE,
        "environment_class": ENVIRONMENT_CLASS,
        "promotable": false,
        "numerical_claim_eligible": false,
        "generated_at_utc": generated_at_utc,
        "source": {
            "sha": source_sha,
            "dirty_worktree": dirty_worktree
        },
        "host_fingerprint_sha256": fingerprint,
        "identity": identity,
        "privacy": {
            "raw_hardware_values_retained": false,
            "excluded_fields": ["hostname", "username", "home_path", "serial_number", "mac_address"]
        },
        "limitations": [
            "local context is not dedicated-host qualification",
            "local context cannot support numerical release claims",
            "mutable load, thermal, and power observations belong in each attempt receipt"
        ]
    })
}

pub fn check_context_at_root(
    root: &Path,
    context: &JsonValue,
) -> Result<Vec<String>, Box<dyn Error>> {
    let schema: JsonValue = serde_json::from_slice(&fs::read(root.join(CONTEXT_SCHEMA))?)?;
    let validator = jsonschema::validator_for(&schema)?;
    let mut problems: Vec<_> = validator
        .iter_errors(context)
        .map(|error| format!("local context schema violation: {error}"))
        .collect();
    problems.extend(check_context(context));
    Ok(problems)
}

pub fn check_context(context: &JsonValue) -> Vec<String> {
    let mut problems = Vec::new();
    let identity = context.get("identity").unwrap_or(&JsonValue::Null);
    let expected = digest_json(identity);
    if context
        .get("host_fingerprint_sha256")
        .and_then(JsonValue::as_str)
        != Some(expected.as_str())
    {
        problems.push("local context host fingerprint does not match identity".to_owned());
    }
    if context.get("promotable").and_then(JsonValue::as_bool) != Some(false)
        || context
            .get("numerical_claim_eligible")
            .and_then(JsonValue::as_bool)
            != Some(false)
    {
        problems.push("local context must remain non-promotable".to_owned());
    }
    if context
        .pointer("/privacy/raw_hardware_values_retained")
        .and_then(JsonValue::as_bool)
        != Some(false)
    {
        problems.push("local context must not retain raw hardware values".to_owned());
    }
    problems
}

pub fn digest_json(value: &JsonValue) -> String {
    crate::memory_contracts::canonical_json_digest(value)
        .strip_prefix("sha256:")
        .expect("canonical digest prefix")
        .to_owned()
}

fn observed_identity(root: &Path) -> Result<JsonValue, Box<dyn Error>> {
    let cpu_model = observed_cpu_model();
    let rustc = command_text(root, "rustc", &["-Vv"])?;
    let cargo = command_text(root, "cargo", &["-V"])?;
    Ok(json!({
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "logical_cpus": std::thread::available_parallelism().map(|value| value.get()).unwrap_or(0),
        "cpu_model_sha256": cpu_model.as_deref().map(digest_text),
        "rustc_vv_sha256": digest_text(&rustc),
        "cargo_version_sha256": digest_text(&cargo)
    }))
}

fn observed_cpu_model() -> Option<String> {
    if cfg!(target_os = "windows") {
        return std::env::var("PROCESSOR_IDENTIFIER")
            .ok()
            .filter(|value| !value.trim().is_empty());
    }
    if cfg!(target_os = "linux") {
        return fs::read_to_string("/proc/cpuinfo").ok().and_then(|text| {
            text.lines().find_map(|line| {
                let (key, value) = line.split_once(':')?;
                (key.trim() == "model name").then(|| value.trim().to_owned())
            })
        });
    }
    Command::new("sysctl")
        .args(["-n", "machdep.cpu.brand_string"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn digest_text(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn command_text(root: &Path, program: &str, args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = Command::new(program)
        .args(args)
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "{program} {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

struct Options {
    root: PathBuf,
    release: String,
    output: Option<PathBuf>,
}

impl Options {
    fn parse(args: Vec<String>) -> Result<Self, Box<dyn Error>> {
        let mut root = crate::doc_check::find_repo_root()?;
        let mut release = None;
        let mut output = None;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--root" => root = PathBuf::from(args.next().ok_or("--root requires a path")?),
                "--release" => release = Some(args.next().ok_or("--release requires a value")?),
                "--output" => {
                    output = Some(PathBuf::from(
                        args.next().ok_or("--output requires a path")?,
                    ))
                }
                other => {
                    return Err(format!("unsupported performance local argument: {other}").into())
                }
            }
        }
        Ok(Self {
            root,
            release: release.ok_or("performance-local-context requires --release")?,
            output,
        })
    }
}
