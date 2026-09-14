use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

const SOURCE: &str = "2222222222222222222222222222222222222222";
const WORKFLOW: &str = "3333333333333333333333333333333333333333";
const B1: &str = "1111111111111111111111111111111111111111";
const SCENARIO: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HOST: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

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

fn write_json(path: &Path, value: &Value) -> String {
    let mut bytes = serde_json::to_vec_pretty(value).expect("serialize fixture");
    bytes.push(b'\n');
    fs::create_dir_all(path.parent().expect("fixture parent")).expect("create fixture parent");
    fs::write(path, &bytes).expect("write fixture");
    let value = Sha256::digest(bytes);
    format!(
        "sha256:{}",
        value
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

fn jobs(case_id: &str) -> Vec<Value> {
    let dimensions: Vec<(u64, Option<&str>)> = match case_id {
        "M3-ttl" => (1..=5).map(|repeat| (repeat, None)).collect(),
        "M8-60m" => ["fixed-keyspace", "ttl", "reset", "hc2-churn"]
            .into_iter()
            .map(|sequence| (1, Some(sequence)))
            .collect(),
        "M9-6h" | "M10-24h" => vec![(1, None)],
        other => panic!("unsupported fixture case {other}"),
    };
    dimensions
        .into_iter()
        .flat_map(|(repetition, sequence)| {
            ["B1-instrumented", "C-candidate"].map(move |cohort| {
                json!({
                    "job_id": format!("{case_id}-{cohort}-{repetition}-{}", sequence.unwrap_or("single")),
                    "case_id": case_id,
                    "cohort": cohort,
                    "repetition": repetition,
                    "dimensions": sequence.map_or_else(|| json!({}), |value| json!({"sequence": value})),
                    "status": "success",
                    "attempts": [{"attempt": 1, "status": "success"}]
                })
            })
        })
        .collect()
}

fn write_campaign(root: &Path, case_id: &str, source: &str, host_fingerprint: &str) -> PathBuf {
    let id = format!("candidate-{}", case_id.to_ascii_lowercase());
    let campaign = root.join(&id);
    fs::create_dir_all(&campaign).expect("campaign directory");
    let identity = json!({
        "schema_version": 1,
        "release": "0.71",
        "campaign_id": id,
        "workflow_sha": WORKFLOW,
        "source_sha": source,
        "campaign_role": "candidate",
        "controller_sha": WORKFLOW,
        "scenario_digest": SCENARIO,
        "case_ids": [case_id],
        "source_shas": {"B1-instrumented": B1, "C-candidate": source}
    });
    let identity_digest = write_json(&campaign.join("campaign-identity.json"), &identity);
    let host = json!({
        "result": "success",
        "ship_evidence_eligible": true,
        "host_fingerprint": host_fingerprint,
        "profile_id": "memory-reference-071-v1"
    });
    let host_digest = write_json(&campaign.join("admission/host-preflight.json"), &host);
    let admission = json!({
        "schema_version": 1,
        "release": "0.71",
        "source_sha": source,
        "workflow_sha": WORKFLOW,
        "campaign_role": "candidate",
        "campaign_identity": {"path": "campaign-identity.json", "sha256": identity_digest},
        "receipts": [{"id": "host-preflight", "path": "host-preflight.json", "sha256": host_digest}]
    });
    let admission_digest = write_json(
        &campaign.join("admission/admission-manifest.json"),
        &admission,
    );
    let case_jobs = jobs(case_id);
    let state = json!({
        "schema_version": 1,
        "release": "0.71",
        "campaign_id": id,
        "workflow_sha": WORKFLOW,
        "source_sha": source,
        "campaign_role": "candidate",
        "scenario_digest": SCENARIO,
        "case_ids": [case_id],
        "identity": {"path": "campaign-identity.json", "sha256": identity_digest},
        "admission": {"manifest": "admission/admission-manifest.json", "manifest_sha256": admission_digest},
        "job_count": case_jobs.len(),
        "jobs": case_jobs,
        "status": "success"
    });
    write_json(&campaign.join("state.json"), &state);
    let receipt = json!({
        "schema_version": 1,
        "campaign_id": id,
        "release": "0.71",
        "source_sha": source,
        "workflow_sha": WORKFLOW,
        "scenario_digest": SCENARIO,
        "campaign_identity_sha256": identity_digest,
        "campaign_role": "candidate",
        "result": "success",
        "ship_evidence_eligible": true,
        "job_count": case_jobs.len(),
        "completed_jobs": case_jobs.len(),
        "case_ids": [case_id]
    });
    write_json(&campaign.join("campaign-receipt.json"), &receipt);
    campaign
}

fn write_chain(root: &Path) {
    for case_id in ["M3-ttl", "M8-60m", "M9-6h", "M10-24h"] {
        write_campaign(root, case_id, SOURCE, HOST);
    }
}

#[test]
fn complete_exact_candidate_chain_is_admitted() {
    let root = scratch("success");
    write_chain(&root);
    let problems =
        xtask::memory_campaign::check_campaigns_for_source(&root, "0.71", true, Some(SOURCE))
            .expect("campaign check");
    assert!(problems.is_empty(), "{problems:?}");
    fs::remove_dir_all(root).expect("clean scratch directory");
}

#[test]
fn m10_alone_cannot_hide_missing_predecessor_proofs() {
    let root = scratch("missing-chain");
    write_campaign(&root, "M10-24h", SOURCE, HOST);
    let problems =
        xtask::memory_campaign::check_campaigns(&root, "0.71", true).expect("campaign check");
    for case_id in ["M3-ttl", "M8-60m", "M9-6h"] {
        assert!(problems.iter().any(|problem| problem.contains(case_id)));
    }
    fs::remove_dir_all(root).expect("clean scratch directory");
}

#[test]
fn mixed_candidate_or_host_identity_is_rejected() {
    let root = scratch("mixed-identity");
    write_chain(&root);
    fs::remove_dir_all(root.join("candidate-m9-6h")).expect("replace M9 fixture");
    write_campaign(
        &root,
        "M9-6h",
        &"4".repeat(40),
        &format!("sha256:{}", "c".repeat(64)),
    );
    let problems =
        xtask::memory_campaign::check_campaigns(&root, "0.71", true).expect("campaign check");
    assert!(
        problems
            .iter()
            .any(|problem| problem.contains("source_sha"))
    );
    assert!(
        problems
            .iter()
            .any(|problem| problem.contains("host_fingerprint"))
    );
    fs::remove_dir_all(root).expect("clean scratch directory");
}

#[test]
fn exact_release_head_mismatch_is_rejected() {
    let root = scratch("head-mismatch");
    write_chain(&root);
    let problems = xtask::memory_campaign::check_campaigns_for_source(
        &root,
        "0.71",
        true,
        Some(&"9".repeat(40)),
    )
    .expect("campaign check");
    assert!(
        problems
            .iter()
            .any(|problem| problem.contains("exact release HEAD"))
    );
    fs::remove_dir_all(root).expect("clean scratch directory");
}

#[test]
fn incomplete_pair_shape_and_tampered_identity_are_rejected() {
    let root = scratch("tampered");
    write_chain(&root);
    let m3 = root.join("candidate-m3-ttl");
    let state_path = m3.join("state.json");
    let mut state: Value =
        serde_json::from_slice(&fs::read(&state_path).expect("state")).expect("state json");
    state["jobs"].as_array_mut().expect("jobs").pop();
    write_json(&state_path, &state);
    fs::write(m3.join("campaign-identity.json"), b"{}\n").expect("tamper identity");
    let problems =
        xtask::memory_campaign::check_campaigns(&root, "0.71", true).expect("campaign check");
    assert!(
        problems
            .iter()
            .any(|problem| problem.contains("identity digest"))
    );
    assert!(
        problems
            .iter()
            .any(|problem| problem.contains("wrong job shape"))
    );
    fs::remove_dir_all(root).expect("clean scratch directory");
}

#[test]
fn missing_identity_or_wrong_release_fails_loud() {
    let root = scratch("identity");
    let campaign = write_campaign(&root, "M10-24h", SOURCE, HOST);
    fs::remove_file(campaign.join("campaign-identity.json")).expect("remove identity");
    let receipt_path = campaign.join("campaign-receipt.json");
    let mut receipt: Value =
        serde_json::from_slice(&fs::read(&receipt_path).expect("receipt")).expect("receipt json");
    receipt["release"] = Value::String("0.70".to_owned());
    write_json(&receipt_path, &receipt);
    let problems =
        xtask::memory_campaign::check_campaigns(&root, "0.71", true).expect("campaign check");
    assert!(
        problems
            .iter()
            .any(|problem| problem.contains("wrong release"))
    );
    assert!(problems.iter().any(|problem| problem.contains("companion")));
    fs::remove_dir_all(root).expect("clean scratch directory");
}
