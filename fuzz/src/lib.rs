use hydracache_client_protocol::{ClientFrame, ClientWireMessage, VersionHandshake};
use hydracache_redis_compat::{
    decode_resp2_command_with_limits, decode_resp3_command_with_limits, RespDecodeLimits,
};
use hydracache_server::config::ServerConfig;
use protobuf::Message;
use raft::eraftpb::Snapshot;

const MAX_FUZZ_INPUT_BYTES: usize = 16 * 1024;

pub fn fuzz_config_parse(data: &[u8]) {
    if data.len() > MAX_FUZZ_INPUT_BYTES {
        return;
    }
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let _ = ServerConfig::from_toml_str(text);
}

pub fn fuzz_kv_codec(data: &[u8]) {
    if data.len() > MAX_FUZZ_INPUT_BYTES {
        return;
    }
    if data == b"handshake" {
        let message = ClientWireMessage::Handshake(VersionHandshake::default());
        let frame = ClientFrame::from_message(&message).expect("handshake frame should encode");
        let encoded = frame.encode().expect("handshake frame should serialize");
        let decoded = ClientFrame::decode(&encoded, MAX_FUZZ_INPUT_BYTES)
            .expect("encoded handshake frame should decode");
        assert_eq!(decoded.decode_message().unwrap(), message);
        return;
    }

    if let Ok(frame) = ClientFrame::decode(data, MAX_FUZZ_INPUT_BYTES) {
        let encoded = frame.encode().expect("decoded frame should re-encode");
        let decoded = ClientFrame::decode(&encoded, MAX_FUZZ_INPUT_BYTES)
            .expect("re-encoded frame should decode");
        assert_eq!(decoded.protocol_version(), frame.protocol_version());
        assert_eq!(decoded.payload(), frame.payload());
        let _ = decoded.decode_message();
    }
}

pub fn fuzz_resp_command(data: &[u8]) {
    if data.len() > MAX_FUZZ_INPUT_BYTES {
        return;
    }
    let limits = RespDecodeLimits {
        max_frame_bytes: MAX_FUZZ_INPUT_BYTES,
        max_array_elements: 64,
        max_bulk_string_bytes: 4096,
    };
    let _ = decode_resp2_command_with_limits(data, limits);
    let _ = decode_resp3_command_with_limits(data, limits);
}

pub fn fuzz_snapshot_decode(data: &[u8]) {
    if data.len() > MAX_FUZZ_INPUT_BYTES {
        return;
    }
    if data == b"empty-snapshot" {
        let snapshot = Snapshot::new();
        let encoded = snapshot
            .write_to_bytes()
            .expect("empty snapshot should encode");
        let decoded = Snapshot::parse_from_bytes(&encoded).expect("empty snapshot should decode");
        assert!(decoded.get_metadata().get_index() == 0);
        return;
    }
    let _ = Snapshot::parse_from_bytes(data);
}
