#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

#[path = "support/resource_budget.rs"]
mod resource_budget;
mod support;

use resource_budget::{ResourceBudget, ResourceBudgetArtifact, ResourceSample};
#[cfg(target_os = "linux")]
use support::daemon_cluster::DaemonCluster;
#[cfg(not(target_os = "linux"))]
use support::daemon_cluster::DAEMON_PROCESS_E2E_ENV;
use support::daemon_cluster::{skip_unless_daemon_process_e2e, TestResult};

const RELEASE: &str = "0.66.0";
const RELEASE_DIRECTORY: &str = "0.66";
const SEED: u64 = 0x0D66_0012;
const PORTABLE_ARTIFACT: &str = "snapshot-resource-budget-portable.json";
const RECEIVER_KILL_ARTIFACT: &str = "snapshot-resource-budget-receiver-kill-linux.json";
const SLOW_RECEIVER_ARTIFACT: &str = "snapshot-resource-budget-slow-receiver-linux.json";
const SNAPSHOT_HANDLER_TEST_DELAY_MS: u64 = 30_000;

fn snapshot_budget() -> ResourceBudget {
    ResourceBudget {
        max_child_delta: 0,
        max_connection_delta: 1,
        // `tracked_connections` is the maximum per-sender reservation. During
        // an actual term handoff, the obsolete and replacement leaders may
        // each briefly own one request, disclosed separately as the cluster
        // total in `held_snapshot_messages`.
        max_held_snapshot_messages: 2,
        max_rss_growth_kib: 96 * 1024,
        max_fd_growth: 32,
    }
}

#[test]
fn receiver_kill_releases_snapshot_sender_resources_after_quiescence() -> TestResult {
    if !skip_unless_daemon_process_e2e(
        "receiver_kill_releases_snapshot_sender_resources_after_quiescence",
    ) {
        return Ok(());
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err(format!(
            "{DAEMON_PROCESS_E2E_ENV}=1 claims the W12 Linux resource lane, but /proc sampling is unavailable on {}",
            std::env::consts::OS
        )
        .into())
    }

    #[cfg(target_os = "linux")]
    {
        run_receiver_kill_resource_proof()
    }
}

#[test]
fn slow_receiver_applies_bounded_backpressure_without_unbounded_tasks_or_rss() -> TestResult {
    if !skip_unless_daemon_process_e2e(
        "slow_receiver_applies_bounded_backpressure_without_unbounded_tasks_or_rss",
    ) {
        return Ok(());
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err(format!(
            "{DAEMON_PROCESS_E2E_ENV}=1 claims the W12 Linux resource lane, but /proc sampling is unavailable on {}",
            std::env::consts::OS
        )
        .into())
    }

    #[cfg(target_os = "linux")]
    {
        run_slow_receiver_resource_proof()
    }
}

#[test]
fn snapshot_resource_artifact_validates_for_release_066() -> TestResult {
    let samples = vec![
        ResourceSample {
            running_children: 3,
            ..ResourceSample::default()
        },
        ResourceSample {
            running_children: 3,
            tracked_connections: 1,
            held_snapshot_messages: 1,
            ..ResourceSample::default()
        },
        ResourceSample {
            running_children: 3,
            ..ResourceSample::default()
        },
    ];
    let artifact = ResourceBudgetArtifact::new(RELEASE, SEED, samples, snapshot_budget());

    artifact.validate_for_release(RELEASE)?;
    artifact.validate_budget()?;
    let linux_claim_error = artifact
        .validate_linux_proof()
        .expect_err("portable samples without RSS/FD must never count as Linux proof");
    assert!(
        linux_claim_error.to_string().contains("Linux")
            || linux_claim_error.to_string().contains("linux"),
        "Linux proof rejection must explain the missing capability: {linux_claim_error}"
    );

    let value = serde_json::to_value(&artifact)?;
    for field in ["baseline", "peak", "final_sample", "platform", "samples"] {
        assert!(value.get(field).is_some(), "artifact is missing {field}");
    }
    assert!(
        value["samples"]
            .as_array()
            .expect("samples must serialize as an array")
            .iter()
            .all(|sample| {
                sample.get("rss_kib").is_none()
                    && sample.get("rss_hwm_kib").is_none()
                    && sample.get("open_fds").is_none()
            }),
        "portable/model evidence must omit unavailable Linux metrics"
    );
    let schema = include_str!("../../../docs/testing/schemas/daemon-resource-budget.schema.json");
    assert!(
        schema.contains("\"release\"")
            && schema.contains("\"type\": \"string\"")
            && schema.contains("\"pattern\""),
        "the shared resource schema must accept release-scoped semver evidence"
    );
    assert!(
        !schema.contains("\"release\": { \"const\": \"0.64.0\" }"),
        "the 0.66 artifact must not rely on a schema pinned to 0.64"
    );
    artifact.write_workspace_evidence(RELEASE_DIRECTORY, PORTABLE_ARTIFACT)?;
    Ok(())
}

