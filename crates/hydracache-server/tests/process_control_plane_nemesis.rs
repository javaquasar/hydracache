mod support;

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use support::daemon_cluster::{
    skip_unless_daemon_process_e2e, DaemonCluster, DaemonStatus, TestResult,
};
use support::external_control_plane_history::{
    ExternalAdminObservation, ExternalHistoryAction, ExternalHistoryShrinker, ExternalHistoryStep,
    ExternalNemesisChecker, ExternalNemesisFailureArtifact, ExternalNemesisGenerator,
    ExternalNemesisPhase, ExternalNemesisRecorder, ExternalNemesisSchedule, FrozenNemesisCorpus,
    FrozenNemesisDefect,
};
use support::membership_history::MembershipObservation;

const FROZEN_NEMESIS_BAD_SEEDS: &str = include_str!("vectors/process_nemesis_bad_seeds.json");

#[derive(Debug, Clone)]
struct FastNemesisModel {
    members: BTreeSet<String>,
    running: BTreeSet<String>,
    leader: String,
    term: u64,
    epoch: u64,
    last_killed: Option<String>,
}

impl FastNemesisModel {
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

    fn apply(&mut self, action: ExternalHistoryAction, lose_committed_drain: bool) {
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
                    .expect("composed schedule retains a majority");
            }
            ExternalHistoryAction::RestartLastKilled => {
                let restarted = self
                    .last_killed
                    .take()
                    .expect("stable schedule restarts only after kill");
                assert!(self.members.contains(&restarted));
                self.running.insert(restarted);
            }
            ExternalHistoryAction::DrainFollower => {
                if lose_committed_drain {
                    return;
                }
                let follower = self
                    .running
                    .iter()
                    .rev()
                    .find(|node| *node != &self.leader)
                    .cloned()
                    .expect("composed schedule retains a follower to drain");
                assert!(self.running.remove(&follower));
                assert!(self.members.remove(&follower));
                self.epoch = self.epoch.saturating_add(1);
            }
        }
    }

    fn snapshot(&self, action: ExternalHistoryAction) -> ExternalHistoryStep {
        let quorum = self.members.len() / 2 + 1;
        let quorum_ok = self
            .running
            .iter()
            .filter(|node| self.members.contains(*node))
            .count()
            >= quorum;
        ExternalHistoryStep {
            action,
            admin_statuses: self
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
                .collect(),
            membership_observations: self
                .running
                .iter()
                .map(|_| MembershipObservation {
                    epoch: self.epoch,
                    term: self.term,
                    leader: Some(self.leader.clone()),
                    members: self.members.clone(),
                })
                .collect(),
        }
    }
}

fn replay_fast(
    schedule: &ExternalNemesisSchedule,
    lose_committed_drain: bool,
) -> ExternalNemesisRecorder {
    let mut model = FastNemesisModel::three_nodes();
    let mut trace = ExternalNemesisRecorder::default();
    for operation in &schedule.operations {
        trace.record_phase(
            operation,
            ExternalNemesisPhase::Invoke,
            model.snapshot(ExternalHistoryAction::Observe),
        );
        for action in &operation.actions {
            model.apply(*action, lose_committed_drain);
            trace.record_intermediate(model.snapshot(*action));
        }
        trace.record_phase(
            operation,
            ExternalNemesisPhase::Complete,
            model.snapshot(
                operation
                    .actions
                    .last()
                    .copied()
                    .unwrap_or(ExternalHistoryAction::Observe),
            ),
        );
    }
    trace
}

#[test]
fn process_nemesis_committed_control_plane_history_is_consistent() -> TestResult {
    let schedule = ExternalNemesisGenerator::new(0x0660_0002).generate();
    assert_composed_fault_coverage(&schedule);
    let checker = ExternalNemesisChecker::default();

    let fast_trace = replay_fast(&schedule, false);
    assert_stable_operation_evidence(&schedule, &fast_trace);
    let fast_report = checker.check(&fast_trace);
    assert!(
        fast_report.is_ok(),
        "fast composed-fault history violated invariants: {:?}",
        fast_report.violations
    );

    if !skip_unless_daemon_process_e2e(
        "process_nemesis_committed_control_plane_history_is_consistent",
    ) {
        return Ok(());
    }

    let (process_trace, daemon_logs) = replay_process(&schedule)?;
    assert_stable_operation_evidence(&schedule, &process_trace);
    let process_report = checker.check(&process_trace);
    if !process_report.is_ok() {
        panic!(
            "real-process nemesis failed; daemon_logs={daemon_logs:?}; violations={:?}",
            process_report.violations
        );
    }
    Ok(())
}

