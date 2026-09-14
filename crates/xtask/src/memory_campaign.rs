use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_CAMPAIGNS: &str = "target/memory-evidence/0.71/campaigns";
const REQUIRED_CHAIN: [(&str, usize); 4] =
    [("M3-ttl", 10), ("M8-60m", 8), ("M9-6h", 2), ("M10-24h", 2)];

pub fn run_contracts(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let options = Options::parse(args, false)?;
    let mut problems = crate::memory_ownership::check(&options.root, &options.release)?;
    problems.extend(crate::memory_contracts::check_static_contracts(
        &options.root,
        &options.release,
        options.require_ship,
    )?);
    finish("memory-contract-check", &options.release, problems)
}

pub fn run_campaign(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let options = Options::parse(args, true)?;
    let campaigns = options
        .campaigns
        .unwrap_or_else(|| options.root.join(DEFAULT_CAMPAIGNS));
    let expected_source = match options.expected_source_sha {
        Some(value) => Some(value),
        None if options.require_ship => Some(git_head(&options.root)?),
        None => None,
    };
    if expected_source
        .as_deref()
        .is_some_and(|value| !full_sha(value))
    {
        return Err("--expected-source-sha must be a full lowercase commit SHA".into());
    }
    let required_cases = match options.require_through.as_deref() {
        Some(case_id) => Some(required_chain_through(case_id)?),
        None if options.require_ship => Some(REQUIRED_CHAIN.as_slice()),
        None => None,
    };
    let problems = check_campaigns_internal(
        &campaigns,
        &options.release,
        required_cases,
        expected_source.as_deref(),
    )?;
    finish("memory-campaign-check", &options.release, problems)
}

pub fn check_campaigns(
    campaigns: &Path,
    release: &str,
    require_ship: bool,
) -> Result<Vec<String>, Box<dyn Error>> {
    check_campaigns_internal(
        campaigns,
        release,
        require_ship.then_some(REQUIRED_CHAIN.as_slice()),
        None,
    )
}

pub fn check_campaigns_for_source(
    campaigns: &Path,
    release: &str,
    require_ship: bool,
    expected_source_sha: Option<&str>,
) -> Result<Vec<String>, Box<dyn Error>> {
    check_campaigns_internal(
        campaigns,
        release,
        require_ship.then_some(REQUIRED_CHAIN.as_slice()),
        expected_source_sha,
    )
}

fn check_campaigns_internal(
    campaigns: &Path,
    release: &str,
    required_cases: Option<&[(&str, usize)]>,
    expected_source_sha: Option<&str>,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut problems = Vec::new();
    if release != "0.71" {
        problems.push(format!("unsupported memory campaign release {release}"));
        return Ok(problems);
    }
    if !campaigns.is_dir() {
        if required_cases.is_some() {
            problems.push(format!(
                "campaign directory is missing: {}",
                campaigns.display()
            ));
        }
        return Ok(problems);
    }

    let mut chain: BTreeMap<String, Vec<CandidateCampaign>> = BTreeMap::new();
    for entry in fs::read_dir(campaigns)? {
        let entry = entry?;
        if !entry.path().is_dir() {
            continue;
        }
        let receipt_path = entry.path().join("campaign-receipt.json");
        if !receipt_path.is_file() {
            continue;
        }
        let receipt = match read_json(&receipt_path) {
            Ok(value) => value,
            Err(error) => {
                problems.push(format!(
                    "{} is invalid JSON: {error}",
                    receipt_path.display()
                ));
                continue;
            }
        };
        let id = text(&receipt, "campaign_id").unwrap_or("<missing>");
        validate_receipt_header(&receipt, id, release, &mut problems);
        let cases = strings(&receipt, "case_ids");
        let successful_candidate = text(&receipt, "campaign_role") == Some("candidate")
            && text(&receipt, "result") == Some("success")
            && receipt
                .get("ship_evidence_eligible")
                .and_then(Value::as_bool)
                == Some(true);
        if !successful_candidate {
            continue;
        }
        if cases.len() != 1 {
            problems.push(format!(
                "candidate campaign {id} must contain exactly one preregistered row"
            ));
            continue;
        }
        let case_id = cases[0].clone();
        if !REQUIRED_CHAIN
            .iter()
            .any(|(required, _)| *required == case_id)
        {
            continue;
        }
        match validate_candidate_companions(&entry.path(), &receipt, &case_id) {
            Ok(candidate) => chain.entry(case_id).or_default().push(candidate),
            Err(found) => problems.extend(found),
        }
    }

    if let Some(required) = required_cases {
        validate_required_chain(&chain, required, expected_source_sha, &mut problems);
    }
    Ok(problems)
}

