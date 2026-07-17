use std::collections::BTreeMap;

use hydracache::{
    ClusterCandidate, ClusterControlPlane, ClusterGeneration, ClusterNodeId, ClusterRole,
};
use hydracache_cluster_raft::RaftMetadataRuntime;
use hydracache_cluster_testkit::reference_model::{ReferenceMetadataModel, ReferenceMetadataView};

#[tokio::test]
async fn runtime_committed_metadata_matches_reference_model() {
    let runtime = RaftMetadataRuntime::single_node("differential-reference", 1).unwrap();
    let mut model = ReferenceMetadataModel::new();
    let mut applied = 0usize;

    for operation in seeded_schedule(0x0660_0008) {
        let before = runtime.command_envelopes().len();
        operation.apply(&runtime).await;
        let commands = runtime.command_envelopes();
        assert_eq!(
            commands.len(),
            before + 1,
            "one successful external operation must commit exactly one command: {operation:?}"
        );
        model.apply_all(&commands[applied..]).unwrap();
        applied = commands.len();
        assert_eq!(runtime_view(&runtime), model.view());
    }
}

#[tokio::test]
async fn prefix_replay_reorder_and_snapshot_tail_relations_hold() {
    let full = RaftMetadataRuntime::single_node("differential-prefix", 1).unwrap();
    Operation::JoinMember("member-a", 1).apply(&full).await;
    Operation::JoinClient("client-a", 1).apply(&full).await;
    let prefix = full.export_snapshot();

    let tail = [
        Operation::JoinMember("member-b", 1),
        Operation::Leave("client-a", 1),
    ];
    for operation in tail {
        operation.apply(&full).await;
    }

    let recovered = RaftMetadataRuntime::from_snapshot(prefix).unwrap();
    for operation in tail {
        operation.apply(&recovered).await;
    }
    assert_eq!(runtime_view(&full), runtime_view(&recovered));

    let member_then_client = RaftMetadataRuntime::single_node("differential-reorder", 1).unwrap();
    Operation::JoinMember("member-independent", 1)
        .apply(&member_then_client)
        .await;
    Operation::JoinClient("client-independent", 1)
        .apply(&member_then_client)
        .await;

    let client_then_member = RaftMetadataRuntime::single_node("differential-reorder", 1).unwrap();
    Operation::JoinClient("client-independent", 1)
        .apply(&client_then_member)
        .await;
    Operation::JoinMember("member-independent", 1)
        .apply(&client_then_member)
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
    Operation::JoinMember("member-a", 1).apply(&runtime).await;
    Operation::JoinMember("member-b", 1).apply(&runtime).await;

    let commands = runtime.command_envelopes();
    let mut incomplete_model = ReferenceMetadataModel::new();
    incomplete_model.apply(&commands[0]).unwrap();
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

#[derive(Debug, Clone, Copy)]
enum Operation {
    JoinMember(&'static str, u64),
    JoinClient(&'static str, u64),
    Leave(&'static str, u64),
}

impl Operation {
    async fn apply(self, runtime: &RaftMetadataRuntime) {
        match self {
            Self::JoinMember(node, generation) => {
                runtime
                    .join_member(
                        ClusterCandidate::member(node)
                            .generation(ClusterGeneration::new(generation)),
                    )
                    .await
                    .unwrap();
            }
            Self::JoinClient(node, generation) => {
                runtime
                    .join_client(
                        ClusterCandidate::client(node)
                            .generation(ClusterGeneration::new(generation)),
                    )
                    .await
                    .unwrap();
            }
            Self::Leave(node, generation) => {
                runtime
                    .leave(
                        &ClusterNodeId::from(node),
                        ClusterGeneration::new(generation),
                    )
                    .await
                    .unwrap()
                    .expect("scheduled node should be present before leave");
            }
        }
    }
}

fn seeded_schedule(seed: u64) -> Vec<Operation> {
    let mut joins = vec![
        Operation::JoinMember("member-a", 1),
        Operation::JoinMember("member-b", 1),
        Operation::JoinMember("member-c", 1),
        Operation::JoinClient("client-a", 1),
        Operation::JoinClient("client-b", 1),
    ];
    deterministic_shuffle(&mut joins, seed);
    let mut leaves = vec![
        Operation::Leave("member-b", 1),
        Operation::Leave("client-a", 1),
    ];
    deterministic_shuffle(&mut leaves, seed.rotate_left(17));
    joins.extend(leaves);
    joins
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
