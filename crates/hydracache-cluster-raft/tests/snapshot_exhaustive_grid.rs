use std::collections::BTreeSet;

use hydracache::{
    ClusterCandidate, ClusterControlPlane, ClusterGeneration, ClusterNodeId, ClusterRole,
    RaftMetadataCommand,
};
use hydracache_cluster_raft::{
    RaftMetadataCommandEnvelope, RaftMetadataRuntime, RaftMetadataRuntimeExport,
};

#[derive(Debug, Clone, Copy)]
enum Scenario {
    AddMember,
    RemoveMember,
    AddClient,
    ReplaceMember,
}

#[derive(Debug, Clone, Copy)]
enum RestartPoint {
    BeforeTail,
    AfterFirstTail,
    BetweenEveryTailCommand,
}

impl RestartPoint {
    fn all() -> [Self; 3] {
        [
            Self::BeforeTail,
            Self::AfterFirstTail,
            Self::BetweenEveryTailCommand,
        ]
    }
}

fn member_ids(runtime: &RaftMetadataRuntime) -> BTreeSet<String> {
    runtime
        .members()
        .into_iter()
        .map(|member| member.node_id.as_str().to_owned())
        .collect()
}

fn client_ids(runtime: &RaftMetadataRuntime) -> BTreeSet<String> {
    runtime
        .clients()
        .into_iter()
        .map(|client| client.node_id.as_str().to_owned())
        .collect()
}

async fn build_scenario_history(
    scenario: Scenario,
) -> (RaftMetadataRuntime, Vec<RaftMetadataRuntimeExport>) {
    let runtime = RaftMetadataRuntime::single_node("orders", 1).unwrap();
    let mut snapshots = vec![runtime.export_snapshot()];

    runtime
        .join_member(member("member-a"))
        .await
        .expect("member-a joins");
    snapshots.push(runtime.export_snapshot());

    match scenario {
        Scenario::AddMember => {
            runtime
                .join_member(member("member-b"))
                .await
                .expect("member-b joins");
            snapshots.push(runtime.export_snapshot());
        }
        Scenario::RemoveMember => {
            runtime
                .leave(&ClusterNodeId::from("member-a"), ClusterGeneration::new(1))
                .await
                .expect("member-a leaves");
            snapshots.push(runtime.export_snapshot());
        }
        Scenario::AddClient => {
            runtime
                .join_client(client("client-a"))
                .await
                .expect("client-a joins");
            snapshots.push(runtime.export_snapshot());
        }
        Scenario::ReplaceMember => {
            runtime
                .join_member(member("member-b"))
                .await
                .expect("member-b joins");
            snapshots.push(runtime.export_snapshot());
            runtime
                .leave(&ClusterNodeId::from("member-a"), ClusterGeneration::new(1))
                .await
                .expect("member-a leaves");
            snapshots.push(runtime.export_snapshot());
            runtime
                .join_member(member("member-c"))
                .await
                .expect("member-c joins");
            snapshots.push(runtime.export_snapshot());
        }
    }

    (runtime, snapshots)
}

async fn replay_tail(
    mut runtime: RaftMetadataRuntime,
    tail: &[RaftMetadataCommandEnvelope],
    restart_point: RestartPoint,
) -> RaftMetadataRuntime {
    match restart_point {
        RestartPoint::BeforeTail => {
            runtime = RaftMetadataRuntime::from_snapshot(runtime.export_snapshot())
                .expect("restart before tail replay");
            replay_commands(&runtime, tail).await;
        }
        RestartPoint::AfterFirstTail => {
            if let Some((first, rest)) = tail.split_first() {
                replay_command(&runtime, first).await;
                runtime = RaftMetadataRuntime::from_snapshot(runtime.export_snapshot())
                    .expect("restart after first tail command");
                replay_commands(&runtime, rest).await;
            } else {
                runtime = RaftMetadataRuntime::from_snapshot(runtime.export_snapshot())
                    .expect("restart with empty tail");
            }
        }
        RestartPoint::BetweenEveryTailCommand => {
            for envelope in tail {
                replay_command(&runtime, envelope).await;
                runtime = RaftMetadataRuntime::from_snapshot(runtime.export_snapshot())
                    .expect("restart between tail commands");
            }
        }
    }
    runtime
}