#[test]
fn canary_snapshot_sender_resource_reservation_never_releases() {
    let outstanding_sender_reservations = 1_u64;
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W12") {
        assert_eq!(
            outstanding_sender_reservations, 0,
            "HC-CANARY-RED:W12 snapshot sender resource reservation never released"
        );
    }
    assert_eq!(
        outstanding_sender_reservations, 1,
        "the W12 canary must model one leaked sender reservation"
    );
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct SnapshotObservation {
    applied_index: u64,
    snapshot_send_attempts: u64,
    snapshot_send_successes: u64,
    snapshot_send_failures: u64,
    snapshot_sends_in_flight: u64,
    snapshot_installs: u64,
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct SnapshotSenderSetObservation {
    total: SnapshotObservation,
    max_in_flight_per_sender: u64,
}

#[cfg(target_os = "linux")]
impl SnapshotObservation {
    fn add(self, other: Self) -> Self {
        Self {
            applied_index: self.applied_index.max(other.applied_index),
            snapshot_send_attempts: self
                .snapshot_send_attempts
                .saturating_add(other.snapshot_send_attempts),
            snapshot_send_successes: self
                .snapshot_send_successes
                .saturating_add(other.snapshot_send_successes),
            snapshot_send_failures: self
                .snapshot_send_failures
                .saturating_add(other.snapshot_send_failures),
            snapshot_sends_in_flight: self
                .snapshot_sends_in_flight
                .saturating_add(other.snapshot_sends_in_flight),
            snapshot_installs: self
                .snapshot_installs
                .saturating_add(other.snapshot_installs),
        }
    }
}

#[cfg(target_os = "linux")]
#[derive(Debug)]
struct PreparedSnapshotTransfer {
    lagger_index: usize,
    compacted_index: u64,
    active_indices: Vec<usize>,
    before_snapshot: SnapshotSenderSetObservation,
}

#[cfg(target_os = "linux")]
fn run_receiver_kill_resource_proof() -> TestResult {
    let mut cluster =
        DaemonCluster::start_bootstrap_with_raft_compaction(3, "w12-snapshot-receiver-kill")?;
    cluster.wait_for_shape(3, 3)?;
    let all_indices = (0..cluster.node_ids().len()).collect::<Vec<_>>();
    let baseline_totals = snapshot_totals(&cluster, &all_indices)?;
    let mut samples = vec![linux_snapshot_sample(&mut cluster, baseline_totals)?];
    let prepared = prepare_delayed_snapshot_receiver(&mut cluster)?;
    cluster.restart_with_snapshot_handler_delay(
        prepared.lagger_index,
        Some(SNAPSHOT_HANDLER_TEST_DELAY_MS),
    )?;
    let in_flight = wait_for_snapshot_in_flight(
        &mut cluster,
        &prepared.active_indices,
        prepared.before_snapshot,
    )?;
    assert_eq!(
        in_flight.total.snapshot_sends_in_flight, 1,
        "one lagging peer must reserve exactly one bounded snapshot sender: {in_flight:?}"
    );
    samples.push(linux_snapshot_sample(&mut cluster, in_flight)?);

    cluster.kill(prepared.lagger_index)?;
    let released = cluster.wait_for(
        "killed receiver releases the in-flight snapshot sender".to_owned(),
        |cluster| {
            let totals = snapshot_totals(cluster, &prepared.active_indices).ok()?;
            (totals.total.snapshot_send_failures > in_flight.total.snapshot_send_failures
                && totals.total.snapshot_sends_in_flight == 0)
                .then_some(totals)
        },
    )?;
    samples.push(linux_snapshot_sample(&mut cluster, released)?);

    cluster.restart_with_snapshot_handler_delay(prepared.lagger_index, None)?;
    wait_for_snapshot_install_and_convergence(
        &mut cluster,
        prepared.lagger_index,
        prepared.compacted_index,
    )?;
    let quiescent = cluster.wait_for(
        "snapshot retry succeeds and sender becomes quiescent".to_owned(),
        |cluster| {
            let totals = snapshot_totals(cluster, &all_indices).ok()?;
            (totals.total.snapshot_send_successes > in_flight.total.snapshot_send_successes
                && totals.total.snapshot_sends_in_flight == 0)
                .then_some(totals)
        },
    )?;
    samples.push(linux_snapshot_sample(&mut cluster, quiescent)?);

    let artifact = ResourceBudgetArtifact::new(RELEASE, SEED, samples, snapshot_budget());
    artifact.validate_for_release(RELEASE)?;
    artifact.validate_linux_proof()?;
    artifact.validate_budget()?;
    assert_snapshot_resources_quiescent(&artifact)?;
    artifact.write_workspace_evidence(RELEASE_DIRECTORY, RECEIVER_KILL_ARTIFACT)?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn run_slow_receiver_resource_proof() -> TestResult {
    let mut cluster =
        DaemonCluster::start_bootstrap_with_raft_compaction(3, "w12-snapshot-slow-receiver")?;
    cluster.wait_for_shape(3, 3)?;
    let all_indices = (0..cluster.node_ids().len()).collect::<Vec<_>>();
    let baseline_totals = snapshot_totals(&cluster, &all_indices)?;
    let mut samples = vec![linux_snapshot_sample(&mut cluster, baseline_totals)?];
    let prepared = prepare_delayed_snapshot_receiver(&mut cluster)?;
    cluster.restart_with_snapshot_handler_delay(
        prepared.lagger_index,
        Some(SNAPSHOT_HANDLER_TEST_DELAY_MS),
    )?;
    let first_in_flight = wait_for_snapshot_in_flight(
        &mut cluster,
        &prepared.active_indices,
        prepared.before_snapshot,
    )?;
    assert_eq!(first_in_flight.total.snapshot_sends_in_flight, 1);
    samples.push(linux_snapshot_sample(&mut cluster, first_in_flight)?);

    let failures_before = first_in_flight.total.snapshot_send_failures;
    for failure_delta in 1..=3 {
        let observed = wait_for_failure_with_sender_bound(
            &mut cluster,
            &prepared.active_indices,
            failures_before.saturating_add(failure_delta),
        )?;
        samples.push(linux_snapshot_sample(&mut cluster, observed)?);
    }

    cluster.kill(prepared.lagger_index)?;
    let released = cluster.wait_for(
        "slow receiver teardown releases the active sender request".to_owned(),
        |cluster| {
            let totals = snapshot_totals(cluster, &prepared.active_indices).ok()?;
            (totals.total.snapshot_sends_in_flight == 0).then_some(totals)
        },
    )?;
    samples.push(linux_snapshot_sample(&mut cluster, released)?);
    cluster.restart_with_snapshot_handler_delay(prepared.lagger_index, None)?;
    wait_for_snapshot_install_and_convergence(
        &mut cluster,
        prepared.lagger_index,
        prepared.compacted_index,
    )?;
    let quiescent = cluster.wait_for(
        "slow receiver catches up and releases sender backpressure".to_owned(),
        |cluster| {
            let totals = snapshot_totals(cluster, &all_indices).ok()?;
            (totals.total.snapshot_send_successes > first_in_flight.total.snapshot_send_successes
                && totals.total.snapshot_sends_in_flight == 0)
                .then_some(totals)
        },
    )?;
    samples.push(linux_snapshot_sample(&mut cluster, quiescent)?);

    let artifact = ResourceBudgetArtifact::new(RELEASE, SEED, samples, snapshot_budget());
    artifact.validate_for_release(RELEASE)?;
    artifact.validate_linux_proof()?;
    artifact.validate_budget()?;
    assert_snapshot_resources_quiescent(&artifact)?;
    assert!(
        artifact
            .samples
            .iter()
            .all(|sample| sample.tracked_connections <= 1),
        "a sender exceeded the one-request-per-peer bound: {artifact:?}"
    );
    assert!(
        artifact
            .samples
            .iter()
            .all(|sample| sample.held_snapshot_messages <= 2),
        "cross-term snapshot handoff exceeded one old plus one replacement sender: {artifact:?}"
    );
    artifact.write_workspace_evidence(RELEASE_DIRECTORY, SLOW_RECEIVER_ARTIFACT)?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn assert_snapshot_resources_quiescent(artifact: &ResourceBudgetArtifact) -> TestResult {
    if artifact.final_sample.running_children != artifact.baseline.running_children {
        return Err(
            format!("daemon count did not return to the pre-fault baseline: {artifact:?}").into(),
        );
    }
    if artifact.final_sample.tracked_connections != 0
        || artifact.final_sample.held_snapshot_messages != 0
    {
        return Err(format!(
            "snapshot sender work remained reserved after quiescence: {artifact:?}"
        )
        .into());
    }
    let baseline_rss = artifact
        .baseline
        .rss_kib
        .ok_or("Linux proof baseline is missing rss_kib")?;
    let final_rss = artifact
        .final_sample
        .rss_kib
        .ok_or("Linux proof final sample is missing rss_kib")?;
    let baseline_fds = artifact
        .baseline
        .open_fds
        .ok_or("Linux proof baseline is missing open_fds")?;
    let final_fds = artifact
        .final_sample
        .open_fds
        .ok_or("Linux proof final sample is missing open_fds")?;
    if final_rss > baseline_rss.saturating_add(artifact.budget.max_rss_growth_kib) {
        return Err(format!(
            "post-quiescence RSS exceeded the disclosed residual budget: {artifact:?}"
        )
        .into());
    }
    if final_fds > baseline_fds.saturating_add(artifact.budget.max_fd_growth) {
        return Err(format!(
            "post-quiescence FD count exceeded the disclosed residual budget: {artifact:?}"
        )
        .into());
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn prepare_delayed_snapshot_receiver(
    cluster: &mut DaemonCluster,
) -> TestResult<PreparedSnapshotTransfer> {
    let statuses = cluster.wait_for_shape(3, 3)?;
    let initial_leader_index = leader_index(cluster, &statuses)?;
    let lagger_index = (0..cluster.node_ids().len())
        .find(|index| *index != initial_leader_index)
        .ok_or("three-node cluster did not expose a follower")?;
    let lagger_before = snapshot_observation(cluster, lagger_index)?;

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
            .ok_or("lagging cluster did not retain a live follower")?;
        cluster.kill(churn_index)?;
        cluster.restart(churn_index)?;
        cluster.wait_for_responsive_shape(2, 3, 3)?;
        previous_applied = wait_for_equal_applied_progress(cluster, 2, previous_applied)?;
    }

    let (active_indices, compacted_index) = cluster.wait_for(
        "active snapshot senders converge before compaction".to_owned(),
        |cluster| {
            let indices = cluster.running_indices();
            if indices.len() != 2 {
                return None;
            }
            let applied = indices
                .iter()
                .map(|index| {
                    snapshot_observation(cluster, *index)
                        .ok()
                        .map(|value| value.applied_index)
                })
                .collect::<Option<Vec<_>>>()?;
            (applied.iter().all(|index| *index == applied[0])
                && applied[0] > lagger_before.applied_index)
                .then_some((indices, applied[0]))
        },
    )?;
    let before_snapshot = snapshot_totals(cluster, &active_indices)?;
    for index in &active_indices {
        let compacted = cluster.compact_raft_log(*index)?;
        assert_eq!(
            u64_field(&compacted, "snapshot_index")?,
            compacted_index,
            "all possible senders must persist the same snapshot boundary"
        );
        assert!(
            u64_field(&compacted, "first_log_index")? > lagger_before.applied_index,
            "compaction must move retained-log progress beyond the receiver"
        );
    }
    Ok(PreparedSnapshotTransfer {
        lagger_index,
        compacted_index,
        active_indices,
        before_snapshot,
    })
}

#[cfg(target_os = "linux")]
fn wait_for_snapshot_in_flight(
    cluster: &mut DaemonCluster,
    active_indices: &[usize],
    before: SnapshotSenderSetObservation,
) -> TestResult<SnapshotSenderSetObservation> {
    let mut last_observation = before;
    let result = cluster.wait_for(
        "real snapshot sender reservation becomes in-flight".to_owned(),
        |cluster| {
            let totals = snapshot_totals(cluster, active_indices).ok()?;
            last_observation = totals;
            (totals.total.snapshot_send_attempts > before.total.snapshot_send_attempts
                && totals.total.snapshot_sends_in_flight > 0)
                .then_some(totals)
        },
    );
    result.map_err(|error| {
        format!(
            "{error}; before_snapshot={before:?}; last_snapshot_observation={last_observation:?}"
        )
        .into()
    })
}

#[cfg(target_os = "linux")]
fn wait_for_failure_with_sender_bound(
    cluster: &mut DaemonCluster,
    active_indices: &[usize],
    minimum_failures: u64,
) -> TestResult<SnapshotSenderSetObservation> {
    let mut bound_violation = None;
    let observed = cluster.wait_for(
        format!("snapshot sender records failure {minimum_failures} under backpressure"),
        |cluster| {
            let totals = snapshot_totals(cluster, active_indices).ok()?;
            if totals.max_in_flight_per_sender > 1 || totals.total.snapshot_sends_in_flight > 2 {
                bound_violation = Some(totals);
                return Some(totals);
            }
            (totals.total.snapshot_send_failures >= minimum_failures).then_some(totals)
        },
    )?;
    if let Some(violation) = bound_violation {
        let per_sender = active_indices
            .iter()
            .map(|index| (*index, snapshot_observation(cluster, *index)))
            .collect::<Vec<_>>();
        let statuses = cluster.statuses();
        return Err(format!(
            "slow receiver exceeded the per-sender or cross-term handoff bound: observation={violation:?}; per_sender={per_sender:?}; statuses={statuses:?}"
        )
        .into());
    }
    Ok(observed)
}

#[cfg(target_os = "linux")]
fn wait_for_snapshot_install_and_convergence(
    cluster: &mut DaemonCluster,
    receiver_index: usize,
    minimum_applied: u64,
) -> TestResult {
    cluster.wait_for(
        format!("receiver {receiver_index} installs the compacted snapshot"),
        |cluster| {
            let receiver = snapshot_observation(cluster, receiver_index).ok()?;
            (receiver.snapshot_installs > 0 && receiver.applied_index >= minimum_applied)
                .then_some(())
        },
    )?;
    cluster.wait_for_responsive_shape(3, 3, 3)?;
    wait_for_equal_applied_progress(cluster, 3, minimum_applied.saturating_sub(1))?;
    Ok(())
}

#[cfg(target_os = "linux")]
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
                    snapshot_observation(cluster, *index)
                        .ok()
                        .map(|value| value.applied_index)
                })
                .collect::<Option<Vec<_>>>()?;
            (applied.iter().all(|index| *index == applied[0]) && applied[0] > minimum_exclusive)
                .then_some(applied[0])
        },
    )
}

