use std::collections::BTreeMap;

use hydracache::{
    ClusterCandidate, ClusterControlPlane, ClusterGeneration, ClusterNodeId, ClusterRole,
};
use hydracache_cluster_raft::{RaftMetadataCommandEnvelope, RaftMetadataRuntime};
use hydracache_cluster_testkit::{
    reference_model::{ReferenceMetadataIntent, ReferenceMetadataModel, ReferenceMetadataView},
    RuntimeRaftCluster,
};

#[tokio::test]
async fn runtime_committed_metadata_matches_reference_model() {
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    let mut model = ReferenceMetadataModel::new();
    let schedule = seeded_schedule(0x0660_0008);
    let fault_at = schedule.len() / 2;
    let mut expected_envelopes = Vec::new();

    for operation in &schedule[..fault_at] {
        let expected = model.apply_intent(operation).unwrap();
        apply_cluster_intent(&mut cluster, 1, operation, &expected).await;
        expected_envelopes.push(expected);
        assert_cluster_matches_model(&cluster, [1, 2, 3], &model, &expected_envelopes);
    }

    cluster.filters().isolate(3, [1, 2, 3]);
    for operation in &schedule[fault_at..] {
        let expected = model.apply_intent(operation).unwrap();
        apply_cluster_intent(&mut cluster, 1, operation, &expected).await;
        expected_envelopes.push(expected);
        assert_cluster_matches_model(&cluster, [1, 2], &model, &expected_envelopes);
    }
    assert!(
        cluster.node(3).command_envelopes().len() < expected_envelopes.len(),
        "partitioned follower must be measurably stale before differential recovery"
    );

    cluster.filters().recover();
    cluster.tick_all(12);
    assert_cluster_matches_model(&cluster, [1, 2, 3], &model, &expected_envelopes);
}

#[tokio::test]
async fn prefix_replay_reorder_and_snapshot_tail_relations_hold() {
    let full = RaftMetadataRuntime::single_node("differential-prefix", 1).unwrap();
    apply_single_runtime_intent(&full, &ReferenceMetadataIntent::member("member-a")).await;
    apply_single_runtime_intent(&full, &ReferenceMetadataIntent::client("client-a")).await;
    let prefix = full.export_snapshot();

    let tail = [
        ReferenceMetadataIntent::member("member-b"),
        ReferenceMetadataIntent::leave("client-a"),
    ];
    for operation in &tail {
        apply_single_runtime_intent(&full, operation).await;
    }

    let recovered = RaftMetadataRuntime::from_snapshot(prefix).unwrap();
    for operation in &tail {
        apply_single_runtime_intent(&recovered, operation).await;
    }
    assert_eq!(runtime_view(&full), runtime_view(&recovered));

    let member_then_client = RaftMetadataRuntime::single_node("differential-reorder", 1).unwrap();
    apply_single_runtime_intent(
        &member_then_client,
        &ReferenceMetadataIntent::member("member-independent"),
    )
    .await;
    apply_single_runtime_intent(
        &member_then_client,
        &ReferenceMetadataIntent::client("client-independent"),
    )
    .await;

    let client_then_member = RaftMetadataRuntime::single_node("differential-reorder", 1).unwrap();
    apply_single_runtime_intent(
        &client_then_member,
        &ReferenceMetadataIntent::client("client-independent"),
    )
    .await;
    apply_single_runtime_intent(
        &client_then_member,
        &ReferenceMetadataIntent::member("member-independent"),
    )
    .await;

    assert_eq!(
        materialized_view(&member_then_client),
        materialized_view(&client_then_member),
        "independent member/client admissions must commute at the materialized boundary"
    );
}

#[tokio::test]
async fn canary_reference_model_misses_a_committed_metadata_command() {
    let runtime = RaftMetadataRuntime::single_node("differential-canary", 1).unwrap();
    let first = ReferenceMetadataIntent::member("member-a");
    let omitted = ReferenceMetadataIntent::member("member-b");
    apply_single_runtime_intent(&runtime, &first).await;
    apply_single_runtime_intent(&runtime, &omitted).await;

    let mut incomplete_model = ReferenceMetadataModel::new();
    incomplete_model.apply_intent(&first).unwrap();
    let disagreement = runtime_view(&runtime) != incomplete_model.view();

    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W8") {
        assert!(
            !disagreement,
            "HC-CANARY-RED:W8 reference model omitted a committed metadata command"
        );
    }
    assert!(
        disagreement,
        "canary fixture must expose an omitted committed command"
    );
}

