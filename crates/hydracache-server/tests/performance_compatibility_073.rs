mod support;

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use support::daemon_cluster::{
    ensure_distinct_daemon_binaries, resolve_shipped_daemon_binary, DaemonCluster, TestResult,
};

const RUN_ENV: &str = "HYDRACACHE_RUN_COMPATIBILITY_073_ROLLING";
const BUILD_B72_ENV: &str = "HYDRACACHE_BUILD_072_DAEMON";
const B72_BINARY_ENV: &str = "HYDRACACHE_072_DAEMON_BINARY";
const B72_SOURCE_REF_ENV: &str = "HYDRACACHE_072_DAEMON_SOURCE_REF";
const B72_SOURCE_COMMIT_ENV: &str = "HYDRACACHE_072_DAEMON_SOURCE_COMMIT";
const C73_BINARY_ENV: &str = "HYDRACACHE_C73_DAEMON_BINARY";
const C73_SOURCE_COMMIT_ENV: &str = "HYDRACACHE_C73_SOURCE_COMMIT";
const OUTPUT_ENV: &str = "HYDRACACHE_COMPATIBILITY_073_ROLLING_OUTPUT";
const B72_TAG: &str = "v0.72.0";
const B72_COMMIT: &str = "24927c28c279c6c34ad90111ee6470b4065e0815";
const C73_COMMIT: &str = "7e3070894aa51af96cdcb3e350eff923a309e1fa";

