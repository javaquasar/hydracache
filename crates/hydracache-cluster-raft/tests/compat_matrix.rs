use std::fs;
use std::path::Path;
use std::process::Command;

use hydracache_cluster_raft::{RaftMetadataCommandEnvelope, RaftWireMessage};
use protobuf::Message as _;
use raft::eraftpb::{ConfState, MessageType, Snapshot};

fn vector(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/vectors")
            .join(name),
    )
    .unwrap()
}

#[test]
fn previous_release_raft_wire_and_snapshot_fixtures_decode_to_frozen_semantics() {
    let wire = RaftWireMessage {
        from: 1,
        to: 2,
        term: 3,
        payload: vector("raft_wire_message_payload.bin"),
    }
    .decode()
    .unwrap();
    assert_eq!(wire.get_msg_type(), MessageType::MsgAppend);
    assert_eq!((wire.from, wire.to, wire.term), (1, 2, 3));

    let conf = ConfState::parse_from_bytes(&vector("conf_state.bin")).unwrap();
    let snapshot = Snapshot::parse_from_bytes(&vector("snapshot_conf_state.bin")).unwrap();
    assert_eq!(conf.voters, vec![1, 2, 3]);
    assert_eq!(snapshot.get_metadata().index, 7);
    assert_eq!(snapshot.get_metadata().term, 4);
    assert_eq!(snapshot.get_metadata().get_conf_state().voters, conf.voters);
}

#[test]
fn unsupported_future_format_fails_loud_without_mutation() {
    let baseline = vector("member_upsert.bin");
    let decoded = RaftMetadataCommandEnvelope::decode(&baseline).unwrap();
    let error = RaftMetadataCommandEnvelope::decode(b"v99|future|member|x|1|1")
        .unwrap_err()
        .to_string();
    assert!(error.contains("invalid") || error.contains("unknown"));
    assert_eq!(
        RaftMetadataCommandEnvelope::decode(&baseline).unwrap(),
        decoded
    );
}

#[test]
fn current_release_emits_next_compat_fixture_manifest_without_overwriting_previous() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let previous = root.join("docs/testing/compat/v0.63.0.json");
    let previous_bytes = fs::read(&previous).unwrap();
    let commit = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(commit.status.success());
    let output = root.join("target/compat/v0.64.0-dev.json");
    fs::create_dir_all(output.parent().unwrap()).unwrap();
    fs::write(
        &output,
        serde_json::to_vec_pretty(&serde_json::json!({
            "producer_release": "0.64.0-dev",
            "producer_commit": String::from_utf8(commit.stdout).unwrap().trim(),
            "source_manifest": "docs/testing/compat/v0.63.0.json",
            "write_policy": "review-and-freeze-on-release"
        }))
        .unwrap(),
    )
    .unwrap();
    assert!(output.is_file());
    assert_eq!(fs::read(previous).unwrap(), previous_bytes);
}

#[test]
fn canary_compat_gate_silently_regenerates_a_changed_golden() {
    let fixture_changed = true;
    let manifest_hash_changed = false;
    let gate_accepted = std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W32");
    assert!(
        !(fixture_changed && !manifest_hash_changed && gate_accepted),
        "HC-CANARY-RED:W32 changed compatibility golden was silently accepted"
    );
}
