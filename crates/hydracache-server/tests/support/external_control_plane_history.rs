#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use hydracache_sim::InvariantReport;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::membership_history::{MembershipHistoryRecorder, MembershipObservation};

/// One externally scheduled process/control-plane action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExternalHistoryAction {
    /// Read `/admin/status` and `/cluster/overview` without changing the cluster.
    Observe,
    /// Compact one non-leader through `/admin/raft/compaction`.
    CompactFollower,
    /// Kill the currently observed leader outside the daemon process.
    KillLeader,
    /// Restart the process most recently killed by the schedule.
    RestartLastKilled,
    /// Drain one non-leader through `/admin/drain`.
    DrainFollower,
    /// Suspend the currently observed leader with SIGSTOP.
    PauseLeader,
    /// Resume the process most recently suspended by the schedule.
    ResumeLastPaused,
    /// Isolate one follower at the loopback transport boundary.
    PartitionFollower,
    /// Remove the loopback transport isolation installed by the schedule.
    HealLastPartition,
    /// Add a bounded delay to the loopback transport.
    DelayTransport,
    /// Remove the bounded loopback transport delay installed by the schedule.
    ClearTransportDelay,
}

/// Deterministic generated action schedule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalHistorySchedule {
    pub seed: u64,
    pub actions: Vec<ExternalHistoryAction>,
}

/// Seeded generator for a safe bounded process-fault schedule.
#[derive(Debug, Clone, Copy)]
pub struct ExternalHistoryGenerator {
    state: u64,
}

impl ExternalHistoryGenerator {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn generate(mut self) -> ExternalHistorySchedule {
        let seed = self.state;
        let compact_before_leader_fault = self.next_u64() & 1 == 0;
        let mut actions = vec![ExternalHistoryAction::Observe];
        if compact_before_leader_fault {
            actions.extend([
                ExternalHistoryAction::CompactFollower,
                ExternalHistoryAction::Observe,
            ]);
        }
        actions.extend([
            ExternalHistoryAction::KillLeader,
            ExternalHistoryAction::Observe,
            ExternalHistoryAction::RestartLastKilled,
            ExternalHistoryAction::Observe,
        ]);
        if !compact_before_leader_fault {
            actions.extend([
                ExternalHistoryAction::CompactFollower,
                ExternalHistoryAction::Observe,
            ]);
        }
        actions.extend([
            ExternalHistoryAction::DrainFollower,
            ExternalHistoryAction::Observe,
        ]);
        ExternalHistorySchedule { seed, actions }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.state
    }
}

/// Explicit deterministic schedule cursor shared by model and process runners.
#[derive(Debug, Clone)]
pub struct ExternalHistoryScheduler {
    pending: VecDeque<ExternalHistoryAction>,
}

impl ExternalHistoryScheduler {
    pub fn new(schedule: &ExternalHistorySchedule) -> Self {
        Self {
            pending: schedule.actions.iter().copied().collect(),
        }
    }

    pub fn next_action(&mut self) -> Option<ExternalHistoryAction> {
        self.pending.pop_front()
    }

    pub fn remaining(&self) -> usize {
        self.pending.len()
    }
}

/// One stable-ID composed nemesis operation built from the W7 action vocabulary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalNemesisOperation {
    pub operation_id: String,
    /// Stable external correlation id. Current daemon admin APIs do not accept
    /// caller-supplied command ids, so this is evidence correlation, not an
    /// idempotency claim about the server.
    pub command_id: String,
    pub invoke_observation_id: String,
    pub complete_observation_id: String,
    pub actions: Vec<ExternalHistoryAction>,
}

impl ExternalNemesisOperation {
    pub fn contains(&self, action: ExternalHistoryAction) -> bool {
        self.actions.contains(&action)
    }
}

/// Stable replay schedule for composed process faults.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalNemesisSchedule {
    pub seed: u64,
    pub operations: Vec<ExternalNemesisOperation>,
}

