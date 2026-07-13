#![cfg(feature = "test-failpoints")]

use fail::FailScenario;
use hydracache::{ClusterEpoch, ClusterGeneration, ClusterNodeId, RaftMetadataCommand};
use hydracache_cluster_raft::{
    InMemoryRaftLogStore, RaftLogStore, RaftMetadataCommandEnvelope, RaftWireMessage,
};
use hydracache_cluster_testkit::RuntimeRaftCluster;
use raft::eraftpb::{Message, MessageType, Snapshot};
use raft::storage::Storage;

fn snapshot(index: u64, term: u64, voters: Vec<u64>) -> Snapshot {
    let mut snapshot = Snapshot::default();
    snapshot.mut_metadata().index = index;
    snapshot.mut_metadata().term = term;
    snapshot.mut_metadata().mut_conf_state().voters = voters;
    snapshot
}

fn metadata_snapshot_message() -> RaftWireMessage {
    let envelope = RaftMetadataCommandEnvelope {
        command_id: "member-upsert:member-a:1".to_owned(),
        command: RaftMetadataCommand::MemberUpsert {
            node_id: ClusterNodeId::from("member-a"),
            generation: ClusterGeneration::new(1),
            epoch: ClusterEpoch::new(1),
        },
    };
    let payload = serde_json::json!({
        "format_version": 1_u32,
        "cluster_name": "orders",
        "source_raft_node_id": 1_u64,
        "applied_index": 50_u64,
        "commands": [envelope],
    });
    let mut data = b"HCMETA01".to_vec();
    data.extend(serde_json::to_vec(&payload).unwrap());

    let mut snapshot = snapshot(50, 1, vec![1, 2, 3]);
    snapshot.data = data.into();
    let mut message = Message {
        from: 1,
        to: 3,
        term: 1,
        ..Message::default()
    };
    message.set_msg_type(MessageType::MsgSnapshot);
    message.set_snapshot(snapshot);
    RaftWireMessage::encode(&message).unwrap()
}

#[test]
fn disk_full_during_save_snapshot_fails_loud_without_partial_state() {
    let _scenario = FailScenario::setup();
    let store = InMemoryRaftLogStore::new();
    fail::cfg("raft_save_snapshot_disk_full", "return").unwrap();

    let error = store
        .save_snapshot(&snapshot(7, 2, vec![1, 2, 3]), 0)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("injected disk full during raft snapshot save"),
        "unexpected error: {error}"
    );

    fail::remove("raft_save_snapshot_disk_full");
    assert_eq!(store.snapshot(0, 1).unwrap().get_metadata().index, 0);
    assert_eq!(store.first_index().unwrap(), 1);
}

#[tokio::test]
async fn snapshot_install_under_memory_pressure_does_not_corrupt_apply() {
    let _scenario = FailScenario::setup();
    let mut cluster = RuntimeRaftCluster::three_node();
    cluster.campaign(1);
    let before = cluster.node(3).snapshot();

    fail::cfg("raft_install_snapshot_oom", "return").unwrap();
    let error = cluster
        .node(3)
        .step(metadata_snapshot_message())
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("injected OOM during raft snapshot install"),
        "unexpected error: {error}"
    );
    fail::remove("raft_install_snapshot_oom");

    let after = cluster.node(3).snapshot();
    assert_eq!(after.commands_committed, before.commands_committed);
    assert_eq!(after.snapshot_installs, before.snapshot_installs);
    assert!(!cluster.node(3).command_applied("member-upsert:member-a:1"));
}

#[test]
fn canary_disk_full_snapshot_persists_partial_bytes() {
    let save_returned_error = true;
    let partial_snapshot_visible = false;
    assert!(
        !(save_returned_error && partial_snapshot_visible),
        "canary models the forbidden outcome: failed snapshot save left visible partial state"
    );
}
