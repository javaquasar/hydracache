use std::sync::Arc;

use hydracache::{ClusterCandidate, ClusterControlPlane, ClusterGeneration, ClusterNodeId};
use hydracache_cluster_raft::{
    InMemoryRaftMetadataStore, RaftMetadataRuntime, RaftMetadataRuntimeConfig,
    RaftMetadataRuntimeExport,
};

fn member(id: &'static str, generation: u64) -> ClusterCandidate {
    ClusterCandidate::member(id).generation(ClusterGeneration::new(generation))
}

fn client(id: &'static str, generation: u64) -> ClusterCandidate {
    ClusterCandidate::client(id).generation(ClusterGeneration::new(generation))
}

fn command_ids(snapshot: &RaftMetadataRuntimeExport) -> Vec<String> {
    snapshot
        .commands
        .iter()
        .map(|command| command.command_id.clone())
        .collect()
}

mod snapshot_immutability {
    use super::*;

    #[tokio::test]
    async fn exported_snapshot_is_immutable_after_live_membership_mutation() {
        let runtime = RaftMetadataRuntime::single_node("orders", 1).unwrap();

        runtime.join_member(member("member-a", 1)).await.unwrap();
        let exported = runtime.export_snapshot();
        let exported_commands = command_ids(&exported);
        let exported_applied_index = exported.applied_index;

        runtime.join_member(member("member-b", 1)).await.unwrap();
        runtime.join_client(client("client-a", 1)).await.unwrap();
        runtime
            .leave(&ClusterNodeId::from("member-a"), ClusterGeneration::new(1))
            .await
            .unwrap();

        assert_eq!(exported.cluster_name, "orders");
        assert_eq!(exported.raft_node_id, 1);
        assert_eq!(exported.applied_index, exported_applied_index);
        assert_eq!(command_ids(&exported), exported_commands);
        assert_eq!(exported.commands.len(), 1);

        let recovered = RaftMetadataRuntime::from_snapshot(exported).unwrap();
        assert_eq!(recovered.members().len(), 1);
        assert_eq!(recovered.clients().len(), 0);
        assert_eq!(
            recovered.members()[0].node_id.as_str(),
            "member-a",
            "export must remain a point-in-time membership value"
        );
    }

    #[tokio::test]
    async fn durable_snapshot_bytes_do_not_change_after_membership_tail_applies() {
        let store = Arc::new(InMemoryRaftMetadataStore::new());
        let runtime = RaftMetadataRuntime::with_config_and_metadata_store(
            RaftMetadataRuntimeConfig::single_node("orders", 1),
            store.clone(),
        )
        .unwrap();

        runtime.join_member(member("member-a", 1)).await.unwrap();
        let captured = store.snapshot().expect("first committed snapshot saved");
        let captured_commands = command_ids(&captured);
        let captured_applied_index = captured.applied_index;

        runtime.join_member(member("member-b", 1)).await.unwrap();
        runtime.join_client(client("client-a", 1)).await.unwrap();
        let current = store.snapshot().expect("latest committed snapshot saved");

        assert_eq!(captured.commands.len(), 1);
        assert_eq!(captured.applied_index, captured_applied_index);
        assert_eq!(command_ids(&captured), captured_commands);
        assert_eq!(current.commands.len(), 3);

        let recovered_from_captured = RaftMetadataRuntime::with_config_and_metadata_store(
            RaftMetadataRuntimeConfig::single_node("orders", 1),
            Arc::new(InMemoryRaftMetadataStore::with_snapshot(captured)),
        )
        .unwrap();
        assert_eq!(recovered_from_captured.members().len(), 1);
        assert_eq!(recovered_from_captured.clients().len(), 0);
        assert_eq!(
            recovered_from_captured.members()[0].node_id.as_str(),
            "member-a"
        );
    }

    #[tokio::test]
    async fn snapshot_restore_does_not_share_member_or_command_state_with_source_runtime() {
        let source = RaftMetadataRuntime::single_node("orders", 1).unwrap();
        source.join_member(member("member-a", 1)).await.unwrap();
        source.join_client(client("client-a", 1)).await.unwrap();

        let exported = source.export_snapshot();
        let exported_clone = exported.clone();
        let restored = RaftMetadataRuntime::from_snapshot(exported.clone()).unwrap();

        restored.join_member(member("member-b", 1)).await.unwrap();
        restored
            .leave(&ClusterNodeId::from("member-a"), ClusterGeneration::new(1))
            .await
            .unwrap();
        source.join_member(member("member-c", 1)).await.unwrap();

        assert_eq!(
            exported, exported_clone,
            "restore and later source mutations must not mutate the exported snapshot object"
        );
        assert_eq!(
            command_ids(&exported),
            vec![
                "member-upsert:member-a:1".to_owned(),
                "client-upsert:client-a:1".to_owned(),
            ]
        );
        assert_eq!(source.members().len(), 2);
        assert_eq!(restored.members().len(), 1);
        assert_eq!(restored.members()[0].node_id.as_str(), "member-b");

        let recovered_again = RaftMetadataRuntime::from_snapshot(exported).unwrap();
        assert_eq!(recovered_again.members().len(), 1);
        assert_eq!(recovered_again.members()[0].node_id.as_str(), "member-a");
        assert_eq!(recovered_again.clients().len(), 1);
    }
}
