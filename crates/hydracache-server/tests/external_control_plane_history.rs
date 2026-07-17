mod support;

use std::collections::BTreeSet;

use support::daemon_cluster::{
    skip_unless_daemon_process_e2e, DaemonCluster, DaemonStatus, TestResult,
};
use support::external_control_plane_history::{
    ExternalAdminObservation, ExternalHistoryAction, ExternalHistoryChecker,
    ExternalHistoryGenerator, ExternalHistoryRecorder, ExternalHistorySchedule,
    ExternalHistoryScheduler, ExternalHistoryShrinker, ExternalHistoryStep, FrozenHistoryCorpus,
};
use support::membership_history::MembershipObservation;

const FROZEN_BAD_SEEDS: &str =
    include_str!("vectors/external_control_plane_history_bad_seeds.json");

#[derive(Debug, Clone)]
struct FastControlPlaneModel {
    members: BTreeSet<String>,
    running: BTreeSet<String>,
    leader: String,
    term: u64,
    epoch: u64,
    last_killed: Option<String>,
}

impl FastControlPlaneModel {
    fn three_nodes() -> Self {
        let members = BTreeSet::from([
            "node-a".to_owned(),
            "node-b".to_owned(),
            "node-c".to_owned(),
        ]);
        Self {
            running: members.clone(),
            members,
            leader: "node-a".to_owned(),
            term: 1,
            epoch: 3,
            last_killed: None,
        }
    }

    fn apply(&mut self, action: ExternalHistoryAction) {
        match action {
            ExternalHistoryAction::Observe | ExternalHistoryAction::CompactFollower => {}
            ExternalHistoryAction::KillLeader => {
                let killed = self.leader.clone();
                assert!(self.running.remove(&killed));
                self.last_killed = Some(killed);
                self.term = self.term.saturating_add(1);
                self.leader = self
                    .running
                    .iter()
                    .next()
                    .cloned()
                    .expect("two-voter majority elects a replacement");
            }
            ExternalHistoryAction::RestartLastKilled => {
                let restarted = self
                    .last_killed
                    .take()
                    .expect("generated schedule kills before restart");
                assert!(self.members.contains(&restarted));
                self.running.insert(restarted);
            }
            ExternalHistoryAction::DrainFollower => {
                let follower = self
                    .running
                    .iter()
                    .rev()
                    .find(|node| *node != &self.leader)
                    .cloned()
                    .expect("generated schedule keeps a follower to drain");
                assert!(self.running.remove(&follower));
                assert!(self.members.remove(&follower));
                self.epoch = self.epoch.saturating_add(1);
            }
        }
    }

    fn record(&self, action: ExternalHistoryAction, recorder: &mut ExternalHistoryRecorder) {
        let quorum = self.members.len() / 2 + 1;
        let quorum_ok = self
            .running
            .iter()
            .filter(|node| self.members.contains(*node))
            .count()
            >= quorum;
        let admin_statuses = self
            .running
            .iter()
            .map(|_| ExternalAdminObservation {
                leader: Some(self.leader.clone()),
                term: self.term,
                members: self.members.len() as u32,
                voters: self.members.len() as u32,
                quorum_ok,
                draining: false,
            })
            .collect();
        let membership_observations = self
            .running
            .iter()
            .map(|_| MembershipObservation {
                epoch: self.epoch,
                term: self.term,
                leader: Some(self.leader.clone()),
                members: self.members.clone(),
            })
            .collect();
        recorder.record_step(ExternalHistoryStep {
            action,
            admin_statuses,
            membership_observations,
        });
    }
}

fn replay_fast(schedule: &ExternalHistorySchedule) -> ExternalHistoryRecorder {
    let mut scheduler = ExternalHistoryScheduler::new(schedule);
    let mut model = FastControlPlaneModel::three_nodes();
    let mut recorder = ExternalHistoryRecorder::default();
    while let Some(action) = scheduler.next_action() {
        model.apply(action);
        model.record(action, &mut recorder);
    }
    assert_eq!(scheduler.remaining(), 0);
    recorder
}

