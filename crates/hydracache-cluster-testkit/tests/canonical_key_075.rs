use hydracache_cluster_testkit::canonical_key_075::{
    ReferenceCanonicalKey, ReferenceKeyBounds, ReferenceKeyError,
};
use serde_json::Value;
use std::fs;
use std::path::Path;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn ascii_and_empty_key_have_a_stable_reference_frame() {
    let key = ReferenceCanonicalKey::new("tenant-a", "orders", 1, Vec::new());
    let encoded = key.encode(ReferenceKeyBounds::default()).unwrap();
    assert_eq!(
        hex(&encoded),
        "4843524b303735000000000874656e616e742d61000000066f7264657273000000000000000100000000"
    );
    assert_eq!(
        ReferenceCanonicalKey::decode(&encoded, ReferenceKeyBounds::default()).unwrap(),
        key
    );
}

#[test]
fn utf8_is_preserved_without_unicode_normalization() {
    let composed = ReferenceCanonicalKey::new("é", "名字", 42, b"k".to_vec());
    let decomposed = ReferenceCanonicalKey::new("e\u{301}", "名字", 42, b"k".to_vec());
    let composed = composed.encode(ReferenceKeyBounds::default()).unwrap();
    let decomposed = decomposed.encode(ReferenceKeyBounds::default()).unwrap();
    assert_eq!(
        hex(&composed),
        "4843524b3037350000000002c3a900000006e5908de5ad97000000000000002a000000016b"
    );
    assert_ne!(composed, decomposed);
}

#[test]
fn hostile_lengths_are_rejected_before_slice_or_allocation() {
    let bounds = ReferenceKeyBounds {
        max_tenant_bytes: 4,
        max_namespace_bytes: 4,
        max_key_bytes: 4,
        max_frame_bytes: 64,
    };
    assert_eq!(
        ReferenceCanonicalKey::new("tenant", "map", 1, vec![]).encode(bounds),
        Err(ReferenceKeyError::FieldTooLarge {
            field: "tenant",
            limit: 4,
            actual: 6
        })
    );

    let mut malicious = b"HCRK075\0".to_vec();
    malicious.extend_from_slice(&u32::MAX.to_be_bytes());
    assert_eq!(
        ReferenceCanonicalKey::decode(&malicious, bounds),
        Err(ReferenceKeyError::FieldTooLarge {
            field: "tenant",
            limit: 4,
            actual: u32::MAX as usize
        })
    );
}

#[test]
fn generation_and_domain_are_mandatory() {
    assert_eq!(
        ReferenceCanonicalKey::new("tenant", "map", 0, vec![])
            .encode(ReferenceKeyBounds::default()),
        Err(ReferenceKeyError::ZeroGeneration)
    );
    let mut encoded = ReferenceCanonicalKey::new("tenant", "map", 1, vec![])
        .encode(ReferenceKeyBounds::default())
        .unwrap();
    encoded[0] ^= 0xff;
    assert_eq!(
        ReferenceCanonicalKey::decode(&encoded, ReferenceKeyBounds::default()),
        Err(ReferenceKeyError::MalformedFrame)
    );
}

#[test]
fn binary_frame_rejects_truncation_and_trailing_data() {
    let bounds = ReferenceKeyBounds::default();
    let key = ReferenceCanonicalKey::new("tenant-a", "orders", 1, vec![0, 0xff, 0x80, 0]);
    let mut frame = key.encode(bounds).unwrap();
    assert_eq!(ReferenceCanonicalKey::decode(&frame, bounds).unwrap(), key);
    for length in 0..frame.len() {
        assert_eq!(
            ReferenceCanonicalKey::decode(&frame[..length], bounds),
            Err(ReferenceKeyError::MalformedFrame),
            "accepted a truncated frame at {length} bytes"
        );
    }
    frame.push(0);
    assert_eq!(
        ReferenceCanonicalKey::decode(&frame, bounds),
        Err(ReferenceKeyError::MalformedFrame)
    );
}

#[test]
fn bounded_frame_accepts_the_exact_limit_and_reports_one_byte_over() {
    let key = ReferenceCanonicalKey::new("é", "map", 42, vec![0, 0xff]);
    let frame = key.encode(ReferenceKeyBounds::default()).unwrap();
    let exact = ReferenceKeyBounds {
        max_tenant_bytes: 2,
        max_namespace_bytes: 3,
        max_key_bytes: 2,
        max_frame_bytes: frame.len(),
    };
    assert_eq!(key.encode(exact).unwrap(), frame);
    assert_eq!(ReferenceCanonicalKey::decode(&frame, exact).unwrap(), key);

    let smaller = ReferenceKeyBounds {
        max_frame_bytes: frame.len() - 1,
        ..exact
    };
    let too_large = ReferenceKeyError::FrameTooLarge {
        limit: frame.len() - 1,
        actual: frame.len(),
    };
    assert_eq!(key.encode(smaller), Err(too_large.clone()));
    assert_eq!(
        ReferenceCanonicalKey::decode(&frame, smaller),
        Err(too_large)
    );
}

