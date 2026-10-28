use super::*;
use ed25519_dalek::SigningKey;
use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
fn line(value: &impl Serialize) -> Vec<u8> {
    let mut raw = canonical_json(value).unwrap();
    raw.push(b'\n');
    raw
}
fn key(byte: u8) -> VerifyingKey {
    SigningKey::from_bytes(&[byte; 32]).verifying_key()
}
struct Fixture {
    root: tempfile::TempDir,
    doc: Document,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("etc")).unwrap();
        fs::create_dir(root.path().join("etc/hydracache-perf")).unwrap();
        let old = super::super::Document {
            schema_version: TRUST_SCHEMA.into(),
            repository_id: crate::diagnostic_builder::REPOSITORY_ID,
            controller_key_hex: crate::hex(key(1).as_bytes()),
            held_builder_key_hex: crate::hex(key(4).as_bytes()),
            held_policy_sha256: "c".repeat(64),
            worker_builder_policy_sha256: "a".repeat(64),
            worker_policy_sha256: "b".repeat(64),
            policy_epoch: 7,
        };
        fs::write(
            root.path().join("etc/hydracache-perf").join(TRUST_FILE),
            line(&old),
        )
        .unwrap();
        let doc = Document {
            schema_version: SCHEMA.into(),
            repository_id: crate::diagnostic_builder::REPOSITORY_ID,
            bootstrap_builder_key_hex: crate::hex(key(5).as_bytes()),
            bootstrap_policy_sha256: "d".repeat(64),
            bootstrap_sha256: "e".repeat(64),
            held_policy_sha256: old.held_policy_sha256,
            policy_epoch: 7,
        };
        let value = Self { root, doc };
        fs::write(value.path(), line(&value.doc)).unwrap();
        value
    }
    fn path(&self) -> PathBuf {
        self.root.path().join("etc/hydracache-perf").join(FILE)
    }
    fn input(&self) -> HeldTrustInput {
        HeldTrustInput {
            source: HeldSource::open(
                self.root.path(),
                unsafe { libc::geteuid() },
                Selection {
                    controller: key(1),
                    worker_builder: key(2),
                    worker_issuer: key(3),
                    worker_builder_policy_sha256: "a".repeat(64),
                },
            )
            .unwrap(),
            refused: false,
            _original_thread: PhantomData,
        }
    }
}
#[test]
fn bootstrap_enrollment_retains_independent_file_without_export_or_mutation() {
    let fixture = Fixture::new();
    let mut held = fixture.input();
    let before = fs::read(fixture.path()).unwrap();
    {
        let mut guard = held.bind_bootstrap_enrollment().unwrap();
        for _ in 0..3 {
            guard.revalidate().unwrap();
            assert!(!guard.is_refused());
        }
    }
    held.revalidate().unwrap();
    assert_eq!(fs::read(fixture.path()).unwrap(), before);
}
#[test]
fn bootstrap_enrollment_strict_shape_future_noncanonical_and_budget_refuse() {
    let fixture = Fixture::new();
    let held = fixture.input();
    let value = serde_json::to_value(&fixture.doc).unwrap();
    for field in value.as_object().unwrap().keys() {
        let mut missing = value.clone();
        missing.as_object_mut().unwrap().remove(field);
        let mut null = value.clone();
        null[field] = serde_json::Value::Null;
        let duplicate = format!(
            "{{\"{field}\":{},{}",
            value[field],
            std::str::from_utf8(&line(&fixture.doc))
                .unwrap()
                .trim_start_matches('{')
        );
        for bad in [line(&missing), line(&null), duplicate.into_bytes()] {
            assert!(decode(&bad, &held.source).is_err(), "{field}");
        }
    }
    let mut extra = value.clone();
    extra["future"] = serde_json::json!(true);
    assert!(decode(&line(&extra), &held.source).is_err());
    for bad in [
        vec![],
        vec![b' '; MAX_TRUST_BYTES + 1],
        canonical_json(&fixture.doc).unwrap(),
        [b" ".as_slice(), line(&fixture.doc).as_slice()].concat(),
    ] {
        assert!(decode(&bad, &held.source).is_err());
    }
    let mut future = value;
    future["schema_version"] = serde_json::json!("diagnostic-bootstrap-enrollment-074-v2");
    assert!(decode(&line(&future), &held.source).is_err());
}
#[test]
fn bootstrap_enrollment_independent_key_and_original_pins_epoch_cannot_be_substituted() {
    let fixture = Fixture::new();
    let held = fixture.input();
    let value = serde_json::to_value(&fixture.doc).unwrap();
    for byte in 1..=4 {
        let mut bad = value.clone();
        bad["bootstrap_builder_key_hex"] = serde_json::json!(crate::hex(key(byte).as_bytes()));
        assert!(decode(&line(&bad), &held.source).is_err());
    }
    for (field, bad_value) in [
        ("repository_id", serde_json::json!(0)),
        ("policy_epoch", serde_json::json!(0)),
        ("policy_epoch", serde_json::json!(6)),
        ("policy_epoch", serde_json::json!(8)),
        ("held_policy_sha256", serde_json::json!("f".repeat(64))),
        (
            "bootstrap_builder_key_hex",
            serde_json::json!("0".repeat(64)),
        ),
    ] {
        let mut bad = value.clone();
        bad[field] = bad_value;
        assert!(decode(&line(&bad), &held.source).is_err(), "{field}");
    }
    for field in [
        "bootstrap_policy_sha256",
        "bootstrap_sha256",
        "held_policy_sha256",
        "bootstrap_builder_key_hex",
    ] {
        for contents in ["bad".into(), "0".repeat(64), "F".repeat(64)] {
            let mut bad = value.clone();
            bad[field] = serde_json::json!(contents);
            assert!(decode(&line(&bad), &held.source).is_err(), "{field}");
        }
    }
}
#[test]
fn bootstrap_enrollment_actual_file_drift_poison_is_shared_and_cannot_heal() {
    for at in 0..4 {
        let fixture = Fixture::new();
        let mut held = fixture.input();
        let raw = fs::read(fixture.path()).unwrap();
        let mut guard = held.bind_bootstrap_enrollment().unwrap();
        match at {
            0 => fs::write(fixture.path(), b"changed\n").unwrap(),
            1 => fs::set_permissions(fixture.path(), fs::Permissions::from_mode(0o666)).unwrap(),
            2 => {
                let next = fixture.path().with_extension("next");
                fs::write(&next, &raw).unwrap();
                fs::rename(next, fixture.path()).unwrap();
            }
            _ => {
                let old = fixture.path().with_extension("old");
                fs::rename(fixture.path(), &old).unwrap();
                symlink(old, fixture.path()).unwrap();
            }
        }
        assert!(guard.revalidate().is_err());
        assert!(guard.is_refused());
        if at < 2 {
            fs::write(fixture.path(), &raw).unwrap();
            fs::set_permissions(fixture.path(), fs::Permissions::from_mode(0o600)).unwrap();
        }
        assert!(guard.revalidate().is_err());
        drop(guard);
        assert!(held.is_refused());
    }
}
#[test]
fn bootstrap_enrollment_missing_hardlinked_and_original_ancestor_drift_refuse() {
    for at in 0..3 {
        let fixture = Fixture::new();
        let mut held = fixture.input();
        match at {
            0 => fs::remove_file(fixture.path()).unwrap(),
            1 => fs::hard_link(fixture.path(), fixture.path().with_extension("link")).unwrap(),
            _ => fs::set_permissions(
                fixture.root.path().join("etc"),
                fs::Permissions::from_mode(0o777),
            )
            .unwrap(),
        }
        assert!(held.bind_bootstrap_enrollment().is_err());
        assert!(held.is_refused());
        assert!(held.bind_bootstrap_enrollment().is_err());
    }
}
