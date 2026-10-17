use hydracache_cluster_testkit::value_plane_security_075::{
    fingerprint, validate_replica_proof, validate_route, RedactedAuditLog, ReplayDisposition,
    ReplayGuard, ReplayIdentity, ReplicaProofClaims, RouteClaims, SecurityBounds, SecurityError,
    TrustEpochWindow,
};

fn valid_claims<'a>() -> RouteClaims<'a> {
    RouteClaims {
        authenticated: true,
        authenticated_tenant: "tenant-a",
        claimed_tenant: "tenant-a",
        expected_namespace_generation: 2,
        actual_namespace_generation: 2,
        expected_topology_epoch: 7,
        actual_topology_epoch: 7,
        proxy_hops: 1,
        deadline_remaining_ticks: 3,
        trust_epoch: 4,
        encoded_envelope_bytes: 512,
        key_bytes: 32,
        value_bytes: 128,
    }
}

fn trust() -> TrustEpochWindow {
    TrustEpochWindow::new(4, 5).unwrap()
}

#[test]
fn authenticated_tenant_cannot_be_substituted_by_payload() {
    let mut claims = valid_claims();
    claims.claimed_tenant = "tenant-b";
    assert_eq!(
        validate_route(&claims, SecurityBounds::default(), trust(), 10),
        Err(SecurityError::TenantSubstitution)
    );
}

#[test]
fn replay_requires_same_tenant_identity_and_payload_digest() {
    let identity = ReplayIdentity {
        tenant: "tenant-a".into(),
        client: "client-a".into(),
        request: 9,
    };
    let mut guard = ReplayGuard::new(1).unwrap();
    assert_eq!(
        guard.admit(identity.clone(), b"put:k:v1"),
        Ok(ReplayDisposition::New)
    );
    assert_eq!(
        guard.admit(identity.clone(), b"put:k:v1"),
        Ok(ReplayDisposition::ExactReplay)
    );
    assert_eq!(
        guard.admit(identity, b"put:k:v2"),
        Err(SecurityError::ReplayDigestConflict)
    );
}

#[test]
fn false_backup_ack_fails_exact_claim_validation() {
    let expected = fingerprint(b"record-v7");
    let mut wrong = expected;
    wrong[0] ^= 0xff;
    assert_eq!(
        validate_replica_proof(ReplicaProofClaims {
            authenticated: true,
            expected_partition: 3,
            partition: 3,
            expected_epoch: 7,
            epoch: 7,
            expected_version: 11,
            version: 11,
            expected_checksum: expected,
            checksum: wrong,
        }),
        Err(SecurityError::InvalidReplicaProof)
    );
}

#[test]
fn redirect_loop_and_stale_generation_are_rejected_before_decode() {
    let mut claims = valid_claims();
    claims.proxy_hops = 2;
    assert_eq!(
        validate_route(&claims, SecurityBounds::default(), trust(), 10),
        Err(SecurityError::RedirectLoop {
            limit: 1,
            actual: 2
        })
    );
    let mut claims = valid_claims();
    claims.actual_namespace_generation = 1;
    assert_eq!(
        validate_route(&claims, SecurityBounds::default(), trust(), 10),
        Err(SecurityError::StaleNamespaceGeneration {
            expected: 2,
            actual: 1
        })
    );
}

#[test]
fn trust_rotation_has_a_finite_overlap_window() {
    let mut window = TrustEpochWindow::new(4, 5).unwrap();
    window.rotate(5, 10).unwrap();
    assert!(window.accepts(4, 15));
    assert!(!window.accepts(4, 16));
    assert!(window.accepts(5, 100));
    let mut claims = valid_claims();
    claims.trust_epoch = 4;
    assert_eq!(
        validate_route(&claims, SecurityBounds::default(), window, 16),
        Err(SecurityError::UnknownTrustEpoch(4))
    );
}

#[test]
fn audit_log_retains_only_fixed_length_fingerprints() {
    let tenant = b"secret-tenant";
    let namespace = b"secret-namespace";
    let key = b"secret-key";
    let mut log = RedactedAuditLog::new(1).unwrap();
    log.record(tenant, namespace, key, "put", "acknowledged")
        .unwrap();
    let record = log.records().next().unwrap();
    assert_eq!(record.tenant_fingerprint, fingerprint(tenant));
    let debug = format!("{record:?}");
    assert!(!debug.contains("secret-tenant"));
    assert!(!debug.contains("secret-namespace"));
    assert!(!debug.contains("secret-key"));
    assert_eq!(
        log.record(tenant, namespace, key, "put", "acknowledged"),
        Err(SecurityError::AuditCapacityExhausted)
    );
}

#[test]
fn hostile_declared_lengths_fail_before_allocation() {
    let bounds = SecurityBounds {
        max_envelope_bytes: 64,
        max_key_bytes: 8,
        max_value_bytes: 16,
        ..SecurityBounds::default()
    };
    let mut claims = valid_claims();
    claims.encoded_envelope_bytes = 65;
    claims.key_bytes = usize::MAX;
    claims.value_bytes = usize::MAX;
    assert_eq!(
        validate_route(&claims, bounds, trust(), 10),
        Err(SecurityError::EnvelopeTooLarge {
            limit: 64,
            actual: 65
        })
    );
}

#[test]
fn security_collections_fail_closed_at_capacity() {
    let mut guard = ReplayGuard::new(1).unwrap();
    guard
        .admit(
            ReplayIdentity {
                tenant: "a".into(),
                client: "c".into(),
                request: 1,
            },
            b"one",
        )
        .unwrap();
    assert_eq!(
        guard.admit(
            ReplayIdentity {
                tenant: "a".into(),
                client: "c".into(),
                request: 2,
            },
            b"two"
        ),
        Err(SecurityError::ReplayCapacityExhausted)
    );
}
