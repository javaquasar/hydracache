mod support;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use support::daemon_cluster::{
    assign_explicit_node_binaries, current_server_binary, ensure_distinct_daemon_binaries,
    resolve_previous_daemon_binary, skip_unless_daemon_process_e2e, DaemonCluster, DaemonStatus,
    PreviousDaemonBinary, TestResult, BUILD_PREVIOUS_DAEMON_ENV, PREVIOUS_DAEMON_BINARY_ENV,
    PREVIOUS_DAEMON_SOURCE_COMMIT_ENV, PREVIOUS_DAEMON_SOURCE_REF_ENV,
};
use support::membership_history::{MembershipHistoryRecorder, MembershipObservation};

#[derive(Debug)]
struct MixedBinaries {
    previous: PreviousDaemonBinary,
    current: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RaftObservation {
    applied_index: u64,
    snapshot_index: u64,
    first_log_index: u64,
    snapshot_send_successes: u64,
}

#[test]
fn daemon_cluster_supports_explicit_binary_per_node() -> TestResult {
    let current = PathBuf::from("target/current/hydracache-server");
    let requested = vec![
        PathBuf::from("target/previous/hydracache-server"),
        current.clone(),
        PathBuf::from("target/candidate/hydracache-server"),
    ];

    let assigned = assign_explicit_node_binaries(3, requested.clone(), &current)?;
    assert_eq!(assigned, requested);
    assert!(assign_explicit_node_binaries(2, vec![current.clone()], &current).is_err());
    assert!(assign_explicit_node_binaries(1, vec![PathBuf::new()], &current).is_err());
    Ok(())
}

#[test]
fn mixed_065_066_daemons_converge_during_snapshot_catchup() -> TestResult {
    let Some(binaries) = mixed_binaries("mixed_065_066_daemons_converge_during_snapshot_catchup")?
    else {
        return Ok(());
    };
    let mut cluster = DaemonCluster::start_bootstrap_with_binaries_and_raft_compaction(
        vec![
            binaries.previous.path.clone(),
            binaries.current.clone(),
            binaries.current.clone(),
        ],
        "mixed-version-snapshot-catchup",
    )?;
    binaries.previous.write_provenance(cluster.root())?;
    assert_eq!(cluster.binary_path(0), binaries.previous.path.as_path());

    let statuses = cluster.wait_for_shape(3, 3)?;
    let old_index = 0usize;
    let current_indices = vec![1usize, 2usize];
    let old_baseline = wait_for_equal_applied_indices(&mut cluster, &current_indices, 0, false)?;
    let expected_members = cluster.node_ids().into_iter().collect::<BTreeSet<_>>();
    cluster.kill(old_index)?;
    cluster.wait_for_responsive_shape(2, 3, 3)?;

    let mut previous_applied = old_baseline;
    for _ in 0..2 {
        let statuses = cluster.wait_for_responsive_shape(2, 3, 3)?;
        let leader = leader_index(&cluster, &statuses)?;
        let churn = cluster
            .running_indices()
            .into_iter()
            .find(|index| *index != leader && *index != old_index)
            .ok_or("mixed cluster needs one live follower for metadata churn")?;
        cluster.kill(churn)?;
        cluster.restart(churn)?;
        cluster.wait_for_responsive_shape(2, 3, 3)?;
        previous_applied =
            wait_for_equal_applied_progress(&mut cluster, 2, previous_applied, true)?;
    }

    let active = cluster.running_indices();
    let compacted_index = wait_for_equal_applied_progress(&mut cluster, 2, old_baseline, true)?;
    let sends_before = snapshot_success_sum(&cluster, &active)?;
    for index in &active {
        let compacted = cluster.compact_raft_log(*index)?;
        assert_eq!(u64_field(&compacted, "snapshot_index")?, compacted_index);
        assert!(
            u64_field(&compacted, "first_log_index")? > old_baseline,
            "compaction must move past the previous-version lagger"
        );
    }

    cluster.restart(old_index)?;
    cluster.wait_for_responsive_shape(3, 3, 3)?;
    let converged = wait_for_equal_applied_indices(&mut cluster, &active, compacted_index, false)?;
    assert!(converged >= compacted_index);
    let observations = wait_for_consensus_members(&mut cluster, 3, &expected_members)?;
    assert_eq!(observations[old_index].members, expected_members);
    assert_eq!(
        cluster.binary_path(old_index),
        binaries.previous.path.as_path()
    );
    assert!(
        snapshot_success_sum(&cluster, &active)? > sends_before,
        "a current daemon must successfully deliver the compacted snapshot to the previous daemon"
    );
    assert!(
        statuses.iter().all(|status| status.members == 3),
        "mixed window must begin from committed three-member metadata"
    );
    Ok(())
}

#[test]
fn rolling_upgrade_during_membership_change_loses_no_committed_metadata() -> TestResult {
    let Some(binaries) =
        mixed_binaries("rolling_upgrade_during_membership_change_loses_no_committed_metadata")?
    else {
        return Ok(());
    };
    let mut cluster = DaemonCluster::start_bootstrap_with_binaries_and_raft_compaction(
        vec![binaries.previous.path.clone(); 3],
        "rolling-upgrade-membership-change",
    )?;
    binaries.previous.write_provenance(cluster.root())?;
    let mut history = MembershipHistoryRecorder::default();

    let statuses = cluster.wait_for_shape(3, 3)?;
    record_overviews(&mut cluster, &mut history);
    let leader = leader_index(&cluster, &statuses)?;
    let drain_index = (0..3)
        .find(|index| *index != leader)
        .ok_or("rolling cluster must expose a follower to drain")?;

    let _ = cluster.drain(drain_index)?;
    cluster.wait_for_non_draining_shape("mixed-version drain commits", 2, 2)?;
    cluster.kill(drain_index)?;
    cluster.wait_for_responsive_shape(2, 2, 2)?;
    record_overviews(&mut cluster, &mut history);
    let committed_members = consensus_member_set(&mut cluster)?;
    let expected_members = cluster
        .node_ids()
        .into_iter()
        .enumerate()
        .filter_map(|(index, node_id)| (index != drain_index).then_some(node_id))
        .collect::<BTreeSet<_>>();
    assert_eq!(committed_members, expected_members);
    let drain_observations = wait_for_consensus_members(&mut cluster, 2, &expected_members)?;
    let drain_epoch = drain_observations
        .iter()
        .map(|observation| observation.epoch)
        .max()
        .unwrap_or_default();

    let active = cluster.running_indices();
    for index in active.iter().copied() {
        cluster.kill(index)?;
        cluster.restart_with_binary(index, binaries.current.clone())?;
        cluster.wait_for_responsive_shape(2, 2, 2)?;
        wait_for_consensus_members(&mut cluster, 2, &expected_members)?;
        record_overviews(&mut cluster, &mut history);
    }

    cluster.wait_for_responsive_shape(2, 2, 2)?;
    record_overviews(&mut cluster, &mut history);
    assert_eq!(consensus_member_set(&mut cluster)?, expected_members);
    let final_observations = wait_for_consensus_members(&mut cluster, 2, &expected_members)?;
    assert!(
        final_observations
            .iter()
            .all(|observation| observation.epoch >= drain_epoch),
        "rolling replacement regressed committed membership epoch"
    );
    assert!(active
        .iter()
        .all(|index| cluster.binary_path(*index) == binaries.current.as_path()));
    assert_eq!(
        cluster.binary_path(drain_index),
        binaries.previous.path.as_path(),
        "a removed voter is outside the committed rolling-upgrade target and must not be silently resurrected"
    );
    for index in active {
        assert!(
            raft_observation(&cluster, index)?.applied_index > 0,
            "upgraded voter {index} lost committed membership metadata"
        );
    }

    let report = history.check();
    assert!(
        report.is_ok(),
        "rolling mixed-version membership history violated invariants: {:?}",
        report.violations
    );
    Ok(())
}

#[test]
fn canary_mixed_daemon_harness_silently_substitutes_current_binary() -> TestResult {
    let previous = PathBuf::from("target/previous/hydracache-server");
    let current = PathBuf::from("target/current/hydracache-server");
    let assigned =
        assign_explicit_node_binaries(2, vec![previous.clone(), current.clone()], &current)?;
    assert!(
        assigned[0] == previous,
        "HC-CANARY-RED:W6 mixed daemon harness silently substituted the current binary"
    );
    Ok(())
}

fn mixed_binaries(test_name: &str) -> TestResult<Option<MixedBinaries>> {
    if !skip_unless_daemon_process_e2e(test_name) {
        return Ok(None);
    }
    assert_w32_compatibility_inputs_are_present()?;
    let Some(previous) = resolve_previous_daemon_binary()? else {
        eprintln!(
            "skipping {test_name}: provide {PREVIOUS_DAEMON_BINARY_ENV}, {PREVIOUS_DAEMON_SOURCE_REF_ENV}, and {PREVIOUS_DAEMON_SOURCE_COMMIT_ENV}, or set {BUILD_PREVIOUS_DAEMON_ENV}=1"
        );
        return Ok(None);
    };
    let current = current_server_binary()?;
    ensure_distinct_daemon_binaries(&previous.path, &current)?;
    Ok(Some(MixedBinaries { previous, current }))
}

fn assert_w32_compatibility_inputs_are_present() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let compat_test = root.join("crates/hydracache-cluster-raft/tests/compat_matrix.rs");
    if !compat_test.is_file() {
        return Err(format!(
            "W32 compatibility owner is missing: {}",
            compat_test.display()
        )
        .into());
    }
    let manifest_path = root.join("docs/testing/compat/v0.63.0.json");
    let manifest: Value = serde_json::from_slice(&fs::read(&manifest_path)?)?;
    let artifacts = manifest
        .get("artifacts")
        .and_then(Value::as_array)
        .ok_or("W32 compatibility manifest has no artifacts")?;
    if artifacts.is_empty() {
        return Err("W32 compatibility manifest artifact list is empty".into());
    }
    for artifact in artifacts {
        let relative = artifact
            .get("path")
            .and_then(Value::as_str)
            .ok_or("W32 compatibility artifact has no path")?;
        if !root.join(relative).is_file() {
            return Err(format!("W32 compatibility fixture is missing: {relative}").into());
        }
    }
    Ok(())
}

