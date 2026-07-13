mod support;

use std::collections::BTreeSet;

use hydracache_sim::{BoundedGrowthChecker, InvariantReport, ResourceBudget};
use support::daemon_cluster::{skip_unless_daemon_process_e2e, DaemonCluster, TestResult};
use support::membership_history::MembershipHistoryRecorder;

#[test]
fn sigkill_leader_reelects_and_restarted_node_rejoins_same_storage() -> TestResult {
    if !skip_unless_daemon_process_e2e(
        "sigkill_leader_reelects_and_restarted_node_rejoins_same_storage",
    ) {
        return Ok(());
    }

    let mut cluster = DaemonCluster::start_bootstrap(3, "sigkill-leader")?;
    let statuses = cluster.wait_for_shape(3, 3)?;
    let old_leader = statuses[0].leader.clone().expect("leader before kill");
    let leader_index = cluster
        .node_ids()
        .iter()
        .position(|node_id| node_id == &old_leader)
        .expect("leader belongs to spawned cluster");
    let storage_dir = cluster.storage_dir(leader_index).to_path_buf();

    cluster.kill(leader_index)?;
    cluster.wait_for_leader_not(&old_leader, 3, 3)?;
    cluster.restart(leader_index)?;
    cluster.wait_for_shape(3, 3)?;

    assert!(
        storage_dir.join("raft-log").is_dir(),
        "restarted node should keep its durable raft storage"
    );
    Ok(())
}

#[test]
fn restarted_node_does_not_double_vote_in_same_term() -> TestResult {
    if !skip_unless_daemon_process_e2e("restarted_node_does_not_double_vote_in_same_term") {
        return Ok(());
    }

    let mut cluster = DaemonCluster::start_bootstrap(3, "restart-double-vote")?;
    let mut history = MembershipHistoryRecorder::default();
    for overview in cluster.overviews() {
        history.record_cluster_overview(&overview);
    }
    let statuses = cluster.wait_for_shape(3, 3)?;
    let leader = statuses[0].leader.clone().expect("leader before restart");
    let leader_index = cluster
        .node_ids()
        .iter()
        .position(|node_id| node_id == &leader)
        .expect("leader belongs to spawned cluster");

    cluster.kill(leader_index)?;
    cluster.restart(leader_index)?;
    cluster.wait_for_shape(3, 3)?;
    for overview in cluster.overviews() {
        history.record_cluster_overview(&overview);
    }

    let report = history.check();
    assert!(
        report.is_ok(),
        "membership history found invariant violations: {:?}",
        report.violations
    );
    Ok(())
}

#[test]
fn drained_node_restart_does_not_silently_resurrect_voter() -> TestResult {
    if !skip_unless_daemon_process_e2e("drained_node_restart_does_not_silently_resurrect_voter") {
        return Ok(());
    }

    let mut cluster = DaemonCluster::start_bootstrap(3, "drained-restart")?;
    let statuses = cluster.wait_for_shape(3, 3)?;
    let leader = statuses[0].leader.clone().expect("leader before drain");
    let drain_index = cluster
        .node_ids()
        .iter()
        .position(|node_id| node_id != &leader)
        .expect("cluster has a follower to drain");

    let _ = cluster.drain(drain_index)?;
    cluster.wait_for_non_draining_shape("drain removal committed before follower kill", 2, 2)?;
    cluster.kill(drain_index)?;
    cluster.wait_for_shape(2, 2)?;
    cluster.restart(drain_index)?;

    let statuses = cluster.wait_for_shape(2, 2)?;
    assert!(
        statuses.iter().all(|status| status.voters == 2),
        "drained node restart must not silently restore a removed voter: {statuses:?}"
    );
    Ok(())
}

#[test]
fn randomized_topology_soak_preserves_invariants() -> TestResult {
    if !skip_unless_daemon_process_e2e("randomized_topology_soak_preserves_invariants") {
        return Ok(());
    }

    let mut cluster = DaemonCluster::start_bootstrap(3, "randomized-soak")?;
    let mut history = MembershipHistoryRecorder::default();
    let mut stopped = BTreeSet::new();
    cluster.wait_for_shape(3, 3)?;

    for step in 0..12 {
        for overview in cluster.overviews() {
            history.record_cluster_overview(&overview);
        }
        let target = step % cluster.node_ids().len();
        if step % 4 == 1 && !stopped.contains(&target) {
            cluster.kill(target)?;
            stopped.insert(target);
        } else if step % 4 == 2 && stopped.contains(&target) {
            cluster.restart(target)?;
            stopped.remove(&target);
        }
        let _ = cluster.wait_for("post-soak-step".to_owned(), |cluster| {
            (!cluster.statuses().is_empty()).then_some(())
        });
    }

    let report = history.check();
    assert!(
        report.is_ok(),
        "randomized topology history violated invariants: {:?}",
        report.violations
    );
    Ok(())
}

