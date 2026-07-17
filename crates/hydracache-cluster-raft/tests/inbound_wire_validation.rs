#![cfg(feature = "sled-log-store")]

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use hydracache::{
    ClusterEpoch, ClusterGeneration, ClusterNodeId, ClusterRole, RaftMetadataCommand,
};
use hydracache_cluster_raft::{
    RaftLogStore, RaftMetadataCommandEnvelope, RaftMetadataRuntime, RaftMetadataRuntimeConfig,
    RaftWireMessage, SledRaftLogStore,
};
use raft::eraftpb::{ConfState, Message, MessageType, Snapshot};
use raft::storage::Storage;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

static TEST_DIRECTORY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct SledTestDirectory {
    path: PathBuf,
}

impl SledTestDirectory {
    fn new(label: &str) -> TestResult<Self> {
        let sequence = TEST_DIRECTORY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = PathBuf::from(format!(
            "target/test-hydracache-raft-wire/{label}-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&path)?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for SledTestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn open_store(path: &Path) -> TestResult<SledRaftLogStore> {
    let store = SledRaftLogStore::open(path)?;
    let conf_state = ConfState::from((vec![1, 2, 3], Vec::<u64>::new()));
    store.initialize_with_conf_state(conf_state.clone());
    store.save_conf_state(&conf_state)?;
    Ok(store)
}

fn metadata_payload(
    cluster_name: &str,
    source_raft_node_id: u64,
    applied_index: u64,
    commands: Vec<RaftMetadataCommandEnvelope>,
) -> Vec<u8> {
    let payload = serde_json::json!({
        "format_version": 1_u32,
        "cluster_name": cluster_name,
        "source_raft_node_id": source_raft_node_id,
        "applied_index": applied_index,
        "commands": commands,
    });
    let mut data = b"HCMETA01".to_vec();
    data.extend(serde_json::to_vec(&payload).unwrap());
    data
}

fn snapshot_wire(data: Vec<u8>, snapshot_index: u64) -> RaftWireMessage {
    let mut snapshot = Snapshot::default();
    snapshot.mut_metadata().index = snapshot_index;
    snapshot.mut_metadata().term = 1;
    snapshot.mut_metadata().mut_conf_state().voters = vec![1, 2, 3];
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

fn semantic_failure_commands() -> Vec<RaftMetadataCommandEnvelope> {
    vec![
        RaftMetadataCommandEnvelope {
            command_id: "member-upsert:partial-prefix:1".to_owned(),
            command: RaftMetadataCommand::MemberUpsert {
                node_id: ClusterNodeId::from("partial-prefix"),
                generation: ClusterGeneration::new(1),
                epoch: ClusterEpoch::new(1),
            },
        },
        RaftMetadataCommandEnvelope {
            command_id: "node-left:missing-member:2".to_owned(),
            command: RaftMetadataCommand::NodeLeft {
                node_id: ClusterNodeId::from("missing-member"),
                role: ClusterRole::Member,
                epoch: ClusterEpoch::new(2),
            },
        },
    ]
}

#[test]
fn malformed_metadata_snapshot_is_rejected_before_sled_mutation_and_reopen() -> TestResult {
    let directory = SledTestDirectory::new("preflight")?;
    let store = open_store(directory.path())?;
    let config = RaftMetadataRuntimeConfig::multi_voter("orders", 3, [1, 2, 3]);
    let runtime = RaftMetadataRuntime::with_storage(config.clone(), store.clone())?;
    let runtime_before = runtime.snapshot();
    let members_before = runtime.members();
    let initial_state_before = store.initial_state()?;
    let snapshot_before = store.snapshot(0, 3)?;
    let entries_before = store.retained_entries()?;
    let applied_before = store.applied_index()?;

    let mut outer_sender_mismatch = snapshot_wire(Vec::new(), 50);
    outer_sender_mismatch.from = 9;
    let cases = vec![
        (
            "outer-sender-mismatch",
            outer_sender_mismatch,
            "from mismatch",
        ),
        (
            "missing-snapshot-field",
            {
                let mut message = Message {
                    from: 1,
                    to: 3,
                    term: 1,
                    ..Message::default()
                };
                message.set_msg_type(MessageType::MsgSnapshot);
                RaftWireMessage::encode(&message).unwrap()
            },
            "missing its snapshot field",
        ),
        (
            "unsupported-payload",
            snapshot_wire(b"not-hydracache-metadata".to_vec(), 50),
            "unsupported raft metadata snapshot payload",
        ),
        (
            "cluster-mismatch",
            snapshot_wire(metadata_payload("billing", 1, 50, Vec::new()), 50),
            "does not match runtime cluster",
        ),
        (
            "source-mismatch",
            snapshot_wire(metadata_payload("orders", 2, 50, Vec::new()), 50),
            "does not match protobuf sender",
        ),
        (
            "index-mismatch",
            snapshot_wire(metadata_payload("orders", 1, 49, Vec::new()), 50),
            "does not match raft snapshot index",
        ),
        (
            "semantic-tail-failure",
            snapshot_wire(
                metadata_payload("orders", 1, 50, semantic_failure_commands()),
                50,
            ),
            "node-left references absent",
        ),
    ];

    for (label, wire, expected_error) in cases {
        let error = runtime
            .step(wire)
            .expect_err("invalid inbound frame must fail before raft-rs dispatch");
        assert!(
            error.to_string().contains(expected_error),
            "case {label} returned unexpected error: {error}"
        );
        assert_eq!(runtime.snapshot(), runtime_before, "case {label}");
        assert_eq!(runtime.members(), members_before, "case {label}");
        let initial_state = store.initial_state()?;
        assert_eq!(
            initial_state.hard_state, initial_state_before.hard_state,
            "case {label}"
        );
        assert_eq!(
            initial_state.conf_state, initial_state_before.conf_state,
            "case {label}"
        );
        assert_eq!(store.snapshot(0, 3)?, snapshot_before, "case {label}");
        assert_eq!(store.retained_entries()?, entries_before, "case {label}");
        assert_eq!(store.applied_index()?, applied_before, "case {label}");
    }

    drop(runtime);
    drop(store);
    let reopened = SledRaftLogStore::open(directory.path())?;
    let reopened_initial_state = reopened.initial_state()?;
    assert_eq!(
        reopened_initial_state.hard_state,
        initial_state_before.hard_state
    );
    assert_eq!(
        reopened_initial_state.conf_state,
        initial_state_before.conf_state
    );
    assert_eq!(reopened.snapshot(0, 3)?, snapshot_before);
    assert_eq!(reopened.retained_entries()?, entries_before);
    assert_eq!(reopened.applied_index()?, applied_before);
    drop(reopened);
    drop(directory);
    Ok(())
}
