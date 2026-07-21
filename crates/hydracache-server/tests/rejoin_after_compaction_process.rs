mod support;

use serde_json::Value;
use support::daemon_cluster::{
    skip_unless_daemon_process_e2e, DaemonCluster, DaemonStatus, TestResult,
};

const SNAPSHOT_HANDLER_TEST_DELAY_MS: u64 = 30_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RaftProcessObservation {
    applied_index: u64,
    snapshot_index: u64,
    snapshot_send_attempts: u64,
    snapshot_send_successes: u64,
    snapshot_send_failures: u64,
    snapshot_sends_in_flight: u64,
    snapshot_installs: u64,
}

#[derive(Debug)]
struct PreparedSnapshotCatchup {
    lagger_index: usize,
    compacted_index: u64,
    active_indices: Vec<usize>,
    successful_sends_before_rejoin: u64,
}

#[test]
fn lagging_daemon_rejoins_via_snapshot_after_real_sled_compaction() -> TestResult {
    if !skip_unless_daemon_process_e2e(
        "lagging_daemon_rejoins_via_snapshot_after_real_sled_compaction",
    ) {
        return Ok(());
    }

    let mut cluster = DaemonCluster::start_bootstrap_with_raft_compaction(
        3,
        "rejoin-after-real-sled-compaction",
    )?;
    let prepared = prepare_compacted_lagger(&mut cluster)?;

    cluster.restart(prepared.lagger_index)?;
    wait_for_snapshot_install_and_convergence(
        &mut cluster,
        prepared.lagger_index,
        prepared.compacted_index,
        3,
    )?;

    let successful_sends = snapshot_success_sum(&cluster, &prepared.active_indices)?;
    assert!(
        successful_sends > prepared.successful_sends_before_rejoin,
        "rejoin must be observed as a successful real HTTP MsgSnapshot delivery"
    );
    Ok(())
}

#[test]
fn leader_killed_mid_snapshot_delivery_still_converges() -> TestResult {
    if !skip_unless_daemon_process_e2e("leader_killed_mid_snapshot_delivery_still_converges") {
        return Ok(());
    }

    let mut cluster =
        DaemonCluster::start_bootstrap_with_raft_compaction(3, "leader-killed-mid-snapshot")?;
    let prepared = prepare_compacted_lagger(&mut cluster)?;
    cluster.restart_with_snapshot_handler_delay(
        prepared.lagger_index,
        Some(SNAPSHOT_HANDLER_TEST_DELAY_MS),
    )?;
    let (old_leader_index, old_leader_status) = wait_for_snapshot_request_in_flight(&mut cluster)?;
    let old_leader = cluster.node_ids()[old_leader_index].clone();

    cluster.kill(old_leader_index)?;
    // Closing the delayed receiver clears the dead leader's accepted socket,
    // so the replacement leader must own the successful retry.
    cluster.kill(prepared.lagger_index)?;
    cluster.restart_with_snapshot_handler_delay(prepared.lagger_index, None)?;
    cluster.wait_for_leader_not(&old_leader, 3, 3)?;
    wait_for_snapshot_install_and_convergence(
        &mut cluster,
        prepared.lagger_index,
        prepared.compacted_index,
        2,
    )?;

    let replacement_indices = prepared
        .active_indices
        .iter()
        .copied()
        .filter(|index| *index != old_leader_index)
        .collect::<Vec<_>>();
    assert!(
        snapshot_success_sum(&cluster, &replacement_indices)? > 0,
        "replacement leader must complete the snapshot retry; old leader observation={old_leader_status:?}"
    );

    cluster.restart(old_leader_index)?;
    cluster.wait_for_shape(3, 3)?;
    wait_for_equal_applied_progress(&mut cluster, 3, prepared.compacted_index)?;
    Ok(())
}