impl ExternalNemesisSchedule {
    /// Reject schedules whose cleanup depends on a different operation. Whole-
    /// operation shrinking can therefore never strand an external fault.
    pub fn validate_dependency_groups(&self) -> Result<(), String> {
        let mut saw_drain = false;
        for operation in &self.operations {
            if saw_drain {
                return Err(format!(
                    "operation {} appears after the terminal drain group",
                    operation.operation_id
                ));
            }
            let mut killed = false;
            let mut paused = false;
            let mut partitioned = false;
            let mut delayed = false;
            for action in &operation.actions {
                match action {
                    ExternalHistoryAction::KillLeader
                        if !killed && !paused && !partitioned && !delayed =>
                    {
                        killed = true;
                    }
                    ExternalHistoryAction::RestartLastKilled if killed => killed = false,
                    ExternalHistoryAction::PauseLeader
                        if !killed && !paused && !partitioned && !delayed =>
                    {
                        paused = true;
                    }
                    ExternalHistoryAction::ResumeLastPaused if paused => paused = false,
                    ExternalHistoryAction::PartitionFollower
                        if !killed && !paused && !partitioned && !delayed =>
                    {
                        partitioned = true;
                    }
                    ExternalHistoryAction::HealLastPartition if partitioned => {
                        partitioned = false;
                    }
                    ExternalHistoryAction::DelayTransport
                        if !killed && !paused && !partitioned && !delayed =>
                    {
                        delayed = true;
                    }
                    ExternalHistoryAction::ClearTransportDelay if delayed => delayed = false,
                    ExternalHistoryAction::DrainFollower
                        if !killed && !paused && !partitioned && !delayed =>
                    {
                        saw_drain = true;
                    }
                    ExternalHistoryAction::Observe | ExternalHistoryAction::CompactFollower => {}
                    action => {
                        return Err(format!(
                            "operation {} has an invalid dependency transition at {action:?}",
                            operation.operation_id
                        ));
                    }
                }
            }
            if killed || paused || partitioned || delayed {
                return Err(format!(
                    "operation {} leaves an external fault active",
                    operation.operation_id
                ));
            }
        }
        Ok(())
    }
}

/// W2 generator layered on the W7 seeded action generator.
#[derive(Debug, Clone, Copy)]
pub struct ExternalNemesisGenerator {
    seed: u64,
}

impl ExternalNemesisGenerator {
    pub fn new(seed: u64) -> Self {
        Self { seed }
    }

    pub fn generate(self) -> ExternalNemesisSchedule {
        let mut groups = vec![
            vec![
                ExternalHistoryAction::CompactFollower,
                ExternalHistoryAction::Observe,
            ],
            vec![
                ExternalHistoryAction::KillLeader,
                ExternalHistoryAction::Observe,
                ExternalHistoryAction::RestartLastKilled,
                ExternalHistoryAction::Observe,
            ],
            vec![
                ExternalHistoryAction::PauseLeader,
                ExternalHistoryAction::Observe,
                ExternalHistoryAction::ResumeLastPaused,
                ExternalHistoryAction::Observe,
            ],
            vec![
                ExternalHistoryAction::PartitionFollower,
                ExternalHistoryAction::Observe,
                ExternalHistoryAction::HealLastPartition,
                ExternalHistoryAction::Observe,
            ],
            vec![
                ExternalHistoryAction::DelayTransport,
                ExternalHistoryAction::Observe,
                ExternalHistoryAction::ClearTransportDelay,
                ExternalHistoryAction::Observe,
            ],
        ];
        let mut state = self.seed;
        for index in (1..groups.len()).rev() {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            groups.swap(index, (state as usize) % (index + 1));
        }
        groups.push(vec![
            ExternalHistoryAction::DrainFollower,
            ExternalHistoryAction::Observe,
        ]);
        let operations = groups
            .into_iter()
            .enumerate()
            .map(|(index, actions)| {
                let prefix = format!("{:016x}-{index:04}", self.seed);
                ExternalNemesisOperation {
                    operation_id: format!("op-{prefix}"),
                    command_id: format!("cmd-{prefix}"),
                    invoke_observation_id: format!("obs-{prefix}-invoke"),
                    complete_observation_id: format!("obs-{prefix}-complete"),
                    actions,
                }
            })
            .collect();
        let schedule = ExternalNemesisSchedule {
            seed: self.seed,
            operations,
        };
        schedule
            .validate_dependency_groups()
            .expect("built-in W2 schedule must be dependency-safe");
        schedule
    }
}

