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
        let key = ReferenceCanonicalKey::new(
            vector["tenant"].as_str().unwrap(),
            vector["namespace"].as_str().unwrap(),
            vector["namespace_generation"].as_u64().unwrap(),
            decode_hex(vector["key_hex"].as_str().unwrap()),
        );
        assert_eq!(
            hex(&key.encode(ReferenceKeyBounds::default()).unwrap()),
            vector["encoded_hex"].as_str().unwrap(),
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
