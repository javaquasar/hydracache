use hydracache::{
    ClusterCandidate, ClusterControlPlane, ClusterGeneration, ClusterMember, ClusterNodeId,
    ClusterRole, RaftMetadataCommand,
};
use hydracache_cluster_raft::{
    RaftMetadataCommandEnvelope, RaftMetadataRuntime, RaftMetadataRuntimeExport,
};

fn member(id: &'static str, generation: u64) -> ClusterCandidate {
    ClusterCandidate::member(id).generation(ClusterGeneration::new(generation))
}

fn client(id: &'static str, generation: u64) -> ClusterCandidate {
    ClusterCandidate::client(id).generation(ClusterGeneration::new(generation))
}

fn ids(mut members: Vec<ClusterMember>) -> Vec<String> {
    members.sort_by(|left, right| left.node_id.cmp(&right.node_id));
    members
        .into_iter()
        .map(|member| member.node_id.as_str().to_owned())
        .collect()
}

fn command_ids(commands: &[RaftMetadataCommandEnvelope]) -> Vec<String> {
    commands
        .iter()
        .map(|command| command.command_id.clone())
        .collect()
}

fn prefix_snapshot(
    snapshot: &RaftMetadataRuntimeExport,
    command_count: usize,
) -> RaftMetadataRuntimeExport {
    let mut prefix = snapshot.clone();
    prefix.commands.truncate(command_count);
    prefix
}

fn leave_generation(runtime: &RaftMetadataRuntime, node_id: &ClusterNodeId) -> ClusterGeneration {
    runtime
        .members()
        .into_iter()
        .chain(runtime.clients())
        .find(|member| member.node_id == *node_id)
        .map(|member| member.generation)
        .unwrap_or_else(|| panic!("tail leave references absent node {node_id}"))
}

async fn replay_tail(
    runtime: &RaftMetadataRuntime,
    tail: &[RaftMetadataCommandEnvelope],
) -> Vec<String> {
    let mut replayed = Vec::new();
    for envelope in tail {
        replayed.push(envelope.command_id.clone());
        match &envelope.command {
            RaftMetadataCommand::MemberUpsert {
                node_id,
                generation,
                ..
            } => {
                runtime
                    .join_member(ClusterCandidate::member(node_id.clone()).generation(*generation))
                    .await
                    .unwrap();
            }
            RaftMetadataCommand::ClientUpsert {
                node_id,
                generation,
                ..
            } => {
                runtime
                    .join_client(ClusterCandidate::client(node_id.clone()).generation(*generation))
                    .await
                    .unwrap();
            }
            RaftMetadataCommand::NodeLeft { node_id, .. } => {
                runtime
                    .leave(node_id, leave_generation(runtime, node_id))
                    .await
                    .unwrap();
            }
            RaftMetadataCommand::CommitTopology { .. } => {}
        }
    }
    replayed
}

#[derive(Debug)]
struct SnapshotReplayTrace {
    seed: &'static str,
    snapshot_command_count: usize,
    snapshot_applied_index: u64,
    tail_command_ids: Vec<String>,
    authoritative_members: Vec<String>,
    restored_members: Vec<String>,
    authoritative_clients: Vec<String>,
    restored_clients: Vec<String>,
}

impl SnapshotReplayTrace {
    fn new(
        seed: &'static str,
        snapshot: &RaftMetadataRuntimeExport,
        tail: &[RaftMetadataCommandEnvelope],
        authoritative: &RaftMetadataRuntime,
        restored: &RaftMetadataRuntime,
    ) -> Self {
        Self {
            seed,
            snapshot_command_count: snapshot.commands.len(),
            snapshot_applied_index: snapshot.applied_index,
            tail_command_ids: command_ids(tail),
            authoritative_members: ids(authoritative.members()),
            restored_members: ids(restored.members()),
            authoritative_clients: ids(authoritative.clients()),
            restored_clients: ids(restored.clients()),
        }
    }

    fn context(&self) -> String {
        format!(
            "seed={}, snapshot_command_count={}, snapshot_applied_index={}, tail_command_ids={:?}",
            self.seed,
            self.snapshot_command_count,
            self.snapshot_applied_index,
            self.tail_command_ids
        )
    }

    fn assert_converged(&self) {
        let context = self.context();
        assert_eq!(
            self.restored_members, self.authoritative_members,
            "restored member set diverged after snapshot+tail replay: {context}; {self:?}"
        );
        assert_eq!(
            self.restored_clients, self.authoritative_clients,
            "restored client set diverged after snapshot+tail replay: {context}; {self:?}"
        );
    }
}