#[test]
fn receiver_killed_mid_snapshot_request_releases_sender_and_retry_converges() -> TestResult {
    if !skip_unless_daemon_process_e2e(
        "receiver_killed_mid_snapshot_request_releases_sender_and_retry_converges",
    ) {
        return Ok(());
    }

    let mut cluster =
        DaemonCluster::start_bootstrap_with_raft_compaction(3, "receiver-killed-mid-snapshot")?;
    let prepared = prepare_compacted_lagger(&mut cluster)?;
    cluster.restart_with_snapshot_handler_delay(
        prepared.lagger_index,
        Some(SNAPSHOT_HANDLER_TEST_DELAY_MS),
    )?;
    let (leader_index, in_flight) = wait_for_snapshot_request_in_flight(&mut cluster)?;

    cluster.kill(prepared.lagger_index)?;
    cluster.wait_for(
        "snapshot sender releases failed receiver request".to_owned(),
        |cluster| {
            let observation = observation(cluster, leader_index).ok()?;
            (observation.snapshot_send_failures > in_flight.snapshot_send_failures
                && observation.snapshot_sends_in_flight == 0)
                .then_some(observation)
        },
    )?;

    cluster.restart_with_snapshot_handler_delay(prepared.lagger_index, None)?;
    wait_for_snapshot_install_and_convergence(
        &mut cluster,
        prepared.lagger_index,
        prepared.compacted_index,
        3,
    )?;
    let after = observation(&cluster, leader_index)?;
    assert!(
        after.snapshot_send_successes > in_flight.snapshot_send_successes,
        "sender must retry successfully after receiver restart: before={in_flight:?} after={after:?}"
    );
    Ok(())
}

#[test]
fn canary_snapshot_send_failure_leaves_peer_progress_stuck() {
    let peer_progress_stuck = std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W1");
    assert!(
        !peer_progress_stuck,
        "HC-CANARY-RED:W1 snapshot delivery failure left peer progress stuck"
    );
}

fn prepare_compacted_lagger(cluster: &mut DaemonCluster) -> TestResult<PreparedSnapshotCatchup> {
    let statuses = cluster.wait_for_shape(3, 3)?;
    let initial_leader_index = leader_index(cluster, &statuses)?;
    let lagger_index = (0..cluster.node_ids().len())
        .find(|index| *index != initial_leader_index)
        .ok_or("three-node cluster did not expose a follower")?;
    let lagger_before = observation(cluster, lagger_index)?;

    cluster.kill(lagger_index)?;
    cluster.wait_for_responsive_shape(2, 3, 3)?;

    let mut previous_applied = lagger_before.applied_index;
    for _ in 0..2 {
        let statuses = cluster.wait_for_responsive_shape(2, 3, 3)?;
        let current_leader = leader_index(cluster, &statuses)?;
        let churn_index = cluster
            .running_indices()
            .into_iter()
            .find(|index| *index != current_leader && *index != lagger_index)
            .ok_or("lagging cluster did not retain a live follower to generate metadata")?;
        cluster.kill(churn_index)?;
        cluster.restart(churn_index)?;
        cluster.wait_for_responsive_shape(2, 3, 3)?;
        previous_applied = wait_for_equal_applied_progress(cluster, 2, previous_applied)?;
    }

    let (active_indices, compacted_index) = cluster.wait_for(
        "two active daemons converge before Sled compaction".to_owned(),
        |cluster| {
            let indices = cluster.running_indices();
            if indices.len() != 2 {
                return None;
            }
            let applied = indices
                .iter()
                .map(|index| {
                    observation(cluster, *index)
                        .ok()
                        .map(|value| value.applied_index)
                })
                .collect::<Option<Vec<_>>>()?;
            (applied.iter().all(|index| *index == applied[0])
                && applied[0] > lagger_before.applied_index)
                .then_some((indices, applied[0]))
        },
    )?;
    let successful_sends_before_rejoin = snapshot_success_sum(cluster, &active_indices)?;

    for index in &active_indices {
        let compacted = cluster.compact_raft_log(*index)?;
        assert_eq!(
            u64_field(&compacted, "snapshot_index")?,
            compacted_index,
            "each possible leader must persist the same snapshot boundary"
        );
        assert!(
            u64_field(&compacted, "first_log_index")? > lagger_before.applied_index,
            "compaction must move retained-log progress beyond the lagger"
        );
    }

    Ok(PreparedSnapshotCatchup {
        lagger_index,
        compacted_index,
        active_indices,
        successful_sends_before_rejoin,
    })
}

