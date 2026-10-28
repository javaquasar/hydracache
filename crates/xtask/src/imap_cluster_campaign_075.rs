//! Executed actual-service schedule checks, deliberately separate from ship admission.
use hydracache_cluster_testkit::value_plane_cluster_chaos_075::{
    decode_cluster_schedule, encode_cluster_schedule, generated_cluster_schedule,
    history_canary_detected, replay_cluster_schedule, shrink_cluster_schedule, ClusterScheduleStep,
};
use serde_json::{json, Value};
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if !args.len().is_multiple_of(2) {
        return Err("cluster campaign arguments require values".into());
    }
    let mut release = None;
    let mut seed = 117;
    let mut rounds = 3;
    let mut schedule = None;
    let mut seen = std::collections::BTreeSet::new();
    for pair in args.chunks_exact(2) {
        if !seen.insert(pair[0].as_str()) {
            return Err("duplicate cluster campaign option".into());
        }
        match pair[0].as_str() {
            "--release" => release = Some(pair[1].as_str()),
            "--seed" => seed = pair[1].parse()?,
            "--rounds" => rounds = pair[1].parse()?,
            "--schedule" => schedule = Some(pair[1].as_str()),
            _ => return Err("unknown cluster campaign option".into()),
        }
    }
    if release != Some("0.75") {
        return Err("cluster campaign requires --release 0.75".into());
    }
    let root = crate::doc_check::find_repo_root()?;
    let problems = crate::imap_cluster_fuzz_policy075::check_at_root(&root)?;
    if !problems.is_empty() {
        return Err(format!("cluster fuzz policy: {problems:?}").into());
    }
    crate::imap_cluster_fuzz_policy075::check_corpus_at_root(&root)?;
    let report = if let Some(path) = schedule {
        if seen.contains("--rounds") {
            return Err("--schedule and --rounds are mutually exclusive".into());
        }
        let bytes = crate::imap_qualification_075::read_relative(&root, path, 65536)?;
        check_script_at_root(&root, seed, &decode_cluster_schedule(&bytes)?, false)?
    } else {
        check_at_root(&root, seed, rounds)?
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

pub fn check_at_root(root: &Path, seed: u64, rounds: usize) -> Result<Value, Box<dyn Error>> {
    let corpus = crate::imap_cluster_fuzz_policy075::check_corpus_at_root(root)?;
    let steps = generated_cluster_schedule(seed, rounds)?;
    let mut report = evaluate_at_root(root, seed, rounds, &steps, false, true)?;
    report["corpus_cases"] = json!(corpus);
    Ok(report)
}
pub fn check_script_at_root(
    root: &Path,
    seed: u64,
    steps: &[ClusterScheduleStep],
    fixture: bool,
) -> Result<Value, Box<dyn Error>> {
    evaluate_at_root(root, seed, 0, steps, fixture, false)
}
fn evaluate_at_root(
    root: &Path,
    seed: u64,
    rounds: usize,
    steps: &[ClusterScheduleStep],
    fixture: bool,
    require_canary: bool,
) -> Result<Value, Box<dyn Error>> {
    match check_steps(seed, rounds, steps, require_canary) {
        Ok(report) => Ok(report),
        Err(error) => {
            let reason = error.to_string();
            let saved = retain_failure(root, seed, steps, fixture, &reason, |candidate| {
                match replay_cluster_schedule(candidate, Default::default()) {
                    Err(failure) => failure.to_string() == reason,
                    Ok(actual) => {
                        require_canary
                            && reason
                                == "actual cluster history checker missed its negative control"
                            && !history_canary_detected(&actual.history)
                    }
                }
            });
            Err(format!(
                "actual cluster campaign seed={seed}: {reason}; failure artifacts: {}",
                match saved {
                    Ok(path) => path.display().to_string(),
                    Err(failure) => format!("retention failed: {failure}"),
                }
            )
            .into())
        }
    }
}

/// Diagnostic counterexamples are never overwritten or promoted into release evidence.
pub fn retain_failure(
    root: &Path,
    seed: u64,
    steps: &[ClusterScheduleStep],
    fixture: bool,
    reason: &str,
    still_fails: impl FnMut(&[ClusterScheduleStep]) -> bool,
) -> Result<PathBuf, Box<dyn Error>> {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    if reason.len() > 2048 {
        return Err("failure reason exceeds bound".into());
    }
    let original = encode_cluster_schedule(steps)?;
    let minimized = shrink_cluster_schedule(steps, 64, still_fails)?;
    let minimal = encode_cluster_schedule(&minimized.steps)?;
    let metadata = serde_json::to_vec_pretty(&json!({"release":"0.75","fixture_version":1,
        "scope":"local_cluster_failure","production_enabled":false,"fixture":fixture,"seed":seed,"reason":reason,
        "source_sha":crate::imap_ci_075::source_state(root).ok(),
        "original_steps":steps.len(),"minimized_steps":minimized.steps.len(),"replays":minimized.replays,
        "budget_exhausted":minimized.budget_exhausted,"original_sha256":crate::imap_qualification_075::digest(&original),
        "minimized_sha256":crate::imap_qualification_075::digest(&minimal)}))?;
    if metadata.len() > 4096 {
        return Err("failure metadata exceeds bound".into());
    }
    let canonical = root.canonicalize()?;
    let mut base = canonical.clone();
    for component in ["target", "imap-075", "cluster-failures"] {
        base.push(component);
        if base.exists() {
            if fs::symlink_metadata(&base)?.file_type().is_symlink()
                || !base.is_dir()
                || !base.canonicalize()?.starts_with(&canonical)
            {
                return Err("unsafe failure artifact parent".into());
            }
        } else {
            fs::create_dir(&base)?;
        }
    }
    if fs::read_dir(&base)?
        .take(129)
        .collect::<std::io::Result<Vec<_>>>()?
        .len()
        >= 128
    {
        return Err("failure artifact directory bound reached".into());
    }
    let mut directory = None;
    for _ in 0..16 {
        let suffix = NEXT
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |value| {
                value.checked_add(1)
            })
            .map_err(|_| "failure artifact identity exhausted")?;
        let path = base.join(format!("seed-{seed}-{}-{suffix}", std::process::id()));
        match fs::create_dir(&path) {
            Ok(()) => {
                directory = Some(path);
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
    }
    let directory = directory.ok_or("failure artifact name collisions exceed bound")?;
    for (name, bytes) in [
        ("original.json", &original),
        ("minimized.json", &minimal),
        ("metadata.json", &metadata),
    ] {
        OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(directory.join(name))?
            .write_all(bytes)?;
    }
    Ok(directory)
}

pub fn check_seed(seed: u64, rounds: usize) -> Result<Value, Box<dyn Error>> {
    let steps = generated_cluster_schedule(seed, rounds)?;
    check_steps(seed, rounds, &steps, true)
}
fn check_steps(
    seed: u64,
    rounds: usize,
    steps: &[ClusterScheduleStep],
    require_canary: bool,
) -> Result<Value, Box<dyn Error>> {
    let report = replay_cluster_schedule(steps, Default::default())?;
    let canary_detected = history_canary_detected(&report.history);
    if require_canary && !canary_detected {
        return Err("actual cluster history checker missed its negative control".into());
    }
    let shrunk = if canary_detected {
        shrink_cluster_schedule(steps, 32, |candidate| {
            replay_cluster_schedule(candidate, Default::default())
                .is_ok_and(|actual| history_canary_detected(&actual.history))
        })?
    } else {
        hydracache_cluster_testkit::value_plane_cluster_chaos_075::ClusterShrinkReport {
            steps: steps.to_vec(),
            replays: 0,
            budget_exhausted: false,
        }
    };
    Ok(
        json!({"release":"0.75","scope":"local_cluster_schedule","production_enabled":false,
        "seed":seed,"rounds":rounds,"steps":steps.len(),"history_calls":report.history.calls().len(),
        "fingerprint":report.fingerprint,"linearizable":report.linearizable,"canary_detected":canary_detected,
        "replica_checks":report.replica_checks,"receipt_checks":report.receipt_checks,
        "bulk_checks":report.bulk_checks,"expiry_checks":report.expiry_checks,
        "canary_shrink_steps":shrunk.steps.len(),"shrink_replays":shrunk.replays,"shrink_budget_exhausted":shrunk.budget_exhausted}),
    )
}