fn record_overviews(cluster: &mut DaemonCluster, history: &mut MembershipHistoryRecorder) {
    for overview in cluster.overviews() {
        history.record_cluster_overview(&overview);
    }
}

fn consensus_member_set(cluster: &mut DaemonCluster) -> TestResult<BTreeSet<String>> {
    let member_sets = cluster
        .overviews()
        .iter()
        .map(MembershipObservation::from_cluster_overview)
        .map(|observation| observation.members)
        .collect::<Vec<_>>();
    let expected = member_sets
        .first()
        .cloned()
        .ok_or("responsive cluster exposed no membership overview")?;
    if member_sets.iter().any(|members| members != &expected) {
        return Err(format!("daemons disagree on committed members: {member_sets:?}").into());
    }
    Ok(expected)
}

fn wait_for_equal_applied_progress(
    cluster: &mut DaemonCluster,
    expected_responsive: usize,
    minimum: u64,
    strictly_greater: bool,
) -> TestResult<u64> {
    cluster.wait_for(
        format!(
            "responsive={expected_responsive} daemons converge at applied {} {minimum}",
            if strictly_greater { ">" } else { ">=" }
        ),
        |cluster| {
            let indices = cluster.running_indices();
            if indices.len() != expected_responsive {
                return None;
            }
            let applied = indices
                .iter()
                .map(|index| {
                    raft_observation(cluster, *index)
                        .ok()
                        .map(|value| value.applied_index)
                })
                .collect::<Option<Vec<_>>>()?;
            let meets_minimum = if strictly_greater {
                applied[0] > minimum
            } else {
                applied[0] >= minimum
            };
            (meets_minimum && applied.iter().all(|index| *index == applied[0]))
                .then_some(applied[0])
        },
    )
}

