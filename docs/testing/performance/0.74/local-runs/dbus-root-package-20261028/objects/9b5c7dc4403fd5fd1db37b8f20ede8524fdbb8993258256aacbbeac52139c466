//! Owned local fixtures and the already public synthetic controller key only.
use super::*;

fn key_file(dir: &Path) -> (std::path::PathBuf, SigningKey) {
    let key = SigningKey::from_bytes(&[76; 32]);
    let path = dir.join("synthetic-controller.hex");
    fs::write(&path, crate::hex(&key.to_bytes())).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    (path, key)
}
fn verify_output(path: &Path, original: &DiagnosticRequest, audience: &str, key: &SigningKey) {
    let bytes = fs::read(path).unwrap();
    let signed: SignedDiagnosticRequest = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(&signed.request, original);
    assert_eq!(line(&signed), bytes);
    let hash = crate::sha256_hex(&crate::canonical_json(&signed.request).unwrap());
    assert_eq!(
        audience,
        format!("urn:hydracache:diagnostic:074:request:{hash}")
    );
    let checked = crate::diagnostic_ipc::authenticate(
        &bytes,
        &crate::diagnostic_ipc::DiagnosticPeer {
            uid: 1000,
            gids: vec![1000],
        },
        &crate::diagnostic_ipc::DiagnosticPolicy {
            repository_id: original.controller.repository_id,
            actor_ids: vec![42],
            client_uids: vec![1000],
            client_gid: 1000,
            verifying_key: key.verifying_key(),
        },
        original.issued_at_unix_seconds,
    )
    .unwrap();
    assert_eq!(&checked, original);
}

#[test]
fn retained_producer_signs_original_body_after_public_files_change_or_disappear() {
    for remove in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let request_path = dir.path().join("request");
        let policy_path = dir.path().join("policy");
        let receipt_path = dir.path().join("receipt");
        let request = model_request();
        fs::write(&request_path, line(&request)).unwrap();
        fs::write(&policy_path, POLICY).unwrap();
        fs::write(&receipt_path, RECEIPT).unwrap();
        let prepared = prepare_files(&request_path, &policy_path, &receipt_path).unwrap();
        let audience = prepared.audience().to_owned();
        let (key_path, key) = key_file(dir.path());
        if remove {
            for path in [&request_path, &policy_path, &receipt_path] {
                fs::remove_file(path).unwrap();
            }
        } else {
            let mut different = request.clone();
            different.controller.run_id += 1;
            fs::write(&request_path, line(&different)).unwrap();
            fs::write(&policy_path, b"changed public policy").unwrap();
            fs::write(&receipt_path, b"changed public receipt").unwrap();
        }
        let output = dir.path().join("packet");
        sign_prepared_files_selected(
            prepared,
            &audience,
            &key_path,
            &output,
            &key.verifying_key(),
        )
        .unwrap();
        verify_output(&output, &request, &audience, &key);
    }
}

#[test]
fn retained_producer_valid_other_body_with_previous_audience_refuses_before_key() {
    let dir = tempfile::tempdir().unwrap();
    let first = model_request();
    let original = prepare_request(&line(&first), POLICY, RECEIPT).unwrap();
    let mut different = first.clone();
    different.controller.run_id += 1;
    different.nonce_sha256 = "d".repeat(64);
    let replacement = prepare_request(&line(&different), POLICY, RECEIPT).unwrap();
    let output = dir.path().join("packet");
    assert!(matches!(
        sign_prepared_files(
            replacement,
            original.audience(),
            &dir.path().join("absent-key"),
            &output
        ),
        Err(ProducerError::Invalid)
    ));
    assert!(!output.exists());
}

#[test]
fn retained_producer_bad_audience_and_expiry_refuse_before_key_io() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("packet");
    let request = model_request();
    let correct = prepare_request(&line(&request), POLICY, RECEIPT)
        .unwrap()
        .audience()
        .to_owned();
    for audience in [
        String::new(),
        "foreign".into(),
        format!("{correct}\n"),
        "x".repeat(MAX_PACKET_BYTES + 1),
    ] {
        let prepared = prepare_request(&line(&request), POLICY, RECEIPT).unwrap();
        assert!(matches!(
            sign_prepared_files(prepared, &audience, &dir.path().join("absent-key"), &output),
            Err(ProducerError::Invalid)
        ));
        assert!(!output.exists());
    }
    let now = job_unix_seconds().unwrap();
    let mut expired = model_request();
    expired.issued_at_unix_seconds = now - 61;
    expired.expires_at_unix_seconds = now - 1;
    let prepared = prepare_request(&line(&expired), POLICY, RECEIPT).unwrap();
    let audience = prepared.audience().to_owned();
    assert!(matches!(
        sign_prepared_files(prepared, &audience, &dir.path().join("absent-key"), &output),
        Err(ProducerError::Time)
    ));
    assert!(!output.exists());
}

#[test]
fn retained_producer_public_entry_keeps_controller_pin_and_exclusive_output() {
    let dir = tempfile::tempdir().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let (key_path, key) = key_file(dir.path());
    let request = model_request();
    let prepare = || prepare_request(&line(&request), POLICY, RECEIPT).unwrap();
    let audience = prepare().audience().to_owned();
    let output = dir.path().join("packet");
    assert!(matches!(
        sign_prepared_files(prepare(), &audience, &key_path, &output),
        Err(ProducerError::Key)
    ));
    assert!(!output.exists());
    sign_prepared_files_selected(
        prepare(),
        &audience,
        &key_path,
        &output,
        &key.verifying_key(),
    )
    .unwrap();
    let original = fs::read(&output).unwrap();
    assert!(matches!(
        sign_prepared_files_selected(
            prepare(),
            &audience,
            &key_path,
            &output,
            &key.verifying_key()
        ),
        Err(ProducerError::File)
    ));
    assert_eq!(fs::read(&output).unwrap(), original);
    fs::set_permissions(&key_path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(matches!(
        sign_prepared_files_selected(
            prepare(),
            &audience,
            &key_path,
            &dir.path().join("other"),
            &key.verifying_key()
        ),
        Err(ProducerError::File)
    ));
    assert!(!dir.path().join("other").exists());
}

fn retained_model(seed: u64) {
    let dir = tempfile::tempdir().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let (key_path, key) = key_file(dir.path());
    let mut original = model_request();
    original.controller.run_id = 1 + seed % 10000;
    original.nonce_sha256 = crate::sha256_hex(&seed.to_le_bytes());
    let mut caller_bytes = line(&original);
    let prepared = prepare_request(&caller_bytes, POLICY, RECEIPT).unwrap();
    let audience = prepared.audience().to_owned();
    caller_bytes.fill(0);
    let output = dir.path().join("packet");
    sign_prepared_files_selected(
        prepared,
        &audience,
        &key_path,
        &output,
        &key.verifying_key(),
    )
    .unwrap();
    verify_output(&output, &original, &audience, &key);
}

#[test]
fn retained_producer_seeded_body_replacements_and_independent_preparations() {
    let mut seed = 0x07403b21_u64;
    for _ in 0..32 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        retained_model(seed);
    }
    std::thread::scope(|scope| {
        let handles: Vec<_> = (1..=4)
            .map(|seed| scope.spawn(move || retained_model(seed)))
            .collect();
        for h in handles {
            h.join().unwrap();
        }
    });
    println!("seed=0x07403b21 retained_body_cases=32 independent_preparations=4 actual_token_or_signer_custody=false");
}
