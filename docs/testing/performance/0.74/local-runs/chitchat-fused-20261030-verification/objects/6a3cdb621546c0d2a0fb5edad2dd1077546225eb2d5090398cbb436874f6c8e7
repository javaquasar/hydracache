use super::*;

#[test]
fn borrowed_delta_fields_equal_original_serializer() {
    let nodes = [
        ChitchatId {
            node_id: "ёж🙂".into(),
            generation_id: u64::MAX,
            gossip_advertise_addr: "[fe80::1%7]:65535".parse().unwrap(),
        },
        ChitchatId {
            node_id: "".into(),
            generation_id: 0,
            gossip_advertise_addr: "127.0.0.1:0".parse().unwrap(),
        },
    ];
    for id in nodes {
        for (gc, from) in [(0, 0), (u64::MAX, u64::MAX), (17, 29)] {
            let op = DeltaOp::Node {
                chitchat_id: id.clone(),
                last_gc_version: gc,
                from_version_excluded: from,
            };
            check(&op);
        }
    }
    for status in [
        DeletionStatusMutation::Set,
        DeletionStatusMutation::Delete,
        DeletionStatusMutation::DeleteAfterTtl,
    ] {
        for (key, value) in [
            ("".to_string(), "".to_string()),
            ("🙂".into(), "ёж\0".into()),
            ("k".into(), "v".repeat(65519)),
        ] {
            check(&DeltaOp::KeyValue(KeyValueMutation {
                key,
                value,
                version: u64::MAX,
                status,
            }));
        }
    }
    for max_version in [0, 1, u64::MAX] {
        check(&DeltaOp::SetMaxVersion { max_version });
    }
    fn check(op: &DeltaOp) {
        let mut projected = Vec::new();
        op.visit(&mut |v| projected.extend_from_slice(v));
        assert_eq!(projected, op.serialize_to_vec());
        assert_eq!(projected.len(), op.serialized_len());
    }
}
