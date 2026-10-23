use super::*;
use serde_json::{json, Value};
pub(crate) struct Fixture {
    pub(crate) key: SigningKey,
    controller: VerifyingKey,
    pub(crate) policy: Policy,
    pub(crate) image: Vec<u8>,
    lock: Vec<u8>,
    log: Vec<u8>,
}
impl Fixture {
    pub(crate) fn new() -> Self {
        let key = SigningKey::from_bytes(&[5; 32]);
        let controller = SigningKey::from_bytes(&[1; 32]).verifying_key();
        let mut image = vec![0; 120];
        image[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
        image[16..18].copy_from_slice(&3u16.to_le_bytes());
        image[18..20].copy_from_slice(&62u16.to_le_bytes());
        image[20..24].copy_from_slice(&1u32.to_le_bytes());
        image[32..40].copy_from_slice(&64u64.to_le_bytes());
        image[52..54].copy_from_slice(&64u16.to_le_bytes());
        image[54..56].copy_from_slice(&56u16.to_le_bytes());
        image[56..58].copy_from_slice(&1u16.to_le_bytes());
        let lock = b"synthetic lock, not an actual build".to_vec();
        let log= [json!({"reason":"compiler-artifact","package_id":"path+file:///fixture/tools/long-run-supervisor-074#0.0.0",
            "target":{"name":BINARY,"kind":["bin"],"crate_types":["bin"],"src_path":"/fixture/tools/long-run-supervisor-074/src/bin/diagnostic_held_bootstrap.rs"},
            "profile":{"opt_level":"3","debug_assertions":false,"test":false},"features":[],
            "executable":"/fixture/target/x86_64-unknown-linux-gnu/release/diagnostic_held_bootstrap"}),
            json!({"reason":"build-finished","success":true})].iter().flat_map(|v|document_bytes(v).unwrap()).collect();
        let policy = Policy {
            schema_version: POLICY_SCHEMA.into(),
            purpose: PURPOSE.into(),
            repository_id: REPOSITORY_ID,
            builder_id: BUILDER_ID.into(),
            builder_key_hex: crate::hex(key.verifying_key().as_bytes()),
            controller_key_hex: crate::hex(controller.as_bytes()),
            authority_epoch: 7,
            source: Source {
                commit: "a".repeat(40),
                tree: "b".repeat(40),
                root_lock: ArtifactDigest::of(&lock),
            },
            bootstrap_sha256: sha256_hex(&image),
            held_policy_sha256: "c".repeat(64),
            held_receipt_sha256: "d".repeat(64),
            original_host_sha256: "e".repeat(64),
            held_launcher_sha256: "f".repeat(64),
        };
        Self {
            key,
            controller,
            policy,
            image,
            lock,
            log,
        }
    }
    fn expected<'a>(&'a self, builder: &'a VerifyingKey) -> ExpectedTrust<'a> {
        ExpectedTrust {
            builder,
            controller: &self.controller,
            epoch: 7,
            held_policy_sha256: &self.policy.held_policy_sha256,
            bootstrap_sha256: &self.policy.bootstrap_sha256,
        }
    }
    pub(crate) fn policy_bytes(&self) -> Vec<u8> {
        document_bytes(
            &sign_policy(
                &self.policy,
                &self.key,
                &self.expected(&self.key.verifying_key()),
            )
            .unwrap(),
        )
        .unwrap()
    }
    fn checked(&self) -> CheckedPolicy {
        let raw = self.policy_bytes();
        load_policy(
            &raw,
            &sha256_hex(&raw),
            &self.expected(&self.key.verifying_key()),
        )
        .unwrap()
    }
    fn observation(&self) -> Observation {
        Observation {
            source_commit_before: self.policy.source.commit.clone(),
            source_commit_after: self.policy.source.commit.clone(),
            source_tree_before: self.policy.source.tree.clone(),
            source_tree_after: self.policy.source.tree.clone(),
            source_clean_before: true,
            source_clean_after: true,
            isolated_build: true,
            rustc_version: "rustc 1.94.0 (4a4ef493e 2026-03-02)".into(),
            cargo_version: "cargo 1.94.0 (85eff7c80 2026-01-15)".into(),
            build_command: BUILD_COMMAND.map(String::from).to_vec(),
        }
    }
    fn contents(&self) -> Contents<'_> {
        Contents {
            bootstrap: &self.image,
            root_lock: &self.lock,
            build_log: &self.log,
        }
    }
    pub(crate) fn receipt_bytes(&self) -> Vec<u8> {
        document_bytes(
            &sign_build(
                &self.checked(),
                &self.key,
                &self.observation(),
                &self.contents(),
            )
            .unwrap(),
        )
        .unwrap()
    }
    pub(crate) fn verified(&self) -> VerifiedBuild {
        verify_receipt(&self.receipt_bytes(), &self.checked()).unwrap()
    }
}
#[test]
fn bootstrap_build_roundtrip_is_static_and_content_bound() {
    let f = Fixture::new();
    let verified = f.verified();
    verified.verify_contents(&f.contents()).unwrap();
    let changed = Contents {
        bootstrap: b"not ELF",
        ..f.contents()
    };
    assert!(verified.verify_contents(&changed).is_err());
    assert!(verified
        .verify_contents(&Contents {
            root_lock: b"wrong lock",
            ..f.contents()
        })
        .is_err());
    assert!(verified
        .verify_contents(&Contents {
            build_log: b"wrong log",
            ..f.contents()
        })
        .is_err());
    assert!(sign_build(
        &f.checked(),
        &SigningKey::from_bytes(&[6; 32]),
        &f.observation(),
        &f.contents()
    )
    .is_err());
}
fn mutate(value: &Value, field: &str, bad: Value) -> Vec<u8> {
    let mut doc = value.clone();
    doc[field] = bad;
    document_bytes(&doc).unwrap()
}
#[test]
fn bootstrap_build_policy_signatures_domains_keys_pins_and_claims_refuse() {
    let f = Fixture::new();
    let raw = f.policy_bytes();
    let key = f.key.verifying_key();
    let expected = f.expected(&key);
    assert!(load_policy(&raw, &"0".repeat(64), &expected).is_err());
    let mut wrong = f.expected(&key);
    wrong.epoch += 1;
    assert!(load_policy(&raw, &sha256_hex(&raw), &wrong).is_err());
    wrong = f.expected(&key);
    wrong.bootstrap_sha256 = "wrong";
    assert!(load_policy(&raw, &sha256_hex(&raw), &wrong).is_err());
    let value = serde_json::to_value(&f.policy).unwrap();
    for (field, bad) in [
        ("schema_version", json!("v2")),
        ("purpose", json!("original-host-and-held-launcher")),
        ("repository_id", json!(0)),
        ("builder_id", json!("other")),
        ("builder_key_hex", json!("0".repeat(64))),
        ("controller_key_hex", json!(f.policy.builder_key_hex)),
        ("authority_epoch", json!(0)),
        ("held_policy_sha256", json!("0".repeat(64))),
        ("held_receipt_sha256", json!("bad")),
        ("original_host_sha256", json!(f.policy.held_launcher_sha256)),
        ("held_launcher_sha256", json!(f.policy.bootstrap_sha256)),
        ("bootstrap_sha256", json!("A".repeat(64))),
        (
            "source",
            json!({"commit":"0".repeat(40),"tree":"b".repeat(40),"root_lock":f.policy.source.root_lock}),
        ),
    ] {
        let p: Policy = serde_json::from_slice(&mutate(&value, field, bad)).unwrap();
        let doc = SignedPolicy {
            schema_version: SIGNED_POLICY_SCHEMA.into(),
            signature_hex: signature(&f.key, POLICY_DOMAIN, &p).unwrap(),
            policy: p,
        };
        let raw = document_bytes(&doc).unwrap();
        assert!(
            load_policy(&raw, &sha256_hex(&raw), &expected).is_err(),
            "{field}"
        );
    }
    let mut signed: SignedPolicy = serde_json::from_slice(&raw).unwrap();
    for domain in [
        RECEIPT_DOMAIN,
        b"hydracache-diagnostic-held-build-074-v1".as_slice(),
    ] {
        signed.signature_hex = signature(&f.key, domain, &signed.policy).unwrap();
        let raw = document_bytes(&signed).unwrap();
        assert!(load_policy(&raw, &sha256_hex(&raw), &expected).is_err());
    }
    signed.signature_hex = signature(
        &SigningKey::from_bytes(&[6; 32]),
        POLICY_DOMAIN,
        &signed.policy,
    )
    .unwrap();
    let raw = document_bytes(&signed).unwrap();
    assert!(load_policy(&raw, &sha256_hex(&raw), &expected).is_err());
    assert!(sign_policy(&f.policy, &SigningKey::from_bytes(&[6; 32]), &expected).is_err());
}
#[test]
fn bootstrap_build_canonical_shape_and_budgets_refuse() {
    let f = Fixture::new();
    for (raw, limit, policy) in [
        (f.policy_bytes(), MAX_POLICY_BYTES, true),
        (f.receipt_bytes(), MAX_RECEIPT_BYTES as usize, false),
    ] {
        let check = |bad: &[u8]| {
            if policy {
                load_policy(bad, &sha256_hex(bad), &f.expected(&f.key.verifying_key())).is_err()
            } else {
                verify_receipt(bad, &f.checked()).is_err()
            }
        };
        assert!(!check(&raw));
        let value: Value = serde_json::from_slice(&raw).unwrap();
        for field in value.as_object().unwrap().keys() {
            let mut missing = value.clone();
            missing.as_object_mut().unwrap().remove(field);
            assert!(check(&document_bytes(&missing).unwrap()));
            assert!(check(&mutate(&value, field, Value::Null)));
            let duplicate = format!(
                "{{\"{field}\":{},{}",
                value[field],
                std::str::from_utf8(&raw).unwrap().trim_start_matches('{')
            );
            assert!(check(duplicate.as_bytes()));
        }
        for bad in [
            vec![],
            vec![b' '; limit + 1],
            raw[..raw.len() - 1].to_vec(),
            [b" ".as_slice(), &raw].concat(),
            mutate(&value, "extra", json!(true)),
            mutate(&value, "schema_version", json!("v2")),
            mutate(&value, "signature_hex", json!("F".repeat(128))),
        ] {
            assert!(check(&bad));
        }
    }
}
#[test]
fn bootstrap_build_valid_signatures_cannot_hide_false_observations_or_bounds() {
    let f = Fixture::new();
    let checked = f.checked();
    let original: SignedReceipt = serde_json::from_slice(&f.receipt_bytes()).unwrap();
    let value = serde_json::to_value(&original.statement).unwrap();
    for (field, bad) in [
        ("schema_version", json!("v2")),
        ("purpose", json!("other")),
        ("policy_sha256", json!("d".repeat(64))),
        ("authority_epoch", json!(6)),
        (
            "bootstrap",
            json!({"sha256":f.policy.bootstrap_sha256,"bytes":MAX_BINARY_BYTES+1}),
        ),
        ("root_lock", json!({"sha256":"a".repeat(64),"bytes":1})),
        ("build_log", json!({"sha256":"0".repeat(64),"bytes":1})),
    ] {
        let statement: Statement = serde_json::from_slice(&mutate(&value, field, bad)).unwrap();
        let doc = SignedReceipt {
            schema_version: RECEIPT_SCHEMA.into(),
            signature_hex: signature(&f.key, RECEIPT_DOMAIN, &statement).unwrap(),
            statement,
        };
        assert!(
            verify_receipt(&document_bytes(&doc).unwrap(), &checked).is_err(),
            "{field}"
        );
    }
    let observed = serde_json::to_value(f.observation()).unwrap();
    for field in observed.as_object().unwrap().keys() {
        let bad = if observed[field].is_boolean() {
            json!(false)
        } else if observed[field].is_array() {
            json!([])
        } else {
            json!("wrong")
        };
        let o: Observation = serde_json::from_slice(&mutate(&observed, field, bad)).unwrap();
        assert!(
            sign_build(&checked, &f.key, &o, &f.contents()).is_err(),
            "{field}"
        );
        let mut statement = original.statement.clone();
        statement.observation = o;
        let doc = SignedReceipt {
            schema_version: RECEIPT_SCHEMA.into(),
            signature_hex: signature(&f.key, RECEIPT_DOMAIN, &statement).unwrap(),
            statement,
        };
        assert!(
            verify_receipt(&document_bytes(&doc).unwrap(), &checked).is_err(),
            "{field}"
        );
    }
    let mut doc = original;
    doc.signature_hex = signature(&f.key, POLICY_DOMAIN, &doc.statement).unwrap();
    assert!(verify_receipt(&document_bytes(&doc).unwrap(), &checked).is_err());
}
#[test]
fn bootstrap_build_cargo_log_rejects_wrong_targets_features_duplicates_and_completion() {
    let f = Fixture::new();
    let events: Vec<Value> = f
        .log
        .split(|b| *b == b'\n')
        .filter(|s| !s.is_empty())
        .map(|s| serde_json::from_slice(s).unwrap())
        .collect();
    for field in ["target", "profile", "features", "package_id", "executable"] {
        let mut values = events.clone();
        values[0][field] = match field {
            "target" => {
                json!({"name":"other","kind":["bin"],"crate_types":["bin"],"src_path":"/wrong"})
            }
            "profile" => json!({"opt_level":"0","debug_assertions":true,"test":true}),
            "features" => json!(["probe"]),
            _ => json!("wrong"),
        };
        let raw: Vec<u8> = values
            .iter()
            .flat_map(|v| document_bytes(v).unwrap())
            .collect();
        assert!(cargo_log(&raw).is_err(), "{field}");
    }
    for raw in [
        f.log[..f.log.len() - 1].to_vec(),
        [f.log.as_slice(), f.log.as_slice()].concat(),
        document_bytes(&events[0]).unwrap(),
        document_bytes(&events[1]).unwrap(),
        b"{\"reason\":\"build-finished\",\"reason\":\"build-finished\",\"success\":true}\n"
            .to_vec(),
        b"{\"reason\":\"compiler-message\",\"message\":{\"level\":\"error\"}}\n".to_vec(),
    ] {
        assert!(cargo_log(&raw).is_err());
    }
    assert!(sign_build(
        &f.checked(),
        &f.key,
        &f.observation(),
        &Contents {
            bootstrap: b"wrong",
            ..f.contents()
        }
    )
    .is_err());
    assert!(sign_build(
        &f.checked(),
        &f.key,
        &f.observation(),
        &Contents {
            root_lock: b"wrong",
            ..f.contents()
        }
    )
    .is_err());
}

