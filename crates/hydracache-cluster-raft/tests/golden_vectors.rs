use std::fs;
use std::path::{Path, PathBuf};

use hydracache::{
    ClusterEpoch, ClusterGeneration, ClusterNodeId, ClusterRole, RaftMetadataCommand,
};
use hydracache_cluster_raft::{RaftMetadataCommandEnvelope, RaftWireMessage};
use protobuf::Message as _;
use raft::eraftpb::{ConfState, Message, MessageType, Snapshot};

fn vector_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("vectors")
        .join(name)
}

#[test]
fn golden_command_envelopes_decode_to_expected() {
    let cases = [
        (
            "member_upsert.bin",
            RaftMetadataCommandEnvelope {
                command_id: "member-upsert:member-a:1".to_owned(),
                command: RaftMetadataCommand::MemberUpsert {
                    node_id: ClusterNodeId::from("member-a"),
                    generation: ClusterGeneration::new(1),
                    epoch: ClusterEpoch::new(1),
                },
            },
        ),
        (
            "client_upsert.bin",
            RaftMetadataCommandEnvelope {
                command_id: "client-upsert:client-a:1".to_owned(),
                command: RaftMetadataCommand::ClientUpsert {
                    node_id: ClusterNodeId::from("client-a"),
                    generation: ClusterGeneration::new(1),
                    epoch: ClusterEpoch::new(1),
                },
            },
        ),
        (
            "node_left.bin",
            RaftMetadataCommandEnvelope {
                command_id: "node-left:member-a:2".to_owned(),
                command: RaftMetadataCommand::NodeLeft {
                    node_id: ClusterNodeId::from("member-a"),
                    role: ClusterRole::Member,
                    epoch: ClusterEpoch::new(2),
                },
            },
        ),
        (
            "commit_topology.bin",
            RaftMetadataCommandEnvelope {
                command_id: "commit-topology:3:member-a,member-b".to_owned(),
                command: RaftMetadataCommand::CommitTopology {
                    epoch: ClusterEpoch::new(3),
                    members: vec![
                        ClusterNodeId::from("member-a"),
                        ClusterNodeId::from("member-b"),
                    ],
                },
            },
        ),
    ];

    for (file_name, expected) in cases {
        let bytes = fs::read(vector_path(file_name)).expect("golden vector exists");

        assert_eq!(
            RaftMetadataCommandEnvelope::decode(&bytes).unwrap(),
            expected
        );
    }
}

#[test]
fn golden_wire_messages_decode_to_expected() {
    let payload =
        fs::read(vector_path("raft_wire_message_payload.bin")).expect("wire vector exists");
    let wire = RaftWireMessage {
        from: 1,
        to: 2,
        term: 3,
        payload,
    };
    let decoded = wire.decode().unwrap();

    assert_eq!(decoded.from, 1);
    assert_eq!(decoded.to, 2);
    assert_eq!(decoded.term, 3);
    assert_eq!(decoded.get_msg_type(), MessageType::MsgAppend);
}

#[test]
fn golden_snapshot_conf_state_decodes_to_expected() {
    let conf_state =
        ConfState::parse_from_bytes(&fs::read(vector_path("conf_state.bin")).unwrap()).unwrap();
    let snapshot =
        Snapshot::parse_from_bytes(&fs::read(vector_path("snapshot_conf_state.bin")).unwrap())
            .unwrap();

    assert_eq!(conf_state.voters, vec![1, 2, 3]);
    assert_eq!(snapshot.get_metadata().index, 7);
    assert_eq!(snapshot.get_metadata().term, 4);
    assert_eq!(
        snapshot.get_metadata().get_conf_state().voters,
        vec![1, 2, 3]
    );
}

#[test]
#[ignore = "regenerates committed golden vectors; run only for intentional format changes"]
fn regenerate_golden_vectors() {
    let vector_dir = vector_path("");
    fs::create_dir_all(&vector_dir).expect("vector dir");

    let command_vectors = [
        (
            "member_upsert.bin",
            RaftMetadataCommandEnvelope {
                command_id: "member-upsert:member-a:1".to_owned(),
                command: RaftMetadataCommand::MemberUpsert {
                    node_id: ClusterNodeId::from("member-a"),
                    generation: ClusterGeneration::new(1),
                    epoch: ClusterEpoch::new(1),
                },
            },
        ),
        (
            "client_upsert.bin",
            RaftMetadataCommandEnvelope {
                command_id: "client-upsert:client-a:1".to_owned(),
                command: RaftMetadataCommand::ClientUpsert {
                    node_id: ClusterNodeId::from("client-a"),
                    generation: ClusterGeneration::new(1),
                    epoch: ClusterEpoch::new(1),
                },
            },
        ),
        (
            "node_left.bin",
            RaftMetadataCommandEnvelope {
                command_id: "node-left:member-a:2".to_owned(),
                command: RaftMetadataCommand::NodeLeft {
                    node_id: ClusterNodeId::from("member-a"),
                    role: ClusterRole::Member,
                    epoch: ClusterEpoch::new(2),
                },
            },
        ),
        (
            "commit_topology.bin",
            RaftMetadataCommandEnvelope {
                command_id: "commit-topology:3:member-a,member-b".to_owned(),
                command: RaftMetadataCommand::CommitTopology {
                    epoch: ClusterEpoch::new(3),
                    members: vec![
                        ClusterNodeId::from("member-a"),
                        ClusterNodeId::from("member-b"),
                    ],
                },
            },
        ),
    ];
    for (file_name, envelope) in command_vectors {
        fs::write(vector_path(file_name), envelope.encode()).expect("write command vector");
    }

    let mut message = Message {
        from: 1,
        to: 2,
        term: 3,
        ..Message::default()
    };
    message.set_msg_type(MessageType::MsgAppend);
    fs::write(
        vector_path("raft_wire_message_payload.bin"),
        message.write_to_bytes().unwrap(),
    )
    .expect("write wire vector");

    let conf_state = ConfState {
        voters: vec![1, 2, 3],
        ..ConfState::default()
    };
    fs::write(
        vector_path("conf_state.bin"),
        conf_state.write_to_bytes().unwrap(),
    )
    .expect("write conf state vector");

    let mut snapshot = Snapshot::default();
    snapshot.mut_metadata().index = 7;
    snapshot.mut_metadata().term = 4;
    snapshot.mut_metadata().mut_conf_state().voters = vec![1, 2, 3];
    fs::write(
        vector_path("snapshot_conf_state.bin"),
        snapshot.write_to_bytes().unwrap(),
    )
    .expect("write snapshot vector");
}