#[test]
fn process_nemesis_same_seed_replays_same_schedule() {
    let seed = 0x0660_2202;
    let first = ExternalNemesisGenerator::new(seed).generate();
    let second = ExternalNemesisGenerator::new(seed).generate();
    let different = ExternalNemesisGenerator::new(seed + 1).generate();

    assert_eq!(first, second);
    assert_eq!(
        serde_json::to_vec(&first).unwrap(),
        serde_json::to_vec(&second).unwrap(),
        "same seed must have byte-identical serialized schedule evidence"
    );
    assert_ne!(first, different);
    assert_eq!(replay_fast(&first, false), replay_fast(&second, false));

    for (index, operation) in first.operations.iter().enumerate() {
        let prefix = format!("{seed:016x}-{index:04}");
        assert_eq!(operation.operation_id, format!("op-{prefix}"));
        assert_eq!(operation.command_id, format!("cmd-{prefix}"));
        assert_eq!(
            operation.invoke_observation_id,
            format!("obs-{prefix}-invoke")
        );
        assert_eq!(
            operation.complete_observation_id,
            format!("obs-{prefix}-complete")
        );
    }
}

#[test]
fn process_nemesis_failure_shrinks_and_frozen_seeds_replay() {
    let original = ExternalNemesisGenerator::new(0x0660_3302).generate();
    let checker = ExternalNemesisChecker::default();
    let shrinker = ExternalHistoryShrinker;
    let failure_persists =
        |schedule: &ExternalNemesisSchedule| !checker.check(&replay_fast(schedule, true)).is_ok();
    assert!(failure_persists(&original));

    let minimized = shrinker.shrink_nemesis_schedule(&original, failure_persists);
    assert_eq!(minimized.operations.len(), 1);
    assert!(minimized.operations[0].contains(ExternalHistoryAction::DrainFollower));
    assert!(failure_persists(&minimized));
    let empty = ExternalNemesisSchedule {
        seed: minimized.seed,
        operations: Vec::new(),
    };
    assert!(!failure_persists(&empty));

    let minimized_trace = replay_fast(&minimized, true);
    let minimized_report = checker.check(&minimized_trace);
    let artifact = ExternalNemesisFailureArtifact::new(
        original.clone(),
        minimized.clone(),
        &minimized_trace,
        &minimized_report,
        Vec::new(),
    );
    let encoded = artifact.encode_pretty_json().unwrap();
    let decoded: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded["schema_version"], 1);
    assert_eq!(decoded["seed"], original.seed);
    assert_eq!(
        decoded["minimized_schedule"]["operations"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let corpus = FrozenNemesisCorpus::parse(FROZEN_NEMESIS_BAD_SEEDS)
        .expect("frozen process nemesis corpus is valid JSON");
    assert_eq!(corpus.schema_version, 1);
    assert!(corpus.cases.len() >= 2);
    for case in &corpus.cases {
        let lose_committed_drain = matches!(case.defect, FrozenNemesisDefect::LoseCommittedDrain);
        let report = checker.check(&replay_fast(&case.schedule, lose_committed_drain));
        assert!(
            !report.is_ok(),
            "frozen process nemesis seed {} unexpectedly passed",
            case.name
        );
        for expected in &case.expected_violations {
            assert!(
                has_violation(&report, expected),
                "frozen seed {} missed {expected}: {:?}",
                case.name,
                report.violations
            );
        }
    }
}

#[test]
fn canary_process_nemesis_accepts_a_lost_committed_metadata_command() {
    let schedule = ExternalNemesisGenerator::new(0x0660_4402).generate();
    let report = ExternalNemesisChecker::default().check(&replay_fast(&schedule, true));
    let accepted = if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W2") {
        true
    } else {
        report.is_ok()
    };
    assert!(
        !accepted,
        "HC-CANARY-RED:W2 process nemesis accepted a lost committed metadata command"
    );
}