#[cfg(target_os = "linux")]
pub(crate) fn sealed_image(raw: &[u8], seals: i32) -> std::fs::File {
    use std::io::Write;
    use std::os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::PermissionsExt,
    };
    let name = std::ffi::CString::new("hc074-bootstrap-model").unwrap();
    let fd =
        unsafe { libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING) };
    assert!(fd >= 0);
    let mut file = unsafe { std::fs::File::from_raw_fd(fd) };
    file.write_all(raw).unwrap();
    file.set_permissions(std::fs::Permissions::from_mode(0o555))
        .unwrap();
    if seals != 0 {
        assert_eq!(unsafe { libc::fcntl(fd, libc::F_ADD_SEALS, seals) }, 0);
    }
    std::fs::File::open(format!("/proc/self/fd/{}", file.as_raw_fd())).unwrap()
}
#[cfg(target_os = "linux")]
#[test]
fn bootstrap_build_sealed_image_requires_exact_original_object_and_metadata() {
    use std::os::fd::AsRawFd;
    let f = Fixture::new();
    let build = f.verified();
    let all = libc::F_SEAL_SEAL | libc::F_SEAL_WRITE | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK;
    let file = sealed_image(&f.image, all);
    let opening = sealed::inspect(&file, &build).unwrap();
    assert!(sealed::inspect(&file, &build).unwrap() == opening);
    let other = sealed_image(&f.image, all);
    assert!(sealed::inspect(&other, &build).unwrap() != opening);
    for seals in [0, libc::F_SEAL_SEAL, all & !libc::F_SEAL_SEAL] {
        assert!(sealed::inspect(&sealed_image(&f.image, seals), &build).is_err());
    }
    let mut changed = f.image.clone();
    changed[119] = 1;
    assert!(sealed::inspect(&sealed_image(&changed, all), &build).is_err());
    assert_eq!(
        unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETFD, 0) },
        0
    );
    assert!(sealed::inspect(&file, &build).is_err());
}
