#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

#[path = "support/resource_budget.rs"]
mod resource_budget;
mod support;

use hydracache_server::ADMIN_RAFT_COMPACTION_PATH;
use resource_budget::{
    ResourceBudget, ResourceBudgetArtifact, ResourceSample, ResourceSamplingDisclosure,
};
#[cfg(target_os = "linux")]
use support::daemon_cluster::DaemonCluster;
#[cfg(not(target_os = "linux"))]
use support::daemon_cluster::DAEMON_PROCESS_E2E_ENV;
use support::daemon_cluster::{
    skip_unless_daemon_process_e2e, TestResult, DAEMON_POLL_INTERVAL_MS,
};

const RELEASE: &str = "0.66.0";
const RELEASE_DIRECTORY: &str = "0.66";
const SEED: u64 = 0x0D66_0012;
const PORTABLE_ARTIFACT: &str = "snapshot-resource-budget-portable.json";
const RECEIVER_KILL_ARTIFACT: &str = "snapshot-resource-budget-receiver-kill-linux.json";
const SLOW_RECEIVER_ARTIFACT: &str = "snapshot-resource-budget-slow-receiver-linux.json";
const SNAPSHOT_HANDLER_TEST_DELAY_MS: u64 = 30_000;

#[cfg(target_os = "linux")]
static SNAPSHOT_RESOURCE_PROOF_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn snapshot_budget() -> ResourceBudget {
    ResourceBudget {
        max_child_delta: 0,
        max_connection_delta: 1,
        // Compatibility fields retain the 0.64 artifact shape. They are
        // event-checkpoint observations, not continuous maxima: the first is
        // the maximum per-daemon request gauge at a retained checkpoint and
        // the second is the observed cluster sum at that checkpoint.
        max_held_snapshot_messages: 2,
        max_snapshot_sender_tasks_current: Some(2),
        max_snapshot_sender_tasks_high_water_per_daemon: Some(1),
        max_rss_growth_kib: 96 * 1024,
        max_fd_growth: 32,
    }
}

fn snapshot_sampling_disclosure() -> ResourceSamplingDisclosure {
    ResourceSamplingDisclosure {
        admin_endpoint: ADMIN_RAFT_COMPACTION_PATH.to_owned(),
        observation_mode: "event-checkpoint".to_owned(),
        poll_interval_ms: DAEMON_POLL_INTERVAL_MS,
        sampled_current_fields: vec![
            "tracked_connections".to_owned(),
            "held_snapshot_messages".to_owned(),
            "snapshot_sender_tasks_current".to_owned(),
        ],
        monotonic_high_water_fields: vec!["snapshot_sender_tasks_high_water_per_daemon".to_owned()],
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
        // These proofs each own three real daemon processes and deliberately
        // hold a snapshot request open. Serializing them keeps the observation
        // window attributable to one cluster rather than runner contention.
        let _proof = SNAPSHOT_RESOURCE_PROOF_LOCK
            .lock()
            .expect("snapshot resource proof lock poisoned");
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
        let _proof = SNAPSHOT_RESOURCE_PROOF_LOCK
            .lock()
            .expect("snapshot resource proof lock poisoned");
        run_slow_receiver_resource_proof()
    }
}