#[test]
fn rejects_invalid_utf8_in_names_but_preserves_the_same_bytes_in_the_key() {
    let bounds = ReferenceKeyBounds::default();
    let key = ReferenceCanonicalKey::new("t", "n", 1, vec![0xff]);
    let mut frame = key.encode(bounds).unwrap();
    assert_eq!(ReferenceCanonicalKey::decode(&frame, bounds).unwrap(), key);

    let tenant_start = b"HCRK075\0".len() + 4;
    let namespace_start = tenant_start + key.tenant.len() + 4;
    frame[tenant_start] = 0xff;
    assert_eq!(
        ReferenceCanonicalKey::decode(&frame, bounds),
        Err(ReferenceKeyError::InvalidUtf8("tenant"))
    );
    frame[tenant_start] = b't';
    frame[namespace_start] = 0xff;
    assert_eq!(
        ReferenceCanonicalKey::decode(&frame, bounds),
        Err(ReferenceKeyError::InvalidUtf8("namespace"))
    );
}

#[test]
fn boundaries_and_namespace_incarnation_distinguish_identical_key_bytes() {
    let bounds = ReferenceKeyBounds::default();
    let bytes = vec![0, b':', 0xff];
    let first = ReferenceCanonicalKey::new("ab", "c", 1, bytes.clone());
    let other_names = ReferenceCanonicalKey::new("a", "bc", 1, bytes.clone());
    let recreated = ReferenceCanonicalKey::new("ab", "c", 2, bytes);
    let frame = first.encode(bounds).unwrap();
    assert_ne!(frame, other_names.encode(bounds).unwrap());
    assert_ne!(frame, recreated.encode(bounds).unwrap());
}

#[test]
fn checked_in_cross_language_vectors_are_generated_by_the_rust_reference() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap();
    let value: Value = serde_json::from_slice(
        &fs::read(root.join("docs/testing/imap/0.75/canonical-key-vectors.json")).unwrap(),
    )
    .unwrap();
    for vector in value["vectors"].as_array().unwrap() {
        let bounds = ReferenceKeyBounds::default();
        let key = ReferenceCanonicalKey::new(
            vector["tenant"].as_str().unwrap(),
            vector["namespace"].as_str().unwrap(),
            vector["namespace_generation"].as_u64().unwrap(),
            decode_hex(vector["key_hex"].as_str().unwrap()),
        );
        let encoded = vector["encoded_hex"].as_str().unwrap();
        assert_eq!(
            hex(&key.encode(bounds).unwrap()),
            encoded,
            "vector {}",
            vector["id"].as_str().unwrap()
        );
        assert_eq!(
            ReferenceCanonicalKey::decode(&decode_hex(encoded), bounds).unwrap(),
            key,
            "vector {}",
            vector["id"].as_str().unwrap()
        );
    }
}

fn decode_hex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair).unwrap();
            u8::from_str_radix(text, 16).unwrap()
        })
        .collect()
}

#[test]
fn provisional_fnv_matches_shared_unsigned_partition_vectors() {
    let vectors =
        include_str!("../../../docs/testing/imap/0.75/partition-hash-fnv1a64-vectors.tsv");
    for row in vectors.lines().skip(1) {
        let parts: Vec<_> = row.split('\t').collect();
        let [id, encoded, hash, partition] = parts.as_slice() else {
            panic!("unexpected vector {row}");
        };
        let frame = decode_hex(encoded);
        let bounds = ReferenceKeyBounds::default();
        let key = ReferenceCanonicalKey::decode(&frame, bounds).unwrap();
        assert_eq!(key.encode(bounds).unwrap(), frame, "{id}");
        let observed = hydracache::fnv1a64_bytes(&frame);
        assert_eq!(observed, u64::from_str_radix(hash, 16).unwrap(), "{id}");
        assert_eq!(observed % 271, partition.parse::<u64>().unwrap(), "{id}");
    }
}

#[test]
fn records_provisional_fnv_distribution_for_three_key_shapes() {
    let bounds = ReferenceKeyBounds::default();
    let mut counts = [[0_u32; 271]; 3];
    for key in 0_u64..100_000 {
        let payloads = [
            key.to_string().into_bytes(),
            key.to_be_bytes().to_vec(),
            format!("order:{key:06}").into_bytes(),
        ];
        for (counts, payload) in counts.iter_mut().zip(payloads) {
            let key = ReferenceCanonicalKey::new("tenant-a", "orders", 1, payload);
            let hash = hydracache::fnv1a64_bytes(&key.encode(bounds).unwrap());
            counts[(hash % 271) as usize] += 1;
        }
    }
    for (counts, (min, max)) in counts.iter().zip([(322, 413), (365, 373), (321, 428)]) {
        assert_eq!(counts.iter().sum::<u32>(), 100_000);
        assert_eq!(counts.iter().min().copied(), Some(min));
        assert_eq!(counts.iter().max().copied(), Some(max));
    }
}
