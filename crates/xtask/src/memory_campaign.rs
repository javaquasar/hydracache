use serde_json::Value;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_CAMPAIGNS: &str = "target/memory-evidence/0.71/campaigns";

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
    let problems = check_campaigns(&campaigns, &options.release, options.require_ship)?;
    finish("memory-campaign-check", &options.release, problems)
}

pub fn check_campaigns(
    campaigns: &Path,
    release: &str,
    require_ship: bool,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut problems = Vec::new();
    if release != "0.71" {
        problems.push(format!("unsupported memory campaign release {release}"));
        return Ok(problems);
    }
    if !campaigns.is_dir() {
        if require_ship {
            problems.push(format!(
                "campaign directory is missing: {}",
                campaigns.display()
            ));
        }
        return Ok(problems);
    }

    let mut admitted_candidate = false;
    for entry in fs::read_dir(campaigns)? {
        let entry = entry?;
        if !entry.path().is_dir() {
            continue;
        }
        let receipt_path = entry.path().join("campaign-receipt.json");
        if !receipt_path.is_file() {
            continue;
        }
        let receipt: Value = match serde_json::from_slice(&fs::read(&receipt_path)?) {
            Ok(value) => value,
            Err(error) => {
                problems.push(format!(
                    "{} is invalid JSON: {error}",
                    receipt_path.display()
                ));
                continue;
            }
        };
        let id = receipt
            .get("campaign_id")
            .and_then(Value::as_str)
            .unwrap_or("<missing>");
        for field in [
            "campaign_id",
            "source_sha",
            "workflow_sha",
            "campaign_identity_sha256",
        ] {
            if receipt
                .get(field)
                .and_then(Value::as_str)
                .is_none_or(str::is_empty)
            {
                problems.push(format!("campaign {id} is missing {field}"));
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
        let cases = receipt
            .get("case_ids")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if receipt.get("campaign_role").and_then(Value::as_str) == Some("candidate")
            && cases.iter().any(|case| case.as_str() == Some("M10-24h"))
            && receipt.get("result").and_then(Value::as_str) == Some("success")
            && receipt
                .get("ship_evidence_eligible")
                .and_then(Value::as_bool)
                == Some(true)
        {
            admitted_candidate = true;
        }
    }
    if require_ship && !admitted_candidate {
        problems.push(
            "no successful candidate campaign receipt contains the required M10-24h proof"
                .to_owned(),
        );
    }
    Ok(problems)
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
}

impl Options {
    fn parse(args: Vec<String>, allow_campaigns: bool) -> Result<Self, Box<dyn Error>> {
        let mut root = crate::doc_check::find_repo_root()?;
        let mut release = None;
        let mut campaigns = None;
        let mut require_ship = false;
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
                other => {
                    return Err(format!("unsupported memory campaign argument: {other}").into())
                }
            }
        }
        Ok(Self {
            root,
            release: release.ok_or("memory command requires --release")?,
            campaigns,
            require_ship,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::check_campaigns;
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
        assert!(check_campaigns(&root.join("missing"), "0.71", false)
            .unwrap()
            .is_empty());
        assert!(!check_campaigns(&root.join("missing"), "0.71", true)
            .unwrap()
            .is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn ship_requires_a_successful_candidate_m10_receipt() {
        let root = scratch("candidate");
        let campaign = root.join("candidate-1");
        fs::create_dir_all(&campaign).unwrap();
        fs::write(
            campaign.join("campaign-receipt.json"),
            br#"{"campaign_id":"candidate-1","release":"0.71","source_sha":"a","workflow_sha":"b","campaign_identity_sha256":"c","campaign_role":"candidate","result":"success","ship_evidence_eligible":true,"job_count":2,"completed_jobs":2,"case_ids":["M9-6h","M10-24h"]}"#,
        )
        .unwrap();
        assert!(check_campaigns(&root, "0.71", true).unwrap().is_empty());
        fs::remove_dir_all(root).unwrap();
    }
}
