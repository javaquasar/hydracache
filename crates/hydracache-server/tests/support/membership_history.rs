#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};

use hydracache_sim::{
    ElectionTopologyNode, ElectionTopologyState, InvariantChecker, InvariantReport, NodeFsmState,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MembershipObservation {
    pub epoch: u64,
    pub term: u64,
    pub leader: Option<String>,
    pub members: BTreeSet<String>,
}

impl MembershipObservation {
    pub fn from_cluster_overview(overview: &Value) -> Self {
        let leader = overview.get("leader").filter(|leader| !leader.is_null());
        let epoch = leader
            .and_then(|leader| leader.get("epoch"))
            .and_then(Value::as_u64)
            .unwrap_or_default();
        let term = leader
            .and_then(|leader| leader.get("term"))
            .and_then(Value::as_u64)
            .unwrap_or_default();
        let leader = leader
            .and_then(|leader| leader.get("node_id"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let members = overview
            .get("members")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|member| member.get("node_id").and_then(Value::as_str))
            .map(ToOwned::to_owned)
            .collect();
        Self {
            epoch,
            term,
            leader,
            members,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MembershipHistoryRecorder {
    observations: Vec<MembershipObservation>,
}

impl MembershipHistoryRecorder {
    pub fn record(&mut self, observation: MembershipObservation) {
        self.observations.push(observation);
    }

    pub fn record_cluster_overview(&mut self, overview: &Value) {
        self.record(MembershipObservation::from_cluster_overview(overview));
    }

    pub fn observations(&self) -> &[MembershipObservation] {
        &self.observations
    }

    pub fn check(&self) -> InvariantReport {
        MembershipHistoryChecker::default().check(self)
    }
}

#[derive(Debug, Clone, Default)]
pub struct MembershipHistoryChecker {
    invariant_checker: InvariantChecker,
}

impl MembershipHistoryChecker {
    pub fn check(&self, history: &MembershipHistoryRecorder) -> InvariantReport {
        let mut report = InvariantReport::default();
        self.check_epoch_monotonicity(history, &mut report);
        self.check_member_set_stability_per_epoch(history, &mut report);
        self.check_leader_safety_with_sim_checker(history, &mut report);
        report
    }

    fn check_epoch_monotonicity(
        &self,
        history: &MembershipHistoryRecorder,
        report: &mut InvariantReport,
    ) {
        report.record_check();
        for window in history.observations.windows(2) {
            let [previous, current] = window else {
                continue;
            };
            if current.epoch < previous.epoch {
                report.record_violation(
                    "membership_epoch_monotonicity",
                    format!(
                        "epoch regressed from {} to {}",
                        previous.epoch, current.epoch
                    ),
                );
            }
        }
    }

    fn check_member_set_stability_per_epoch(
        &self,
        history: &MembershipHistoryRecorder,
        report: &mut InvariantReport,
    ) {
        report.record_check();
        let mut members_by_epoch = BTreeMap::<u64, BTreeSet<String>>::new();
        for observation in &history.observations {
            if observation.members.is_empty() {
                continue;
            }
            match members_by_epoch.get(&observation.epoch) {
                Some(expected) if expected != &observation.members => report.record_violation(
                    "membership_set_stability_per_epoch",
                    format!(
                        "epoch {} observed member sets {:?} and {:?}",
                        observation.epoch, expected, observation.members
                    ),
                ),
                Some(_) => {}
                None => {
                    members_by_epoch.insert(observation.epoch, observation.members.clone());
                }
            }
        }
    }

    fn check_leader_safety_with_sim_checker(
        &self,
        history: &MembershipHistoryRecorder,
        report: &mut InvariantReport,
    ) {
        for topology in term_topologies(history) {
            let term_report = self.invariant_checker.check_election_topology(&topology);
            for _ in 0..term_report.checked {
                report.record_check();
            }
            for violation in term_report.violations {
                report.record_violation(violation.name, violation.message);
            }
        }
    }
}

fn term_topologies(history: &MembershipHistoryRecorder) -> Vec<ElectionTopologyState> {
    let mut leaders_by_term = BTreeMap::<u64, BTreeSet<String>>::new();
    let mut members_by_term = BTreeMap::<u64, BTreeSet<String>>::new();
    for observation in &history.observations {
        if observation.term == 0 {
            continue;
        }
        members_by_term
            .entry(observation.term)
            .or_default()
            .extend(observation.members.iter().cloned());
        if let Some(leader) = &observation.leader {
            leaders_by_term
                .entry(observation.term)
                .or_default()
                .insert(leader.clone());
        }
    }

    leaders_by_term
        .into_iter()
        .map(|(term, leaders)| {
            let mut all_nodes = members_by_term.remove(&term).unwrap_or_default();
            all_nodes.extend(leaders.iter().cloned());
            let total_nodes = all_nodes.len().max(leaders.len()).max(1);
            let quorum = total_nodes / 2 + 1;
            let mut nodes = Vec::new();
            for node_id in all_nodes {
                let is_leader = leaders.contains(&node_id);
                let role = if is_leader {
                    NodeFsmState::Leader
                } else {
                    NodeFsmState::Follower
                };
                let votes_received = if is_leader { quorum } else { 0 };
                nodes.push(
                    ElectionTopologyNode::new(node_id.clone())
                        .role(role, term)
                        .vote(node_id, votes_received),
                );
            }
            ElectionTopologyState::new(total_nodes, nodes)
        })
        .collect()
}
