mod support;

use std::collections::BTreeSet;

use support::daemon_cluster::{skip_unless_daemon_process_e2e, DaemonCluster, TestResult};
use support::membership_history::{MembershipHistoryRecorder, MembershipObservation};

#[test]
fn membership_history_is_epoch_monotone_under_partition_heal() -> TestResult {
    if !skip_unless_daemon_process_e2e("membership_history_is_epoch_monotone_under_partition_heal")
    {
        return Ok(());
    }

    let mut cluster = DaemonCluster::start_bootstrap(3, "membership-epoch")?;
    let mut history = MembershipHistoryRecorder::default();
    cluster.wait_for_shape(3, 3)?;
    for overview in cluster.overviews() {
        history.record_cluster_overview(&overview);
    }

    let statuses = cluster.wait_for_shape(3, 3)?;
    let old_leader = statuses[0].leader.clone().expect("leader before heal");
    let leader_index = cluster
        .node_ids()
        .iter()
        .position(|node_id| node_id == &old_leader)
        .expect("leader belongs to spawned cluster");
    cluster.kill(leader_index)?;
    cluster.wait_for_leader_not(&old_leader, 3, 3)?;
    cluster.restart(leader_index)?;
    cluster.wait_for_shape(3, 3)?;

    for overview in cluster.overviews() {
        history.record_cluster_overview(&overview);
    }
    let report = history.check();
    assert!(
        report.is_ok(),
        "membership history violated invariants: {:?}",
        report.violations
    );
    Ok(())
}

#[test]
fn membership_history_rejects_two_leaders_in_same_term() -> TestResult {
    if !skip_unless_daemon_process_e2e("membership_history_rejects_two_leaders_in_same_term") {
        return Ok(());
    }

    let history = split_brain_history();
    let report = history.check();
    assert!(
        report
            .violations
            .iter()
            .any(|violation| violation.name == "election_safety"),
        "same-term split leader history should be rejected: {report:?}"
    );
    Ok(())
}

#[test]
fn membership_history_checker_rejects_synthetic_same_term_leaders() {
    let history = split_brain_history();
    let report = history.check();

    assert!(
        report
            .violations
            .iter()
            .any(|violation| violation.name == "election_safety"),
        "same-term split leader history should be rejected: {report:?}"
    );
}

fn split_brain_history() -> MembershipHistoryRecorder {
    let mut history = MembershipHistoryRecorder::default();
    let members = BTreeSet::from([
        "node-a".to_owned(),
        "node-b".to_owned(),
        "node-c".to_owned(),
    ]);
    history.record(MembershipObservation {
        epoch: 3,
        term: 7,
        leader: Some("node-a".to_owned()),
        members: members.clone(),
    });
    history.record(MembershipObservation {
        epoch: 3,
        term: 7,
        leader: Some("node-b".to_owned()),
        members,
    });
    history
}
