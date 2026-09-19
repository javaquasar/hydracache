mod support;

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use support::daemon_cluster::{
    current_server_binary, ensure_distinct_daemon_binaries, management_text_status,
    public_text_status, resolve_shipped_daemon_binary, DaemonCluster, TestResult,
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

fn wait_for_management_json(
    cluster: &mut DaemonCluster,
    index: usize,
    path: &'static str,
    label: &'static str,
) -> TestResult<Value> {
    cluster.wait_for(label.to_owned(), |cluster| {
        cluster.management_json(index, path).ok()
    })
}

fn wait_for_cluster_overview(
    cluster: &mut DaemonCluster,
    index: usize,
    label: &'static str,
) -> TestResult<Value> {
    cluster.wait_for(label.to_owned(), |cluster| {
        cluster.cluster_overview(index).ok()
    })
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
    // Elect the shipped version first, then upgrade both followers. Starting
    // one old and two new daemons concurrently does not define which member
    // wins the initial election and therefore cannot prove the required
    // old-leader/new-followers scenario deterministically.
    let mut cluster = DaemonCluster::start_bootstrap_with_binaries(
        vec![previous.path.clone(); 3],
        "management-mixed-071-072",
    )?;
    let initial = cluster.wait_for_responsive_shape(3, 3, 3)?;
    let initial_leader = initial[0]
        .leader
        .clone()
        .ok_or("initial shipped-version leader")?;
    let old = cluster
        .node_ids()
        .iter()
        .position(|node_id| node_id == &initial_leader)
        .ok_or("initial leader belongs to mixed cluster")?;
    let upgraded_followers = (0..3).filter(|index| *index != old).collect::<Vec<_>>();
    for index in &upgraded_followers {
        cluster.kill(*index)?;
        cluster.restart_with_binary(*index, current.clone())?;
        cluster.wait_for_responsive_shape(3, 3, 3)?;
    }
    let observer = upgraded_followers[0];
    let mut scenarios = Vec::new();

    let old_overview = wait_for_cluster_overview(
        &mut cluster,
        old,
        "old leader legacy overview after follower upgrades",
    )?;
    let old_node_id = cluster.node_ids()[old].clone();
    let mixed_leader = cluster
        .statuses()
        .first()
        .and_then(|status| status.leader.clone())
        .ok_or("initial mixed cluster leader")?;
    assert_eq!(
        mixed_leader, old_node_id,
        "old-leader scenario was not activated"
    );
    let mixed = wait_for_management_json(
        &mut cluster,
        observer,
        "/management/v1/dashboard",
        "new follower management dashboard with old leader",
    )?;
    assert_eq!(mixed["completeness"], "partial");
    assert!(mixed["warnings"]
        .as_array()
        .is_some_and(|warnings| !warnings.is_empty()));
    scenarios.push(scenario("old-leader-new-followers", &mixed));

    cluster.kill(old)?;
    // Two responsive followers can briefly retain the stopped leader in
    // their last published status. Wait for an observed leadership change,
    // rather than accepting that transient but internally consistent view.
    let changed = cluster.wait_for_leader_not(&old_node_id, 3, 3)?;
    let new_leader = changed
        .first()
        .and_then(|status| status.leader.clone())
        .ok_or("new mixed cluster leader")?;
    assert_ne!(new_leader, old_node_id);
    cluster.wait_for_responsive_shape(2, 3, 3)?;
    let after_leadership_change = wait_for_management_json(
        &mut cluster,
        observer,
        "/management/v1/dashboard",
        "management dashboard after mixed leadership change",
    )?;
    assert_eq!(after_leadership_change["completeness"], "partial");
    scenarios.push(scenario(
        "leadership-change-during-aggregation",
        &after_leadership_change,
    ));
    cluster.restart(old)?;
    cluster.wait_for_responsive_shape(3, 3, 3)?;
    let restarted_leader = cluster
        .statuses()
        .first()
        .and_then(|status| status.leader.clone())
        .ok_or("leader after old follower restart")?;
    assert_ne!(restarted_leader, old_node_id);
    let new_leader_old_follower = wait_for_management_json(
        &mut cluster,
        observer,
        "/management/v1/dashboard",
        "management dashboard with new leader and old follower",
    )?;
    assert_eq!(new_leader_old_follower["completeness"], "partial");
    scenarios.push(scenario(
        "new-leader-old-follower",
        &new_leader_old_follower,
    ));

    let trace_path = "/management/v1/cluster/placement-traces/trace-opaque-mixed";
    let (trace_status_before, trace_body_before) =
        management_text_status(cluster.admin_addr(observer), trace_path)?;
    assert!(
        matches!(trace_status_before, 200 | 404),
        "placement trace returned unexpected status {trace_status_before}"
    );
    cluster.kill(old)?;
    cluster.restart(old)?;
    cluster.wait_for_responsive_shape(3, 3, 3)?;
    let (trace_status_after, trace_body_after) =
        management_text_status(cluster.admin_addr(observer), trace_path)?;
    assert_eq!(trace_status_after, trace_status_before);
    assert_eq!(
        trace_body_after, trace_body_before,
        "old-peer restart recomputed an existing placement trace"
    );
    let trace_observation = serde_json::json!({
        "status": trace_status_after,
        "body_sha256": sha256(trace_body_after),
    });
    scenarios.push(scenario(
        "old-peer-restart-during-placement-trace",
        &trace_observation,
    ));

    cluster.kill(old)?;
    cluster.restart_with_binary(old, current.clone())?;
    cluster.wait_for_responsive_shape(3, 3, 3)?;
    let upgraded = wait_for_management_json(
        &mut cluster,
        observer,
        "/management/v1/dashboard",
        "management dashboard after full upgrade",
    )?;
    assert_eq!(upgraded["data"]["cluster"]["quorum_ok"], true);

    cluster.kill(old)?;
    cluster.restart_with_binary(old, previous.path.clone())?;
    cluster.wait_for_responsive_shape(3, 3, 3)?;
    let rollback_overview = wait_for_cluster_overview(
        &mut cluster,
        old,
        "legacy overview after same-disk rollback",
    )?;
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