fn wait_for_snapshot_install_and_convergence(
    cluster: &mut DaemonCluster,
    receiver_index: usize,
    minimum_applied: u64,
    expected_responsive: usize,
) -> TestResult {
    cluster.wait_for(
        format!("daemon {receiver_index} installs real HTTP MsgSnapshot"),
        |cluster| {
            let receiver = observation(cluster, receiver_index).ok()?;
            (receiver.snapshot_installs > 0 && receiver.applied_index >= minimum_applied)
                .then_some(())
        },
    )?;
    cluster.wait_for_responsive_shape(expected_responsive, 3, 3)?;
    wait_for_equal_applied_progress(cluster, expected_responsive, minimum_applied)?;
    Ok(())
}

fn wait_for_snapshot_request_in_flight(
    cluster: &mut DaemonCluster,
) -> TestResult<(usize, RaftProcessObservation)> {
    cluster.wait_for(
        "real HTTP MsgSnapshot request becomes in-flight".to_owned(),
        |cluster| {
            let statuses = cluster.statuses();
            let leader = leader_index(cluster, &statuses).ok()?;
            let observation = observation(cluster, leader).ok()?;
            (observation.snapshot_send_attempts > 0 && observation.snapshot_sends_in_flight > 0)
                .then_some((leader, observation))
        },
    )
}

fn wait_for_equal_applied_progress(
    cluster: &mut DaemonCluster,
    expected_responsive: usize,
    minimum_exclusive: u64,
) -> TestResult<u64> {
    cluster.wait_for(
        format!(
            "responsive={expected_responsive} daemons converge above applied={minimum_exclusive}"
        ),
        |cluster| {
            let indices = cluster.running_indices();
            if indices.len() != expected_responsive {
                return None;
            }
            let applied = indices
                .iter()
                .map(|index| {
                    observation(cluster, *index)
                        .ok()
                        .map(|value| value.applied_index)
                })
                .collect::<Option<Vec<_>>>()?;
            (applied.iter().all(|index| *index == applied[0]) && applied[0] > minimum_exclusive)
                .then_some(applied[0])
        },
    )
}

fn snapshot_success_sum(cluster: &DaemonCluster, indices: &[usize]) -> TestResult<u64> {
    indices.iter().try_fold(0_u64, |total, index| {
        Ok(total.saturating_add(observation(cluster, *index)?.snapshot_send_successes))
    })
}

fn leader_index(cluster: &DaemonCluster, statuses: &[DaemonStatus]) -> TestResult<usize> {
    let leader = statuses
        .iter()
        .find_map(|status| status.leader.as_deref())
        .ok_or("responsive cluster status did not expose a leader")?;
    cluster
        .node_ids()
        .iter()
        .position(|node_id| node_id == leader)
        .ok_or_else(|| format!("leader {leader} is not a spawned daemon").into())
}

fn observation(cluster: &DaemonCluster, index: usize) -> TestResult<RaftProcessObservation> {
    let value = cluster.raft_compaction_status(index)?;
    Ok(RaftProcessObservation {
        applied_index: u64_field(&value, "applied_index")?,
        snapshot_index: u64_field(&value, "snapshot_index")?,
        snapshot_send_attempts: u64_field(&value, "snapshot_send_attempts")?,
        snapshot_send_successes: u64_field(&value, "snapshot_send_successes")?,
        snapshot_send_failures: u64_field(&value, "snapshot_send_failures")?,
        snapshot_sends_in_flight: u64_field(&value, "snapshot_sends_in_flight")?,
        snapshot_installs: u64_field(&value, "snapshot_installs")?,
    })
}

fn u64_field(value: &Value, field: &'static str) -> TestResult<u64> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("raft compaction status missing {field}: {value}").into())
}
