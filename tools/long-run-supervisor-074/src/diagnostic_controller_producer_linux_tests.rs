use super::super::tests::{line, model_request};
use super::*;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
const POLICY: &[u8] =
    include_bytes!("../../../docs/testing/performance/0.74/diagnostic-builder-public-policy.json");
const RECEIPT:&[u8]=include_bytes!("../../../docs/testing/performance/0.74/local-runs/diagnostic-builder-hosted-37923423889/build-receipt-v1.json");
#[test]
fn producer_linux_private_synthetic_output_is_exclusive_and_never_real_controller() {
    let dir = tempfile::tempdir().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let request = dir.path().join("request");
    let policy = dir.path().join("policy");
    let receipt = dir.path().join("receipt");
    let key_path = dir.path().join("key");
    let output = dir.path().join("packet");
    fs::write(&request, line(&model_request())).unwrap();
    fs::write(&policy, POLICY).unwrap();
    fs::write(&receipt, RECEIPT).unwrap();
    let key = SigningKey::from_bytes(&[76; 32]);
    fs::write(&key_path, crate::hex(&key.to_bytes())).unwrap();
    fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600)).unwrap();
    prepare_files(&request, &policy, &receipt).unwrap();
    assert!(matches!(
        sign_files(&request, &policy, &receipt, &key_path, &output),
        Err(ProducerError::Key)
    ));
    assert!(!output.exists());
    sign_files_selected(
        &request,
        &policy,
        &receipt,
        &key_path,
        &output,
        &key.verifying_key(),
    )
    .unwrap();
    let original = fs::read(&output).unwrap();
    assert_eq!(fs::metadata(&output).unwrap().mode() & 0o777, 0o600);
    assert!(sign_files_selected(
        &request,
        &policy,
        &receipt,
        &key_path,
        &output,
        &key.verifying_key()
    )
    .is_err());
    assert_eq!(fs::read(&output).unwrap(), original);
    fs::set_permissions(&key_path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(sign_files_selected(
        &request,
        &policy,
        &receipt,
        &key_path,
        &dir.path().join("other"),
        &key.verifying_key()
    )
    .is_err());
    fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(&key_path, b"uppercase-or-invalid").unwrap();
    assert!(sign_files_selected(
        &request,
        &policy,
        &receipt,
        &key_path,
        &dir.path().join("other"),
        &key.verifying_key()
    )
    .is_err());
    for raw in [vec![], vec![b'a'; 66], vec![b'A'; 64]] {
        fs::write(&key_path, raw).unwrap();
        assert!(sign_files_selected(
            &request,
            &policy,
            &receipt,
            &key_path,
            &dir.path().join("other"),
            &key.verifying_key()
        )
        .is_err());
        assert!(!dir.path().join("other").exists());
    }
    fs::write(&key_path, format!("{}\n", crate::hex(&key.to_bytes()))).unwrap();
    let linked = dir.path().join("linked-key");
    std::os::unix::fs::symlink(&key_path, &linked).unwrap();
    assert!(sign_files_selected(
        &request,
        &policy,
        &receipt,
        &linked,
        &dir.path().join("other"),
        &key.verifying_key()
    )
    .is_err());
    fs::hard_link(&key_path, dir.path().join("hard-key")).unwrap();
    assert!(sign_files_selected(
        &request,
        &policy,
        &receipt,
        &key_path,
        &dir.path().join("other"),
        &key.verifying_key()
    )
    .is_err());
    assert!(!dir.path().join("other").exists());
}
#[test]
fn producer_linux_checks_public_request_before_opening_any_secret() {
    let dir = tempfile::tempdir().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let request = dir.path().join("request");
    let policy = dir.path().join("policy");
    let receipt = dir.path().join("receipt");
    fs::write(&request, b"{}").unwrap();
    fs::write(&policy, POLICY).unwrap();
    fs::write(&receipt, RECEIPT).unwrap();
    assert!(matches!(
        sign_files(
            &request,
            &policy,
            &receipt,
            &dir.path().join("absent-secret"),
            &dir.path().join("packet")
        ),
        Err(ProducerError::Invalid)
    ));
    assert!(!dir.path().join("packet").exists());
}