#[tokio::test]
async fn mid_membership_snapshot_then_tail_replay_converges_to_authoritative_membership() {
    let authoritative = RaftMetadataRuntime::single_node("orders", 1).unwrap();
    authoritative
        .join_member(member("member-a", 1))
        .await
        .unwrap();
    let snapshot = authoritative.export_snapshot();

    authoritative
        .join_member(member("member-b", 1))
        .await
        .unwrap();
    authoritative
        .leave(&ClusterNodeId::from("member-a"), ClusterGeneration::new(1))
        .await
        .unwrap();
    authoritative
        .join_member(member("member-c", 1))
        .await
        .unwrap();
    authoritative
        .join_client(client("client-a", 1))
        .await
        .unwrap();

    let full = authoritative.export_snapshot();
    let tail = &full.commands[snapshot.commands.len()..];
    let restored = RaftMetadataRuntime::from_snapshot(snapshot.clone()).unwrap();
    let replayed_ids = replay_tail(&restored, tail).await;

    assert_eq!(replayed_ids, command_ids(tail));
    SnapshotReplayTrace::new(
        "w2-mid-membership",
        &snapshot,
        tail,
        &authoritative,
        &restored,
    )
    .assert_converged();
}

#[tokio::test]
async fn snapshot_between_remove_and_add_voter_applies_tail_in_order() {
    let authoritative = RaftMetadataRuntime::single_node("orders", 1).unwrap();
    authoritative
        .join_member(member("member-a", 1))
        .await
        .unwrap();
    authoritative
        .join_member(member("member-b", 1))
        .await
        .unwrap();
    authoritative
        .leave(&ClusterNodeId::from("member-a"), ClusterGeneration::new(1))
        .await
        .unwrap();
    let snapshot = authoritative.export_snapshot();

    authoritative
        .join_member(member("member-c", 1))
        .await
        .unwrap();
    authoritative
        .join_member(member("member-d", 1))
        .await
        .unwrap();

    let full = authoritative.export_snapshot();
    let tail = &full.commands[snapshot.commands.len()..];
    let restored = RaftMetadataRuntime::from_snapshot(snapshot.clone()).unwrap();
    let replayed_ids = replay_tail(&restored, tail).await;

    assert_eq!(
        replayed_ids,
        vec![
            "member-upsert:member-c:1".to_owned(),
            "member-upsert:member-d:1".to_owned(),
        ],
        "tail replay must preserve add order"
    );
    assert_eq!(replayed_ids, command_ids(tail));
    SnapshotReplayTrace::new(
        "w2-remove-before-add",
        &snapshot,
        tail,
        &authoritative,
        &restored,
    )
    .assert_converged();
}

#[tokio::test]
async fn restored_joiner_does_not_keep_removed_voter_or_miss_self_after_tail_replay() {
    let authoritative = RaftMetadataRuntime::single_node("orders", 1).unwrap();
    authoritative
        .join_member(member("removed-voter", 1))
        .await
        .unwrap();
    authoritative
        .join_member(member("stable-voter", 1))
        .await
        .unwrap();
    let snapshot = authoritative.export_snapshot();

    authoritative
        .leave(
            &ClusterNodeId::from("removed-voter"),
            ClusterGeneration::new(1),
        )
        .await
        .unwrap();
    authoritative
        .join_member(member("joining-self", 1))
        .await
        .unwrap();

    let full = authoritative.export_snapshot();
    let tail = &full.commands[snapshot.commands.len()..];
    let restored = RaftMetadataRuntime::from_snapshot(snapshot.clone()).unwrap();
    let replayed_ids = replay_tail(&restored, tail).await;

    assert_eq!(replayed_ids, command_ids(tail));
    let restored_members = ids(restored.members());
    assert!(
        !restored_members.contains(&"removed-voter".to_owned()),
        "restored runtime kept removed voter after tail replay"
    );
    assert!(
        restored_members.contains(&"joining-self".to_owned()),
        "restored runtime missed the joiner after tail replay"
    );
    SnapshotReplayTrace::new("w2-joiner-tail", &snapshot, tail, &authoritative, &restored)
        .assert_converged();
}

#[tokio::test]
async fn snapshot_prefix_fixture_does_not_hide_tail_membership_changes() {
    let authoritative = RaftMetadataRuntime::single_node("orders", 1).unwrap();
    authoritative
        .join_member(member("member-a", 1))
        .await
        .unwrap();
    authoritative
        .join_member(member("member-b", 1))
        .await
        .unwrap();
    authoritative
        .leave(&ClusterNodeId::from("member-a"), ClusterGeneration::new(1))
        .await
        .unwrap();
    authoritative
        .join_member(member("member-c", 1))
        .await
        .unwrap();

    let full = authoritative.export_snapshot();
    let snapshot = prefix_snapshot(&full, 2);
    let tail = &full.commands[snapshot.commands.len()..];
    let restored = RaftMetadataRuntime::from_snapshot(snapshot.clone()).unwrap();

    assert_eq!(
        ids(restored.members()),
        vec!["member-a".to_owned(), "member-b".to_owned()]
    );
    replay_tail(&restored, tail).await;

    assert_eq!(ids(restored.members()), ids(authoritative.members()));
    assert!(
        tail.iter().any(|envelope| matches!(
            envelope.command,
            RaftMetadataCommand::NodeLeft {
                role: ClusterRole::Member,
                ..
            }
        )),
        "fixture tail must include the removal that proves stale members are not hidden"
    );
}