/// Projection of the authenticated public `/admin/status` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalAdminObservation {
    pub leader: Option<String>,
    pub term: u64,
    pub members: u32,
    pub voters: u32,
    pub quorum_ok: bool,
    pub draining: bool,
}

/// One atomic schedule step and all public observations recorded after it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalHistoryStep {
    pub action: ExternalHistoryAction,
    #[serde(default)]
    pub admin_statuses: Vec<ExternalAdminObservation>,
    #[serde(default)]
    pub membership_observations: Vec<MembershipObservation>,
}

impl ExternalHistoryStep {
    pub fn from_public_surfaces(
        action: ExternalHistoryAction,
        admin_statuses: Vec<ExternalAdminObservation>,
        cluster_overviews: &[Value],
    ) -> Self {
        Self {
            action,
            admin_statuses,
            membership_observations: cluster_overviews
                .iter()
                .map(MembershipObservation::from_cluster_overview)
                .collect(),
        }
    }
}

/// Append-only external history recorder.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExternalHistoryRecorder {
    steps: Vec<ExternalHistoryStep>,
}

impl ExternalHistoryRecorder {
    pub fn from_steps(steps: Vec<ExternalHistoryStep>) -> Self {
        Self { steps }
    }

    pub fn record_step(&mut self, step: ExternalHistoryStep) {
        self.steps.push(step);
    }

    pub fn record_public_surfaces(
        &mut self,
        action: ExternalHistoryAction,
        admin_statuses: Vec<ExternalAdminObservation>,
        cluster_overviews: &[Value],
    ) {
        self.record_step(ExternalHistoryStep::from_public_surfaces(
            action,
            admin_statuses,
            cluster_overviews,
        ));
    }

    pub fn steps(&self) -> &[ExternalHistoryStep] {
        &self.steps
    }

    pub fn into_steps(self) -> Vec<ExternalHistoryStep> {
        self.steps
    }

    fn membership_history(&self) -> MembershipHistoryRecorder {
        let mut membership = MembershipHistoryRecorder::default();
        for step in &self.steps {
            if !step.admin_statuses.is_empty()
                && step.admin_statuses.len() == step.membership_observations.len()
            {
                for (_, observation) in step
                    .admin_statuses
                    .iter()
                    .zip(step.membership_observations.iter())
                    .filter(|(status, _)| status.quorum_ok && !status.draining)
                {
                    membership.record(observation.clone());
                }
            } else {
                for observation in &step.membership_observations {
                    membership.record(observation.clone());
                }
            }
        }
        membership
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalNemesisPhase {
    Invoke,
    Complete,
}

/// Stable-ID invoke/complete evidence projected from public control-plane views.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalNemesisEvent {
    pub operation_id: String,
    pub command_id: String,
    pub observation_id: String,
    pub phase: ExternalNemesisPhase,
    pub actions: Vec<ExternalHistoryAction>,
    pub committed_epoch: Option<u64>,
    pub public_membership: Option<BTreeSet<String>>,
    pub expected_admin_responses: usize,
    pub observed_admin_responses: usize,
    pub expected_overview_responses: usize,
    pub observed_overview_responses: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalPublicResponseCount {
    pub action: ExternalHistoryAction,
    pub expected_admin_responses: usize,
    pub observed_admin_responses: usize,
    pub expected_overview_responses: usize,
    pub observed_overview_responses: usize,
}

/// W2 recorder that retains W7 black-box history plus operation boundaries.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExternalNemesisRecorder {
    events: Vec<ExternalNemesisEvent>,
    history: ExternalHistoryRecorder,
    response_counts: Vec<ExternalPublicResponseCount>,
}

impl ExternalNemesisRecorder {
    pub fn record_phase(
        &mut self,
        operation: &ExternalNemesisOperation,
        phase: ExternalNemesisPhase,
        step: ExternalHistoryStep,
        expected_responses: usize,
    ) {
        self.response_counts.push(ExternalPublicResponseCount {
            action: step.action,
            expected_admin_responses: expected_responses,
            observed_admin_responses: step.admin_statuses.len(),
            expected_overview_responses: expected_responses,
            observed_overview_responses: step.membership_observations.len(),
        });
        let authoritative = step
            .admin_statuses
            .iter()
            .zip(step.membership_observations.iter())
            .filter(|(status, observation)| {
                status.quorum_ok && !status.draining && !observation.members.is_empty()
            })
            .map(|(_, observation)| observation)
            .collect::<Vec<_>>();
        let committed_epoch =
            unique_value(authoritative.iter().map(|observation| observation.epoch));
        let public_membership = unique_value(
            authoritative
                .iter()
                .map(|observation| observation.members.clone()),
        );
        let observation_id = match phase {
            ExternalNemesisPhase::Invoke => operation.invoke_observation_id.clone(),
            ExternalNemesisPhase::Complete => operation.complete_observation_id.clone(),
        };
        self.events.push(ExternalNemesisEvent {
            operation_id: operation.operation_id.clone(),
            command_id: operation.command_id.clone(),
            observation_id,
            phase,
            actions: operation.actions.clone(),
            committed_epoch,
            public_membership,
            expected_admin_responses: expected_responses,
            observed_admin_responses: step.admin_statuses.len(),
            expected_overview_responses: expected_responses,
            observed_overview_responses: step.membership_observations.len(),
        });
        self.history.record_step(step);
    }