fn seeded_schedule(seed: u64) -> Vec<ReferenceMetadataIntent> {
    let mut joins = vec![
        ReferenceMetadataIntent::member("member-a"),
        ReferenceMetadataIntent::member("member-b"),
        ReferenceMetadataIntent::member("member-c"),
        ReferenceMetadataIntent::client("client-a"),
        ReferenceMetadataIntent::client("client-b"),
    ];
    deterministic_shuffle(&mut joins, seed);
    let mut leaves = vec![
        ReferenceMetadataIntent::leave("member-b"),
        ReferenceMetadataIntent::leave("client-a"),
    ];
    deterministic_shuffle(&mut leaves, seed.rotate_left(17));
    joins.extend(leaves);
    joins
}

async fn apply_cluster_intent(
    cluster: &mut RuntimeRaftCluster,
    leader_id: u64,
    intent: &ReferenceMetadataIntent,
    expected: &RaftMetadataCommandEnvelope,
) {
    let leader = cluster.node(leader_id);
    let pending = tokio::spawn({
        let leader = leader.clone();
        let intent = intent.clone();
        async move { apply_single_runtime_intent(&leader, &intent).await }
    });
    for _ in 0..200 {
        cluster.drain_until_idle(leader.take_outbound_messages());
        if leader.command_applied(&expected.command_id) {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(
        leader.command_applied(&expected.command_id),
        "runtime did not commit independently predicted command {} for {intent:?}",
        expected.command_id
    );
    pending.await.expect("runtime intent task should not panic");
    assert_eq!(
        leader.command_envelopes().last(),
        Some(expected),
        "runtime envelope disagreed with the intent-derived reference envelope"
    );
}

async fn apply_single_runtime_intent(
    runtime: &RaftMetadataRuntime,
    intent: &ReferenceMetadataIntent,
) {
    match intent {
        ReferenceMetadataIntent::JoinMember {
            node_id,
            generation,
        } => {
            runtime
                .join_member(ClusterCandidate::member(node_id.clone()).generation(*generation))
                .await
                .unwrap();
        }
        ReferenceMetadataIntent::JoinClient {
            node_id,
            generation,
        } => {
            runtime
                .join_client(ClusterCandidate::client(node_id.clone()).generation(*generation))
                .await
                .unwrap();
        }
        ReferenceMetadataIntent::Leave {
            node_id,
            generation,
        } => {
            runtime
                .leave(node_id, *generation)
                .await
                .unwrap()
                .expect("scheduled node should be present before leave");
        }
    }
}

fn assert_cluster_matches_model<const N: usize>(
    cluster: &RuntimeRaftCluster,
    node_ids: [u64; N],
    model: &ReferenceMetadataModel,
    expected_envelopes: &[RaftMetadataCommandEnvelope],
) {
    for node_id in node_ids {
        let runtime = cluster.node(node_id);
        assert_eq!(runtime.command_envelopes(), expected_envelopes);
        assert_eq!(runtime_view(&runtime), model.view());
    }
}

fn deterministic_shuffle<T>(values: &mut [T], mut state: u64) {
    for index in (1..values.len()).rev() {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        values.swap(index, (state as usize) % (index + 1));
    }
}

fn runtime_view(runtime: &RaftMetadataRuntime) -> ReferenceMetadataView {
    let commands = runtime.command_envelopes();
    ReferenceMetadataView {
        epoch: runtime.metadata_snapshot().epoch,
        members: generations(runtime.members()),
        clients: generations(runtime.clients()),
        command_ids: commands
            .into_iter()
            .map(|envelope| envelope.command_id)
            .collect(),
        committed_topology: None,
    }
}

fn materialized_view(
    runtime: &RaftMetadataRuntime,
) -> (
    hydracache::ClusterEpoch,
    BTreeMap<ClusterNodeId, ClusterGeneration>,
    BTreeMap<ClusterNodeId, ClusterGeneration>,
) {
    (
        runtime.metadata_snapshot().epoch,
        generations(runtime.members()),
        generations(runtime.clients()),
    )
}

fn generations(
    nodes: Vec<hydracache::ClusterMember>,
) -> BTreeMap<ClusterNodeId, ClusterGeneration> {
    nodes
        .into_iter()
        .filter(|node| matches!(node.role, ClusterRole::Member | ClusterRole::Client))
        .map(|node| (node.node_id, node.generation))
        .collect()
}