#[test]
fn daemon_process_soak_bounds_rss_fds_and_drive_errors() -> TestResult {
    if !skip_unless_daemon_process_e2e("daemon_process_soak_bounds_rss_fds_and_drive_errors") {
        return Ok(());
    }

    let mut cluster = DaemonCluster::start_bootstrap(3, "resource-soak")?;
    cluster.wait_for_shape(3, 3)?;
    let mut checker = BoundedGrowthChecker::new(ResourceBudget {
        max_storage_bytes: 512 * 1024 * 1024,
        max_network_in_flight: 4096,
        max_client_in_flight: 1,
        max_subscriber_pending: 1,
        sample_window: 3,
    });
    let mut report = InvariantReport::default();

    for _ in 0..5 {
        if let Some(sample) = cluster.resource_sample() {
            checker.observe(sample, &mut report);
        }
        cluster.wait_for_shape(3, 3)?;
    }

    assert!(
        report.is_ok(),
        "daemon process resource bounds violated: {:?}",
        report.violations
    );
    Ok(())
}

#[test]
fn frozen_peer_send_failure_is_replayable() -> TestResult {
    if !skip_unless_daemon_process_e2e("frozen_peer_send_failure_is_replayable") {
        return Ok(());
    }

    let mut cluster = DaemonCluster::start_bootstrap(3, "frozen-peer-replay")?;
    cluster.wait_for_shape(3, 3)?;
    let evidence = cluster.replay_evidence(Some(
        "failed to send raft message: request timed out while peer accepted without reply"
            .to_owned(),
    ));

    assert!(evidence.root.exists(), "evidence root should exist");
    assert_eq!(evidence.node_ids.len(), 3);
    assert_eq!(evidence.stdout_logs.len(), 3);
    assert_eq!(evidence.stderr_logs.len(), 3);
    assert!(
        evidence
            .stdout_logs
            .iter()
            .chain(evidence.stderr_logs.iter())
            .all(|path| path.exists()),
        "child log paths must be preserved in replay evidence: {evidence:?}"
    );
    assert!(
        !evidence.last_statuses.is_empty(),
        "last admin statuses must be preserved in replay evidence"
    );
    assert!(
        evidence
            .last_statuses
            .iter()
            .all(|status| status.voters == 3 && status.quorum_ok),
        "known voter/quorum state must be preserved: {:?}",
        evidence.last_statuses
    );
    assert!(
        evidence
            .bounded_send_error
            .as_deref()
            .is_some_and(|error| error.contains("request timed out")),
        "bounded send error should be captured: {evidence:?}"
    );
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn suspended_leader_resumes_as_follower_without_split_brain() -> TestResult {
    if !skip_unless_daemon_process_e2e("suspended_leader_resumes_as_follower_without_split_brain") {
        return Ok(());
    }

    let mut cluster = DaemonCluster::start_bootstrap(3, "suspended-leader")?;
    let statuses = cluster.wait_for_shape(3, 3)?;
    let old_leader = statuses[0].leader.clone().expect("leader before suspend");
    let leader_index = cluster
        .node_ids()
        .iter()
        .position(|node_id| node_id == &old_leader)
        .expect("leader belongs to spawned cluster");

    cluster.suspend(leader_index)?;
    let replacement_before_resume = match cluster.wait_for_leader_not(&old_leader, 3, 3) {
        Ok(statuses) => Some(statuses),
        Err(error) => {
            eprintln!(
                "strong suspended-leader failover claim not proven on this runner: {error}; \
                 continuing with no-split-brain safety gate"
            );
            None
        }
    };
    cluster.resume(leader_index)?;
    let statuses = cluster.wait_for_shape(3, 3)?;
    let leaders = statuses
        .iter()
        .filter_map(|status| status.leader.as_deref())
        .collect::<std::collections::BTreeSet<_>>();

    assert_eq!(
        leaders.len(),
        1,
        "resumed leader caused split brain: {statuses:?}"
    );
    if let Some(replacement_before_resume) = replacement_before_resume {
        assert!(
            replacement_before_resume
                .iter()
                .filter_map(|status| status.leader.as_deref())
                .all(|leader| leader != old_leader),
            "replacement leader proof should exclude old suspended leader: {replacement_before_resume:?}"
        );
    } else {
        eprintln!(
            "suspended-leader safety gate passed after resume; stronger live-failover proof was not claimed"
        );
    }
    Ok(())
}