    pub fn record_intermediate(&mut self, step: ExternalHistoryStep, expected_responses: usize) {
        self.response_counts.push(ExternalPublicResponseCount {
            action: step.action,
            expected_admin_responses: expected_responses,
            observed_admin_responses: step.admin_statuses.len(),
            expected_overview_responses: expected_responses,
            observed_overview_responses: step.membership_observations.len(),
        });
        self.history.record_step(step);
    }

    pub fn events(&self) -> &[ExternalNemesisEvent] {
        &self.events
    }

    pub fn history(&self) -> &ExternalHistoryRecorder {
        &self.history
    }

    pub fn response_counts(&self) -> &[ExternalPublicResponseCount] {
        &self.response_counts
    }
}

/// Serializable original/minimized failure evidence emitted by fast or process
/// nemesis runners. Process callers attach preserved daemon log paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalNemesisFailureArtifact {
    pub schema_version: u32,
    pub seed: u64,
    pub original_schedule: ExternalNemesisSchedule,
    pub minimized_schedule: ExternalNemesisSchedule,
    pub events: Vec<ExternalNemesisEvent>,
    pub observed_history: Vec<ExternalHistoryStep>,
    pub violations: Vec<String>,
    pub daemon_logs: Vec<String>,
}

impl ExternalNemesisFailureArtifact {
    pub fn new(
        original_schedule: ExternalNemesisSchedule,
        minimized_schedule: ExternalNemesisSchedule,
        trace: &ExternalNemesisRecorder,
        report: &InvariantReport,
        daemon_logs: Vec<String>,
    ) -> Self {
        Self {
            schema_version: 1,
            seed: original_schedule.seed,
            original_schedule,
            minimized_schedule,
            events: trace.events.clone(),
            observed_history: trace.history.steps.clone(),
            violations: report.violations.iter().map(ToString::to_string).collect(),
            daemon_logs,
        }
    }

    pub fn encode_pretty_json(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec_pretty(self)
    }
}

fn unique_value<T>(values: impl IntoIterator<Item = T>) -> Option<T>
where
    T: Ord,
{
    let mut unique = values.into_iter().collect::<BTreeSet<_>>();
    (unique.len() == 1).then(|| unique.pop_first().expect("length checked"))
}

/// Offline checker over external admin and overview observations only.
#[derive(Debug, Clone, Default)]
pub struct ExternalHistoryChecker;

impl ExternalHistoryChecker {
    pub fn check(&self, history: &ExternalHistoryRecorder) -> InvariantReport {
        let membership_report = history.membership_history().check();
        let mut report = InvariantReport {
            checked: membership_report.checked,
            violations: membership_report.violations,
        };
        for (step_index, step) in history.steps.iter().enumerate() {
            self.check_admin_agreement(step_index, step, &mut report);
            self.check_admin_membership_projection(step_index, step, &mut report);
        }
        report
    }

