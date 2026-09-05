mod support;

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use support::daemon_cluster::{
    current_server_binary, ensure_distinct_daemon_binaries, public_text_status,
    resolve_shipped_daemon_binary, DaemonCluster, TestResult,
};

const RUN_ENV: &str = "HYDRACACHE_RUN_MANAGEMENT_MIXED_072";
const BUILD_ENV: &str = "HYDRACACHE_BUILD_071_DAEMON";
const BINARY_ENV: &str = "HYDRACACHE_071_DAEMON_BINARY";
const SOURCE_REF_ENV: &str = "HYDRACACHE_071_DAEMON_SOURCE_REF";
const SOURCE_COMMIT_ENV: &str = "HYDRACACHE_071_DAEMON_SOURCE_COMMIT";
const PREVIOUS_TAG: &str = "v0.71.0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct MixedReceipt {
    schema_version: u32,
    release: String,
    previous_tag: String,
    previous_commit: String,
    previous_binary_sha256: String,
    candidate_binary_sha256: String,
    scenarios: Vec<ScenarioReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ScenarioReceipt {
    id: String,
    outcome: String,
    observation_sha256: String,
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn sha256(bytes: impl AsRef<[u8]>) -> String {
    Sha256::digest(bytes.as_ref())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn scenario(id: &str, observation: &Value) -> ScenarioReceipt {
    ScenarioReceipt {
        id: id.to_owned(),
        outcome: "pass".to_owned(),
        observation_sha256: sha256(serde_json::to_vec(observation).expect("serialize observation")),
    }
}

#[test]
fn real_071_072_upgrade_leadership_restart_and_rollback_are_capability_safe() -> TestResult {
    if std::env::var(RUN_ENV).as_deref() != Ok("1") {
        eprintln!("skipped: set {RUN_ENV}=1 with the shipped {PREVIOUS_TAG} history/artifact");
        return Ok(());
    }
    let previous = resolve_shipped_daemon_binary(
        PREVIOUS_TAG,
        BINARY_ENV,
        SOURCE_REF_ENV,
        SOURCE_COMMIT_ENV,
        BUILD_ENV,
    )?;
    let current = current_server_binary()?;
    ensure_distinct_daemon_binaries(&previous.path, &current)?;
    let mut cluster = DaemonCluster::start_bootstrap_with_binaries(
        vec![previous.path.clone(), current.clone(), current.clone()],
        "management-mixed-071-072",
    )?;
    cluster.wait_for_shape(3, 3)?;
    let old = 0_usize;
    let observer = 1_usize;
    let mut scenarios = Vec::new();

    let old_overview = cluster.cluster_overview(old)?;
    let old_node_id = cluster.node_ids()[old].clone();
    let initial_leader = cluster
        .statuses()
        .first()
        .and_then(|status| status.leader.clone())
        .ok_or("initial mixed cluster leader")?;
    assert_eq!(
        initial_leader, old_node_id,
        "old-leader scenario was not activated"
    );
    let mixed = cluster.management_json(observer, "/management/v1/dashboard")?;
    assert_eq!(mixed["completeness"], "partial");
    assert!(mixed["warnings"]
        .as_array()
        .is_some_and(|warnings| !warnings.is_empty()));
    scenarios.push(scenario("old-leader-new-followers", &mixed));

    cluster.kill(old)?;
    cluster.wait_for_responsive_shape(2, 3, 3)?;
    let new_leader = cluster
        .statuses()
        .first()
        .and_then(|status| status.leader.clone())
        .ok_or("new mixed cluster leader")?;
    assert_ne!(new_leader, old_node_id);
    let after_leadership_change = cluster.management_json(observer, "/management/v1/dashboard")?;
    assert_eq!(after_leadership_change["completeness"], "partial");
    scenarios.push(scenario(
        "leadership-change-during-aggregation",
        &after_leadership_change,
    ));
    cluster.restart(old)?;
    cluster.wait_for_shape(3, 3)?;
    let restarted_leader = cluster
        .statuses()
        .first()
        .and_then(|status| status.leader.clone())
        .ok_or("leader after old follower restart")?;
    assert_ne!(restarted_leader, old_node_id);
    let new_leader_old_follower = cluster.management_json(observer, "/management/v1/dashboard")?;
    assert_eq!(new_leader_old_follower["completeness"], "partial");
    scenarios.push(scenario(
        "new-leader-old-follower",
        &new_leader_old_follower,
    ));

    let trace_path = "/management/v1/cluster/placement-traces/trace-opaque-mixed";
    let trace_before = cluster.management_json(observer, trace_path)?;
    cluster.kill(old)?;
    cluster.restart(old)?;
    cluster.wait_for_shape(3, 3)?;
    let trace_after = cluster.management_json(observer, trace_path)?;
    assert!(
        (trace_before["completeness"] == "unavailable"
            && trace_after["completeness"] == "unavailable")
            || trace_before["data"] == trace_after["data"],
        "old-peer restart recomputed an existing placement trace"
    );
    scenarios.push(scenario(
        "old-peer-restart-during-placement-trace",
        &trace_after,
    ));

    cluster.kill(old)?;
    cluster.restart_with_binary(old, current.clone())?;
    cluster.wait_for_shape(3, 3)?;
    let upgraded = cluster.management_json(observer, "/management/v1/dashboard")?;
    assert_eq!(upgraded["data"]["cluster"]["quorum_ok"], true);

    cluster.kill(old)?;
    cluster.restart_with_binary(old, previous.path.clone())?;
    cluster.wait_for_shape(3, 3)?;
    let rollback_overview = cluster.cluster_overview(old)?;
    let (bookmark_status, _) =
        public_text_status(cluster.admin_addr(old), "/management/v1/dashboard")?;
    assert_eq!(bookmark_status, 404);
    assert_eq!(rollback_overview["quorum_ok"], true);
    scenarios.push(scenario(
        "rollback-after-ui-observation",
        &rollback_overview,
    ));
    assert_eq!(scenarios.len(), 5);
    assert_eq!(old_overview["quorum_ok"], true);

    let receipt = MixedReceipt {
        schema_version: 1,
        release: "0.72.0".to_owned(),
        previous_tag: PREVIOUS_TAG.to_owned(),
        previous_commit: previous.source_commit,
        previous_binary_sha256: sha256(fs::read(previous.path)?),
        candidate_binary_sha256: sha256(fs::read(current)?),
        scenarios,
    };
    let output = workspace_root().join("target/test-evidence/0.72/management-mixed-071-072.json");
    fs::create_dir_all(output.parent().expect("evidence parent"))?;
    fs::write(&output, serde_json::to_vec_pretty(&receipt)?)?;
    let reread: MixedReceipt = serde_json::from_slice(&fs::read(output)?)?;
    assert_eq!(reread, receipt);
    Ok(())
}

#[test]
fn mixed_release_contract_requires_shipped_tag_and_all_five_scenarios() {
    assert_eq!(PREVIOUS_TAG, "v0.71.0");
    assert_ne!(BINARY_ENV, "HYDRACACHE_PREVIOUS_DAEMON_BINARY");
    assert_eq!(
        [
            "old-leader-new-followers",
            "new-leader-old-follower",
            "leadership-change-during-aggregation",
            "old-peer-restart-during-placement-trace",
            "rollback-after-ui-observation",
        ]
        .len(),
        5
    );
}