#[derive(Clone)]
struct CandidateCampaign {
    id: String,
    source_sha: String,
    workflow_sha: String,
    scenario_digest: String,
    host_fingerprint: String,
    b1_sha: String,
}

fn validate_receipt_header(receipt: &Value, id: &str, release: &str, problems: &mut Vec<String>) {
    for field in [
        "campaign_id",
        "source_sha",
        "workflow_sha",
        "campaign_identity_sha256",
        "scenario_digest",
    ] {
        if text(receipt, field).is_none_or(str::is_empty) {
            problems.push(format!("campaign {id} is missing {field}"));
        }
    }
    for field in ["source_sha", "workflow_sha"] {
        if text(receipt, field).is_some_and(|value| !full_sha(value)) {
            problems.push(format!("campaign {id} has invalid {field}"));
        }
    }
    if receipt.get("release").and_then(Value::as_str) != Some(release) {
        problems.push(format!("campaign {id} has the wrong release identity"));
    }
    if receipt.get("result").and_then(Value::as_str) != Some("success") {
        problems.push(format!("campaign {id} is not successful"));
    }
    let job_count = receipt.get("job_count").and_then(Value::as_u64);
    let completed_jobs = receipt.get("completed_jobs").and_then(Value::as_u64);
    if job_count.is_none_or(|count| count == 0) || completed_jobs != job_count {
        problems.push(format!("campaign {id} has an incomplete job count"));
    }
}