    fn check_admin_agreement(
        &self,
        step_index: usize,
        step: &ExternalHistoryStep,
        report: &mut InvariantReport,
    ) {
        report.record_check();
        let authoritative = step
            .admin_statuses
            .iter()
            .filter(|status| status.quorum_ok && !status.draining)
            .collect::<Vec<_>>();
        let leaders = authoritative
            .iter()
            .filter_map(|status| status.leader.clone())
            .collect::<BTreeSet<_>>();
        if leaders.len() > 1 {
            report.record_violation(
                "external_admin_single_leader",
                format!(
                    "step {step_index} {:?} reported authoritative leaders {leaders:?}",
                    step.action
                ),
            );
        }
        let shapes = authoritative
            .iter()
            .map(|status| (status.term, status.members, status.voters))
            .collect::<BTreeSet<_>>();
        if shapes.len() > 1 {
            report.record_violation(
                "external_admin_committed_shape_agreement",
                format!(
                    "step {step_index} {:?} reported committed shapes {shapes:?}",
                    step.action
                ),
            );
        }
    }

    fn check_admin_membership_projection(
        &self,
        step_index: usize,
        step: &ExternalHistoryStep,
        report: &mut InvariantReport,
    ) {
        report.record_check();
        let overview_shapes = step
            .membership_observations
            .iter()
            .filter(|observation| !observation.members.is_empty())
            .map(|observation| observation.members.len() as u32)
            .collect::<BTreeSet<_>>();
        if overview_shapes.len() > 1 {
            report.record_violation(
                "external_overview_membership_agreement",
                format!(
                    "step {step_index} {:?} reported overview member counts {overview_shapes:?}",
                    step.action
                ),
            );
        }
        let Some(overview_members) = step
            .membership_observations
            .iter()
            .find(|observation| !observation.members.is_empty())
            .map(|observation| &observation.members)
        else {
            return;
        };
        for status in step
            .admin_statuses
            .iter()
            .filter(|status| status.quorum_ok && !status.draining)
        {
            if status.members != overview_members.len() as u32 {
                report.record_violation(
                    "external_admin_overview_member_count",
                    format!(
                        "step {step_index} {:?} admin members={} but overview members={}",
                        step.action,
                        status.members,
                        overview_members.len()
                    ),
                );
            }
            if let Some(leader) = &status.leader {
                if !overview_members.contains(leader) {
                    report.record_violation(
                        "external_leader_belongs_to_membership",
                        format!(
                            "step {step_index} {:?} leader {leader} is absent from {overview_members:?}",
                            step.action
                        ),
                    );
                }
            }
        }
    }
}

/// W2 checker for stable invoke/complete operation evidence layered on W7.
#[derive(Debug, Clone, Default)]
pub struct ExternalNemesisChecker {
    external: ExternalHistoryChecker,
}

