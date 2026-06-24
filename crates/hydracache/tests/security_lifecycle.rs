use hydracache::{
    AtRestSealer, CertificateBundle, CertificateRotationWindow, SecurityError,
    StaticAtRestKeyProvider, AT_REST_ARTIFACT_FORMAT_VERSION,
};

#[test]
fn security_lifecycle_at_rest_sealed_bytes_only_persisted() {
    let provider = StaticAtRestKeyProvider::new("k2", b"operator-secret".to_vec()).unwrap();
    let sealer = AtRestSealer::new(provider);

    let sealed = sealer.seal("snapshot", b"tenant-secret-profile").unwrap();

    assert_eq!(sealed.format_version, AT_REST_ARTIFACT_FORMAT_VERSION);
    assert_eq!(sealed.key_id, "k2");
    assert_ne!(sealed.ciphertext, b"tenant-secret-profile");
    assert_eq!(sealer.open(&sealed).unwrap(), b"tenant-secret-profile");
}

#[test]
fn security_lifecycle_undecryptable_artifact_is_rejected_not_served() {
    let provider = StaticAtRestKeyProvider::new("k2", b"operator-secret".to_vec()).unwrap();
    let sealer = AtRestSealer::new(provider);
    let mut sealed = sealer.seal("pitr-log", b"write:user:42").unwrap();

    sealed.ciphertext[0] ^= 0x80;

    assert_eq!(
        sealer.open(&sealed),
        Err(SecurityError::UndecryptableArtifact)
    );
}

#[test]
fn security_lifecycle_key_rotation_accepts_previous_for_reads_only() {
    let old_provider = StaticAtRestKeyProvider::new("k1", b"old-secret".to_vec()).unwrap();
    let old_sealer = AtRestSealer::new(old_provider);
    let old_artifact = old_sealer.seal("snapshot", b"before-rotation").unwrap();

    let rotated_provider = StaticAtRestKeyProvider::new("k2", b"new-secret".to_vec())
        .unwrap()
        .with_previous_key("k1", b"old-secret".to_vec())
        .unwrap();
    let rotated_sealer = AtRestSealer::new(rotated_provider);
    let new_artifact = rotated_sealer.seal("snapshot", b"after-rotation").unwrap();

    assert_eq!(
        rotated_sealer.open(&old_artifact).unwrap(),
        b"before-rotation"
    );
    assert_eq!(new_artifact.key_id, "k2");
}

#[test]
fn security_lifecycle_wrong_key_id_or_secret_fails_closed() {
    let provider = StaticAtRestKeyProvider::new("k2", b"operator-secret".to_vec()).unwrap();
    let sealer = AtRestSealer::new(provider);
    let mut sealed = sealer.seal("snapshot", b"secret").unwrap();

    sealed.key_id = "missing".to_owned();
    assert_eq!(
        sealer.open(&sealed),
        Err(SecurityError::UnknownKey("missing".to_owned()))
    );

    let wrong_provider = StaticAtRestKeyProvider::new("k2", b"wrong-secret".to_vec()).unwrap();
    let wrong_sealer = AtRestSealer::new(wrong_provider);
    sealed.key_id = "k2".to_owned();
    assert_eq!(
        wrong_sealer.open(&sealed),
        Err(SecurityError::UndecryptableArtifact)
    );
}

#[test]
fn security_lifecycle_cert_rotation_window_accepts_old_and_new() {
    let old = CertificateBundle::new("cert-old", "CN=member-a", 2_000).unwrap();
    let new = CertificateBundle::new("cert-new", "CN=member-a", 3_000).unwrap();
    let window = CertificateRotationWindow::new(old.clone()).promote(new.clone());

    assert_eq!(window.current_id(), "cert-new");
    assert!(window.accepts("cert-old", 1_000));
    assert!(window.accepts("cert-new", 1_000));
    assert!(!window.accepts("cert-old", 2_001));
    assert!(!window.accepts("unknown", 1_000));
}