#[cfg(target_os = "linux")]
fn linux_snapshot_sample(
    cluster: &mut DaemonCluster,
    snapshot: SnapshotSenderSetObservation,
) -> TestResult<ResourceSample> {
    let totals = cluster.os_resource_totals().ok_or(
        "W12 Linux resource gate was claimed but /proc RSS/VmHWM/FD sampling is unavailable",
    )?;
    Ok(ResourceSample {
        running_children: cluster.running_child_count() as u64,
        tracked_connections: snapshot.max_in_flight_per_sender,
        held_snapshot_messages: snapshot.total.snapshot_sends_in_flight,
        rss_kib: Some(totals.rss_kib),
        rss_hwm_kib: Some(totals.rss_hwm_kib),
        open_fds: Some(totals.open_fds),
    })
}

#[cfg(target_os = "linux")]
fn snapshot_totals(
    cluster: &DaemonCluster,
    indices: &[usize],
) -> TestResult<SnapshotSenderSetObservation> {
    indices
        .iter()
        .try_fold(SnapshotSenderSetObservation::default(), |mut set, index| {
            let sender = snapshot_observation(cluster, *index)?;
            set.total = set.total.add(sender);
            set.max_in_flight_per_sender = set
                .max_in_flight_per_sender
                .max(sender.snapshot_sends_in_flight);
            Ok(set)
        })
}

