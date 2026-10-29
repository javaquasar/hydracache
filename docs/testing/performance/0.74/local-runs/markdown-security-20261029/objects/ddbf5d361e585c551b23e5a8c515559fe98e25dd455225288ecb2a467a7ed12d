//! Frozen encoder reference and byte/binding equivalence, never enrollment.
use super::*;
use ed25519_dalek::SigningKey;

#[derive(Clone, Copy)]
pub(crate) enum Encoder {
    Reference,
    Proposed,
    Actual,
}
pub(crate) fn fixture_trust() -> WorkerPolicyTrust {
    super::tests::checked(7).trust
}
fn reference(trust: &WorkerPolicyTrust) -> String {
    trust
        .key
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
pub(crate) fn encode(trust: &WorkerPolicyTrust, mode: Encoder) -> String {
    match mode {
        Encoder::Reference => reference(trust),
        Encoder::Proposed => crate::hex(trust.key.as_bytes()),
        Encoder::Actual => key_hex(trust),
    }
}
fn reference_bind(
    record: &Record,
    trust: &WorkerPolicyTrust,
    host: &AssertedWorkerHost,
) -> Result<(), AuthorityError> {
    if record.issuer_key_hex != reference(trust)
        || record.policy_sha256 != trust.policy_sha256
        || record.policy_epoch != trust.epoch
        || record.context() != *host
    {
        return Err(AuthorityError::Binding);
    }
    Ok(())
}
#[test]
fn authority_hex_seeded_keys_preserve_lowercase_bytes_canonical_records_and_digests() {
    let mut seed = 0x074a_17e5_u64;
    let mut nibbles = 0_u16;
    let mut policy = super::tests::checked(7);
    for sample in 0..256 {
        let mut bytes = [sample as u8; 32];
        if sample >= 2 {
            for byte in &mut bytes {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                *byte = seed as u8;
            }
        }
        policy.trust.key = SigningKey::from_bytes(&bytes).verifying_key();
        let old = encode(&policy.trust, Encoder::Reference);
        for mode in [Encoder::Proposed, Encoder::Actual] {
            assert_eq!(encode(&policy.trust, mode), old);
        }
        assert_eq!(old.len(), 64);
        assert!(old
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
        for byte in policy.trust.key.as_bytes() {
            nibbles |= 1 << (byte >> 4);
            nibbles |= 1 << (byte & 15);
        }
        let actual = Record::from_checked(&policy).unwrap();
        let mut expected = actual.clone();
        expected.issuer_key_hex = old;
        let a = actual.bytes().unwrap();
        let b = expected.bytes().unwrap();
        assert_eq!(a, b);
        assert_eq!(sha256_hex(&a), sha256_hex(&b));
        assert_eq!(Record::parse(&a).unwrap(), expected);
    }
    assert_eq!(nibbles, u16::MAX);
    assert_eq!(key_hex(&fixture_trust()), reference(&fixture_trust()));
    eprintln!("authority hex equivalence seed=0x074a17e5 samples=256 all_nibbles=true");
}
#[test]
fn authority_hex_preserves_independent_pin_and_context_binding_refusals() {
    let policy = super::tests::checked(7);
    for case in 0..13 {
        let mut record = Record::from_checked(&policy).unwrap();
        let mut trust = policy.trust.clone();
        let mut host = policy.context.clone();
        match case {
            0 => (),
            1 => record.issuer_key_hex.make_ascii_uppercase(),
            2 => record.issuer_key_hex = "d".repeat(64),
            3 => trust.key = SigningKey::from_bytes(&[75; 32]).verifying_key(),
            4 => trust.policy_sha256 = "e".repeat(64),
            5 => trust.epoch += 1,
            6 => host.machine_id = "f".repeat(32),
            7 => host.boot_id = "00000000-0000-0000-0000-000000000002".into(),
            8 => host.user_namespace.device += 1,
            9 => host.user_namespace.inode += 1,
            10 => host.mount_namespace.device += 1,
            11 => host.mount_namespace.inode += 1,
            _ => record.issuer_key_hex.truncate(63),
        }
        let actual = record.bind(&trust, &host);
        let reference = reference_bind(&record, &trust, &host);
        assert_eq!(actual.is_ok(), case == 0);
        assert_eq!(actual.is_ok(), reference.is_ok());
        if case != 0 {
            assert!(matches!(actual, Err(AuthorityError::Binding)));
            assert!(matches!(reference, Err(AuthorityError::Binding)));
        }
    }
}
