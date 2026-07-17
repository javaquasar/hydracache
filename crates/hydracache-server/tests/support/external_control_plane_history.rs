#![allow(dead_code)]

use std::collections::{BTreeSet, VecDeque};

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
        let membership_observations = cluster_overviews
            .iter()
            .map(MembershipObservation::from_cluster_overview)
            .collect();
        self.record_step(ExternalHistoryStep {
            action,
            admin_statuses,
            membership_observations,
        });
    }

    pub fn steps(&self) -> &[ExternalHistoryStep] {
        &self.steps
    }

    pub fn into_steps(self) -> Vec<ExternalHistoryStep> {
        self.steps
    }

    fn membership_history(&self) -> MembershipHistoryRecorder {
        let mut membership = MembershipHistoryRecorder::default();
        for observation in self
            .steps
            .iter()
            .flat_map(|step| step.membership_observations.iter())
        {
            membership.record(observation.clone());
        }
        membership
    }
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