fn validate_candidate_companions(
    campaign_dir: &Path,
    receipt: &Value,
    case_id: &str,
) -> Result<CandidateCampaign, Vec<String>> {
    let id = text(receipt, "campaign_id").unwrap_or("<missing>");
    let mut problems = Vec::new();
    let identity_path = campaign_dir.join("campaign-identity.json");
    let state_path = campaign_dir.join("state.json");
    let admission_path = campaign_dir.join("admission/admission-manifest.json");
    let host_path = campaign_dir.join("admission/host-preflight.json");
    let identity = companion(&identity_path, id, &mut problems);
    let state = companion(&state_path, id, &mut problems);
    let admission = companion(&admission_path, id, &mut problems);
    let host = companion(&host_path, id, &mut problems);
    if !problems.is_empty() {
        return Err(problems);
    }
    let (identity, state, admission, host) = (
        identity.expect("checked"),
        state.expect("checked"),
        admission.expect("checked"),
        host.expect("checked"),
    );

    let identity_digest = digest(&identity_path).expect("read companion after parse");
    if text(receipt, "campaign_identity_sha256") != Some(identity_digest.as_str()) {
        problems.push(format!(
            "campaign {id} identity digest does not match its receipt"
        ));
    }
    if state.pointer("/identity/sha256").and_then(Value::as_str) != Some(identity_digest.as_str()) {
        problems.push(format!("campaign {id} state does not seal its identity"));
    }
    let admission_digest = digest(&admission_path).expect("read companion after parse");
    if state
        .pointer("/admission/manifest_sha256")
        .and_then(Value::as_str)
        != Some(admission_digest.as_str())
    {
        problems.push(format!(
            "campaign {id} state does not seal its admission manifest"
        ));
    }

    for (field, expected) in [
        ("campaign_id", text(receipt, "campaign_id")),
        ("release", text(receipt, "release")),
        ("source_sha", text(receipt, "source_sha")),
        ("workflow_sha", text(receipt, "workflow_sha")),
        ("campaign_role", Some("candidate")),
        ("scenario_digest", text(receipt, "scenario_digest")),
    ] {
        for (label, value) in [("identity", &identity), ("state", &state)] {
            if text(value, field) != expected {
                problems.push(format!("campaign {id} {label} mismatches {field}"));
            }
        }
    }
    for (field, expected) in [
        ("release", text(receipt, "release")),
        ("source_sha", text(receipt, "source_sha")),
        ("workflow_sha", text(receipt, "workflow_sha")),
        ("campaign_role", Some("candidate")),
    ] {
        if text(&admission, field) != expected {
            problems.push(format!("campaign {id} admission mismatches {field}"));
        }
    }
    if admission
        .pointer("/campaign_identity/sha256")
        .and_then(Value::as_str)
        != Some(identity_digest.as_str())
    {
        problems.push(format!(
            "campaign {id} admission does not seal its identity"
        ));
    }
    if strings(&identity, "case_ids") != [case_id] || strings(&state, "case_ids") != [case_id] {
        problems.push(format!("campaign {id} companion files mismatch case_ids"));
    }
    let source_shas = identity.get("source_shas").and_then(Value::as_object);
    let b1_sha = source_shas
        .and_then(|value| value.get("B1-instrumented"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let candidate_sha = source_shas
        .and_then(|value| value.get("C-candidate"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !full_sha(b1_sha)
        || candidate_sha != text(receipt, "source_sha").unwrap_or_default()
        || b1_sha == candidate_sha
    {
        problems.push(format!(
            "campaign {id} has invalid frozen B1/C source identities"
        ));
    }
    let host_fingerprint = text(&host, "host_fingerprint").unwrap_or_default();
    if !host_fingerprint.starts_with("sha256:")
        || host.get("ship_evidence_eligible").and_then(Value::as_bool) != Some(true)
    {
        problems.push(format!(
            "campaign {id} lacks an admitted ship-eligible host fingerprint"
        ));
    }
    let host_digest = digest(&host_path).expect("read companion after parse");
    let host_sealed = admission
        .get("receipts")
        .and_then(Value::as_array)
        .is_some_and(|receipts| {
            receipts.iter().any(|entry| {
                text(entry, "id") == Some("host-preflight")
                    && text(entry, "sha256") == Some(host_digest.as_str())
            })
        });
    if !host_sealed {
        problems.push(format!(
            "campaign {id} admission does not seal host-preflight"
        ));
    }
    validate_job_shape(id, case_id, &state, &mut problems);

    if !problems.is_empty() {
        return Err(problems);
    }
    Ok(CandidateCampaign {
        id: id.to_owned(),
        source_sha: text(receipt, "source_sha").unwrap_or_default().to_owned(),
        workflow_sha: text(receipt, "workflow_sha").unwrap_or_default().to_owned(),
        scenario_digest: text(receipt, "scenario_digest")
            .unwrap_or_default()
            .to_owned(),
        host_fingerprint: host_fingerprint.to_owned(),
        b1_sha: b1_sha.to_owned(),
    })
}

fn validate_job_shape(id: &str, case_id: &str, state: &Value, problems: &mut Vec<String>) {
    let expected_count = REQUIRED_CHAIN
        .iter()
        .find_map(|(required, count)| (*required == case_id).then_some(*count))
        .unwrap_or_default();
    let jobs = state
        .get("jobs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if jobs.len() != expected_count
        || state.get("job_count").and_then(Value::as_u64) != Some(expected_count as u64)
    {
        problems.push(format!(
            "campaign {id} {case_id} has the wrong job shape: expected {expected_count} jobs"
        ));
        return;
    }
    let mut observed = BTreeSet::new();
    for job in jobs {
        if text(&job, "case_id") != Some(case_id)
            || text(&job, "status") != Some("success")
            || job
                .get("attempts")
                .and_then(Value::as_array)
                .and_then(|attempts| attempts.last())
                .and_then(|attempt| attempt.get("status"))
                .and_then(Value::as_str)
                != Some("success")
        {
            problems.push(format!(
                "campaign {id} contains an invalid or unsuccessful job"
            ));
            continue;
        }
        let cohort = text(&job, "cohort").unwrap_or_default();
        let repetition = job
            .get("repetition")
            .and_then(Value::as_u64)
            .unwrap_or_default();
        let dimension = if case_id == "M8-60m" {
            job.pointer("/dimensions/sequence")
                .and_then(Value::as_str)
                .unwrap_or_default()
        } else {
            "single"
        };
        observed.insert((cohort.to_owned(), repetition, dimension.to_owned()));
    }
    let expected: BTreeSet<(String, u64, String)> = match case_id {
        "M3-ttl" => (1..=5)
            .flat_map(|repetition| {
                ["B1-instrumented", "C-candidate"]
                    .map(move |cohort| (cohort.to_owned(), repetition, "single".to_owned()))
            })
            .collect(),
        "M8-60m" => ["fixed-keyspace", "ttl", "reset", "hc2-churn"]
            .into_iter()
            .flat_map(|sequence| {
                ["B1-instrumented", "C-candidate"]
                    .map(move |cohort| (cohort.to_owned(), 1, sequence.to_owned()))
            })
            .collect(),
        "M9-6h" | "M10-24h" => ["B1-instrumented", "C-candidate"]
            .into_iter()
            .map(|cohort| (cohort.to_owned(), 1, "single".to_owned()))
            .collect(),
        _ => BTreeSet::new(),
    };
    if observed != expected {
        problems.push(format!(
            "campaign {id} {case_id} is missing or duplicates required B1/C jobs"
        ));
    }
}

fn validate_required_chain(
    chain: &BTreeMap<String, Vec<CandidateCampaign>>,
    required: &[(&str, usize)],
    expected_source_sha: Option<&str>,
    problems: &mut Vec<String>,
) {
    let mut selected = Vec::new();
    for (case_id, _) in required {
        match chain.get(*case_id).map(Vec::as_slice).unwrap_or_default() {
            [campaign] => selected.push(campaign),
            [] => problems.push(format!("candidate chain is missing required {case_id} proof")),
            campaigns => problems.push(format!(
                "candidate chain has {} successful {case_id} proofs; exactly one designated attempt is required",
                campaigns.len()
            )),
        }
    }
    if selected.is_empty() {
        return;
    }
    let first = selected[0];
    for campaign in &selected {
        for (field, actual, frozen) in [
            (
                "source_sha",
                campaign.source_sha.as_str(),
                first.source_sha.as_str(),
            ),
            (
                "workflow_sha",
                campaign.workflow_sha.as_str(),
                first.workflow_sha.as_str(),
            ),
            (
                "scenario_digest",
                campaign.scenario_digest.as_str(),
                first.scenario_digest.as_str(),
            ),
            (
                "host_fingerprint",
                campaign.host_fingerprint.as_str(),
                first.host_fingerprint.as_str(),
            ),
            (
                "B1 source_sha",
                campaign.b1_sha.as_str(),
                first.b1_sha.as_str(),
            ),
        ] {
            if actual != frozen {
                problems.push(format!(
                    "candidate campaign {} changes frozen chain {field}",
                    campaign.id
                ));
            }
        }
        if expected_source_sha.is_some_and(|expected| campaign.source_sha != expected) {
            problems.push(format!(
                "candidate campaign {} measures {}, not exact release HEAD {}",
                campaign.id,
                campaign.source_sha,
                expected_source_sha.unwrap_or_default()
            ));
        }
    }
}

fn required_chain_through(
    case_id: &str,
) -> Result<&'static [(&'static str, usize)], Box<dyn Error>> {
    let index = REQUIRED_CHAIN
        .iter()
        .position(|(candidate, _)| *candidate == case_id)
        .ok_or_else(|| format!("--require-through does not recognize {case_id}"))?;
    Ok(&REQUIRED_CHAIN[..=index])
}

fn companion(path: &Path, id: &str, problems: &mut Vec<String>) -> Option<Value> {
    match read_json(path) {
        Ok(value) => Some(value),
        Err(error) => {
            problems.push(format!(
                "campaign {id} companion {} is unavailable or invalid: {error}",
                path.display()
            ));
            None
        }
    }
}

fn read_json(path: &Path) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn digest(path: &Path) -> Result<String, Box<dyn Error>> {
    let value = Sha256::digest(fs::read(path)?);
    Ok(format!(
        "sha256:{}",
        value
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    ))
}

fn text<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(Value::as_str)
}

fn strings(value: &Value, field: &str) -> Vec<String> {
    value
        .get(field)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

fn full_sha(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn git_head(root: &Path) -> Result<String, Box<dyn Error>> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err("cannot resolve exact release HEAD".into());
    }
    let value = String::from_utf8(output.stdout)?.trim().to_owned();
    if !full_sha(&value) {
        return Err("git HEAD is not a full lowercase commit SHA".into());
    }
    Ok(value)
}

fn finish(label: &str, release: &str, problems: Vec<String>) -> Result<(), Box<dyn Error>> {
    if problems.is_empty() {
        println!("{label}: OK (release {release})");
        Ok(())
    } else {
        Err(format!("{label} failed:\n- {}", problems.join("\n- ")).into())
    }
}

struct Options {
    root: PathBuf,
    release: String,
    campaigns: Option<PathBuf>,
    require_ship: bool,
    require_through: Option<String>,
    expected_source_sha: Option<String>,
}

impl Options {
    fn parse(args: Vec<String>, allow_campaigns: bool) -> Result<Self, Box<dyn Error>> {
        let mut root = crate::doc_check::find_repo_root()?;
        let mut release = None;
        let mut campaigns = None;
        let mut require_ship = false;
        let mut require_through = None;
        let mut expected_source_sha = None;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--root" => root = PathBuf::from(args.next().ok_or("--root requires a path")?),
                "--release" => release = Some(args.next().ok_or("--release requires a value")?),
                "--campaigns" if allow_campaigns => {
                    let value = PathBuf::from(args.next().ok_or("--campaigns requires a path")?);
                    campaigns = Some(if value.is_absolute() {
                        value
                    } else {
                        root.join(value)
                    });
                }
                "--require-ship" => require_ship = true,
                "--require-through" if allow_campaigns => {
                    require_through = Some(args.next().ok_or("--require-through requires a case")?)
                }
                "--expected-source-sha" if allow_campaigns => {
                    expected_source_sha =
                        Some(args.next().ok_or("--expected-source-sha requires a SHA")?)
                }
                other => {
                    return Err(format!("unsupported memory campaign argument: {other}").into());
                }
            }
        }
        Ok(Self {
            root,
            release: release.ok_or("memory command requires --release")?,
            campaigns,
            require_ship,
            require_through,
            expected_source_sha,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{check_campaigns, full_sha};
    use std::fs;
    use std::path::PathBuf;

    fn scratch(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "hydracache-memory-campaign-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn missing_campaigns_are_allowed_before_ship_and_rejected_for_ship() {
        let root = scratch("missing");
        assert!(
            check_campaigns(&root.join("missing"), "0.71", false)
                .unwrap()
                .is_empty()
        );
        assert!(
            !check_campaigns(&root.join("missing"), "0.71", true)
                .unwrap()
                .is_empty()
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn sha_identity_is_exact_and_lowercase() {
        assert!(full_sha(&"a".repeat(40)));
        assert!(!full_sha(&"A".repeat(40)));
        assert!(!full_sha("main"));
    }
}