#[test]
fn external_control_plane_history_is_consistent_under_process_faults() -> TestResult {
    let schedule = ExternalHistoryGenerator::new(0x0660_0007).generate();
    let generated_actions = schedule.actions.iter().copied().collect::<BTreeSet<_>>();
    assert!(generated_actions.contains(&ExternalHistoryAction::KillLeader));
    assert!(generated_actions.contains(&ExternalHistoryAction::RestartLastKilled));
    assert!(generated_actions.contains(&ExternalHistoryAction::DrainFollower));
    assert!(generated_actions.contains(&ExternalHistoryAction::CompactFollower));

    let checker = ExternalHistoryChecker;
    let fast_history = replay_fast(&schedule);
    let fast_report = checker.check(&fast_history);
    assert!(
        fast_report.is_ok(),
        "generated fast external history violated invariants: {:?}",
        fast_report.violations
    );

    if !skip_unless_daemon_process_e2e(
        "external_control_plane_history_is_consistent_under_process_faults",
    ) {
        return Ok(());
    }

    let process_history = replay_process(&schedule)?;
    let process_report = checker.check(&process_history);
    assert!(
        process_report.is_ok(),
        "real-process external history violated invariants: {:?}",
        process_report.violations
    );
    Ok(())
}

#[test]
fn external_history_failure_shrinks_to_one_step_minimal_schedule() {
    let checker = ExternalHistoryChecker;
    let history = history_with_one_atomic_invalid_batch_and_noise();
    assert!(!checker.check(&history).is_ok());

    let shrunk =
        ExternalHistoryShrinker.shrink(&history, |candidate| !checker.check(candidate).is_ok());

    assert_eq!(
        shrunk.steps().len(),
        1,
        "same-term split-leader batch should shrink to one atomic schedule step: {:?}",
        shrunk.steps()
    );
    let report = checker.check(&shrunk);
    assert!(has_violation(&report, "election_safety"));
    assert!(
        checker.check(&ExternalHistoryRecorder::default()).is_ok(),
        "removing the final failing step must make the one-step schedule minimal"
    );
}

#[test]
fn external_frozen_bad_seed_corpus_replays_fast() {
    let corpus = FrozenHistoryCorpus::parse(FROZEN_BAD_SEEDS).expect("frozen corpus is valid JSON");
    assert_eq!(corpus.schema_version, 1);
    assert!(corpus.cases.len() >= 3);

    let checker = ExternalHistoryChecker;
    for case in &corpus.cases {
        assert!(
            case.steps.len() <= 2,
            "frozen bad seed {} must remain a bounded fast replay",
            case.name
        );
        let report = checker.check(&case.recorder());
        assert!(
            !report.is_ok(),
            "frozen bad seed {} unexpectedly passed",
            case.name
        );
        for expected in &case.expected_violations {
            assert!(
                has_violation(&report, expected),
                "frozen bad seed {} missed {expected}: {:?}",
                case.name,
                report.violations
            );
        }
    }
}

#[test]
fn canary_external_checker_accepts_a_known_invalid_membership_history() {
    let checker = ExternalHistoryChecker;
    let invalid = known_invalid_same_term_leader_batch();
    let report = checker.check(&invalid);
    let accepted = if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W7") {
        true
    } else {
        report.is_ok()
    };
    assert!(
        !accepted,
        "HC-CANARY-RED:W7 external checker accepted a known invalid membership history"
    );
}