const SCENARIOS: [&str; 6] = [
    "B72-leader-C73-followers",
    "leadership-change-during-mixed-cluster",
    "C73-leader-B72-follower",
    "B72-follower-same-disk-restart",
    "full-C73-cluster",
    "same-disk-rollback-to-B72",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RollingReceipt {
    schema_version: u32,
    release: String,
    profile_id: String,
    published_tag: String,
    published_commit: String,
    candidate_commit: String,
    published_binary_sha256: String,
    candidate_binary_sha256: String,
    topology_setup: Vec<String>,
    scenarios: Vec<ScenarioReceipt>,
    result: String,
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

fn stable_raft_id(value: &str) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

    let mut hash = FNV_OFFSET;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash.max(1)
}

fn wait_for_leader(
    cluster: &mut DaemonCluster,
    expected_leader: &str,
    expected_statuses: usize,
) -> TestResult {
    cluster.wait_for(
        format!("leader {expected_leader} with {expected_statuses} responsive nodes"),
        |cluster| {
            let statuses = cluster.statuses();
            (statuses.len() == expected_statuses
                && statuses.iter().all(|status| {
                    status.leader.as_deref() == Some(expected_leader)
                        && status.members == 3
                        && status.voters == 3
                        && status.quorum_ok
                }))
            .then_some(())
        },
    )
}

fn scenario(id: &str, observation: &Value) -> ScenarioReceipt {
    ScenarioReceipt {
        id: id.to_owned(),
        outcome: "pass".to_owned(),
        observation_sha256: sha256(serde_json::to_vec(observation).expect("serialize observation")),
    }
}

fn candidate_binary() -> TestResult<PathBuf> {
    let source_commit = std::env::var(C73_SOURCE_COMMIT_ENV)
        .map_err(|_| format!("{C73_SOURCE_COMMIT_ENV}={C73_COMMIT} is required"))?;
    if source_commit != C73_COMMIT {
        return Err(format!(
            "candidate provenance mismatch: expected {C73_COMMIT}, got {source_commit}"
        )
        .into());
    }
    let binary =
        std::env::var_os(C73_BINARY_ENV).ok_or_else(|| format!("{C73_BINARY_ENV} is required"))?;
    let binary = fs::canonicalize(binary)
        .map_err(|error| format!("{C73_BINARY_ENV} is not readable: {error}"))?;
    if !binary.is_file() {
        return Err(format!("{C73_BINARY_ENV} is not a file: {}", binary.display()).into());
    }
    Ok(binary)
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

fn assert_legacy_overview_usable(overview: &Value) {
    assert_eq!(overview["source"], "live");
    assert_eq!(overview["members"].as_array().map(Vec::len), Some(3));
    assert!(overview["leader"]["node_id"].as_str().is_some());
}

#[test]
fn real_published_072_candidate_upgrade_restart_and_same_disk_rollback() -> TestResult {
    if std::env::var(RUN_ENV).as_deref() != Ok("1") {
        eprintln!("skipped: set {RUN_ENV}=1 with exact B72 and C73 binaries");
        return Ok(());
    }
    let baseline = resolve_shipped_daemon_binary(
        B72_TAG,
        B72_BINARY_ENV,
        B72_SOURCE_REF_ENV,
        B72_SOURCE_COMMIT_ENV,
        BUILD_B72_ENV,
    )?;
    if baseline.source_commit != B72_COMMIT || !baseline.shipped_tag {
        return Err("published B72 provenance was not exact".into());
    }
    let candidate = candidate_binary()?;
    ensure_distinct_daemon_binaries(&baseline.path, &candidate)?;

    // Bootstrap all three nodes on the published binary so the first leader is
    // deterministically B72. Upgrade only followers before observing the first
    // mixed state; concurrent mixed bootstrap cannot guarantee this topology.
    let mut cluster = DaemonCluster::start_bootstrap_with_binaries(
        vec![baseline.path.clone(); 3],
        "performance-compatibility-072-073",
    )?;
    let initial = cluster.wait_for_responsive_shape(3, 3, 3)?;
    let initial_leader = initial[0].leader.clone().ok_or("initial B72 leader")?;
    let node_ids = cluster.node_ids();
    let initial_leader_index = node_ids
        .iter()
        .position(|node_id| node_id == &initial_leader)
        .ok_or("initial leader belongs to cluster")?;
    let baseline_index = node_ids
        .iter()
        .enumerate()
        .min_by_key(|(_, node_id)| stable_raft_id(node_id))
        .map(|(index, _)| index)
        .ok_or("cluster has a lowest-rank B72 node")?;
    let baseline_node_id = node_ids[baseline_index].clone();
    let mut topology_setup = Vec::new();
    if initial_leader_index == baseline_index {
        topology_setup.push("lowest-rank-B72-was-bootstrap-leader".to_owned());
    } else {
        cluster.kill(initial_leader_index)?;
        wait_for_leader(&mut cluster, &baseline_node_id, 2)?;
        cluster.restart(initial_leader_index)?;
        wait_for_leader(&mut cluster, &baseline_node_id, 3)?;
        topology_setup.push("stopped-bootstrap-leader-to-elect-lowest-rank-B72".to_owned());
    }
    let candidate_indices = (0..3)
        .filter(|index| *index != baseline_index)
        .collect::<Vec<_>>();
    for index in &candidate_indices {
        cluster.kill(*index)?;
        cluster.restart_with_binary(*index, candidate.clone())?;
        cluster.wait_for_responsive_shape(3, 3, 3)?;
    }
    let observer = candidate_indices[0];
    let mut scenarios = Vec::new();

    let mixed_leader = cluster
        .statuses()
        .first()
        .and_then(|status| status.leader.clone())
        .ok_or("mixed leader")?;
    assert_eq!(mixed_leader, baseline_node_id);
    let old_leader = wait_for_management_json(
        &mut cluster,
        observer,
        "/management/v1/dashboard",
        "C73 dashboard with B72 leader",
    )?;
    assert_eq!(old_leader["completeness"], "partial");
    scenarios.push(scenario(SCENARIOS[0], &old_leader));

    cluster.kill(baseline_index)?;
    let changed = cluster.wait_for_leader_not(&baseline_node_id, 3, 3)?;
    let candidate_leader = changed
        .first()
        .and_then(|status| status.leader.clone())
        .ok_or("C73 leader after mixed leadership change")?;
    assert_ne!(candidate_leader, baseline_node_id);
    cluster.wait_for_responsive_shape(2, 3, 3)?;
    let leadership_change = wait_for_management_json(
        &mut cluster,
        observer,
        "/management/v1/dashboard",
        "dashboard after mixed leadership change",
    )?;
    assert_eq!(leadership_change["data"]["cluster"]["quorum_ok"], true);
    scenarios.push(scenario(SCENARIOS[1], &leadership_change));

    cluster.restart(baseline_index)?;
    cluster.wait_for_responsive_shape(3, 3, 3)?;
    let leader_after_restart = cluster
        .statuses()
        .first()
        .and_then(|status| status.leader.clone())
        .ok_or("leader with B72 follower")?;
    assert_ne!(leader_after_restart, baseline_node_id);
    let new_leader_old_follower = wait_for_management_json(
        &mut cluster,
        observer,
        "/management/v1/dashboard",
        "C73 leader with B72 follower",
    )?;
    assert_eq!(new_leader_old_follower["completeness"], "partial");
    scenarios.push(scenario(SCENARIOS[2], &new_leader_old_follower));

    cluster.kill(baseline_index)?;
    cluster.restart(baseline_index)?;
    cluster.wait_for_responsive_shape(3, 3, 3)?;
    assert_eq!(cluster.binary_path(baseline_index), baseline.path);
    let restarted_follower = wait_for_management_json(
        &mut cluster,
        observer,
        "/management/v1/dashboard",
        "dashboard after same-disk B72 follower restart",
    )?;
    assert_eq!(restarted_follower["data"]["cluster"]["quorum_ok"], true);
    scenarios.push(scenario(SCENARIOS[3], &restarted_follower));

    cluster.kill(baseline_index)?;
    cluster.restart_with_binary(baseline_index, candidate.clone())?;
    cluster.wait_for_responsive_shape(3, 3, 3)?;
    let full_candidate = wait_for_management_json(
        &mut cluster,
        observer,
        "/management/v1/dashboard",
        "dashboard after full C73 upgrade",
    )?;
    assert_eq!(full_candidate["data"]["cluster"]["quorum_ok"], true);
    assert!(cluster
        .binary_paths()
        .iter()
        .all(|binary| binary == &candidate));
    scenarios.push(scenario(SCENARIOS[4], &full_candidate));

    cluster.kill(baseline_index)?;
    cluster.restart_with_binary(baseline_index, baseline.path.clone())?;
    cluster.wait_for_responsive_shape(3, 3, 3)?;
    let rollback = wait_for_cluster_overview(
        &mut cluster,
        baseline_index,
        "B72 legacy overview after same-disk C73 rollback",
    )?;
    assert_legacy_overview_usable(&rollback);
    let rollback_dashboard = wait_for_management_json(
        &mut cluster,
        baseline_index,
        "/management/v1/dashboard",
        "B72 management dashboard after same-disk C73 rollback",
    )?;
    assert_eq!(rollback_dashboard["schema_version"], 1);
    assert_eq!(rollback_dashboard["data"]["cluster"]["quorum_ok"], true);
    scenarios.push(scenario(
        SCENARIOS[5],
        &serde_json::json!({
            "legacy_overview": rollback,
            "management_dashboard": rollback_dashboard,
        }),
    ));
    assert_eq!(
        scenarios
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>(),
        SCENARIOS
    );

    let receipt = RollingReceipt {
        schema_version: 1,
        release: "0.73".to_owned(),
        profile_id: "published-072-compatibility-073-v1".to_owned(),
        published_tag: B72_TAG.to_owned(),
        published_commit: baseline.source_commit,
        candidate_commit: C73_COMMIT.to_owned(),
        published_binary_sha256: sha256(fs::read(baseline.path)?),
        candidate_binary_sha256: sha256(fs::read(candidate)?),
        topology_setup,
        scenarios,
        result: "passed".to_owned(),
    };
    let output = std::env::var_os(OUTPUT_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            workspace_root().join("target/test-evidence/0.73/compatibility-rolling-072-073.json")
        });
    fs::create_dir_all(output.parent().ok_or("rolling output has no parent")?)?;
    fs::write(&output, serde_json::to_vec_pretty(&receipt)?)?;
    let reread: RollingReceipt = serde_json::from_slice(&fs::read(output)?)?;
    assert_eq!(reread, receipt);
    Ok(())
}

#[test]
fn rolling_contract_requires_exact_published_candidate_and_six_scenarios() {
    assert_eq!(B72_TAG, "v0.72.0");
    assert_eq!(B72_COMMIT.len(), 40);
    assert_eq!(C73_COMMIT.len(), 40);
    assert_ne!(B72_BINARY_ENV, C73_BINARY_ENV);
    assert_eq!(SCENARIOS.len(), 6);
    assert_eq!(
        SCENARIOS,
        [
            "B72-leader-C73-followers",
            "leadership-change-during-mixed-cluster",
            "C73-leader-B72-follower",
            "B72-follower-same-disk-restart",
            "full-C73-cluster",
            "same-disk-rollback-to-B72",
        ]
    );
}

#[test]
fn workflow_builds_exact_products_falsifies_and_seals_every_matrix() {
    let workflow = include_str!("../../../.github/workflows/compatibility-073.yml");
    let entry = include_str!("../../../.github/workflows/performance-host-admission-073.yml");
    for required in [
        "git worktree add --detach \"$b72_root\" \"$B72_SHA\"",
        "git worktree add --detach \"$c73_root\" \"$C73_SHA\"",
        "Cargo.b72.lock",
        "--mode canary",
        "--mode campaign",
        "real_published_072_candidate_upgrade_restart_and_same_disk_rollback",
        "wire_cells_passed",
        "durable_transitions_passed",
        "rolling_scenarios_passed",
        "retention-days: 30",
    ] {
        assert!(workflow.contains(required), "workflow omitted {required}");
    }
    assert!(workflow.contains("if: always()"));
    assert!(workflow.contains("result\": \"incomplete"));
    assert!(entry.contains("inputs.lease_owner != 'compatibility-073'"));
    assert!(entry.contains("inputs.lease_owner == 'compatibility-073'"));
    assert!(entry.contains("runs-on: ubuntu-latest"));
    assert!(entry.contains("scripts/perf/run_compatibility_073_ci.sh"));
    assert!(entry.contains("if: always()"));
    assert!(entry.contains("retention-days: 30"));
}
