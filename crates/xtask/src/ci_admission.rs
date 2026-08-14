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
    let status = evaluate(
        &options.release,
        &options.source_commit,
        &options.head_commit,
        &options.base_commit,
        options.upstream,
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
}

impl Options {
    fn parse(args: Vec<String>) -> Result<Self, Box<dyn Error>> {
        let mut release = None;
        let mut source_commit = None;
        let mut head_commit = None;
        let mut base_commit = None;
        let mut output = None;
        let mut upstream = BTreeMap::new();
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
                    if lane.is_empty()
                        || !lane
                            .bytes()
                            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
                        || upstream
                            .insert(lane.to_owned(), result.to_owned())
                            .is_some()
                    {
                        return Err(format!("invalid or duplicate upstream lane {lane:?}").into());
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
        })
    }
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
}
