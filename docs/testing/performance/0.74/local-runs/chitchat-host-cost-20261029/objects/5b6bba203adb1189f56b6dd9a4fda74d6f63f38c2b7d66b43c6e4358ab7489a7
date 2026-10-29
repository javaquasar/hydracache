//! Supported DeltaSerializer calls, without live clocks or network activity.
use super::*;
use serde_json::json;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn value(len: usize) -> String {
    let mut seed = 0x0741028_u64;
    (0..len)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            char::from(32 + (seed % 95) as u8)
        })
        .collect()
}

#[test]
fn supported_delta_operations() {
    let mut rows = Vec::new();
    for mtu in [100usize, 140, 198, 1000, 16384, 32768, 40000, 65507] {
        let mut lengths = vec![0, 1, 64, 4096, 16384, 32700, 65000];
        lengths.extend((48..=70).map(|overhead| mtu.saturating_sub(overhead)));
        lengths.sort_unstable();
        lengths.dedup();
        for len in lengths {
            let node = ChitchatId::new("node".to_owned(), 1, "127.0.0.1:7280".parse().unwrap());
            let mut serializer = DeltaSerializer::with_mtu(mtu);
            assert!(serializer.try_add_node(node.clone(), 0, 0));
            let text = value(len);
            let mutation = KeyValueMutation {
                key: "k".to_owned(),
                value: text.clone(),
                version: 1,
                status: DeletionStatusMutation::Set,
            };
            let op = DeltaOp::KeyValue(mutation);
            let bound = serializer
                .compressed_stream_writer
                .serialized_len_upperbound_after(&op);
            let accepted = serializer.try_add_kv("k", VersionedValue::for_test(&text, 1));
            assert_eq!(accepted, bound <= mtu);
            let delta = serializer.finish();
            let bytes = delta.serialize_to_vec();
            assert_eq!(bytes.len(), delta.serialized_len());
            assert!(bytes.len() <= mtu);
            let mut cursor = bytes.as_slice();
            let decoded = Delta::deserialize(&mut cursor).unwrap();
            assert_eq!(decoded, delta);
            assert!(cursor.is_empty());
            assert_eq!(delta.node_deltas.len(), 1);
            assert_eq!(delta.node_deltas[0].key_values.len(), usize::from(accepted));
            rows.push(
                json!({"mtu":mtu,"value_len":len,"bound":bound,"accepted":accepted,
                "wire":hex(&bytes),"actual":bytes.len(),"remaining":cursor.len()}),
            );
        }
    }
    println!(
        "HC074_SUPPORTED_DELTA:{}",
        json!({"seed":0x0741028_u64,"cases":rows})
    );
}