impl ExternalNemesisChecker {
    pub fn check(&self, trace: &ExternalNemesisRecorder) -> InvariantReport {
        let external = self.external.check(trace.history());
        let mut report = InvariantReport {
            checked: external.checked,
            violations: external.violations,
        };
        let mut by_operation = BTreeMap::<String, Vec<(usize, &ExternalNemesisEvent)>>::new();
        let mut operation_ids = BTreeSet::new();
        let mut command_ids = BTreeSet::new();
        let mut observation_ids = BTreeSet::new();
        let mut previous_epoch = None;
        let mut previous_membership: Option<BTreeSet<String>> = None;

        let invokes = trace
            .events
            .iter()
            .filter(|event| event.phase == ExternalNemesisPhase::Invoke)
            .collect::<Vec<_>>();
        let expected_history_actions = invokes
            .iter()
            .flat_map(|event| {
                let complete_action = event
                    .actions
                    .last()
                    .copied()
                    .unwrap_or(ExternalHistoryAction::Observe);
                std::iter::once(ExternalHistoryAction::Observe)
                    .chain(event.actions.iter().copied())
                    .chain(std::iter::once(complete_action))
            })
            .collect::<Vec<_>>();
        let observed_history_actions = trace
            .history
            .steps
            .iter()
            .map(|step| step.action)
            .collect::<Vec<_>>();
        report.record_check();
        if observed_history_actions != expected_history_actions {
            report.record_violation(
                "nemesis_exact_action_response_count",
                format!(
                    "expected action observations {expected_history_actions:?}, observed {observed_history_actions:?}"
                ),
            );
        }
        report.record_check();
        let counted_actions = trace
            .response_counts
            .iter()
            .map(|count| count.action)
            .collect::<Vec<_>>();
        if counted_actions != expected_history_actions
            || trace.response_counts.iter().any(|count| {
                count.observed_admin_responses != count.expected_admin_responses
                    || count.observed_overview_responses != count.expected_overview_responses
            })
        {
            report.record_violation(
                "nemesis_exact_public_response_count",
                format!(
                    "expected response actions {expected_history_actions:?}, observed counts {:?}",
                    trace.response_counts
                ),
            );
        }

        for (index, event) in trace.events.iter().enumerate() {
            report.record_check();
            by_operation
                .entry(event.operation_id.clone())
                .or_default()
                .push((index, event));
            operation_ids.insert(event.operation_id.clone());
            command_ids.insert(event.command_id.clone());
            if !observation_ids.insert(event.observation_id.clone()) {
                report.record_violation(
                    "nemesis_stable_observation_id_unique",
                    format!("duplicate observation id {}", event.observation_id),
                );
            }
            if event.observed_admin_responses != event.expected_admin_responses
                || event.observed_overview_responses != event.expected_overview_responses
            {
                report.record_violation(
                    "nemesis_exact_public_response_count",
                    format!(
                        "{} expected {} admin/overview responses but observed admin={} overview={}",
                        event.observation_id,
                        event.expected_admin_responses,
                        event.observed_admin_responses,
                        event.observed_overview_responses
                    ),
                );
            }
            let Some(epoch) = event.committed_epoch else {
                report.record_violation(
                    "nemesis_public_committed_epoch_present",
                    format!(
                        "{} had no unique public committed epoch",
                        event.observation_id
                    ),
                );
                continue;
            };
            if previous_epoch.is_some_and(|previous| epoch < previous) {
                report.record_violation(
                    "nemesis_committed_epoch_monotonicity",
                    format!(
                        "{} regressed committed epoch from {} to {epoch}",
                        event.observation_id,
                        previous_epoch.expect("checked as some")
                    ),
                );
            }
            previous_epoch = Some(epoch);
            let Some(membership) = &event.public_membership else {
                report.record_violation(
                    "nemesis_public_membership_present",
                    format!("{} had no unique public membership", event.observation_id),
                );
                continue;
            };
            if let Some(previous) = &previous_membership {
                if !membership.is_subset(previous) {
                    report.record_violation(
                        "nemesis_committed_membership_never_resurrects",
                        format!(
                            "{} resurrected membership from {previous:?} to {membership:?}",
                            event.observation_id
                        ),
                    );
                }
            }
            previous_membership = Some(membership.clone());
        }

        report.record_check();
        if command_ids.len() != operation_ids.len() {
            report.record_violation(
                "nemesis_stable_command_id_unique",
                format!(
                    "operation ids={} but command ids={}",
                    operation_ids.len(),
                    command_ids.len()
                ),
            );
        }

        for (operation_id, events) in by_operation {
            report.record_check();
            let invokes = events
                .iter()
                .filter(|(_, event)| event.phase == ExternalNemesisPhase::Invoke)
                .collect::<Vec<_>>();
            let completes = events
                .iter()
                .filter(|(_, event)| event.phase == ExternalNemesisPhase::Complete)
                .collect::<Vec<_>>();
            if invokes.len() != 1 || completes.len() != 1 || invokes[0].0 >= completes[0].0 {
                report.record_violation(
                    "nemesis_invoke_complete_pair",
                    format!(
                        "operation {operation_id} had invokes={} completes={} events={events:?}",
                        invokes.len(),
                        completes.len()
                    ),
                );
                continue;
            }
            let invoke = invokes[0].1;
            let complete = completes[0].1;
            if invoke.command_id != complete.command_id || invoke.actions != complete.actions {
                report.record_violation(
                    "nemesis_operation_identity_stable",
                    format!(
                        "operation {operation_id} changed identity between invoke and complete"
                    ),
                );
            }
            let contains_drain = invoke
                .actions
                .contains(&ExternalHistoryAction::DrainFollower);
            match (
                invoke.committed_epoch,
                complete.committed_epoch,
                invoke.public_membership.as_ref(),
                complete.public_membership.as_ref(),
            ) {
                (Some(before_epoch), Some(after_epoch), Some(before), Some(after))
                    if contains_drain =>
                {
                    if after_epoch <= before_epoch
                        || before.len() != after.len().saturating_add(1)
                        || !after.is_subset(before)
                    {
                        report.record_violation(
                            "nemesis_drain_commit_visible",
                            format!(
                                "operation {operation_id} did not expose one committed membership removal: epoch {before_epoch}->{after_epoch}, members {before:?}->{after:?}"
                            ),
                        );
                    }
                }
                (Some(before_epoch), Some(after_epoch), Some(before), Some(after)) => {
                    if after_epoch < before_epoch || before != after {
                        report.record_violation(
                            "nemesis_non_membership_fault_preserves_commit",
                            format!(
                                "operation {operation_id} changed committed membership during non-membership faults: epoch {before_epoch}->{after_epoch}, members {before:?}->{after:?}"
                            ),
                        );
                    }
                }
                _ => {}
            }
        }
        report
    }
}

