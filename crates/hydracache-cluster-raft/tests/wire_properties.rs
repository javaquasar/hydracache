use std::panic::{catch_unwind, AssertUnwindSafe};

use hydracache_cluster_raft::{RaftMetadataRuntime, RaftWireMessage};
use proptest::prelude::*;
use raft::eraftpb::{Message, MessageType};

#[test]
fn raft_wire_envelope_must_match_protobuf_header() {
    let mut message = Message {
        from: 1,
        to: 2,
        term: 3,
        ..Message::default()
    };
    message.set_msg_type(MessageType::MsgHeartbeat);
    let valid = RaftWireMessage::encode(&message).unwrap();

    for (field, forged) in [
        (
            "from",
            RaftWireMessage {
                from: 9,
                ..valid.clone()
            },
        ),
        (
            "to",
            RaftWireMessage {
                to: 9,
                ..valid.clone()
            },
        ),
        (
            "term",
            RaftWireMessage {
                term: 9,
                ..valid.clone()
            },
        ),
    ] {
        let error = forged.decode().unwrap_err();
        assert!(error.to_string().contains(&format!("{field} mismatch")));
    }
    assert_eq!(valid.decode().unwrap(), message);
}

#[test]
fn runtime_rejects_a_valid_frame_addressed_to_another_node_without_mutation() {
    let runtime = RaftMetadataRuntime::single_node("orders", 1).unwrap();
    let before = runtime.snapshot();
    let mut message = Message {
        from: 2,
        to: 3,
        term: before.term,
        ..Message::default()
    };
    message.set_msg_type(MessageType::MsgHeartbeat);

    let error = runtime
        .step(RaftWireMessage::encode(&message).unwrap())
        .unwrap_err();

    assert!(error.to_string().contains("does not match runtime node 1"));
    assert_eq!(runtime.snapshot(), before);
}

proptest! {
    #[test]
    fn raft_wire_message_decode_never_panics(
        from in 1_u64..u64::MAX,
        to in 1_u64..u64::MAX,
        term in 0_u64..u64::MAX,
        payload in proptest::collection::vec(any::<u8>(), 0..1024)
    ) {
        let wire = RaftWireMessage { from, to, term, payload };

        let decoded = catch_unwind(AssertUnwindSafe(|| wire.decode()));

        prop_assert!(decoded.is_ok());
    }
}
