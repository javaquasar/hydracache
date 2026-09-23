use serde_json::{json, Value as JsonValue};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const RELEASE: &str = "0.73";
const PROFILE: &str = "instrumentation-overhead-073-v1";

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let options = Options::parse(args)?;
    if options.release != RELEASE || options.pairs < 3 {
        return Err(
            "performance-overhead-screen requires release 0.73 and at least 3 pairs".into(),
        );
    }
    let context_path = resolve(&options.root, &options.context);
    let context: JsonValue = serde_json::from_slice(&fs::read(&context_path)?)?;
    let context_problems =
        crate::performance_local::check_context_at_root(&options.root, &context)?;
    if !context_problems.is_empty() {
        return Err(format!(
            "invalid local context:\n- {}",
            context_problems.join("\n- ")
        )
        .into());
    }
    if context
        .pointer("/source/dirty_worktree")
        .and_then(JsonValue::as_bool)
        != Some(false)
    {
        return Err("overhead screening requires a clean-worktree context".into());
    }
    let current_sha = git(&options.root, &["rev-parse", "HEAD"])?;
    if context.pointer("/source/sha").and_then(JsonValue::as_str) != Some(current_sha.as_str()) {
        return Err("overhead screening context does not match the current source SHA".into());
    }
    if !git(
        &options.root,
        &["status", "--porcelain", "--untracked-files=normal"],
    )?
    .is_empty()
    {
        return Err("overhead screening requires the current worktree to remain clean".into());
    }
    let binary = resolve(&options.root, &options.binary);
    if !binary.is_file() {
        return Err(format!("prebuilt loadgen binary is missing: {}", binary.display()).into());
    }
    let output = resolve(&options.root, &options.output);
    if output.exists() {
        return Err(format!(
            "append-only screening output already exists: {}",
            output.display()
        )
        .into());
    }
    fs::create_dir_all(&output)?;

    let mut attempts = Vec::new();
    let mut samples: BTreeMap<&str, Vec<JsonValue>> =
        BTreeMap::from([("off", Vec::new()), ("production", Vec::new())]);
    let mut failures = Vec::new();
    for pair in 1..=options.pairs {
        let order = pair_order(options.seed, pair);
        for (position, mode) in order.into_iter().enumerate() {
            let attempt_id = format!("pair-{pair:02}-{:02}-{mode}", position + 1);
            let attempt_dir = output.join(&attempt_id);
            fs::create_dir_all(&attempt_dir)?;
            let process = Command::new(&binary)
                .current_dir(&options.root)
                .args([
                    "memory-efficiency",
                    "--profile",
                    PROFILE,
                    "--provider",
                    "system",
                    "--instrumentation-mode",
                    mode,
                    "--output-dir",
                ])
                .arg(&attempt_dir)
                .output()?;
            fs::write(attempt_dir.join("stdout.txt"), &process.stdout)?;
            fs::write(attempt_dir.join("stderr.txt"), &process.stderr)?;
            let status = if process.status.success() {
                "success"
            } else {
                "failed"
            };
            let receipt_path = attempt_dir.join("receipt.json");
            let mut attempt = json!({
                "attempt_id": attempt_id,
                "pair_index": pair,
                "position": position + 1,
                "mode": mode,
                "result": status,
                "exit_code": process.status.code(),
                "stdout_sha256": digest_file(&attempt_dir.join("stdout.txt"))?,
                "stderr_sha256": digest_file(&attempt_dir.join("stderr.txt"))?,
                "receipt_sha256": JsonValue::Null,
                "resource_series_sha256": JsonValue::Null
            });
            if process.status.success() {
                let receipt: JsonValue = serde_json::from_slice(&fs::read(&receipt_path)?)?;
                let series_path = receipt
                    .get("resource_series")
                    .and_then(JsonValue::as_str)
                    .ok_or("loadgen receipt omitted resource_series")?;
                let series_path = resolve(&options.root, Path::new(series_path));
                let records = read_series(&series_path)?;
                let sample = summarize_attempt(&receipt, &records)?;
                samples.get_mut(mode).expect("known mode").push(sample);
                attempt["receipt_sha256"] = json!(digest_file(&receipt_path)?);
                attempt["resource_series_sha256"] = json!(digest_file(&series_path)?);
            } else {
                failures.push(attempt_id.clone());
            }
            fs::write(
                attempt_dir.join("attempt.json"),
                serde_json::to_vec_pretty(&attempt)?,
            )?;
            attempts.push(attempt);
        }
    }
    let aggregate = json!({
        "schema_version": 1,
        "release": RELEASE,
        "profile_id": PROFILE,
        "environment_class": "local_screening",
        "promotable": false,
        "numerical_claim_eligible": false,
        "thresholds_status": "screening_only_unqualified",
        "source_sha": context.pointer("/source/sha").and_then(JsonValue::as_str),
        "host_fingerprint_sha256": context.get("host_fingerprint_sha256"),
        "binary_sha256": digest_file(&binary)?,
        "run_order_seed": options.seed,
        "pair_count": options.pairs,
        "counterbalanced": counterbalanced(options.seed, options.pairs),
        "attempts": attempts,
        "samples": samples,
        "failures": failures,
        "limitations": [
            "local debug or release screening cannot freeze I73 thresholds",
            "three local pairs validate collection and expose gross regressions only",
            "dedicated-host qualification still requires at least five admitted pairs"
        ]
    });
    fs::write(
        output.join("screening.json"),
        serde_json::to_vec_pretty(&aggregate)?,
    )?;
    if aggregate["failures"].as_array().is_some_and(Vec::is_empty) {
        println!(
            "performance-overhead-screen: OK ({} pairs, non-promotable, {})",
            options.pairs,
            output.display()
        );
        Ok(())
    } else {
        Err(format!(
            "overhead screening retained failed attempts in {}",
            output.display()
        )
        .into())
    }
}