fn wait_for_equal_applied_indices(
    cluster: &mut DaemonCluster,
    indices: &[usize],
    minimum: u64,
    strictly_greater: bool,
) -> TestResult<u64> {
    let indices = indices.to_vec();
    cluster.wait_for(
        format!(
            "daemons {indices:?} converge at applied {} {minimum}",
            if strictly_greater { ">" } else { ">=" }
        ),
        move |cluster| {
            let applied = indices
                .iter()
                .map(|index| {
                    raft_observation(cluster, *index)
                        .ok()
                        .map(|value| value.applied_index)
                })
                .collect::<Option<Vec<_>>>()?;
            let first = *applied.first()?;
            let meets_minimum = if strictly_greater {
                first > minimum
            } else {
                first >= minimum
            };
            (meets_minimum && applied.iter().all(|index| *index == first)).then_some(first)
        },
    )
}

fn wait_for_consensus_members(
    cluster: &mut DaemonCluster,
    expected_responsive: usize,
    expected_members: &BTreeSet<String>,
) -> TestResult<Vec<MembershipObservation>> {
    let expected_members = expected_members.clone();
    cluster.wait_for(
        format!(
            "responsive={expected_responsive} daemons retain committed members {expected_members:?}"
        ),
        move |cluster| {
            let overviews = cluster.overviews();
            if overviews.len() != expected_responsive {
                return None;
            }
            let observations = overviews
                .iter()
                .map(MembershipObservation::from_cluster_overview)
                .collect::<Vec<_>>();
            (observations
                .iter()
                .all(|observation| observation.members == expected_members))
            .then_some(observations)
        },
    )
}

fn snapshot_success_sum(cluster: &DaemonCluster, indices: &[usize]) -> TestResult<u64> {
    indices.iter().try_fold(0_u64, |total, index| {
        Ok(total.saturating_add(raft_observation(cluster, *index)?.snapshot_send_successes))
    })
}

fn leader_index(cluster: &DaemonCluster, statuses: &[DaemonStatus]) -> TestResult<usize> {
    let leader = statuses
        .iter()
        .find_map(|status| status.leader.as_deref())
        .ok_or("responsive mixed cluster has no leader")?;
    cluster
        .node_ids()
        .iter()
        .position(|node_id| node_id == leader)
        .ok_or_else(|| format!("leader {leader} is not a spawned daemon").into())
}

fn raft_observation(cluster: &DaemonCluster, index: usize) -> TestResult<RaftObservation> {
    let value = cluster.raft_compaction_status(index)?;
    Ok(RaftObservation {
        applied_index: u64_field(&value, "applied_index")?,
        snapshot_index: u64_field(&value, "snapshot_index")?,
        first_log_index: u64_field(&value, "first_log_index")?,
        snapshot_send_successes: value
            .get("snapshot_send_successes")
            .and_then(Value::as_u64)
            .unwrap_or_default(),
    })
}

fn u64_field(value: &Value, field: &'static str) -> TestResult<u64> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("raft compaction status missing {field}: {value}").into())
}
