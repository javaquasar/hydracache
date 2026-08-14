use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AdmissionOutcome {
    Pass,
    Fail,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CiAdmissionStatus {
    pub schema_version: u32,
    pub release: String,
    pub source_commit: String,
    pub head_commit: String,
    pub base_commit: String,
    pub generated_at: String,
    pub outcome: AdmissionOutcome,
    pub upstream: BTreeMap<String, String>,
    pub rejected_upstream: Vec<String>,
}

pub fn run(args: Vec<String>) -> Result<i32, Box<dyn Error>> {
    let options = Options::parse(args)?;
    let mut upstream = options.upstream;
    for (lane, path) in options.lane_statuses {
        let result = lane_status_result(
            &path,
            &options.release,
            &options.source_commit,
            &options.head_commit,
            &options.base_commit,
        );
        upstream.insert(lane, result);
    }
    let status = evaluate(
        &options.release,
        &options.source_commit,
        &options.head_commit,
        &options.base_commit,
        upstream,
    )?;
    write_atomic(&options.output, &status)?;
    println!(
        "ci-admission-status: {:?} release={} source={} output={}",
        status.outcome,
        status.release,
        status.source_commit,
        options.output.display()
    );
    if status.outcome == AdmissionOutcome::Pass {
        Ok(0)
    } else {
        eprintln!(
            "ci-admission-status: rejected upstream lanes: {}",
            status.rejected_upstream.join(", ")
        );
        Ok(1)
    }
}

fn lane_status_result(
    path: &Path,
    release: &str,
    source_commit: &str,
    head_commit: &str,
    base_commit: &str,
) -> String {
    let loaded = fs::read(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))
        .and_then(|bytes| {
            serde_json::from_slice::<CiAdmissionStatus>(&bytes)
                .map_err(|error| format!("cannot parse {}: {error}", path.display()))
        });
    let problems = match loaded {
        Ok(status) => {
            lane_status_problems(release, source_commit, head_commit, base_commit, &status)
        }
        Err(problem) => vec![problem],
    };
    if problems.is_empty() {
        "success".to_owned()
    } else {
        format!(
            "invalid:{}",
            problems.join(" | ").replace(['\r', '\n'], " ")
        )
    }
}

/// Validate one downloaded lane status against the exact admission candidate.
pub fn lane_status_problems(
    release: &str,
    source_commit: &str,
    head_commit: &str,
    base_commit: &str,
    status: &CiAdmissionStatus,
) -> Vec<String> {
    let mut problems = Vec::new();
    if status.schema_version != 1 {
        problems.push("unsupported lane status schema".to_owned());
    }
    if normalize_release(&status.release) != normalize_release(release) {
        problems.push("lane status release mismatch".to_owned());
    }
    for (name, observed, expected) in [
        ("source", status.source_commit.as_str(), source_commit),
        ("head", status.head_commit.as_str(), head_commit),
        ("base", status.base_commit.as_str(), base_commit),
    ] {
        if observed != expected {
            problems.push(format!("lane status {name} commit mismatch"));
        }
    }
    if OffsetDateTime::parse(&status.generated_at, &Rfc3339).is_err() {
        problems.push("lane status generated_at is not RFC3339".to_owned());
    }
    if status.upstream.is_empty() {
        problems.push("lane status has no upstream results".to_owned());
    }
    let expected_rejected = status
        .upstream
        .iter()
        .filter(|(_, result)| result.as_str() != "success")
        .map(|(lane, result)| format!("{lane}={result}"))
        .collect::<Vec<_>>();
    if status.rejected_upstream != expected_rejected {
        problems.push("lane status rejected_upstream is inconsistent".to_owned());
    }
    let expected_outcome = if expected_rejected.is_empty() {
        AdmissionOutcome::Pass
    } else {
        AdmissionOutcome::Fail
    };
    if status.outcome != expected_outcome {
        problems.push("lane status outcome is inconsistent".to_owned());
    }
    if status.outcome != AdmissionOutcome::Pass {
        problems.push("lane status outcome is not pass".to_owned());
    }
    problems
}

pub fn evaluate(
    release: &str,
    source_commit: &str,
    head_commit: &str,
    base_commit: &str,
    upstream: BTreeMap<String, String>,
) -> Result<CiAdmissionStatus, Box<dyn Error>> {
    if release.trim().is_empty() {
        return Err("release must not be empty".into());
    }
    for (name, commit) in [
        ("source", source_commit),
        ("head", head_commit),
        ("base", base_commit),
    ] {
        if !is_sha(commit) {
            return Err(format!("{name} commit must be a 40-character hexadecimal SHA").into());
        }
    }
    if upstream.is_empty() {
        return Err("at least one upstream lane is required".into());
    }
    let rejected_upstream = upstream
        .iter()
        .filter(|(_, result)| result.as_str() != "success")
        .map(|(lane, result)| format!("{lane}={result}"))
        .collect::<Vec<_>>();
    Ok(CiAdmissionStatus {
        schema_version: 1,
        release: release.to_owned(),
        source_commit: source_commit.to_owned(),
        head_commit: head_commit.to_owned(),
        base_commit: base_commit.to_owned(),
        generated_at: OffsetDateTime::now_utc().format(&Rfc3339)?,
        outcome: if rejected_upstream.is_empty() {
            AdmissionOutcome::Pass
        } else {
            AdmissionOutcome::Fail
        },
        upstream,
        rejected_upstream,
    })
}

