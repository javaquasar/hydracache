mod support;

use std::collections::BTreeSet;
use std::fs;
use std::panic::{self, AssertUnwindSafe};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::Value;
use support::daemon_cluster::{
    skip_unless_daemon_process_e2e, DaemonCluster, DaemonRaftFaultProof, DaemonStatus, TestResult,
};
use support::external_control_plane_history::{
    ExternalAdminObservation, ExternalHistoryAction, ExternalHistoryShrinker, ExternalHistoryStep,
    ExternalNemesisChecker, ExternalNemesisFailureArtifact, ExternalNemesisGenerator,
    ExternalNemesisPhase, ExternalNemesisRecorder, ExternalNemesisSchedule, FrozenNemesisCorpus,
    FrozenNemesisDefect,
};
use support::membership_history::MembershipObservation;

const FROZEN_NEMESIS_BAD_SEEDS: &str = include_str!("vectors/process_nemesis_bad_seeds.json");
const PROCESS_NEMESIS_SEED: u64 = 0x0660_0002;
const PROCESS_NEMESIS_ARTIFACT: &str =
    "target/test-evidence/0.66/process-control-plane-nemesis.json";
const PROCESS_NEMESIS_RELEASE: &str = "0.66.0";
const PROCESS_NEMESIS_GATE: &str = "env.hydracache-run-066-daemon-process-e2e";
const RAFT_TRANSPORT_DELAY_MS: u64 = 150;

#[derive(Debug, Clone)]
struct FastNemesisModel {
    members: BTreeSet<String>,
    running: BTreeSet<String>,
    leader: String,
    term: u64,
    epoch: u64,
    last_killed: Option<String>,
    last_paused: Option<String>,
    partitioned: bool,
    delayed: bool,
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
            last_paused: None,
            partitioned: false,
            delayed: false,
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
            ExternalHistoryAction::PauseLeader => {
                let paused = self.leader.clone();
                assert!(self.running.remove(&paused));
                self.last_paused = Some(paused);
                self.term = self.term.saturating_add(1);
                self.leader = self
                    .running
                    .iter()
                    .next()
                    .cloned()
                    .expect("composed schedule retains a majority");
            }
            ExternalHistoryAction::ResumeLastPaused => {
                let resumed = self
                    .last_paused
                    .take()
                    .expect("stable schedule resumes only after pause");
                assert!(self.members.contains(&resumed));
                self.running.insert(resumed);
            }
            ExternalHistoryAction::PartitionFollower => {
                assert!(!self.partitioned);
                self.partitioned = true;
            }
            ExternalHistoryAction::HealLastPartition => {
                assert!(self.partitioned);
                self.partitioned = false;
            }
            ExternalHistoryAction::DelayTransport => {
                assert!(!self.delayed);
                self.delayed = true;
            }
            ExternalHistoryAction::ClearTransportDelay => {
                assert!(self.delayed);
                self.delayed = false;
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
            model.running.len(),
        );
        for action in &operation.actions {
            model.apply(*action, lose_committed_drain);
            trace.record_intermediate(model.snapshot(*action), model.running.len());
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
            model.running.len(),
        );
    }
    trace
}