#[test]
fn snapshot_resource_artifact_validates_for_release_066() -> TestResult {
    let samples = vec![
        ResourceSample {
            running_children: 3,
            snapshot_sender_tasks_current: Some(0),
            snapshot_sender_tasks_high_water_per_daemon: Some(0),
            ..ResourceSample::default()
        },
        ResourceSample {
            running_children: 3,
            tracked_connections: 1,
            held_snapshot_messages: 1,
            snapshot_sender_tasks_current: Some(1),
            snapshot_sender_tasks_high_water_per_daemon: Some(1),
            ..ResourceSample::default()
        },
        ResourceSample {
            running_children: 3,
            snapshot_sender_tasks_current: Some(0),
            snapshot_sender_tasks_high_water_per_daemon: Some(1),
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
    let disclosed = artifact
        .clone()
        .with_sampling(snapshot_sampling_disclosure());
    let disclosed = serde_json::to_value(disclosed)?;
    let sampling = &disclosed["sampling"];
    assert_eq!(sampling["admin_endpoint"], ADMIN_RAFT_COMPACTION_PATH);
    assert_eq!(sampling["observation_mode"], "event-checkpoint");
    assert_eq!(sampling["poll_interval_ms"], DAEMON_POLL_INTERVAL_MS);
    assert_eq!(
        sampling["monotonic_high_water_fields"],
        serde_json::json!(["snapshot_sender_tasks_high_water_per_daemon"])
    );
    assert!(schema.contains("\"sampling\""));
    artifact.write_workspace_evidence(RELEASE_DIRECTORY, PORTABLE_ARTIFACT)?;
    Ok(())
}

#[test]
fn snapshot_task_observation_uses_cluster_current_and_max_daemon_high_water() -> TestResult {
    let status = |current: u64, high_water: u64, in_flight: u64| {
        serde_json::json!({
            "applied_index": 10,
            "snapshot_send_attempts": 2,
            "snapshot_send_successes": 1,
            "snapshot_send_failures": 0,
            "snapshot_sends_in_flight": in_flight,
            "snapshot_sender_tasks_current": current,
            "snapshot_sender_tasks_high_water": high_water,
            "snapshot_installs": 0
        })
    };
    let first = snapshot_observation_from_value(&status(1, 1, 1))?;
    let second = snapshot_observation_from_value(&status(1, 1, 0))?;
    let aggregate = aggregate_snapshot_observations(&[first, second]);

    assert_eq!(aggregate.total.snapshot_sender_tasks_current, 2);
    assert_eq!(aggregate.total.snapshot_sender_tasks_high_water, 1);
    assert_eq!(aggregate.max_in_flight_per_daemon, 1);

    let mut missing_high_water = status(1, 1, 1);
    missing_high_water
        .as_object_mut()
        .expect("status fixture is an object")
        .remove("snapshot_sender_tasks_high_water");
    let error = snapshot_observation_from_value(&missing_high_water)
        .expect_err("missing task HWM must fail loud");
    assert!(error
        .to_string()
        .contains("snapshot_sender_tasks_high_water"));
    Ok(())
}

#[test]
fn snapshot_task_budget_rejects_overshoot_and_missing_metrics() {
    let sample = |current: Option<u64>, high_water: Option<u64>| ResourceSample {
        running_children: 3,
        snapshot_sender_tasks_current: current,
        snapshot_sender_tasks_high_water_per_daemon: high_water,
        ..ResourceSample::default()
    };
    let baseline = sample(Some(0), Some(0));

    let current_overshoot = ResourceBudgetArtifact::new(
        RELEASE,
        SEED,
        vec![baseline, sample(Some(3), Some(1))],
        snapshot_budget(),
    );
    assert!(current_overshoot
        .validate_budget()
        .expect_err("sampled cluster task current above two must fail")
        .to_string()
        .contains("task peak"));

    let high_water_overshoot = ResourceBudgetArtifact::new(
        RELEASE,
        SEED,
        vec![baseline, sample(Some(1), Some(2))],
        snapshot_budget(),
    );
    assert!(high_water_overshoot
        .validate_budget()
        .expect_err("daemon task HWM above one must fail")
        .to_string()
        .contains("high-water"));

    let missing = ResourceBudgetArtifact::new(
        RELEASE,
        SEED,
        vec![sample(None, Some(0)), sample(Some(1), Some(1))],
        snapshot_budget(),
    );
    assert!(missing
        .validate_budget()
        .expect_err("declared task budget without task-current samples must fail")
        .to_string()
        .contains("requires current-task samples"));

    let missing_high_water = ResourceBudgetArtifact::new(
        RELEASE,
        SEED,
        vec![sample(Some(0), None), sample(Some(1), Some(1))],
        snapshot_budget(),
    );
    assert!(missing_high_water
        .validate_budget()
        .expect_err("declared task HWM budget without every daemon HWM sample must fail")
        .to_string()
        .contains("requires per-daemon samples"));
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct SnapshotObservation {
    applied_index: u64,
    snapshot_send_attempts: u64,
    snapshot_send_successes: u64,
    snapshot_send_failures: u64,
    snapshot_sends_in_flight: u64,
    snapshot_sender_tasks_current: u64,
    snapshot_sender_tasks_high_water: u64,
    snapshot_installs: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct SnapshotSenderSetObservation {
    total: SnapshotObservation,
    max_in_flight_per_daemon: u64,
}

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
            snapshot_sender_tasks_current: self
                .snapshot_sender_tasks_current
                .saturating_add(other.snapshot_sender_tasks_current),
            // Each daemon publishes a monotonic local HWM. Across daemons the
            // meaningful concurrency bound is their maximum, never their sum.
            snapshot_sender_tasks_high_water: self
                .snapshot_sender_tasks_high_water
                .max(other.snapshot_sender_tasks_high_water),
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
    let in_flight = wait_for_snapshot_in_flight(&mut cluster, prepared.before_snapshot)?;
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
                && totals.total.snapshot_sends_in_flight == 0
                && totals.total.snapshot_sender_tasks_current == 0)
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
                && totals.total.snapshot_sends_in_flight == 0
                && totals.total.snapshot_sender_tasks_current == 0)
                .then_some(totals)
        },
    )?;
    samples.push(linux_snapshot_sample(&mut cluster, quiescent)?);

    let artifact = ResourceBudgetArtifact::new(RELEASE, SEED, samples, snapshot_budget())
        .with_sampling(snapshot_sampling_disclosure());
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
    let first_in_flight = wait_for_snapshot_in_flight(&mut cluster, prepared.before_snapshot)?;
    assert_eq!(first_in_flight.total.snapshot_sends_in_flight, 1);
    samples.push(linux_snapshot_sample(&mut cluster, first_in_flight)?);

    let failures_before = first_in_flight.total.snapshot_send_failures;
    for failure_delta in 1..=3 {
        let observed = wait_for_failure_with_sender_bound(
            &mut cluster,
            failures_before.saturating_add(failure_delta),
        )?;
        samples.push(linux_snapshot_sample(&mut cluster, observed)?);
    }

    cluster.kill(prepared.lagger_index)?;
    let released = cluster.wait_for(
        "slow receiver teardown releases the active sender request".to_owned(),
        |cluster| {
            let totals = snapshot_totals(cluster, &prepared.active_indices).ok()?;
            (totals.total.snapshot_sends_in_flight == 0
                && totals.total.snapshot_sender_tasks_current == 0)
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
        "slow receiver catches up and releases sender backpressure".to_owned(),
        |cluster| {
            let totals = snapshot_totals(cluster, &all_indices).ok()?;
            (totals.total.snapshot_send_successes > first_in_flight.total.snapshot_send_successes
                && totals.total.snapshot_sends_in_flight == 0
                && totals.total.snapshot_sender_tasks_current == 0)
                .then_some(totals)
        },
    )?;
    samples.push(linux_snapshot_sample(&mut cluster, quiescent)?);

    let artifact = ResourceBudgetArtifact::new(RELEASE, SEED, samples, snapshot_budget())
        .with_sampling(snapshot_sampling_disclosure());
    artifact.validate_for_release(RELEASE)?;
    artifact.validate_linux_proof()?;
    artifact.validate_budget()?;
    assert_snapshot_resources_quiescent(&artifact)?;
    assert!(
        artifact
            .samples
            .iter()
            .all(|sample| sample.tracked_connections <= 1),
        "a retained checkpoint observed more than one request on a daemon: {artifact:?}"
    );
    assert!(
        artifact
            .samples
            .iter()
            .all(|sample| sample.held_snapshot_messages <= 2),
        "a retained checkpoint observed more than two cluster snapshot requests: {artifact:?}"
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
        || artifact.final_sample.snapshot_sender_tasks_current != Some(0)
    {
        return Err(format!(
            "snapshot sender work remained reserved after quiescence: {artifact:?}"
        )
        .into());
    }
    if artifact.sampling.as_ref() != Some(&snapshot_sampling_disclosure()) {
        return Err("snapshot Linux proof is missing the exact sampling disclosure".into());
    }
    let current_limit = artifact
        .budget
        .max_snapshot_sender_tasks_current
        .ok_or("snapshot Linux proof is missing its sampled task-current budget")?;
    let high_water_limit = artifact
        .budget
        .max_snapshot_sender_tasks_high_water_per_daemon
        .ok_or("snapshot Linux proof is missing its per-daemon task high-water budget")?;
    for (index, sample) in artifact.samples.iter().enumerate() {
        let current = sample.snapshot_sender_tasks_current.ok_or_else(|| {
            format!("snapshot Linux sample {index} is missing task-current evidence")
        })?;
        let high_water = sample
            .snapshot_sender_tasks_high_water_per_daemon
            .ok_or_else(|| {
                format!("snapshot Linux sample {index} is missing task high-water evidence")
            })?;
        if current > current_limit || high_water > high_water_limit {
            return Err(format!(
                "snapshot sender task bound failed at sample {index}: current={current}/{current_limit} per_daemon_hwm={high_water}/{high_water_limit}"
            )
            .into());
        }
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
    before: SnapshotSenderSetObservation,
) -> TestResult<SnapshotSenderSetObservation> {
    let mut last_observation = before;
    let result = cluster.wait_for(
        "real snapshot sender reservation becomes in-flight".to_owned(),
        |cluster| {
            // Poll the current leader first. Aggregating every daemon requires
            // several sequential admin requests and can miss the intentionally
            // short sender reservation even though the monotonic high-water
            // counter proves that it existed.
            let statuses = cluster.statuses();
            let leader = leader_index(cluster, &statuses).ok()?;
            let leader_snapshot = snapshot_observation(cluster, leader).ok()?;
            if leader_snapshot.snapshot_send_attempts <= before.total.snapshot_send_attempts
                || leader_snapshot.snapshot_sends_in_flight == 0
                || leader_snapshot.snapshot_sender_tasks_current == 0
            {
                return None;
            }
            let indices = cluster.running_indices();
            let totals = snapshot_totals(cluster, &indices).ok()?;
            last_observation = totals;
            (totals.total.snapshot_send_attempts > before.total.snapshot_send_attempts
                && totals.total.snapshot_sends_in_flight > 0
                && totals.total.snapshot_sender_tasks_current > 0)
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
    minimum_failures: u64,
) -> TestResult<SnapshotSenderSetObservation> {
    let mut bound_violation = None;
    let observed = cluster.wait_for(
        format!("snapshot sender records failure {minimum_failures} under backpressure"),
        |cluster| {
            let indices = cluster.running_indices();
            let totals = snapshot_totals(cluster, &indices).ok()?;
            if totals.max_in_flight_per_daemon > 1
                || totals.total.snapshot_sends_in_flight > 2
                || totals.total.snapshot_sender_tasks_current > 2
                || totals.total.snapshot_sender_tasks_high_water > 1
            {
                bound_violation = Some(totals);
                return Some(totals);
            }
            (totals.total.snapshot_send_failures >= minimum_failures).then_some(totals)
        },
    )?;
    if let Some(violation) = bound_violation {
        let per_daemon = cluster
            .running_indices()
            .iter()
            .map(|index| (*index, snapshot_observation(cluster, *index)))
            .collect::<Vec<_>>();
        let statuses = cluster.statuses();
        return Err(format!(
            "slow receiver exceeded a retained request/task observation or daemon-local task HWM: observation={violation:?}; per_daemon={per_daemon:?}; statuses={statuses:?}"
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
        tracked_connections: snapshot.max_in_flight_per_daemon,
        held_snapshot_messages: snapshot.total.snapshot_sends_in_flight,
        snapshot_sender_tasks_current: Some(snapshot.total.snapshot_sender_tasks_current),
        snapshot_sender_tasks_high_water_per_daemon: Some(
            snapshot.total.snapshot_sender_tasks_high_water,
        ),
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
    let observations = indices
        .iter()
        .map(|index| snapshot_observation(cluster, *index))
        .collect::<TestResult<Vec<_>>>()?;
    Ok(aggregate_snapshot_observations(&observations))
}

#[cfg(target_os = "linux")]
fn snapshot_observation(cluster: &DaemonCluster, index: usize) -> TestResult<SnapshotObservation> {
    let value = cluster.raft_compaction_status(index)?;
    snapshot_observation_from_value(&value)
}

fn snapshot_observation_from_value(value: &serde_json::Value) -> TestResult<SnapshotObservation> {
    Ok(SnapshotObservation {
        applied_index: u64_field(value, "applied_index")?,
        snapshot_send_attempts: u64_field(value, "snapshot_send_attempts")?,
        snapshot_send_successes: u64_field(value, "snapshot_send_successes")?,
        snapshot_send_failures: u64_field(value, "snapshot_send_failures")?,
        snapshot_sends_in_flight: u64_field(value, "snapshot_sends_in_flight")?,
        snapshot_sender_tasks_current: u64_field(value, "snapshot_sender_tasks_current")?,
        snapshot_sender_tasks_high_water: u64_field(value, "snapshot_sender_tasks_high_water")?,
        snapshot_installs: u64_field(value, "snapshot_installs")?,
    })
}

fn aggregate_snapshot_observations(
    observations: &[SnapshotObservation],
) -> SnapshotSenderSetObservation {
    observations.iter().copied().fold(
        SnapshotSenderSetObservation::default(),
        |mut set, sender| {
            set.total = set.total.add(sender);
            set.max_in_flight_per_daemon = set
                .max_in_flight_per_daemon
                .max(sender.snapshot_sends_in_flight);
            set
        },
    )
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

fn u64_field(value: &serde_json::Value, field: &'static str) -> TestResult<u64> {
    value
        .get(field)
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| format!("raft compaction status missing {field}: {value}").into())
}