/// Deterministic failing-schedule reducer. It removes whole atomic steps so a
/// batch of simultaneous black-box endpoint observations stays indivisible.
#[derive(Debug, Clone, Copy, Default)]
pub struct ExternalHistoryShrinker;

impl ExternalHistoryShrinker {
    pub fn shrink<F>(
        &self,
        history: &ExternalHistoryRecorder,
        failure_persists: F,
    ) -> ExternalHistoryRecorder
    where
        F: Fn(&ExternalHistoryRecorder) -> bool,
    {
        let mut current = history.clone();
        loop {
            let mut reduced = None;
            for index in 0..current.steps.len() {
                let mut candidate_steps = current.steps.clone();
                candidate_steps.remove(index);
                if candidate_steps.is_empty() {
                    continue;
                }
                let candidate = ExternalHistoryRecorder::from_steps(candidate_steps);
                if failure_persists(&candidate) {
                    reduced = Some(candidate);
                    break;
                }
            }
            match reduced {
                Some(candidate) => current = candidate,
                None => return current,
            }
        }
    }

    pub fn shrink_nemesis_schedule<F>(
        &self,
        schedule: &ExternalNemesisSchedule,
        failure_persists: F,
    ) -> ExternalNemesisSchedule
    where
        F: Fn(&ExternalNemesisSchedule) -> bool,
    {
        schedule
            .validate_dependency_groups()
            .expect("nemesis shrink input must have dependency-safe atomic groups");
        let mut current = schedule.clone();
        loop {
            let mut reduced = None;
            for index in 0..current.operations.len() {
                let mut candidate = current.clone();
                candidate.operations.remove(index);
                if candidate.operations.is_empty() {
                    continue;
                }
                if candidate.validate_dependency_groups().is_err() {
                    continue;
                }
                if failure_persists(&candidate) {
                    reduced = Some(candidate);
                    break;
                }
            }
            match reduced {
                Some(candidate) => current = candidate,
                None => return current,
            }
        }
    }
}

/// Committed bad-seed replay corpus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrozenHistoryCorpus {
    pub schema_version: u32,
    pub cases: Vec<FrozenHistoryCase>,
}

impl FrozenHistoryCorpus {
    pub fn parse(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrozenHistoryCase {
    pub name: String,
    pub seed: u64,
    pub steps: Vec<ExternalHistoryStep>,
    pub expected_violations: Vec<String>,
}

impl FrozenHistoryCase {
    pub fn recorder(&self) -> ExternalHistoryRecorder {
        ExternalHistoryRecorder::from_steps(self.steps.clone())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FrozenNemesisDefect {
    LoseCommittedDrain,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrozenNemesisCorpus {
    pub schema_version: u32,
    pub cases: Vec<FrozenNemesisCase>,
}

impl FrozenNemesisCorpus {
    pub fn parse(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrozenNemesisCase {
    pub name: String,
    pub schedule: ExternalNemesisSchedule,
    pub defect: FrozenNemesisDefect,
    pub expected_violations: Vec<String>,
}
