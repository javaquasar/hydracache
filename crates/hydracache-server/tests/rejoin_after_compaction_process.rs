mod support;

use serde_json::Value;
use std::fs;
use support::daemon_cluster::{
    skip_unless_daemon_process_e2e, DaemonCluster, DaemonStatus, TestResult,
};

// Keep ordinary in-flight requests observable across multiple 200 ms polls.
const SNAPSHOT_HANDLER_TEST_DELAY_MS: u64 = 1_000;
const SNAPSHOT_HANDLER_DELAY_STARTED_MARKER: &str = "HC_TEST_RAFT_SNAPSHOT_HANDLER_DELAY_STARTED";
const SHARED_COMPACTION_ATTEMPTS: usize = 4;

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

    wait_for_snapshot_success_sum(
        &mut cluster,
        &prepared.active_indices,
        prepared.successful_sends_before_rejoin,
        "rejoin is acknowledged as a successful real HTTP MsgSnapshot delivery",
    )?;
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
    let (old_leader_index, old_leader_status) =
        wait_for_snapshot_handler_delay_started(&mut cluster, prepared.lagger_index, true)?;
    let old_leader = cluster.node_ids()[old_leader_index].clone();

    // Stop both ends of the observed request together. This removes the race
    // where a slow process kill could leave the receiver enough time to apply
    // the interrupted leader's snapshot before the receiver itself stopped.
    cluster.kill_pair_concurrently(old_leader_index, prepared.lagger_index)?;
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
    wait_for_snapshot_success_sum(
        &mut cluster,
        &replacement_indices,
        0,
        &format!(
            "replacement leader completes the snapshot retry; old leader observation={old_leader_status:?}"
        ),
    )?;

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
    let (leader_index, in_flight) =
        wait_for_snapshot_handler_delay_started(&mut cluster, prepared.lagger_index, false)?;

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
    wait_for_snapshot_success_sum(
        &mut cluster,
        &[leader_index],
        in_flight.snapshot_send_successes,
        &format!("sender retries successfully after receiver restart; before={in_flight:?}"),
    )?;
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

    let (active_indices, converged_index) = cluster.wait_for(
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
    let compacted_index = compact_active_logs_to_shared_boundary(
        cluster,
        &active_indices,
        converged_index,
        lagger_before.applied_index,
    )?;

    Ok(PreparedSnapshotCatchup {
        lagger_index,
        compacted_index,
        active_indices,
        successful_sends_before_rejoin,
    })
}

fn compact_active_logs_to_shared_boundary(
    cluster: &DaemonCluster,
    active_indices: &[usize],
    minimum_snapshot_index: u64,
    lagger_applied_index: u64,
) -> TestResult<u64> {
    let mut last_snapshot_indices = Vec::new();
    for _ in 0..SHARED_COMPACTION_ATTEMPTS {
        last_snapshot_indices.clear();
        for index in active_indices {
            let compacted = cluster.compact_raft_log(*index)?;
            let snapshot_index = u64_field(&compacted, "snapshot_index")?;
            let first_log_index = u64_field(&compacted, "first_log_index")?;
            assert!(
                snapshot_index >= minimum_snapshot_index,
                "compaction must not move a snapshot behind the converged applied boundary"
            );
            assert!(
                first_log_index > lagger_applied_index,
                "compaction must move retained-log progress beyond the lagger"
            );
            last_snapshot_indices.push(snapshot_index);
        }

        if let Some(shared_index) = last_snapshot_indices.first().copied() {
            if last_snapshot_indices
                .iter()
                .all(|snapshot_index| *snapshot_index == shared_index)
            {
                return Ok(shared_index);
            }
        }
    }

    Err(format!(
        "active daemons did not persist one snapshot boundary after {SHARED_COMPACTION_ATTEMPTS} attempts; last_snapshot_indices={last_snapshot_indices:?}"
    )
    .into())
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

fn wait_for_snapshot_handler_delay_started(
    cluster: &mut DaemonCluster,
    receiver_index: usize,
    require_current_leader: bool,
) -> TestResult<(usize, RaftProcessObservation)> {
    let log_suffix = format!("-{receiver_index}.stderr.log");
    let receiver_log = fs::read_dir(cluster.root())?
        .filter_map(Result::ok)
        .find(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.ends_with(&log_suffix))
        })
        .map(|entry| entry.path())
        .ok_or_else(|| format!("receiver {receiver_index} stderr log is missing"))?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while std::time::Instant::now() < deadline {
        let stderr = fs::read_to_string(&receiver_log).unwrap_or_default();
        if let Some(line) = stderr
            .lines()
            .rev()
            .find(|line| line.contains(SNAPSHOT_HANDLER_DELAY_STARTED_MARKER))
        {
            let sender = line
                .split_whitespace()
                .find_map(|field| field.strip_prefix("from="))
                .ok_or_else(|| format!("snapshot delay marker is missing sender: {line}"))?;
            let sender_index = cluster
                .node_ids()
                .iter()
                .position(|node_id| node_id == sender)
                .ok_or_else(|| format!("snapshot sender {sender} is not a spawned daemon"))?;
            if require_current_leader {
                let sender_status = cluster.admin_status(sender_index).ok();
                if sender_status
                    .as_ref()
                    .and_then(|status| status.leader.as_deref())
                    != Some(sender)
                {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    continue;
                }
            }
            let Ok(sender_observation) = observation(cluster, sender_index) else {
                std::thread::sleep(std::time::Duration::from_millis(10));
                continue;
            };
            if sender_observation.snapshot_send_attempts == 0
                || sender_observation.snapshot_sends_in_flight == 0
            {
                std::thread::sleep(std::time::Duration::from_millis(10));
                continue;
            }
            return Ok((sender_index, sender_observation));
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let observations = (0..cluster.node_ids().len())
        .map(|index| (index, observation(cluster, index)))
        .collect::<Vec<_>>();
    let receiver_stderr = fs::read_to_string(receiver_log)
        .unwrap_or_else(|_| "<receiver stderr unavailable>".to_owned());
    Err(format!(
        "real HTTP MsgSnapshot handler delay did not become observable; receiver_index={receiver_index} require_current_leader={require_current_leader} observations={observations:?} receiver_stderr={receiver_stderr:?}"
    )
    .into())
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

fn wait_for_snapshot_success_sum(
    cluster: &mut DaemonCluster,
    indices: &[usize],
    minimum_exclusive: u64,
    label: &str,
) -> TestResult<u64> {
    let indices = indices.to_vec();
    let result = cluster.wait_for(label.to_owned(), |cluster| {
        let successful_sends = snapshot_success_sum(cluster, &indices).ok()?;
        (successful_sends > minimum_exclusive).then_some(successful_sends)
    });
    if result.is_err() {
        let observations = (0..cluster.node_ids().len())
            .map(|index| (index, observation(cluster, index)))
            .collect::<Vec<_>>();
        return Err(format!(
            "{label} did not expose the expected successful sender metric; indices={indices:?} minimum_exclusive={minimum_exclusive} observations={observations:?} root={}",
            cluster.root().display()
        )
        .into());
    }
    result
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
