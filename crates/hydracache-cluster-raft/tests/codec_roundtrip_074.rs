use raft::eraftpb::{ConfState, Message, Snapshot};
use raft::protocompat::PbMessage;

fn assert_stable_encoding<M: PbMessage + Default>(bytes: &[u8]) {
    let message = M::decode(bytes).unwrap();
    assert_eq!(message.encode_to_vec(), bytes);
}

#[test]
fn rafts_frozen_messages_and_snapshots_reencode_byte_for_byte() {
    assert_stable_encoding::<Message>(include_bytes!("vectors/raft_wire_message_payload.bin"));
    assert_stable_encoding::<ConfState>(include_bytes!("vectors/conf_state.bin"));
    assert_stable_encoding::<Snapshot>(include_bytes!("vectors/snapshot_conf_state.bin"));
}
