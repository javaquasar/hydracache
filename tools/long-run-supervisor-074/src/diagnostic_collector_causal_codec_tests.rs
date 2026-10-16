use super::*;
use serde_json::{json, Value};

fn fixture() -> Value {
    json!({"schema_version":CHALLENGE_SCHEMA,
        "identity":{"lease_id":"a".repeat(64),"binary_sha256":"b".repeat(64),"build_provenance_sha256":"c".repeat(64),"boot_id":"00000000-0000-0000-0000-000000000000"},
        "expected_state_revision":1,"controller_attempt_sha256":"d".repeat(64),"collector_reserved_sha256":"e".repeat(64),"reserved_state_sha256":"f".repeat(64),"random_hex":"0".repeat(64),"issued_boottime_ns":1})
}
fn line(v: &Value) -> Vec<u8> {
    let mut b = canonical_json(v).unwrap();
    b.push(b'\n');
    b
}

#[test]
fn collector_causal_codec_canonical_domain_bounds_and_unknowns_are_strict() {
    let v = fixture();
    let raw = line(&v);
    let nonce = collector_challenge_nonce(&raw).unwrap();
    let mut expected = CHALLENGE_DOMAIN.to_vec();
    expected.push(0);
    expected.extend_from_slice(&raw);
    assert_eq!(nonce, sha256_hex(&expected));
    assert_ne!(nonce, sha256_hex(&raw));
    assert_ne!(
        nonce,
        sha256_hex(&[b"other-domain\0".as_slice(), raw.as_slice()].concat())
    );
    for bad in [
        vec![],
        vec![b' '; MAX_CHALLENGE_BYTES + 1],
        raw[..raw.len() - 1].to_vec(),
        [raw.as_slice(), b"\n"].concat(),
        b"{}".to_vec(),
    ] {
        assert!(collector_challenge_nonce(&bad).is_err());
    }
    for case in 0..10 {
        let mut n = v.clone();
        match case {
            0 => n["schema_version"] = "future".into(),
            1 => n["expected_state_revision"] = 2.into(),
            2 => n["issued_boottime_ns"] = 0.into(),
            3 => n["random_hex"] = "G".repeat(64).into(),
            4 => n["collector_reserved_sha256"] = "a".repeat(63).into(),
            5 => n["identity"]["boot_id"] = "not-a-boot".into(),
            6 => n["unknown"] = true.into(),
            7 => n["identity"]["unknown"] = true.into(),
            8 => {
                n.as_object_mut().unwrap().remove("random_hex");
            }
            _ => n["random_hex"] = Value::Null,
        }
        assert!(collector_challenge_nonce(&line(&n)).is_err(), "case={case}");
    }
    let duplicate = String::from_utf8(raw.clone()).unwrap().replacen(
        '{',
        "{\"random_hex\":\"0000000000000000000000000000000000000000000000000000000000000000\",",
        1,
    );
    assert!(collector_challenge_nonce(duplicate.as_bytes()).is_err());
    // All-zero entropy is valid byte grammar, not proof of a system RNG observation.
    assert!(collector_challenge_nonce(&raw).is_ok());
}
#[test]
fn collector_causal_seeded_every_commitment_changes_the_signed_nonce() {
    let v = fixture();
    let original = collector_challenge_nonce(&line(&v)).unwrap();
    let mut seed = 0x0740c09d_u64;
    for i in 0..64 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut n = v.clone();
        let field = [
            "random_hex",
            "controller_attempt_sha256",
            "collector_reserved_sha256",
            "reserved_state_sha256",
        ][i % 4];
        let mut bytes = n[field].as_str().unwrap().as_bytes().to_vec();
        let index = seed as usize % 64;
        bytes[index] = if bytes[index] == b'a' { b'b' } else { b'a' };
        n[field] = String::from_utf8(bytes).unwrap().into();
        assert_ne!(collector_challenge_nonce(&line(&n)).unwrap(), original);
    }
    for (field, value) in [
        ("lease_id", "f".repeat(64)),
        ("binary_sha256", "f".repeat(64)),
        ("build_provenance_sha256", "f".repeat(64)),
        ("boot_id", "10000000-0000-0000-0000-000000000000".into()),
    ] {
        let mut n = v.clone();
        n["identity"][field] = value.into();
        assert_ne!(collector_challenge_nonce(&line(&n)).unwrap(), original);
    }
    let mut n = v;
    n["issued_boottime_ns"] = 2.into();
    assert_ne!(collector_challenge_nonce(&line(&n)).unwrap(), original);
    println!("collector_causal seed=0x0740c09d commitment_mutations=69 byte_inspector_is_authority=false");
}
#[test]
fn collector_causal_exact_issuance_deadline_never_renews() {
    let limit = crate::diagnostic_lease::CONTROLLER_LOSS_SECONDS * 1_000_000_000;
    for (issued, now, ok) in [
        (1, 1, true),
        (1, limit, true),
        (1, limit + 1, false),
        (1, 0, false),
        (0, 0, false),
        (u64::MAX, u64::MAX, true),
    ] {
        assert_eq!(age(issued, now).is_ok(), ok);
    }
}
