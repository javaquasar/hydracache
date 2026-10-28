//! Reject structurally oversized operations before any original writer effects.
use super::*;

#[test]
fn oversized_rejection_preserves_writer() {
    for (key_len, value_len, mtu) in [
        (1usize, 65535, 65507),
        (32768, 32768, 65507),
        (65535, 1, 1000),
    ] {
        let node = ChitchatId::new("node".to_owned(), 1, "127.0.0.1:7280".parse().unwrap());
        let mut expected = DeltaSerializer::with_mtu(mtu);
        assert!(expected.try_add_node(node.clone(), 0, 0));
        let expected = expected.finish().serialize_to_vec();
        let mut writer = DeltaSerializer::with_mtu(mtu);
        assert!(writer.try_add_node(node, 0, 0));
        let rejected = !writer.try_add_kv(
            &"k".repeat(key_len),
            VersionedValue::for_test(&"v".repeat(value_len), 1),
        );
        assert!(rejected);
        let delta = writer.finish();
        let bytes = delta.serialize_to_vec();
        assert_eq!(bytes, expected);
        assert!(bytes.len() <= mtu);
        let mut cursor = bytes.as_slice();
        assert_eq!(Delta::deserialize(&mut cursor).unwrap(), delta);
        assert!(cursor.is_empty());
    }
}
