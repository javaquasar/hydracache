use std::fs;
use std::path::{Path, PathBuf};

fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "hydracache-memory-admission-071-{name}-{}",
        std::process::id()
    ));
    if path.exists() {
        fs::remove_dir_all(&path).expect("remove stale scratch directory");
    }
    fs::create_dir_all(&path).expect("create scratch directory");
    path
}

fn write_receipt(root: &Path, body: &str) {
    let campaign = root.join("candidate");
    fs::create_dir_all(&campaign).expect("campaign directory");
    fs::write(campaign.join("campaign-receipt.json"), body).expect("campaign receipt");
}

#[test]
fn complete_exact_candidate_m10_receipt_is_admitted() {
    let root = scratch("success");
    write_receipt(
        &root,
        r#"{"campaign_id":"candidate","release":"0.71","source_sha":"source","workflow_sha":"workflow","campaign_identity_sha256":"identity","campaign_role":"candidate","result":"success","ship_evidence_eligible":true,"job_count":2,"completed_jobs":2,"case_ids":["M9-6h","M10-24h"]}"#,
    );
    let problems =
        xtask::memory_campaign::check_campaigns(&root, "0.71", true).expect("campaign check");
    assert!(problems.is_empty(), "{problems:?}");
    fs::remove_dir_all(root).expect("clean scratch directory");
}

#[test]
fn incomplete_or_non_candidate_long_run_is_rejected_for_ship() {
    let root = scratch("incomplete");
    write_receipt(
        &root,
        r#"{"campaign_id":"baseline","release":"0.71","source_sha":"source","workflow_sha":"workflow","campaign_identity_sha256":"identity","campaign_role":"baseline","result":"success","ship_evidence_eligible":true,"job_count":2,"completed_jobs":1,"case_ids":["M10-24h"]}"#,
    );
    let problems =
        xtask::memory_campaign::check_campaigns(&root, "0.71", true).expect("campaign check");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("incomplete job count")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("no successful candidate")));
    fs::remove_dir_all(root).expect("clean scratch directory");
}

#[test]
fn missing_identity_or_wrong_release_fails_loud() {
    let root = scratch("identity");
    write_receipt(
        &root,
        r#"{"campaign_id":"candidate","release":"0.70","source_sha":"","workflow_sha":"workflow","campaign_identity_sha256":"identity","campaign_role":"candidate","result":"success","ship_evidence_eligible":true,"job_count":1,"completed_jobs":1,"case_ids":["M10-24h"]}"#,
    );
    let problems =
        xtask::memory_campaign::check_campaigns(&root, "0.71", true).expect("campaign check");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("source_sha")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("wrong release")));
    fs::remove_dir_all(root).expect("clean scratch directory");
}