#[cfg(target_os = "linux")]
fn snapshot_observation(cluster: &DaemonCluster, index: usize) -> TestResult<SnapshotObservation> {
    let value = cluster.raft_compaction_status(index)?;
    Ok(SnapshotObservation {
        applied_index: u64_field(&value, "applied_index")?,
        snapshot_send_attempts: u64_field(&value, "snapshot_send_attempts")?,
        snapshot_send_successes: u64_field(&value, "snapshot_send_successes")?,
        snapshot_send_failures: u64_field(&value, "snapshot_send_failures")?,
        snapshot_sends_in_flight: u64_field(&value, "snapshot_sends_in_flight")?,
        snapshot_installs: u64_field(&value, "snapshot_installs")?,
    })
}

#[cfg(target_os = "linux")]
fn leader_index(
    cluster: &DaemonCluster,
    statuses: &[support::daemon_cluster::DaemonStatus],
) -> TestResult<usize> {
    let leader = statuses
        .iter()
        .find_map(|status| status.leader.as_deref())
        .ok_or("responsive cluster did not expose a leader")?;
    cluster
        .node_ids()
        .iter()
        .position(|node_id| node_id == leader)
        .ok_or_else(|| format!("leader {leader} is not a spawned daemon").into())
}

#[cfg(target_os = "linux")]
fn u64_field(value: &serde_json::Value, field: &'static str) -> TestResult<u64> {
    value
        .get(field)
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| format!("raft compaction status missing {field}: {value}").into())
}
