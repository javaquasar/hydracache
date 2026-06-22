use hydracache::{
    prepare_replicated_payload, HydraCache, RedactReplicatedValue, ReplicatedValueSecurityPosture,
    Replication, ReplicationCryptoError, ReplicationKeyProvider,
};

#[derive(Debug)]
struct XorProvider(u8);

impl ReplicationKeyProvider for XorProvider {
    fn seal(&self, plaintext: &[u8]) -> Result<Vec<u8>, ReplicationCryptoError> {
        Ok(plaintext.iter().map(|byte| byte ^ self.0).collect())
    }

    fn open(&self, sealed: &[u8]) -> Result<Vec<u8>, ReplicationCryptoError> {
        Ok(sealed.iter().map(|byte| byte ^ self.0).collect())
    }
}

struct PrefixRedactor;

impl RedactReplicatedValue for PrefixRedactor {
    fn redact(&self, plaintext: &[u8]) -> Vec<u8> {
        plaintext
            .iter()
            .copied()
            .take_while(|byte| *byte != b':')
            .collect()
    }
}

#[test]
fn local_only_value_is_cached_but_never_replicated() {
    let payload = prepare_replicated_payload(
        b"secret-profile",
        Replication::LocalOnly,
        false,
        None,
        None,
    )
    .expect("prepare payload");

    assert!(payload.is_none());
}

#[test]
fn encrypted_roundtrip_seals_and_opens() {
    let provider = XorProvider(0xaa);

    let payload = prepare_replicated_payload(
        b"profile-bytes",
        Replication::Eligible,
        false,
        Some(&provider),
        None,
    )
    .expect("prepare payload")
    .expect("eligible payload");

    assert_eq!(payload.posture, ReplicatedValueSecurityPosture::Encrypted);
    assert_ne!(payload.bytes, b"profile-bytes");
    assert_eq!(provider.open(&payload.bytes).unwrap(), b"profile-bytes");
}

#[test]
fn redaction_strips_fields_before_send() {
    let payload = prepare_replicated_payload(
        b"public:ssn=123",
        Replication::Eligible,
        true,
        None,
        Some(&PrefixRedactor),
    )
    .expect("prepare payload")
    .expect("eligible payload");

    assert_eq!(payload.bytes, b"public");
    assert_eq!(
        payload.posture,
        ReplicatedValueSecurityPosture::PlaintextAcknowledged
    );
}

#[test]
fn plaintext_without_ack_is_flagged_in_readiness() {
    let cache = HydraCache::local()
        .replicate_values(true)
        .max_replicated_entry_bytes(1024)
        .build();

    assert_eq!(
        cache.replicated_value_security_posture(),
        ReplicatedValueSecurityPosture::PlaintextUnacknowledged
    );
    assert!(cache
        .cluster_pilot_report()
        .highlights
        .contains(&"REPLICATED VALUES PLAINTEXT".to_owned()));
}

#[test]
fn plaintext_ack_removes_readiness_highlight() {
    let cache = HydraCache::local()
        .replicate_values(true)
        .max_replicated_entry_bytes(1024)
        .acknowledge_plaintext_replicated_values(true)
        .build();

    assert_eq!(
        cache.replicated_value_security_posture(),
        ReplicatedValueSecurityPosture::PlaintextAcknowledged
    );
    assert!(!cache
        .cluster_pilot_report()
        .highlights
        .contains(&"REPLICATED VALUES PLAINTEXT".to_owned()));
}
