#[path = "support/daemon_cluster.rs"]
mod daemon_cluster;

use std::collections::BTreeMap;

use daemon_cluster::{skip_unless_daemon_process_e2e, DaemonCluster, DaemonStatus, TestResult};
use hydracache::{
    ClusterEpoch, ClusterGeneration, ClusterNodeId, ClusterRole, RaftMetadataCommand,
};
use hydracache_cluster_raft::RaftMetadataCommandEnvelope;
use hydracache_cluster_testkit::reference_model::ReferenceMetadataModel;
use serde_json::Value;

const WIDE_SEED: u64 = 0x0660_0008_0001;

#[test]
fn process_committed_metadata_matches_reference_model_wide() -> TestResult {
    if !skip_unless_daemon_process_e2e("process_committed_metadata_matches_reference_model_wide") {
        return Ok(());
    }

    let mut cluster =
        DaemonCluster::start_bootstrap_with_raft_compaction(3, "process-differential-reference")?;
    cluster.wait_for_shape(3, 3)?;
    let initial = external_view(&mut cluster)?;
    let mut model = model_from_initial_view(&initial)?;
    assert_external_matches_model(&initial, &model, "initial bootstrap")?;

    let mut state = WIDE_SEED;
    for step in 0..6 {
        let statuses = cluster.wait_for_shape(3, 3)?;
        let current_leader_index = leader_index(&cluster, &statuses)?;
        let followers = (0..cluster.node_ids().len())
            .filter(|index| *index != current_leader_index)
            .collect::<Vec<_>>();
        state = next_seed(state);
        let restart_index = followers[(state as usize) % followers.len()];
        let restarting_node = ClusterNodeId::from(cluster.node_ids()[restart_index].clone());
        let before_generation = model
            .view()
            .members
            .get(&restarting_node)
            .copied()
            .ok_or_else(|| format!("model lost scheduled node {restarting_node}"))?;

        cluster.kill(restart_index)?;
        cluster.wait_for_responsive_shape(2, 3, 3)?;
        let during_fault = external_view(&mut cluster)?;
        assert_external_matches_model(
            &during_fault,
            &model,
            &format!("step {step} follower stopped"),
        )?;

        if step % 2 == 0 {
            let live_statuses = cluster.wait_for_responsive_shape(2, 3, 3)?;
            let live_leader = leader_index(&cluster, &live_statuses)?;
            let _ = cluster.compact_raft_log(live_leader)?;
            let after_compaction = external_view(&mut cluster)?;
            assert_external_matches_model(
                &after_compaction,
                &model,
                &format!("step {step} compaction"),
            )?;
        }

        cluster.restart(restart_index)?;
        cluster.wait_for_shape(3, 3)?;
        let predicted_epoch = ClusterEpoch::new(model.view().epoch.value().saturating_add(1));
        let predicted_generation =
            ClusterGeneration::new(before_generation.value().saturating_add(1));
        apply_member_upsert(
            &mut model,
            restarting_node,
            predicted_generation,
            predicted_epoch,
        )?;
        let recovered = external_view(&mut cluster)?;
        assert_external_matches_model(
            &recovered,
            &model,
            &format!("step {step} follower restarted"),
        )?;
    }

    let statuses = cluster.wait_for_shape(3, 3)?;
    let current_leader_index = leader_index(&cluster, &statuses)?;
    let drain_index = (0..cluster.node_ids().len())
        .find(|index| *index != current_leader_index)
        .ok_or("wide differential schedule needs a follower to drain")?;
    let drained_node = ClusterNodeId::from(cluster.node_ids()[drain_index].clone());
    let _ = cluster.drain(drain_index)?;
    cluster.wait_for_non_draining_shape("differential drain commits", 2, 2)?;
    let drain_epoch = ClusterEpoch::new(model.view().epoch.value().saturating_add(1));
    apply_node_left(&mut model, drained_node, drain_epoch)?;
    let after_drain = external_view(&mut cluster)?;
    assert_external_matches_model(&after_drain, &model, "committed follower drain")?;

    cluster.kill(drain_index)?;
    cluster.restart(drain_index)?;
    cluster.wait_for_shape(2, 2)?;
    let after_removed_restart = external_view(&mut cluster)?;
    assert_external_matches_model(
        &after_removed_restart,
        &model,
        "removed process restart cannot resurrect membership",
    )?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExternalMetadataView {
    epoch: ClusterEpoch,
    members: BTreeMap<ClusterNodeId, ClusterGeneration>,
}

fn model_from_initial_view(view: &ExternalMetadataView) -> TestResult<ReferenceMetadataModel> {
    let mut model = ReferenceMetadataModel::new();
    for (node_id, generation) in &view.members {
        apply_member_upsert(&mut model, node_id.clone(), *generation, view.epoch)?;
    }
    Ok(model)
}

fn apply_member_upsert(
    model: &mut ReferenceMetadataModel,
    node_id: ClusterNodeId,
    generation: ClusterGeneration,
    epoch: ClusterEpoch,
) -> TestResult {
    let envelope = RaftMetadataCommandEnvelope {
        command_id: format!("member-upsert:{node_id}:{}", generation.value()),
        command: RaftMetadataCommand::MemberUpsert {
            node_id,
            generation,
            epoch,
        },
    };
    model.apply(&envelope).map(|_| ()).map_err(Into::into)
}

fn apply_node_left(
    model: &mut ReferenceMetadataModel,
    node_id: ClusterNodeId,
    epoch: ClusterEpoch,
) -> TestResult {
    let envelope = RaftMetadataCommandEnvelope {
        command_id: format!("node-left:{node_id}:{}", epoch.value()),
        command: RaftMetadataCommand::NodeLeft {
            node_id,
            role: ClusterRole::Member,
            epoch,
        },
    };
    model.apply(&envelope).map(|_| ()).map_err(Into::into)
}

fn assert_external_matches_model(
    external: &ExternalMetadataView,
    model: &ReferenceMetadataModel,
    stage: &str,
) -> TestResult {
    let expected = model.view();
    if external.epoch != expected.epoch || external.members != expected.members {
        return Err(format!(
            "process/reference disagreement at {stage}: external={external:?} model={expected:?}"
        )
        .into());
    }
    if !expected.clients.is_empty() {
        return Err(
            format!("process member schedule unexpectedly modeled clients at {stage}").into(),
        );
    }
    Ok(())
}

fn external_view(cluster: &mut DaemonCluster) -> TestResult<ExternalMetadataView> {
    cluster
        .overviews()
        .into_iter()
        .filter_map(|overview| parse_external_view(&overview).ok())
        .max_by_key(|view| (view.epoch.value(), view.members.len()))
        .ok_or_else(|| "no live cluster overview exposed committed metadata".into())
}

fn parse_external_view(overview: &Value) -> TestResult<ExternalMetadataView> {
    if overview.get("source").and_then(Value::as_str) != Some("live") {
        return Err("cluster overview is not live".into());
    }
    let leader = overview
        .get("leader")
        .filter(|leader| !leader.is_null())
        .ok_or("cluster overview has no authoritative leader")?;
    let epoch = leader
        .get("epoch")
        .and_then(Value::as_u64)
        .ok_or("cluster overview leader has no epoch")?;
    let members = overview
        .get("members")
        .and_then(Value::as_array)
        .ok_or("cluster overview has no members")?
        .iter()
        .map(|member| {
            let node_id = member
                .get("node_id")
                .and_then(Value::as_str)
                .ok_or("cluster overview member has no node_id")?;
            let generation = member
                .get("generation")
                .and_then(Value::as_u64)
                .ok_or("cluster overview member has no generation")?;
            Ok((
                ClusterNodeId::from(node_id),
                ClusterGeneration::new(generation),
            ))
        })
        .collect::<TestResult<BTreeMap<_, _>>>()?;
    Ok(ExternalMetadataView {
        epoch: ClusterEpoch::new(epoch),
        members,
    })
}

fn leader_index(cluster: &DaemonCluster, statuses: &[DaemonStatus]) -> TestResult<usize> {
    let leader = statuses
        .iter()
        .find_map(|status| status.leader.as_deref())
        .ok_or("cluster status did not expose a leader")?;
    cluster
        .node_ids()
        .iter()
        .position(|node_id| node_id == leader)
        .ok_or_else(|| format!("leader {leader} is not a spawned daemon").into())
}

fn next_seed(state: u64) -> u64 {
    state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407)
}
