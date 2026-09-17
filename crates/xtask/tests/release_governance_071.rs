use std::fs;
use std::path::{Path, PathBuf};

use toml::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

fn policy() -> Value {
    toml::from_str(
        &fs::read_to_string(root().join("docs/testing/memory/0.71/release-policy.toml"))
            .expect("release policy"),
    )
    .expect("policy TOML")
}

#[test]
fn complete_foundation_with_evidenced_deferrals_allows_no_win_ship() {
    let problems = xtask::memory_contracts::check_release_policy(&policy(), "0.71", true);
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn missing_foundation_or_pending_optional_work_blocks_ship() {
    let mut value = policy();
    value["mandatory_foundation"]
        .as_array_mut()
        .expect("foundation")
        .retain(|item| item.as_str() != Some("W13-governance"));
    value["optional_work"][0]["disposition"] = Value::String("pending-evidence".to_owned());
    let problems = xtask::memory_contracts::check_release_policy(&value, "0.71", true);
    assert!(problems
        .iter()
        .any(|problem| problem.contains("W13-governance")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("refuses pending optional work")));
}

#[test]
fn safety_defect_cannot_be_deferred_or_hidden_as_a_schedule_choice() {
    let mut value = policy();
    value["deferred_safety_defects_allowed"] = Value::Boolean(true);
    value["optional_work"][0]["reason"] = Value::String("schedule pressure".to_owned());
    let problems = xtask::memory_contracts::check_release_policy(&value, "0.71", true);
    assert!(problems
        .iter()
        .any(|problem| problem.contains("deferred_safety_defects_allowed")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("bounded and correct")));
}

#[test]
fn every_release_work_item_has_a_registered_fast_receipt_contract() {
    let manifest: Value = toml::from_str(
        &fs::read_to_string(root().join("docs/testing/release-evidence/0.71.toml"))
            .expect("release evidence manifest"),
    )
    .expect("release evidence TOML");
    let registry: Value = toml::from_str(
        &fs::read_to_string(root().join("docs/testing/fast-suite-registry.toml"))
            .expect("fast suite registry"),
    )
    .expect("fast suite TOML");
    let suites = registry["suite"].as_array().expect("registered suites");
    let items = manifest["work_item"]
        .as_array()
        .expect("release work items");
    assert_eq!(items.len(), 14, "W0-W13 must be registered");

    for item in items {
        let id = item["id"].as_str().expect("work item id");
        let gate_ids = item["fast_gate_ids"].as_array().expect("fast gate IDs");
        assert!(!gate_ids.is_empty(), "{id} has no fast receipt contract");
        for gate_id in gate_ids {
            let gate_id = gate_id.as_str().expect("fast gate ID");
            assert!(
                suites.iter().any(|suite| {
                    suite["id"].as_str() == Some(gate_id)
                        && suite["work_items"].as_array().is_some_and(|work_items| {
                            work_items
                                .iter()
                                .any(|work_item| work_item.as_str() == Some(id))
                        })
                }),
                "{id} maps to an unregistered or unrelated fast suite {gate_id}"
            );
        }
    }
}

#[test]
fn dispatched_fast_receipt_installs_pinned_nextest_before_running() {
    let ci = fs::read_to_string(root().join(".github/workflows/ci.yml"))
        .expect("CI workflow")
        .replace("\r\n", "\n");
    let job = ci
        .split_once("  gated-proof-registry:\n")
        .expect("registered proof job")
        .1;
    let install = job
        .find("- name: Install pinned cargo-nextest for fast suite\n")
        .expect("fast suite nextest installer");
    let run = job
        .find("- name: Run registered gated proofs\n")
        .expect("registered proof command");
    assert!(
        install < run,
        "nextest must be installed before the fast gate"
    );
    assert!(job[install..run].contains("if: startsWith(inputs.gated_gate_id, 'fast.')"));
    assert!(job[install..run].contains("tool: cargo-nextest@0.9.137"));
    assert!(job[install..run].contains("uses: taiki-e/install-action@cargo-deny"));
    assert!(job[run..].contains("target/nextest/ci/junit.xml"));
}

#[test]
fn d4_claim_gate_rejects_numeric_or_identity_broadening() {
    let acceptance: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(root().join("docs/testing/memory/0.71/d4-acceptance.json"))
            .expect("D4 acceptance"),
    )
    .expect("D4 acceptance JSON");
    let policy = policy();
    let dispositions = policy["optional_work"]
        .as_array()
        .expect("optional dispositions")
        .iter()
        .map(|item| {
            serde_json::json!({
                "id": item["id"].as_str(),
                "disposition": item["disposition"].as_str(),
                "reason": item["reason"].as_str(),
                "next_evidence": item["next_evidence"].as_str(),
            })
        })
        .collect::<Vec<_>>();
    let mut claims = serde_json::json!({
        "release": "0.71",
        "measured_source_sha": acceptance["measured_source_sha"],
        "evidence_branch": acceptance["evidence_branch"],
        "evidence_commit": acceptance["evidence_commit"],
        "campaign_ids": acceptance["campaigns"]
            .as_array()
            .expect("campaigns")
            .iter()
            .map(|item| item["id"].clone())
            .collect::<Vec<_>>(),
        "numeric_memory_improvement_claims": [],
        "negative_result": "No optional numerical memory improvement is claimed.",
        "optional_dispositions": dispositions,
    });
    assert!(
        xtask::release_evidence::check_071_claim_values(&claims, &acceptance, &policy).is_empty()
    );
    claims["numeric_memory_improvement_claims"] = serde_json::json!(["unreviewed RSS win"]);
    assert!(
        xtask::release_evidence::check_071_claim_values(&claims, &acceptance, &policy)
            .iter()
            .any(|problem| problem.contains("numerical memory claims"))
    );
    claims["numeric_memory_improvement_claims"] = serde_json::json!([]);
    claims["measured_source_sha"] = serde_json::json!("b".repeat(40));
    assert!(
        xtask::release_evidence::check_071_claim_values(&claims, &acceptance, &policy)
            .iter()
            .any(|problem| problem.contains("measured_source_sha"))
    );
}