fn is_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn normalize_release(release: &str) -> &str {
    release.strip_suffix(".0").unwrap_or(release)
}

fn write_atomic(path: &Path, status: &CiAdmissionStatus) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension(format!("json.{}.tmp", std::process::id()));
    fs::write(&temporary, serde_json::to_vec_pretty(status)?)?;
    fs::rename(temporary, path)?;
    Ok(())
}

struct Options {
    release: String,
    source_commit: String,
    head_commit: String,
    base_commit: String,
    output: PathBuf,
    upstream: BTreeMap<String, String>,
    lane_statuses: BTreeMap<String, PathBuf>,
}

impl Options {
    fn parse(args: Vec<String>) -> Result<Self, Box<dyn Error>> {
        let mut release = None;
        let mut source_commit = None;
        let mut head_commit = None;
        let mut base_commit = None;
        let mut output = None;
        let mut upstream = BTreeMap::new();
        let mut lane_statuses = BTreeMap::new();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--release" => release = args.next(),
                "--source" => source_commit = args.next(),
                "--head" => head_commit = args.next(),
                "--base" => base_commit = args.next(),
                "--output" => output = args.next().map(PathBuf::from),
                "--require" => {
                    let pair = args.next().ok_or("--require needs lane=result")?;
                    let (lane, result) =
                        pair.split_once('=').ok_or("--require needs lane=result")?;
                    if !valid_lane_name(lane)
                        || lane_statuses.contains_key(lane)
                        || upstream
                            .insert(lane.to_owned(), result.to_owned())
                            .is_some()
                    {
                        return Err(format!("invalid or duplicate upstream lane {lane:?}").into());
                    }
                }
                "--lane-status" => {
                    let pair = args.next().ok_or("--lane-status needs lane=path")?;
                    let (lane, path) = pair
                        .split_once('=')
                        .ok_or("--lane-status needs lane=path")?;
                    if !valid_lane_name(lane)
                        || path.trim().is_empty()
                        || upstream.contains_key(lane)
                        || lane_statuses
                            .insert(lane.to_owned(), PathBuf::from(path))
                            .is_some()
                    {
                        return Err(format!("invalid or duplicate lane status {lane:?}").into());
                    }
                }
                other => return Err(format!("unknown ci-admission-status argument {other}").into()),
            }
        }
        Ok(Self {
            release: release.ok_or("--release is required")?,
            source_commit: source_commit.ok_or("--source is required")?,
            head_commit: head_commit.ok_or("--head is required")?,
            base_commit: base_commit.ok_or("--base is required")?,
            output: output.ok_or("--output is required")?,
            upstream,
            lane_statuses,
        })
    }
}

fn valid_lane_name(lane: &str) -> bool {
    !lane.is_empty()
        && lane
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

    #[test]
    fn admission_status_is_fail_loud_for_failed_or_skipped_lanes() {
        let status = evaluate(
            "0.69",
            SHA,
            SHA,
            SHA,
            BTreeMap::from([
                ("fast".to_owned(), "success".to_owned()),
                ("postgres".to_owned(), "failure".to_owned()),
                ("java".to_owned(), "skipped".to_owned()),
            ]),
        )
        .unwrap();
        assert_eq!(status.outcome, AdmissionOutcome::Fail);
        assert_eq!(
            status.rejected_upstream,
            ["java=skipped", "postgres=failure"]
        );
    }

    #[test]
    fn admission_status_accepts_only_all_success_and_exact_shas() {
        let status = evaluate(
            "0.69",
            SHA,
            SHA,
            SHA,
            BTreeMap::from([("fast".to_owned(), "success".to_owned())]),
        )
        .unwrap();
        assert_eq!(status.outcome, AdmissionOutcome::Pass);
        assert!(evaluate("0.69", "short", SHA, SHA, status.upstream).is_err());
    }

    #[test]
    fn downloaded_lane_status_must_be_an_exact_consistent_pass() {
        let status = evaluate(
            "0.69",
            SHA,
            SHA,
            SHA,
            BTreeMap::from([("fast".to_owned(), "success".to_owned())]),
        )
        .unwrap();
        assert!(lane_status_problems("0.69", SHA, SHA, SHA, &status).is_empty());
        let path = std::env::temp_dir().join(format!(
            "hydracache-ci-admission-lane-{}.json",
            std::process::id()
        ));
        fs::write(&path, serde_json::to_vec(&status).unwrap()).unwrap();
        assert_eq!(lane_status_result(&path, "0.69", SHA, SHA, SHA), "success");
        fs::remove_file(path).unwrap();

        let mut wrong_commit = status.clone();
        wrong_commit.source_commit = "ffffffffffffffffffffffffffffffffffffffff".to_owned();
        assert!(lane_status_problems("0.69", SHA, SHA, SHA, &wrong_commit)
            .iter()
            .any(|problem| problem.contains("source commit mismatch")));

        let failed = evaluate(
            "0.69",
            SHA,
            SHA,
            SHA,
            BTreeMap::from([("fast".to_owned(), "failure".to_owned())]),
        )
        .unwrap();
        assert!(lane_status_problems("0.69", SHA, SHA, SHA, &failed)
            .iter()
            .any(|problem| problem.contains("outcome is not pass")));

        let mut false_green = status;
        false_green
            .upstream
            .insert("fast".to_owned(), "failure".to_owned());
        assert!(lane_status_problems("0.69", SHA, SHA, SHA, &false_green)
            .iter()
            .any(|problem| problem.contains("inconsistent")));
    }
}