fn replay_process(schedule: &ExternalHistorySchedule) -> TestResult<ExternalHistoryRecorder> {
    let mut cluster =
        DaemonCluster::start_bootstrap_with_raft_compaction(3, "external-control-history")?;
    cluster.wait_for_shape(3, 3)?;
    let mut scheduler = ExternalHistoryScheduler::new(schedule);
    let mut recorder = ExternalHistoryRecorder::default();
    let mut last_killed = None;
    let mut members = 3;

    while let Some(action) = scheduler.next_action() {
        match action {
            ExternalHistoryAction::Observe => {}
            ExternalHistoryAction::CompactFollower => {
                let follower = follower_index(&mut cluster, members, members)?;
                let compacted = cluster.compact_raft_log(follower)?;
                assert_eq!(compacted["enabled"], true);
                cluster.wait_for_shape(members, members)?;
            }
            ExternalHistoryAction::KillLeader => {
                let statuses = cluster.wait_for_shape(members, members)?;
                let old_leader = statuses[0]
                    .leader
                    .clone()
                    .ok_or("external schedule observed no leader before kill")?;
                let index = cluster
                    .node_ids()
                    .iter()
                    .position(|node_id| node_id == &old_leader)
                    .ok_or("external schedule leader did not belong to DaemonCluster")?;
                cluster.kill(index)?;
                cluster.wait_for_leader_not(&old_leader, members, members)?;
                last_killed = Some(index);
            }
            ExternalHistoryAction::RestartLastKilled => {
                let index = last_killed
                    .take()
                    .ok_or("external generated schedule restarted before kill")?;
                cluster.restart(index)?;
                cluster.wait_for_shape(members, members)?;
            }
            ExternalHistoryAction::DrainFollower => {
                let follower = follower_index(&mut cluster, members, members)?;
                let accepted = cluster.drain(follower)?;
                assert_eq!(accepted["outcome"], "accepted");
                members -= 1;
                cluster.wait_for_non_draining_shape(
                    "external drain committed before history observation",
                    members,
                    members,
                )?;
            }
        }
        record_process_surfaces(action, &mut cluster, &mut recorder);
    }
    assert_eq!(scheduler.remaining(), 0);
    Ok(recorder)
}

fn follower_index(cluster: &mut DaemonCluster, members: u32, voters: u32) -> TestResult<usize> {
    let statuses = cluster.wait_for_shape(members, voters)?;
    let leader = statuses[0]
        .leader
        .as_deref()
        .ok_or("external schedule could not select follower without a leader")?;
    cluster
        .node_ids()
        .iter()
        .position(|node_id| node_id != leader)
        .ok_or_else(|| "external schedule found no follower".into())
}

fn record_process_surfaces(
    action: ExternalHistoryAction,
    cluster: &mut DaemonCluster,
    recorder: &mut ExternalHistoryRecorder,
) {
    let admin_statuses = cluster
        .statuses()
        .into_iter()
        .map(external_admin_observation)
        .collect();
    let overviews = cluster.overviews();
    recorder.record_public_surfaces(action, admin_statuses, &overviews);
}

fn external_admin_observation(status: DaemonStatus) -> ExternalAdminObservation {
    ExternalAdminObservation {
        leader: status.leader,
        term: status.term,
        members: status.members,
        voters: status.voters,
        quorum_ok: status.quorum_ok,
        draining: status.draining,
    }
}

fn history_with_one_atomic_invalid_batch_and_noise() -> ExternalHistoryRecorder {
    let mut steps = vec![valid_step(1, 1, "node-a")];
    steps.extend(known_invalid_same_term_leader_batch().into_steps());
    steps.push(valid_step(4, 9, "node-c"));
    ExternalHistoryRecorder::from_steps(steps)
}

fn known_invalid_same_term_leader_batch() -> ExternalHistoryRecorder {
    let members = all_members();
    ExternalHistoryRecorder::from_steps(vec![ExternalHistoryStep {
        action: ExternalHistoryAction::Observe,
        admin_statuses: Vec::new(),
        membership_observations: vec![
            MembershipObservation {
                epoch: 3,
                term: 7,
                leader: Some("node-a".to_owned()),
                members: members.clone(),
            },
            MembershipObservation {
                epoch: 3,
                term: 7,
                leader: Some("node-b".to_owned()),
                members,
            },
        ],
    }])
}

fn valid_step(epoch: u64, term: u64, leader: &str) -> ExternalHistoryStep {
    ExternalHistoryStep {
        action: ExternalHistoryAction::Observe,
        admin_statuses: Vec::new(),
        membership_observations: vec![MembershipObservation {
            epoch,
            term,
            leader: Some(leader.to_owned()),
            members: all_members(),
        }],
    }
}

fn all_members() -> BTreeSet<String> {
    BTreeSet::from([
        "node-a".to_owned(),
        "node-b".to_owned(),
        "node-c".to_owned(),
    ])
}

fn has_violation(report: &hydracache_sim::InvariantReport, name: &str) -> bool {
    report
        .violations
        .iter()
        .any(|violation| violation.name == name)
}