#[test]
fn process_nemesis_committed_control_plane_history_is_consistent() -> TestResult {
    let schedule = ExternalNemesisGenerator::new(PROCESS_NEMESIS_SEED).generate();
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

    let original_replay = replay_process(&schedule);
    let original_report = checker.check(&original_replay.trace);
    let signature = checker_failure_signature(&original_report);
    let infrastructure_clean =
        original_replay.execution_error.is_none() && original_replay.cleanup_errors.is_empty();
    if infrastructure_clean && signature.is_empty() {
        assert_stable_operation_evidence(&schedule, &original_replay.trace);
        write_process_nemesis_artifact(
            "pass",
            &schedule,
            None,
            &original_replay,
            &original_report,
            None,
            true,
        )?;
        return Ok(());
    }

    if !infrastructure_clean || signature.is_empty() {
        write_process_nemesis_artifact(
            "fail",
            &schedule,
            None,
            &original_replay,
            &original_report,
            Some(&original_replay.daemon_logs),
            false,
        )?;
        panic!(
            "real-process nemesis infrastructure failed without schedule shrinking; execution_error={:?}; cleanup_errors={:?}; violations={:?}; daemon_logs={:?}",
            original_replay.execution_error,
            original_replay.cleanup_errors,
            original_report.violations,
            original_replay.daemon_logs,
        );
    }

    let minimized = ExternalHistoryShrinker.shrink_nemesis_schedule(&schedule, |candidate| {
        let replay = replay_process(candidate);
        let report = checker.check(&replay.trace);
        replay.execution_error.is_none()
            && replay.cleanup_errors.is_empty()
            && checker_failure_signature(&report) == signature
    });
    let minimized_replay = replay_process(&minimized);
    let minimized_report = checker.check(&minimized_replay.trace);
    let minimized_reproduced = minimized_replay.execution_error.is_none()
        && minimized_replay.cleanup_errors.is_empty()
        && checker_failure_signature(&minimized_report) == signature;
    write_process_nemesis_artifact(
        "fail",
        &schedule,
        Some(&minimized),
        &minimized_replay,
        &minimized_report,
        Some(&original_replay.daemon_logs),
        minimized_reproduced,
    )?;
    panic!(
        "real-process nemesis failed; signature={signature:?}; minimized_reproduced={minimized_reproduced}; daemon_logs={:?}; violations={:?}; execution_error={:?}; cleanup_errors={:?}",
        original_replay.daemon_logs,
        original_report.violations,
        original_replay.execution_error,
        original_replay.cleanup_errors,
    );
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
fn process_public_history_rejects_malformed_or_disagreeing_overviews() {
    let authoritative = ExternalAdminObservation {
        leader: Some("node-a".to_owned()),
        term: 7,
        members: 2,
        voters: 2,
        quorum_ok: true,
        draining: false,
    };
    let valid = serde_json::json!({
        "members": [{"node_id": "node-a"}, {"node_id": "node-b"}],
        "leader": {"node_id": "node-a", "term": 7, "epoch": 11}
    });
    assert!(strict_process_history_step(
        ExternalHistoryAction::Observe,
        vec![authoritative.clone()],
        std::slice::from_ref(&valid),
    )
    .is_ok());

    let missing_epoch = serde_json::json!({
        "members": [{"node_id": "node-a"}, {"node_id": "node-b"}],
        "leader": {"node_id": "node-a", "term": 7}
    });
    assert!(strict_process_history_step(
        ExternalHistoryAction::Observe,
        vec![authoritative.clone()],
        &[missing_epoch],
    )
    .is_err());

    let disagreeing = serde_json::json!({
        "members": [{"node_id": "node-a"}, {"node_id": "node-b"}],
        "leader": {"node_id": "node-b", "term": 8, "epoch": 11}
    });
    assert!(strict_process_history_step(
        ExternalHistoryAction::Observe,
        vec![authoritative.clone()],
        &[disagreeing],
    )
    .is_err());

    let malformed_member = serde_json::json!({
        "members": [{"node_id": "node-a"}, {}],
        "leader": {"node_id": "node-a", "term": 7, "epoch": 11}
    });
    assert!(strict_process_history_step(
        ExternalHistoryAction::Observe,
        vec![authoritative],
        &[malformed_member],
    )
    .is_err());
}

#[test]
fn dependency_validator_rejects_overlapping_external_faults() {
    let mut schedule = ExternalNemesisGenerator::new(PROCESS_NEMESIS_SEED).generate();
    schedule.operations[0].actions = vec![
        ExternalHistoryAction::PartitionFollower,
        ExternalHistoryAction::DelayTransport,
        ExternalHistoryAction::ClearTransportDelay,
        ExternalHistoryAction::HealLastPartition,
    ];
    assert!(schedule.validate_dependency_groups().is_err());
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActiveTransportFault {
    Partition,
    Delay,
}

#[derive(Debug)]
struct ProcessNemesisState {
    members: u32,
    last_killed: Option<usize>,
    last_paused: Option<usize>,
    active_transport_fault: Option<ActiveTransportFault>,
    fault_proofs: Vec<DaemonRaftFaultProof>,
}

impl Default for ProcessNemesisState {
    fn default() -> Self {
        Self {
            members: 3,
            last_killed: None,
            last_paused: None,
            active_transport_fault: None,
            fault_proofs: Vec::new(),
        }
    }
}

impl ProcessNemesisState {
    fn expected_responders(&self, cluster: &DaemonCluster) -> usize {
        cluster
            .node_ids()
            .len()
            .saturating_sub(usize::from(self.last_killed.is_some()))
            .saturating_sub(usize::from(self.last_paused.is_some()))
    }
}

#[derive(Debug)]
struct ProcessReplay {
    trace: ExternalNemesisRecorder,
    daemon_logs: Vec<String>,
    fault_proofs: Vec<DaemonRaftFaultProof>,
    execution_error: Option<String>,
    cleanup_errors: Vec<String>,
}

impl ProcessReplay {
    fn setup_failure(error: impl ToString) -> Self {
        Self {
            trace: ExternalNemesisRecorder::default(),
            daemon_logs: Vec::new(),
            fault_proofs: Vec::new(),
            execution_error: Some(error.to_string()),
            cleanup_errors: vec![
                "cleanup was not attempted because daemon cluster setup failed".to_owned(),
            ],
        }
    }
}

#[derive(Debug, Serialize)]
struct InvokeCompleteAccounting {
    expected_events: usize,
    observed_events: usize,
    expected_action_observations: usize,
    observed_action_observations: usize,
    expected_admin_responses: usize,
    observed_admin_responses: usize,
    expected_overview_responses: usize,
    observed_overview_responses: usize,
}

#[derive(Debug, Serialize)]
struct ProcessNemesisCheckerEvidence {
    checked: usize,
    violations: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ProcessNemesisRuntimeArtifact<'a> {
    schema_version: u32,
    release: &'static str,
    gate_id: &'static str,
    test_name: &'static str,
    platform: &'static str,
    seed: u64,
    outcome: &'a str,
    original_schedule: &'a ExternalNemesisSchedule,
    minimized_schedule: Option<&'a ExternalNemesisSchedule>,
    minimized_reproduced: bool,
    invoke_complete_accounting: InvokeCompleteAccounting,
    fault_proofs: &'a [DaemonRaftFaultProof],
    checker: ProcessNemesisCheckerEvidence,
    execution_error: &'a Option<String>,
    cleanup_errors: &'a [String],
    cleanup_ok: bool,
    events: &'a [support::external_control_plane_history::ExternalNemesisEvent],
    response_counts: &'a [support::external_control_plane_history::ExternalPublicResponseCount],
    observed_history: &'a [ExternalHistoryStep],
    daemon_logs: &'a [String],
    original_daemon_logs: Option<&'a [String]>,
}

fn checker_failure_signature(report: &hydracache_sim::InvariantReport) -> BTreeSet<String> {
    report
        .violations
        .iter()
        .map(|violation| violation.name.to_owned())
        .collect()
}

fn write_process_nemesis_artifact(
    outcome: &str,
    original_schedule: &ExternalNemesisSchedule,
    minimized_schedule: Option<&ExternalNemesisSchedule>,
    replay: &ProcessReplay,
    report: &hydracache_sim::InvariantReport,
    original_daemon_logs: Option<&[String]>,
    minimized_reproduced: bool,
) -> TestResult<PathBuf> {
    let events = replay.trace.events();
    let response_counts = replay.trace.response_counts();
    let executed_schedule = minimized_schedule.unwrap_or(original_schedule);
    let accounting = InvokeCompleteAccounting {
        expected_events: executed_schedule.operations.len() * 2,
        observed_events: events.len(),
        expected_action_observations: executed_schedule
            .operations
            .iter()
            .map(|operation| operation.actions.len() + 2)
            .sum(),
        observed_action_observations: replay.trace.history().steps().len(),
        expected_admin_responses: response_counts
            .iter()
            .map(|count| count.expected_admin_responses)
            .sum(),
        observed_admin_responses: response_counts
            .iter()
            .map(|count| count.observed_admin_responses)
            .sum(),
        expected_overview_responses: response_counts
            .iter()
            .map(|count| count.expected_overview_responses)
            .sum(),
        observed_overview_responses: response_counts
            .iter()
            .map(|count| count.observed_overview_responses)
            .sum(),
    };
    let daemon_logs = replay.daemon_logs.as_slice();
    let artifact = ProcessNemesisRuntimeArtifact {
        schema_version: 1,
        release: PROCESS_NEMESIS_RELEASE,
        gate_id: PROCESS_NEMESIS_GATE,
        test_name: "process_nemesis_committed_control_plane_history_is_consistent",
        platform: std::env::consts::OS,
        seed: original_schedule.seed,
        outcome,
        original_schedule,
        minimized_schedule,
        minimized_reproduced,
        invoke_complete_accounting: accounting,
        fault_proofs: &replay.fault_proofs,
        checker: ProcessNemesisCheckerEvidence {
            checked: report.checked,
            violations: report.violations.iter().map(ToString::to_string).collect(),
        },
        execution_error: &replay.execution_error,
        cleanup_errors: &replay.cleanup_errors,
        cleanup_ok: replay.cleanup_errors.is_empty(),
        events,
        response_counts,
        observed_history: replay.trace.history().steps(),
        daemon_logs,
        original_daemon_logs,
    };
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(PROCESS_NEMESIS_ARTIFACT);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, serde_json::to_vec_pretty(&artifact)?)?;
    Ok(path)
}

fn replay_process(schedule: &ExternalNemesisSchedule) -> ProcessReplay {
    let mut cluster = match DaemonCluster::start_bootstrap_with_raft_compaction_and_outbound_faults(
        3,
        "process-nemesis",
    ) {
        Ok(cluster) => cluster,
        Err(error) => return ProcessReplay::setup_failure(error),
    };
    let mut trace = ExternalNemesisRecorder::default();
    let mut state = ProcessNemesisState::default();
    let execution = panic::catch_unwind(AssertUnwindSafe(|| {
        cluster.wait_for_shape(3, 3)?;
        replay_process_operations(schedule, &mut cluster, &mut state, &mut trace)
    }));
    let execution_error = match execution {
        Ok(Ok(())) => None,
        Ok(Err(error)) => Some(error.to_string()),
        Err(payload) => Some(format!(
            "panic: {}",
            panic_payload_message(payload.as_ref())
        )),
    };
    let cleanup_errors = cleanup_process_replay(&mut cluster, &mut state);
    let evidence = cluster.replay_evidence(None);
    let daemon_logs = evidence
        .stdout_logs
        .iter()
        .chain(evidence.stderr_logs.iter())
        .map(|path| path.display().to_string())
        .collect();
    ProcessReplay {
        trace,
        daemon_logs,
        fault_proofs: state.fault_proofs,
        execution_error,
        cleanup_errors,
    }
}

fn replay_process_operations(
    schedule: &ExternalNemesisSchedule,
    cluster: &mut DaemonCluster,
    state: &mut ProcessNemesisState,
    trace: &mut ExternalNemesisRecorder,
) -> TestResult {
    schedule.validate_dependency_groups()?;
    for operation in &schedule.operations {
        let expected = state.expected_responders(cluster);
        trace.record_phase(
            operation,
            ExternalNemesisPhase::Invoke,
            process_committed_snapshot(ExternalHistoryAction::Observe, cluster, expected)?,
            expected,
        );
        for action in &operation.actions {
            apply_process_action(*action, cluster, state)?;
            let expected = state.expected_responders(cluster);
            trace.record_intermediate(
                process_committed_snapshot(*action, cluster, expected)?,
                expected,
            );
        }
        let expected = state.expected_responders(cluster);
        trace.record_phase(
            operation,
            ExternalNemesisPhase::Complete,
            process_committed_snapshot(
                operation
                    .actions
                    .last()
                    .copied()
                    .unwrap_or(ExternalHistoryAction::Observe),
                cluster,
                expected,
            )?,
            expected,
        );
    }
    Ok(())
}

fn apply_process_action(
    action: ExternalHistoryAction,
    cluster: &mut DaemonCluster,
    state: &mut ProcessNemesisState,
) -> TestResult {
    match action {
        ExternalHistoryAction::Observe => {}
        ExternalHistoryAction::CompactFollower => {
            let follower = follower_index(cluster, state.members)?;
            let compacted = cluster.compact_raft_log(follower)?;
            if compacted["enabled"] != true {
                return Err("process nemesis compaction endpoint was not enabled".into());
            }
            cluster.wait_for_shape(state.members, state.members)?;
        }
        ExternalHistoryAction::KillLeader => {
            let (index, old_leader) = leader_index_and_id(cluster, state.members)?;
            cluster.kill(index)?;
            state.last_killed = Some(index);
            cluster.wait_for_leader_not(&old_leader, state.members, state.members)?;
        }
        ExternalHistoryAction::RestartLastKilled => {
            let index = state
                .last_killed
                .take()
                .ok_or("nemesis restarted before a kill")?;
            cluster.restart(index)?;
            cluster.wait_for_shape(state.members, state.members)?;
        }
        ExternalHistoryAction::PauseLeader => {
            let (index, old_leader) = leader_index_and_id(cluster, state.members)?;
            suspend_daemon(cluster, index)?;
            state.last_paused = Some(index);
            cluster.wait_for_leader_not(&old_leader, state.members, state.members)?;
        }
        ExternalHistoryAction::ResumeLastPaused => {
            let index = state
                .last_paused
                .take()
                .ok_or("nemesis resumed before a pause")?;
            resume_daemon(cluster, index)?;
            cluster.wait_for_shape(state.members, state.members)?;
        }
        ExternalHistoryAction::PartitionFollower => {
            if state.active_transport_fault.is_some() {
                return Err("nemesis attempted overlapping transport faults".into());
            }
            let follower = follower_index(cluster, state.members)?;
            let proof = cluster.install_symmetric_raft_partition(follower)?;
            state.active_transport_fault = Some(ActiveTransportFault::Partition);
            state.fault_proofs.push(proof);
            cluster.wait_for_raft_fault_hit(
                state
                    .fault_proofs
                    .last_mut()
                    .expect("partition proof was just pushed"),
            )?;
            wait_for_partition_effect(cluster, follower, state.members)?;
        }
        ExternalHistoryAction::HealLastPartition => {
            if state.active_transport_fault != Some(ActiveTransportFault::Partition) {
                return Err("nemesis healed without an active partition".into());
            }
            let cleared_generation = cluster.clear_raft_outbound_faults()?;
            cluster.wait_for_shape(state.members, state.members)?;
            mark_fault_healed(state, "partition", cleared_generation)?;
            state.active_transport_fault = None;
        }
        ExternalHistoryAction::DelayTransport => {
            if state.active_transport_fault.is_some() {
                return Err("nemesis attempted overlapping transport faults".into());
            }
            let follower = follower_index(cluster, state.members)?;
            let proof = cluster.install_symmetric_raft_delay(follower, RAFT_TRANSPORT_DELAY_MS)?;
            state.active_transport_fault = Some(ActiveTransportFault::Delay);
            state.fault_proofs.push(proof);
            cluster.wait_for_raft_fault_hit(
                state
                    .fault_proofs
                    .last_mut()
                    .expect("delay proof was just pushed"),
            )?;
        }
        ExternalHistoryAction::ClearTransportDelay => {
            if state.active_transport_fault != Some(ActiveTransportFault::Delay) {
                return Err("nemesis cleared delay without an active delay".into());
            }
            let cleared_generation = cluster.clear_raft_outbound_faults()?;
            cluster.wait_for_shape(state.members, state.members)?;
            mark_fault_healed(state, "delay", cleared_generation)?;
            state.active_transport_fault = None;
        }
        ExternalHistoryAction::DrainFollower => {
            let follower = follower_index(cluster, state.members)?;
            let accepted = cluster.drain(follower)?;
            if accepted["outcome"] != "accepted" {
                return Err(format!("process nemesis drain was not accepted: {accepted}").into());
            }
            state.members = state.members.saturating_sub(1);
            cluster.wait_for_non_draining_shape(
                "process nemesis drain commit",
                state.members,
                state.members,
            )?;
        }
    }
    Ok(())
}

fn process_committed_snapshot(
    action: ExternalHistoryAction,
    cluster: &mut DaemonCluster,
    expected_responses: usize,
) -> TestResult<ExternalHistoryStep> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut latest_error = None;
    while Instant::now() < deadline {
        let indices = cluster.running_indices();
        if indices.len() != expected_responses {
            latest_error = Some(format!(
                "expected {expected_responses} serving daemons, found {}",
                indices.len()
            ));
            std::thread::sleep(Duration::from_millis(50));
            continue;
        }
        let observed = (|| -> TestResult<_> {
            let admin_statuses = indices
                .iter()
                .map(|index| cluster.admin_status(*index).map(external_admin_observation))
                .collect::<TestResult<Vec<_>>>()?;
            let overviews = indices
                .iter()
                .map(|index| cluster.cluster_overview(*index))
                .collect::<TestResult<Vec<_>>>()?;
            Ok((admin_statuses, overviews))
        })();
        let (admin_statuses, overviews) = match observed {
            Ok(observed) => observed,
            Err(error) => {
                latest_error = Some(error.to_string());
                std::thread::sleep(Duration::from_millis(50));
                continue;
            }
        };
        match strict_process_history_step(action, admin_statuses, &overviews) {
            Ok(step) => return Ok(step),
            Err(error) => {
                latest_error = Some(error.to_string());
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err(format!(
        "public /admin/status and /cluster/overview did not return exactly {expected_responses} strict responses with one authoritative committed view for {action:?}; latest_error={latest_error:?}"
    )
    .into())
}

fn strict_process_history_step(
    action: ExternalHistoryAction,
    admin_statuses: Vec<ExternalAdminObservation>,
    overviews: &[Value],
) -> TestResult<ExternalHistoryStep> {
    if admin_statuses.len() != overviews.len() {
        return Err(format!(
            "public response cardinality mismatch: admin={} overview={}",
            admin_statuses.len(),
            overviews.len()
        )
        .into());
    }
    let membership_observations = overviews
        .iter()
        .map(strict_membership_observation)
        .collect::<TestResult<Vec<_>>>()?;
    let mut authoritative = BTreeSet::new();
    for (status, observation) in admin_statuses.iter().zip(&membership_observations) {
        if !status.quorum_ok || status.draining {
            continue;
        }
        let leader = status
            .leader
            .as_deref()
            .filter(|leader| !leader.trim().is_empty())
            .ok_or("authoritative admin status is missing a non-empty leader")?;
        if status.term == 0 || status.members == 0 || status.voters == 0 {
            return Err(format!(
                "authoritative admin status has zero term/members/voters: {status:?}"
            )
            .into());
        }
        if observation.leader.as_deref() != Some(leader)
            || observation.term != status.term
            || observation.epoch == 0
            || observation.members.len() != status.members as usize
            || status.voters != status.members
        {
            return Err(format!(
                "authoritative admin/overview mismatch: admin={status:?} overview={observation:?}"
            )
            .into());
        }
        authoritative.insert((
            observation.epoch,
            observation.term,
            leader.to_owned(),
            observation.members.clone(),
        ));
    }
    if authoritative.len() != 1 {
        return Err(format!(
            "expected one authoritative (epoch, term, leader, membership) view, got {authoritative:?}"
        )
        .into());
    }
    Ok(ExternalHistoryStep {
        action,
        admin_statuses,
        membership_observations,
    })
}

fn strict_membership_observation(overview: &Value) -> TestResult<MembershipObservation> {
    let object = overview
        .as_object()
        .ok_or("cluster overview must be a JSON object")?;
    let raw_members = object
        .get("members")
        .and_then(Value::as_array)
        .ok_or("cluster overview members must be an array")?;
    if raw_members.is_empty() {
        return Err("cluster overview members must not be empty".into());
    }
    let mut members = BTreeSet::new();
    for (index, member) in raw_members.iter().enumerate() {
        let node_id = member
            .as_object()
            .and_then(|member| member.get("node_id"))
            .and_then(Value::as_str)
            .filter(|node_id| !node_id.trim().is_empty())
            .ok_or_else(|| format!("cluster overview member {index} has no non-empty node_id"))?;
        if !members.insert(node_id.to_owned()) {
            return Err(format!("cluster overview contains duplicate member {node_id}").into());
        }
    }
    let raw_leader = object
        .get("leader")
        .ok_or("cluster overview must contain leader, including explicit null")?;
    let (epoch, term, leader) = if raw_leader.is_null() {
        (0, 0, None)
    } else {
        let leader = raw_leader
            .as_object()
            .ok_or("cluster overview leader must be an object or null")?;
        let node_id = leader
            .get("node_id")
            .and_then(Value::as_str)
            .filter(|node_id| !node_id.trim().is_empty())
            .ok_or("cluster overview leader has no non-empty node_id")?;
        let term = leader
            .get("term")
            .and_then(Value::as_u64)
            .filter(|term| *term > 0)
            .ok_or("cluster overview leader has no positive term")?;
        let epoch = leader
            .get("epoch")
            .and_then(Value::as_u64)
            .filter(|epoch| *epoch > 0)
            .ok_or("cluster overview leader has no positive epoch")?;
        if !members.contains(node_id) {
            return Err(format!(
                "cluster overview leader {node_id} is absent from membership {members:?}"
            )
            .into());
        }
        (epoch, term, Some(node_id.to_owned()))
    };
    Ok(MembershipObservation {
        epoch,
        term,
        leader,
        members,
    })
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

fn leader_index_and_id(cluster: &mut DaemonCluster, members: u32) -> TestResult<(usize, String)> {
    let statuses = cluster.wait_for_shape(members, members)?;
    let leader = statuses[0]
        .leader
        .clone()
        .ok_or("nemesis observed no leader")?;
    let index = cluster
        .node_ids()
        .iter()
        .position(|node_id| node_id == &leader)
        .ok_or("nemesis leader did not belong to DaemonCluster")?;
    Ok((index, leader))
}

#[cfg(target_os = "linux")]
fn suspend_daemon(cluster: &mut DaemonCluster, index: usize) -> TestResult {
    cluster.suspend(index)
}

#[cfg(not(target_os = "linux"))]
fn suspend_daemon(_cluster: &mut DaemonCluster, _index: usize) -> TestResult {
    Err("real pause/resume nemesis proof is supported only by the Linux daemon-process gate".into())
}

#[cfg(target_os = "linux")]
fn resume_daemon(cluster: &mut DaemonCluster, index: usize) -> TestResult {
    cluster.resume(index)
}

#[cfg(not(target_os = "linux"))]
fn resume_daemon(_cluster: &mut DaemonCluster, _index: usize) -> TestResult {
    Err("real pause/resume nemesis proof is supported only by the Linux daemon-process gate".into())
}

fn wait_for_partition_effect(
    cluster: &mut DaemonCluster,
    target_index: usize,
    members: u32,
) -> TestResult {
    let peer_indices = (0..cluster.node_ids().len())
        .filter(|index| *index != target_index)
        .collect::<Vec<_>>();
    cluster.wait_for(
        "symmetric raft partition is externally visible".to_owned(),
        |cluster| {
            let target = cluster.admin_status(target_index).ok()?;
            let peers = peer_indices
                .iter()
                .map(|index| cluster.admin_status(*index))
                .collect::<Result<Vec<_>, _>>()
                .ok()?;
            let leaders = peers
                .iter()
                .filter_map(|status| status.leader.clone())
                .collect::<BTreeSet<_>>();
            (!target.quorum_ok
                && leaders.len() == 1
                && peers.iter().all(|status| {
                    status.quorum_ok && status.members == members && status.voters == members
                }))
            .then_some(())
        },
    )
}

fn cleanup_process_replay(
    cluster: &mut DaemonCluster,
    state: &mut ProcessNemesisState,
) -> Vec<String> {
    let mut errors = Vec::new();
    let active_fault = state.active_transport_fault;
    let cleared_generation = match cluster.clear_raft_outbound_faults() {
        Ok(generation) => Some(generation),
        Err(error) => {
            errors.push(format!("clear transport faults: {error}"));
            None
        }
    };
    if let Some(index) = state.last_paused.take() {
        if let Err(error) = resume_daemon(cluster, index) {
            errors.push(format!("resume paused daemon {index}: {error}"));
        }
    }
    if let Some(index) = state.last_killed.take() {
        if let Err(error) = cluster.restart(index) {
            errors.push(format!("restart killed daemon {index}: {error}"));
        }
    }
    let expected_statuses = cluster.node_ids().len();
    let converged = match cluster.wait_for_non_draining_responsive_shape(
        "process nemesis cleanup convergence",
        expected_statuses,
        state.members,
        state.members,
    ) {
        Ok(_) => true,
        Err(error) => {
            errors.push(format!("cleanup convergence: {error}"));
            false
        }
    };
    if converged {
        if let (Some(active_fault), Some(generation)) = (active_fault, cleared_generation) {
            let action = match active_fault {
                ActiveTransportFault::Partition => "partition",
                ActiveTransportFault::Delay => "delay",
            };
            if let Err(error) = mark_fault_healed(state, action, generation) {
                errors.push(format!("record {action} cleanup: {error}"));
            }
        }
    }
    state.active_transport_fault = None;
    errors
}

fn mark_fault_healed(
    state: &mut ProcessNemesisState,
    action: &str,
    cleared_generation: u64,
) -> TestResult {
    let proof = state
        .fault_proofs
        .iter_mut()
        .rev()
        .find(|proof| proof.action == action && !proof.healed)
        .ok_or_else(|| format!("no active {action} proof to mark healed"))?;
    proof.healed = true;
    proof.cleared_generation = Some(cleared_generation);
    Ok(())
}

fn panic_payload_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| {
            payload
                .downcast_ref::<&str>()
                .map(|value| (*value).to_owned())
        })
        .unwrap_or_else(|| "non-string panic payload".to_owned())
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
        ExternalHistoryAction::PauseLeader,
        ExternalHistoryAction::ResumeLastPaused,
        ExternalHistoryAction::PartitionFollower,
        ExternalHistoryAction::HealLastPartition,
        ExternalHistoryAction::DelayTransport,
        ExternalHistoryAction::ClearTransportDelay,
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
    for (start, finish) in [
        (
            ExternalHistoryAction::PauseLeader,
            ExternalHistoryAction::ResumeLastPaused,
        ),
        (
            ExternalHistoryAction::PartitionFollower,
            ExternalHistoryAction::HealLastPartition,
        ),
        (
            ExternalHistoryAction::DelayTransport,
            ExternalHistoryAction::ClearTransportDelay,
        ),
    ] {
        assert!(schedule
            .operations
            .iter()
            .any(|operation| operation.contains(start) && operation.contains(finish)));
    }
    schedule
        .validate_dependency_groups()
        .expect("generated fault groups must be dependency-safe");
}

fn assert_stable_operation_evidence(
    schedule: &ExternalNemesisSchedule,
    trace: &ExternalNemesisRecorder,
) {
    assert_eq!(trace.events().len(), schedule.operations.len() * 2);
    assert_eq!(
        trace.response_counts().len(),
        schedule
            .operations
            .iter()
            .map(|operation| operation.actions.len() + 2)
            .sum::<usize>()
    );
    assert!(trace.response_counts().iter().all(|count| {
        count.expected_admin_responses == count.observed_admin_responses
            && count.expected_overview_responses == count.observed_overview_responses
    }));
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
