// Included only inside the existing synthetic IO-binding test module.
#[test]
fn synthetic_material_lock_receipt_binds_supplied_bytes_and_both_mismatches_refuse() {
    use std::os::unix::fs::PermissionsExt;
    for wrong_receipt in [false, true] {
        let mut fixture = Fixture::new();
        assert!(fixture
            .root
            .canonicalize()
            .unwrap()
            .starts_with(fixture.temporary.path().canonicalize().unwrap()));
        let receipt_path = fixture.root.join("build-receipt-v1.json");
        let lock_path = fixture.root.join("Cargo.lock.root");
        let mut receipt: SignedBuild =
            serde_json::from_slice(&fs::read(&receipt_path).unwrap()).unwrap();
        assert_eq!(
            receipt.statement.root_lock,
            ArtifactDigest::of(&fs::read(&lock_path).unwrap())
        );
        assert_eq!(
            receipt.statement.root_lock.sha256,
            crate::diagnostic_artifacts::ROOT_LOCK_SHA256
        );
        assert_eq!(receipt.statement.root_lock.bytes, 175300);
        let mut pinned = fixture.material();
        if wrong_receipt {
            receipt.statement.root_lock = ArtifactDigest::of(b"foreign synthetic lock\n");
            let key = SigningKey::from_bytes(&[74; 32]);
            let message = signing_message(&receipt.statement).unwrap();
            let signature = key.sign(&message);
            key.verifying_key()
                .verify_strict(&message, &signature)
                .unwrap();
            receipt.signature_hex = crate::hex(&signature.to_bytes());
            let raw = receipt_bytes(&receipt).unwrap();
            fs::set_permissions(&receipt_path, fs::Permissions::from_mode(0o644)).unwrap();
            fs::write(&receipt_path, &raw).unwrap();
            fs::set_permissions(&receipt_path, fs::Permissions::from_mode(0o444)).unwrap();
            fixture.state.identity.build_provenance_sha256 = sha256_hex(&raw);
            // A valid signature cannot replace the independently pinned lock.
            assert!(matches!(
                fixture.policy.verify_receipt(&raw, &fixture.state.identity),
                Err(ArtifactError::Invalid)
            ));
        } else {
            fs::set_permissions(&lock_path, fs::Permissions::from_mode(0o644)).unwrap();
            fs::write(&lock_path, b"different supplied lock\n").unwrap();
            fs::set_permissions(&lock_path, fs::Permissions::from_mode(0o444)).unwrap();
            assert!(pinned.revalidate_for(&fixture.state).is_err());
            assert!(pinned.is_refused());
            assert!(pinned.revalidate_for(&fixture.state).is_err());
        }
        let (uid, gid) = unsafe { (libc::getuid(), libc::getgid()) };
        let result = prepare_fixture_start_material(
            &fixture.root,
            uid,
            gid,
            &fixture.state,
            &fixture.policy,
        );
        if wrong_receipt {
            assert!(matches!(result, Err(ArtifactError::Invalid)));
        } else {
            assert!(matches!(result, Err(ArtifactError::Contents)));
        }
    }
}
