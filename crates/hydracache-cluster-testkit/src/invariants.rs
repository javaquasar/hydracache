use std::collections::{BTreeMap, BTreeSet};

use hydracache_cluster_raft::RaftRuntimeRole;

use crate::RuntimeRaftCluster;

/// Stable view consumed by the shared cluster invariant catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterInvariantView {
    /// Raft leaders grouped by term.
    pub leaders_by_term: BTreeMap<u64, Vec<u64>>,
    /// Raft ConfState voters observed by each node.
    pub voter_sets_by_node: BTreeMap<u64, BTreeSet<u64>>,
    /// Materialized HydraCache member ids observed by each node.
    pub member_sets_by_node: BTreeMap<u64, BTreeSet<String>>,
    /// Command ids that are considered committed for the settled view.
    pub committed_command_ids: BTreeSet<String>,
    /// Command ids materialized by each node.
    pub applied_command_ids_by_node: BTreeMap<u64, BTreeSet<String>>,
}

impl ClusterInvariantView {
    /// Build an invariant view from the in-process Raft runtime cluster.
    pub fn from_runtime_raft_cluster(cluster: &RuntimeRaftCluster) -> Self {
        let mut leaders_by_term = BTreeMap::<u64, Vec<u64>>::new();
        let mut voter_sets_by_node: BTreeMap<u64, BTreeSet<u64>> = BTreeMap::new();
        let mut member_sets_by_node = BTreeMap::new();
        let mut applied_command_ids_by_node = BTreeMap::new();

        for node_id in cluster.node_ids() {
            let node = cluster.node(node_id);
            let snapshot = node.snapshot();
            if snapshot.role == RaftRuntimeRole::Leader {
                leaders_by_term
                    .entry(snapshot.term)
                    .or_default()
                    .push(node_id);
            }
            voter_sets_by_node.insert(
                node_id,
                node.voter_ids()
                    .expect("testkit node should expose voters")
                    .into_iter()
                    .collect(),
            );
            member_sets_by_node.insert(
                node_id,
                node.members()
                    .into_iter()
                    .map(|member| member.node_id.as_str().to_owned())
                    .collect(),
            );
            let applied = node
                .export_snapshot()
                .commands
                .into_iter()
                .map(|command| command.command_id)
                .collect::<BTreeSet<_>>();
            applied_command_ids_by_node.insert(node_id, applied);
        }

        let active_voters: BTreeSet<u64> = voter_sets_by_node
            .values()
            .next()
            .cloned()
            .unwrap_or_default();
        let committed_command_ids = applied_command_ids_by_node
            .iter()
            .filter(|(node_id, _)| active_voters.contains(node_id))
            .flat_map(|(_, applied)| applied.iter().cloned())
            .collect();

        Self {
            leaders_by_term,
            voter_sets_by_node,
            member_sets_by_node,
            committed_command_ids,
            applied_command_ids_by_node,
        }
    }
}

/// Assert the common settled-cluster invariants shared by Raft proof tests.
pub fn assert_cluster_invariants(view: &ClusterInvariantView) {
    let violations = cluster_invariant_violations(view);
    assert!(
        violations.is_empty(),
        "cluster invariant violation(s): {violations:?}; view={view:?}"
    );
}

/// Return every invariant violation without panicking, useful for canary tests.
pub fn cluster_invariant_violations(view: &ClusterInvariantView) -> Vec<String> {
    let mut violations = Vec::new();

    for (term, leaders) in &view.leaders_by_term {
        if leaders.len() > 1 {
            violations.push(format!("multiple leaders in term {term}: {leaders:?}"));
        }
    }

    if let Some(expected) = view.voter_sets_by_node.values().next() {
        for (node_id, voters) in &view.voter_sets_by_node {
            if voters != expected {
                violations.push(format!(
                    "voter set diverged on node {node_id}: expected {expected:?}, got {voters:?}"
                ));
            }
        }
    }

    let active_voters = view
        .voter_sets_by_node
        .values()
        .next()
        .cloned()
        .unwrap_or_default();

    if let Some((_, expected)) = view
        .member_sets_by_node
        .iter()
        .find(|(node_id, _)| active_voters.contains(node_id))
    {
        for (node_id, members) in view
            .member_sets_by_node
            .iter()
            .filter(|(node_id, _)| active_voters.contains(node_id))
        {
            if members != expected {
                violations.push(format!(
                    "member set diverged on node {node_id}: expected {expected:?}, got {members:?}"
                ));
            }
        }
    }

    for (node_id, applied) in view
        .applied_command_ids_by_node
        .iter()
        .filter(|(node_id, _)| active_voters.contains(node_id))
    {
        let missing = view
            .committed_command_ids
            .difference(applied)
            .cloned()
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            violations.push(format!(
                "node {node_id} lost committed command(s): {missing:?}"
            ));
        }
    }

    violations
}