pub fn pair_order(seed: u64, pair: u64) -> [&'static str; 2] {
    if (seed.wrapping_add(pair)) & 1 == 0 {
        ["off", "production"]
    } else {
        ["production", "off"]
    }
}

fn counterbalanced(seed: u64, pairs: u64) -> bool {
    let off_first = (1..=pairs)
        .filter(|pair| pair_order(seed, *pair)[0] == "off")
        .count();
    off_first.abs_diff(pairs as usize - off_first) <= 1
}

fn read_series(path: &Path) -> Result<Vec<JsonValue>, Box<dyn Error>> {
    fs::read_to_string(path)?
        .lines()
        .map(|line| serde_json::from_str(line).map_err(Into::into))
        .collect()
}

fn summarize_attempt(
    receipt: &JsonValue,
    records: &[JsonValue],
) -> Result<JsonValue, Box<dyn Error>> {
    if records.len() != 8 {
        return Err("resource series must contain eight phases".into());
    }
    let cold_rss = records[0]
        .get("rss_bytes")
        .and_then(JsonValue::as_u64)
        .ok_or("cold phase RSS is unavailable")?;
    let value = |phase: &str, field: &str| {
        records
            .iter()
            .find(|record| record.get("phase").and_then(JsonValue::as_str) == Some(phase))
            .and_then(|record| record.get(field))
            .and_then(JsonValue::as_f64)
    };
    let post_idle_rss = records[6]
        .get("rss_bytes")
        .and_then(JsonValue::as_u64)
        .ok_or("post-idle RSS is unavailable")?;
    let peak_rss = records
        .iter()
        .filter_map(|record| record.get("peak_rss_bytes").and_then(JsonValue::as_u64))
        .max()
        .ok_or("peak RSS is unavailable")?;
    Ok(json!({
        "elapsed_ns": receipt.get("elapsed_ns").and_then(JsonValue::as_u64),
        "fill_allocated_bytes_per_operation": value("fill", "gross_allocated_bytes_per_operation"),
        "steady_allocated_bytes_per_operation": value("steady", "gross_allocated_bytes_per_operation"),
        "expire_delete_allocated_bytes_per_operation": value("expire_or_delete", "gross_allocated_bytes_per_operation"),
        "refill_allocated_bytes_per_operation": value("refill", "gross_allocated_bytes_per_operation"),
        "post_idle_rss_delta_bytes": post_idle_rss.saturating_sub(cold_rss),
        "peak_rss_delta_bytes": peak_rss.saturating_sub(cold_rss)
    }))
}

fn digest_file(path: &Path) -> Result<String, Box<dyn Error>> {
    let digest = Sha256::digest(fs::read(path)?);
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn git(root: &Path, args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = Command::new("git").args(args).current_dir(root).output()?;
    if !output.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn resolve(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_owned()
    } else {
        root.join(path)
    }
}

struct Options {
    root: PathBuf,
    release: String,
    context: PathBuf,
    binary: PathBuf,
    output: PathBuf,
    pairs: u64,
    seed: u64,
}

impl Options {
    fn parse(args: Vec<String>) -> Result<Self, Box<dyn Error>> {
        let mut root = crate::doc_check::find_repo_root()?;
        let mut values = BTreeMap::new();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            if arg == "--root" {
                root = PathBuf::from(args.next().ok_or("--root requires a path")?);
            } else if let Some(name) = arg.strip_prefix("--") {
                values.insert(
                    name.to_owned(),
                    args.next()
                        .ok_or_else(|| format!("{arg} requires a value"))?,
                );
            } else {
                return Err(format!("unsupported overhead screening argument: {arg}").into());
            }
        }
        let mut take = |name: &str| {
            values
                .remove(name)
                .ok_or_else(|| format!("performance-overhead-screen requires --{name}"))
        };
        let options = Self {
            root,
            release: take("release")?,
            context: PathBuf::from(take("context")?),
            binary: PathBuf::from(take("binary")?),
            output: PathBuf::from(take("output")?),
            pairs: take("pairs")?.parse()?,
            seed: take("seed")?.parse()?,
        };
        if !values.is_empty() {
            return Err(format!(
                "unsupported overhead screening arguments: {:?}",
                values.keys()
            )
            .into());
        }
        Ok(options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pair_order_is_deterministic_and_counterbalanced() {
        assert_eq!(pair_order(73, 1), ["off", "production"]);
        assert_eq!(pair_order(73, 2), ["production", "off"]);
        assert_eq!(pair_order(73, 3), ["off", "production"]);
        assert!(counterbalanced(73, 3));
        assert!(counterbalanced(73001, 5));
    }
}