fn replay_process(
    schedule: &ExternalNemesisSchedule,
) -> TestResult<(ExternalNemesisRecorder, Vec<String>)> {
    let mut cluster = DaemonCluster::start_bootstrap_with_raft_compaction(3, "process-nemesis")?;
    cluster.wait_for_shape(3, 3)?;
    let mut trace = ExternalNemesisRecorder::default();
    let mut last_killed = None;
    let mut members = 3_u32;

    for operation in &schedule.operations {
        trace.record_phase(
            operation,
            ExternalNemesisPhase::Invoke,
            process_committed_snapshot(ExternalHistoryAction::Observe, &mut cluster),
        );
        for action in &operation.actions {
            match action {
                ExternalHistoryAction::Observe => {}
                ExternalHistoryAction::CompactFollower => {
                    let follower = follower_index(&mut cluster, members)?;
                    let compacted = cluster.compact_raft_log(follower)?;
                    assert_eq!(compacted["enabled"], true);
                    cluster.wait_for_shape(members, members)?;
                }
                ExternalHistoryAction::KillLeader => {
                    let statuses = cluster.wait_for_shape(members, members)?;
                    let old_leader = statuses[0]
                        .leader
                        .clone()
                        .ok_or("nemesis observed no leader before kill")?;
                    let index = cluster
                        .node_ids()
                        .iter()
                        .position(|node_id| node_id == &old_leader)
                        .ok_or("nemesis leader did not belong to DaemonCluster")?;
                    cluster.kill(index)?;
                    cluster.wait_for_leader_not(&old_leader, members, members)?;
                    last_killed = Some(index);
                }
                ExternalHistoryAction::RestartLastKilled => {
                    let index = last_killed
                        .take()
                        .ok_or("nemesis restarted before a kill")?;
                    cluster.restart(index)?;
                    cluster.wait_for_shape(members, members)?;
                }
                ExternalHistoryAction::DrainFollower => {
                    let follower = follower_index(&mut cluster, members)?;
                    let accepted = cluster.drain(follower)?;
                    assert_eq!(accepted["outcome"], "accepted");
                    members -= 1;
                    cluster.wait_for_non_draining_shape(
                        "process nemesis drain commit",
                        members,
                        members,
                    )?;
                }
            }
            trace.record_intermediate(process_committed_snapshot(*action, &mut cluster));
        }
        trace.record_phase(
            operation,
            ExternalNemesisPhase::Complete,
            process_committed_snapshot(
                operation
                    .actions
                    .last()
                    .copied()
                    .unwrap_or(ExternalHistoryAction::Observe),
                &mut cluster,
            ),
        );
    }

    let evidence = cluster.replay_evidence(None);
    let daemon_logs = evidence
        .stdout_logs
        .iter()
        .chain(evidence.stderr_logs.iter())
        .map(|path| path.display().to_string())
        .collect();
    Ok((trace, daemon_logs))
}

fn process_committed_snapshot(
    action: ExternalHistoryAction,
    cluster: &mut DaemonCluster,
) -> ExternalHistoryStep {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut latest = None;
    while Instant::now() < deadline {
        let admin_statuses = cluster
            .statuses()
            .into_iter()
            .map(external_admin_observation)
            .collect();
        let overviews = cluster.overviews();
        let step = ExternalHistoryStep::from_public_surfaces(action, admin_statuses, &overviews);
        let committed_views = step
            .membership_observations
            .iter()
            .filter(|observation| !observation.members.is_empty())
            .map(|observation| (observation.epoch, observation.members.clone()))
            .collect::<BTreeSet<_>>();
        if committed_views.len() == 1 && !step.admin_statuses.is_empty() {
            return step;
        }
        latest = Some(step);
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!(
        "public /admin/status and /cluster/overview did not converge to one committed view for {action:?}; latest={latest:?}"
    );
}

fn follower_index(cluster: &mut DaemonCluster, members: u32) -> TestResult<usize> {
    let statuses = cluster.wait_for_shape(members, members)?;
    let leader = statuses[0]
        .leader
        .as_deref()
        .ok_or("nemesis could not select a follower without a leader")?;
    cluster
        .node_ids()
        .iter()
        .position(|node_id| node_id != leader)
        .ok_or_else(|| "nemesis found no follower".into())
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

fn assert_composed_fault_coverage(schedule: &ExternalNemesisSchedule) {
    let actions = schedule
        .operations
        .iter()
        .flat_map(|operation| operation.actions.iter().copied())
        .collect::<BTreeSet<_>>();
    for required in [
        ExternalHistoryAction::KillLeader,
        ExternalHistoryAction::RestartLastKilled,
        ExternalHistoryAction::CompactFollower,
        ExternalHistoryAction::DrainFollower,
    ] {
        assert!(
            actions.contains(&required),
            "missing composed fault {required:?}"
        );
    }
    assert!(schedule.operations.iter().any(|operation| {
        operation.contains(ExternalHistoryAction::KillLeader)
            && operation.contains(ExternalHistoryAction::RestartLastKilled)
    }));
}

fn assert_stable_operation_evidence(
    schedule: &ExternalNemesisSchedule,
    trace: &ExternalNemesisRecorder,
) {
    assert_eq!(trace.events().len(), schedule.operations.len() * 2);
    for operation in &schedule.operations {
        let events = trace
            .events()
            .iter()
            .filter(|event| event.operation_id == operation.operation_id)
            .collect::<Vec<_>>();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].phase, ExternalNemesisPhase::Invoke);
        assert_eq!(events[1].phase, ExternalNemesisPhase::Complete);
        assert_eq!(events[0].observation_id, operation.invoke_observation_id);
        assert_eq!(events[1].observation_id, operation.complete_observation_id);
        assert!(events.iter().all(|event| event.committed_epoch.is_some()));
        assert!(events.iter().all(|event| event.public_membership.is_some()));
    }
}

fn has_violation(report: &hydracache_sim::InvariantReport, name: &str) -> bool {
    report
        .violations
        .iter()
        .any(|violation| violation.name == name)
}