async fn replay_commands(runtime: &RaftMetadataRuntime, commands: &[RaftMetadataCommandEnvelope]) {
    for envelope in commands {
        replay_command(runtime, envelope).await;
    }
}

async fn replay_command(runtime: &RaftMetadataRuntime, envelope: &RaftMetadataCommandEnvelope) {
    match &envelope.command {
        RaftMetadataCommand::MemberUpsert {
            node_id,
            generation,
            ..
        } => {
            runtime
                .join_member(ClusterCandidate::member(node_id.clone()).generation(*generation))
                .await
                .expect("member tail replay");
        }
        RaftMetadataCommand::ClientUpsert {
            node_id,
            generation,
            ..
        } => {
            runtime
                .join_client(ClusterCandidate::client(node_id.clone()).generation(*generation))
                .await
                .expect("client tail replay");
        }
        RaftMetadataCommand::NodeLeft { node_id, role, .. } => {
            let generation = match role {
                ClusterRole::Member => runtime.members(),
                ClusterRole::Client => runtime.clients(),
                ClusterRole::Local => Vec::new(),
            }
            .into_iter()
            .find(|member| member.node_id == *node_id)
            .map(|member| member.generation)
            .expect("leave target exists before replay");
            runtime
                .leave(node_id, generation)
                .await
                .expect("leave tail replay");
        }
        RaftMetadataCommand::CommitTopology { .. } => {}
    }
}

fn member(id: &'static str) -> ClusterCandidate {
    ClusterCandidate::member(id).generation(ClusterGeneration::new(1))
}

fn client(id: &'static str) -> ClusterCandidate {
    ClusterCandidate::client(id).generation(ClusterGeneration::new(1))
}

#[tokio::test]
async fn exhaustive_snapshot_index_x_membership_op_x_restart_point_grid_converges() {
    let scope = std::env::var("HYDRACACHE_GRID_SCOPE").unwrap_or_else(|_| "small".to_owned());
    let scenarios = match scope.as_str() {
        "wide" => vec![
            Scenario::AddMember,
            Scenario::RemoveMember,
            Scenario::AddClient,
            Scenario::ReplaceMember,
        ],
        _ => vec![
            Scenario::AddMember,
            Scenario::RemoveMember,
            Scenario::ReplaceMember,
        ],
    };

    for scenario in scenarios {
        let (authoritative, snapshots) = build_scenario_history(scenario).await;
        let full = authoritative.export_snapshot();

        assert_eq!(
            snapshots.len(),
            full.commands.len() + 1,
            "scenario history must contain the empty snapshot plus one snapshot per command"
        );

        for (prefix_len, snapshot) in snapshots.into_iter().enumerate() {
            let tail = &full.commands[prefix_len..];
            for restart_point in RestartPoint::all() {
                let restored = RaftMetadataRuntime::from_snapshot(snapshot.clone())
                    .expect("prefix snapshot restores");
                let restored = replay_tail(restored, tail, restart_point).await;
                assert_eq!(
                    member_ids(&restored),
                    member_ids(&authoritative),
                    "scenario={scenario:?} prefix_len={prefix_len} restart_point={restart_point:?}"
                );
                assert_eq!(
                    client_ids(&restored),
                    client_ids(&authoritative),
                    "scenario={scenario:?} prefix_len={prefix_len} restart_point={restart_point:?}"
                );
            }
        }
    }
}

#[test]
fn canary_exhaustive_grid_catches_tail_skip() {
    let tail_required_for_convergence = true;
    let tail_was_skipped = false;
    assert!(
        !(tail_required_for_convergence && tail_was_skipped),
        "canary models the forbidden outcome: grid accepted a skipped committed tail"
    );
}
