//! Synthetic keys, ELF headers and Cargo events: format checks, not a real build.
#[path = "support/held_role_binding.rs"]
mod role_binding;
#[path = "support/held_role_files.rs"]
mod role_files;
use ed25519_dalek::{Signer, SigningKey};
use hydracache_long_run_supervisor_074::{
    diagnostic_artifacts::{ArtifactDigest, MAX_LOG_BYTES},
    diagnostic_builder::{self as worker, held_build as held},
    diagnostic_two_phase_launch::LaunchArtifacts,
};
use serde_json::{json, Value};

fn hash(raw: &[u8]) -> String {
    ArtifactDigest::of(raw).sha256
}
fn hex(raw: &[u8]) -> String {
    raw.iter().map(|b| format!("{b:02x}")).collect()
}
fn elf(tag: u8) -> Vec<u8> {
    let mut raw = vec![0; 128];
    raw[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    raw[16] = 2;
    raw[18] = 62;
    raw[20] = 1;
    raw[32] = 64;
    raw[52] = 64;
    raw[54] = 56;
    raw[56] = 1;
    raw[127] = tag;
    raw
}
fn event(name: &str, source: &str) -> Value {
    json!({"reason":"compiler-artifact",
        "package_id":"path+file:///isolated/tools/long-run-supervisor-074#0.0.0",
        "target":{"name":name,"kind":["bin"],"crate_types":["bin"],
            "src_path":format!("/isolated/tools/long-run-supervisor-074/src/{source}")},
        "profile":{"opt_level":"3","debug_assertions":false,"test":false},
        "features":[],"executable":format!("/target/x86_64-unknown-linux-gnu/release/{name}"),
        "fresh":false})
}
fn events() -> Vec<Value> {
    vec![
        event(held::HOST, "main.rs"),
        event(held::LAUNCHER, "bin/diagnostic_held_launcher.rs"),
        json!({"reason":"build-finished","success":true}),
    ]
}
fn log(events: &[Value]) -> Vec<u8> {
    events
        .iter()
        .flat_map(|e| held::document_bytes(e).unwrap())
        .collect()
}
struct Fixture {
    controller: SigningKey,
    builder: SigningKey,
    worker: SigningKey,
    worker_raw: Vec<u8>,
    policy: held::Policy,
    host: Vec<u8>,
    launcher: Vec<u8>,
    lock: Vec<u8>,
    log: Vec<u8>,
}
impl Fixture {
    fn new() -> Self {
        let controller = SigningKey::from_bytes(&[17; 32]);
        let builder = SigningKey::from_bytes(&[18; 32]);
        let worker = SigningKey::from_bytes(&[19; 32]);
        let worker_raw = worker::policy_bytes(&worker::BuilderPolicy {
            schema_version: worker::POLICY_SCHEMA.into(),
            repository_id: worker::REPOSITORY_ID,
            builder_id: worker::BUILDER_ID.into(),
            builder_key_hex: hex(worker.verifying_key().as_bytes()),
            controller_key_hex: hex(controller.verifying_key().as_bytes()),
        })
        .unwrap();
        let lock = b"synthetic-lock\n".to_vec();
        let policy = held::Policy {
            schema_version: held::POLICY_SCHEMA.into(),
            purpose: held::PURPOSE.into(),
            repository_id: worker::REPOSITORY_ID,
            builder_id: held::BUILDER_ID.into(),
            builder_key_hex: hex(builder.verifying_key().as_bytes()),
            controller_key_hex: hex(controller.verifying_key().as_bytes()),
            authority_epoch: 7,
            source: held::Source {
                commit: "a".repeat(40),
                tree: "b".repeat(40),
                root_lock: ArtifactDigest::of(&lock),
            },
        };
        Self {
            controller,
            builder,
            worker,
            worker_raw,
            policy,
            host: elf(1),
            launcher: elf(2),
            lock,
            log: log(&events()),
        }
    }
    fn load(
        &self,
        raw: &[u8],
        pin: &str,
    ) -> Result<
        held::CheckedPolicy,
        hydracache_long_run_supervisor_074::diagnostic_artifacts::ArtifactError,
    > {
        held::load_policy(
            raw,
            pin,
            &held::ExpectedTrust {
                controller_key: &self.controller.verifying_key(),
                authority_epoch: 7,
                worker_policy: &self.worker_raw,
                worker_policy_sha256: &hash(&self.worker_raw),
            },
        )
    }
    fn checked(&self) -> held::CheckedPolicy {
        let raw = held::document_bytes(&self.policy).unwrap();
        self.load(&raw, &hash(&raw)).unwrap()
    }
    fn contents(&self) -> held::Contents<'_> {
        held::Contents {
            original_host: &self.host,
            held_launcher: &self.launcher,
            root_lock: &self.lock,
            build_log: &self.log,
        }
    }
    fn observation(&self) -> held::Observation {
        held::Observation {
            source_commit_before: self.policy.source.commit.clone(),
            source_commit_after: self.policy.source.commit.clone(),
            source_tree_before: self.policy.source.tree.clone(),
            source_tree_after: self.policy.source.tree.clone(),
            source_clean_before: true,
            source_clean_after: true,
            isolated_build: true,
            rustc_version: "rustc 1.94.0 (4a4ef493e 2026-03-02)".into(),
            cargo_version: "cargo 1.94.0 (85eff7c80 2026-01-15)".into(),
            build_command: held::BUILD_COMMAND.iter().map(|s| (*s).into()).collect(),
        }
    }
    fn receipt(&self) -> held::SignedReceipt {
        held::sign_build(
            &self.checked(),
            &self.builder,
            &self.observation(),
            &self.contents(),
        )
        .unwrap()
    }
    fn accepts_log(&mut self, raw: Vec<u8>) -> bool {
        self.log = raw;
        held::sign_build(
            &self.checked(),
            &self.builder,
            &self.observation(),
            &self.contents(),
        )
        .is_ok()
    }
}

#[test]
fn separate_policy_signer_receipt_contents_and_three_roles_roundtrip() {
    let f = Fixture::new();
    let policy = f.checked();
    let receipt = f.receipt();
    let raw = held::document_bytes(&receipt).unwrap();
    let checked = held::verify_receipt(&raw, &policy).unwrap();
    checked.verify_contents(&f.contents()).unwrap();
    checked
        .matches_launch_artifacts(&LaunchArtifacts {
            launcher_sha256: hash(&f.launcher),
            watchdog_sha256: hash(&f.host),
            collector_sha256: hash(&f.host),
        })
        .unwrap();
    assert_ne!(
        held::SIGNATURE_DOMAIN,
        hydracache_long_run_supervisor_074::diagnostic_artifacts::SIGNATURE_DOMAIN
    );
}

#[test]
fn independent_pins_controller_epoch_and_three_distinct_nonweak_keys_required() {
    let mut f = Fixture::new();
    let raw = held::document_bytes(&f.policy).unwrap();
    f.checked();
    assert!(f.load(&raw, &"0".repeat(64)).is_err());
    for key in [f.worker.verifying_key(), f.controller.verifying_key()] {
        f.policy.builder_key_hex = hex(key.as_bytes());
        let changed = held::document_bytes(&f.policy).unwrap();
        assert!(f.load(&changed, &hash(&changed)).is_err());
    }
    f.policy.builder_key_hex = "01".to_owned() + &"00".repeat(31); // weak identity
    let changed = held::document_bytes(&f.policy).unwrap();
    assert!(f.load(&changed, &hash(&changed)).is_err());
    let f = Fixture::new();
    for (controller, epoch, worker_pin) in [
        (
            SigningKey::from_bytes(&[20; 32]).verifying_key(),
            7,
            hash(&f.worker_raw),
        ),
        (f.controller.verifying_key(), 0, hash(&f.worker_raw)),
        (f.controller.verifying_key(), 8, hash(&f.worker_raw)),
        (f.controller.verifying_key(), 7, "0".repeat(64)),
    ] {
        let trust = held::ExpectedTrust {
            controller_key: &controller,
            authority_epoch: epoch,
            worker_policy: &f.worker_raw,
            worker_policy_sha256: &worker_pin,
        };
        assert!(held::load_policy(&raw, &hash(&raw), &trust).is_err());
    }
    assert!(held::sign_build(&f.checked(), &f.worker, &f.observation(), &f.contents()).is_err());
}

#[test]
fn repinned_invalid_policy_fields_and_unsigned_unknown_fields_refuse() {
    let f = Fixture::new();
    let base = serde_json::to_value(&f.policy).unwrap();
    for (path, bad) in [
        ("/schema_version", json!("future")),
        ("/purpose", json!("worker")),
        ("/repository_id", json!(0)),
        ("/builder_id", json!(worker::BUILDER_ID)),
        (
            "/controller_key_hex",
            json!(hex(f.worker.verifying_key().as_bytes())),
        ),
        ("/builder_key_hex", json!("FF".repeat(32))),
        ("/authority_epoch", json!(8)),
        ("/source/commit", json!("0".repeat(40))),
        ("/source/tree", json!("A".repeat(40))),
        ("/source/root_lock/sha256", json!("bad")),
        ("/source/root_lock/bytes", json!(0)),
        ("/source/root_lock/bytes", json!(1_048_577)),
    ] {
        let mut value = base.clone();
        *value.pointer_mut(path).unwrap() = bad;
        let raw = held::document_bytes(&value).unwrap();
        assert!(f.load(&raw, &hash(&raw)).is_err(), "{path}");
    }
    let mut value = base;
    value["extra"] = json!(true);
    let raw = held::document_bytes(&value).unwrap();
    assert!(f.load(&raw, &hash(&raw)).is_err());
}

fn resign(f: &Fixture, value: &mut Value, domain: &[u8]) {
    let mut message = domain.to_vec();
    message.push(0);
    message.extend(held::document_bytes(&value["statement"]).unwrap());
    value["signature_hex"] = json!(hex(&f.builder.sign(&message).to_bytes()));
}
#[test]
fn every_receipt_binding_refuses_unsigned_mutation_and_resigned_invalid_semantics() {
    let f = Fixture::new();
    let policy = f.checked();
    let base = serde_json::to_value(f.receipt()).unwrap();
    held::verify_receipt(&held::document_bytes(&base).unwrap(), &policy).unwrap();
    for (path, bad) in [
        ("/schema_version", json!("future")),
        ("/statement/schema_version", json!("future")),
        ("/statement/purpose", json!("worker")),
        ("/statement/policy_sha256", json!("0".repeat(64))),
        ("/statement/authority_epoch", json!(8)),
        ("/statement/roles/launcher", json!("original_host")),
        ("/statement/roles/watchdog", json!("held_launcher")),
        ("/statement/roles/collector", json!("held_launcher")),
        ("/statement/images/original_host/sha256", json!("bad")),
        ("/statement/images/original_host/bytes", json!(0)),
        (
            "/statement/images/held_launcher/sha256",
            json!(hash(&f.host)),
        ),
        ("/statement/images/held_launcher/bytes", json!(134_217_729)),
        ("/statement/root_lock/sha256", json!("0".repeat(64))),
        ("/statement/root_lock/bytes", json!(1)),
        ("/statement/build_log/sha256", json!("bad")),
        ("/statement/build_log/bytes", json!(16_777_217)),
    ] {
        let mut value = base.clone();
        *value.pointer_mut(path).unwrap() = bad;
        assert!(
            held::verify_receipt(&held::document_bytes(&value).unwrap(), &policy).is_err(),
            "unsigned {path}"
        );
        resign(&f, &mut value, held::SIGNATURE_DOMAIN);
        assert!(
            held::verify_receipt(&held::document_bytes(&value).unwrap(), &policy).is_err(),
            "signed {path}"
        );
    }
}

#[test]
fn all_observation_fields_checked_before_signing_and_after_signature() {
    let f = Fixture::new();
    let policy = f.checked();
    let base = serde_json::to_value(f.observation()).unwrap();
    for (field, original) in base.as_object().unwrap() {
        let bad = match original {
            Value::Bool(_) => json!(false),
            Value::Array(_) => json!(["cargo"]),
            _ => json!("wrong"),
        };
        let mut value = base.clone();
        value[field] = bad;
        let o: held::Observation = serde_json::from_value(value).unwrap();
        assert!(
            held::sign_build(&policy, &f.builder, &o, &f.contents()).is_err(),
            "{field}"
        );
        let mut receipt = serde_json::to_value(f.receipt()).unwrap();
        receipt["statement"]["observation"] = serde_json::to_value(o).unwrap();
        resign(&f, &mut receipt, held::SIGNATURE_DOMAIN);
        assert!(
            held::verify_receipt(&held::document_bytes(&receipt).unwrap(), &policy).is_err(),
            "{field}"
        );
    }
}

#[test]
fn canonical_bounded_documents_signature_encoding_and_domains_are_strict() {
    let f = Fixture::new();
    let policy = f.checked();
    let raw = held::document_bytes(&f.receipt()).unwrap();
    for bad in [
        vec![],
        raw[..raw.len() - 1].to_vec(),
        [b" ".as_slice(), &raw].concat(),
        vec![b' '; 65_537],
        [raw.as_slice(), b"\n"].concat(),
        b"{\"schema_version\":1,\"schema_version\":1}\n".to_vec(),
    ] {
        assert!(held::verify_receipt(&bad, &policy).is_err());
    }
    let p = held::document_bytes(&f.policy).unwrap();
    for bad in [
        vec![],
        p[..p.len() - 1].to_vec(),
        vec![b' '; 4097],
        [b" ".as_slice(), &p].concat(),
        b"{\"purpose\":1,\"purpose\":1}\n".to_vec(),
    ] {
        assert!(f.load(&bad, &hash(&bad)).is_err());
    }
    for signature in [
        "00".repeat(64),
        "FF".repeat(64),
        "a".repeat(127),
        "g".repeat(128),
    ] {
        let mut receipt = f.receipt();
        receipt.signature_hex = signature;
        assert!(held::verify_receipt(&held::document_bytes(&receipt).unwrap(), &policy).is_err());
    }
    let mut value = serde_json::to_value(f.receipt()).unwrap();
    resign(
        &f,
        &mut value,
        hydracache_long_run_supervisor_074::diagnostic_artifacts::SIGNATURE_DOMAIN,
    );
    assert!(held::verify_receipt(&held::document_bytes(&value).unwrap(), &policy).is_err());
    for pointer in [
        "",
        "/statement",
        "/statement/observation",
        "/statement/roles",
        "/statement/images",
        "/statement/images/original_host",
    ] {
        let mut value = serde_json::to_value(f.receipt()).unwrap();
        value.pointer_mut(pointer).unwrap()["unknown"] = json!(true);
        assert!(held::verify_receipt(&held::document_bytes(&value).unwrap(), &policy).is_err());
    }
}

#[test]
fn independent_policy_source_and_epoch_binding_prevents_receipt_reuse() {
    let f = Fixture::new();
    let receipt = held::document_bytes(&f.receipt()).unwrap();
    let mut changed = f.policy.clone();
    changed.source.commit = "c".repeat(40);
    let raw = held::document_bytes(&changed).unwrap();
    assert!(held::verify_receipt(&receipt, &f.load(&raw, &hash(&raw)).unwrap()).is_err());
    let old = worker::load_policy(
        &f.worker_raw,
        &hash(&f.worker_raw),
        &f.controller.verifying_key(),
    )
    .unwrap();
    // An old worker policy cannot parse as the new purpose-bound policy.
    assert!(f.load(&f.worker_raw, &hash(&f.worker_raw)).is_err());
    assert!(worker::load_policy(
        &held::document_bytes(&f.policy).unwrap(),
        &hash(&held::document_bytes(&f.policy).unwrap()),
        &f.controller.verifying_key()
    )
    .is_err());
    let _ = old;
}

#[test]
fn changed_contents_and_each_expected_role_refuse_without_execution_authority() {
    let mut f = Fixture::new();
    let build =
        held::verify_receipt(&held::document_bytes(&f.receipt()).unwrap(), &f.checked()).unwrap();
    build.verify_contents(&f.contents()).unwrap();
    for which in 0..4 {
        let bytes = match which {
            0 => &mut f.host,
            1 => &mut f.launcher,
            2 => &mut f.lock,
            _ => &mut f.log,
        };
        let index = bytes.len() - 1;
        bytes[index] ^= 1;
        assert!(build.verify_contents(&f.contents()).is_err());
        let bytes = match which {
            0 => &mut f.host,
            1 => &mut f.launcher,
            2 => &mut f.lock,
            _ => &mut f.log,
        };
        bytes[index] ^= 1;
    }
    for which in 0..3 {
        let mut expected = LaunchArtifacts {
            launcher_sha256: hash(&f.launcher),
            watchdog_sha256: hash(&f.host),
            collector_sha256: hash(&f.host),
        };
        match which {
            0 => expected.launcher_sha256 = "0".repeat(64),
            1 => expected.watchdog_sha256 = "0".repeat(64),
            _ => expected.collector_sha256 = "0".repeat(64),
        }
        assert!(build.matches_launch_artifacts(&expected).is_err());
    }
    std::mem::swap(&mut f.host, &mut f.launcher);
    assert!(build.verify_contents(&f.contents()).is_err());
}

#[test]
fn signer_requires_distinct_valid_elf_and_exact_nonempty_root_lock() {
    let f = Fixture::new();
    let policy = f.checked();
    for c in [
        held::Contents {
            original_host: &f.host,
            held_launcher: &f.host,
            ..f.contents()
        },
        held::Contents {
            original_host: b"not ELF",
            ..f.contents()
        },
        held::Contents {
            held_launcher: &[],
            ..f.contents()
        },
        held::Contents {
            root_lock: b"changed-lock\n",
            ..f.contents()
        },
        held::Contents {
            root_lock: &[],
            ..f.contents()
        },
    ] {
        assert!(held::sign_build(&policy, &f.builder, &f.observation(), &c).is_err());
    }
    for index in [0, 4, 5, 6, 16, 18, 20, 32, 52, 54, 56] {
        let mut host = f.host.clone();
        host[index] = 255;
        assert!(
            held::sign_build(
                &policy,
                &f.builder,
                &f.observation(),
                &held::Contents {
                    original_host: &host,
                    ..f.contents()
                }
            )
            .is_err(),
            "ELF {index}"
        );
    }
}

#[test]
fn cargo_exact_targets_package_sources_output_profiles_and_features_required() {
    let mut f = Fixture::new();
    assert!(f.accepts_log(log(&events())));
    for i in 0..2 {
        for (path, bad) in [
            ("/target/name", json!("foreign")),
            ("/target/kind", json!(["lib"])),
            ("/target/crate_types", json!(["lib"])),
            ("/target/src_path", json!("/foreign/main.rs")),
            (
                "/package_id",
                json!("registry+https://example.invalid#0.0.0"),
            ),
            ("/profile/opt_level", json!("0")),
            ("/profile/debug_assertions", json!(true)),
            ("/profile/test", json!(true)),
            ("/features", json!(["fixed-leaf-alloc-probe"])),
            ("/executable", json!("/target/release/wrong")),
        ] {
            let mut e = events();
            *e[i].pointer_mut(path).unwrap() = bad;
            assert!(!f.accepts_log(log(&e)), "target {i} {path}");
        }
        for field in ["target", "profile", "features", "executable", "package_id"] {
            let mut e = events();
            e[i].as_object_mut().unwrap().remove(field);
            assert!(!f.accepts_log(log(&e)), "missing {i} {field}");
        }
    }
}

#[test]
fn cargo_missing_duplicate_foreign_failure_and_trailing_events_refuse() {
    let mut f = Fixture::new();
    let good = events();
    assert!(f.accepts_log(log(&good)));
    for e in [
        vec![good[0].clone(), good[2].clone()],
        vec![good[1].clone(), good[2].clone()],
        vec![
            good[0].clone(),
            good[0].clone(),
            good[1].clone(),
            good[2].clone(),
        ],
        vec![
            good[0].clone(),
            good[1].clone(),
            event("foreign", "main.rs"),
            good[2].clone(),
        ],
        vec![good[0].clone(), good[1].clone()],
        vec![
            good[0].clone(),
            good[1].clone(),
            json!({"reason":"build-finished","success":false}),
        ],
        vec![
            good[0].clone(),
            good[1].clone(),
            good[2].clone(),
            good[2].clone(),
        ],
        vec![
            good[0].clone(),
            good[1].clone(),
            json!({"reason":"unknown"}),
            good[2].clone(),
        ],
        vec![
            good[0].clone(),
            good[1].clone(),
            json!({"reason":"compiler-message","message":{"level":"error"}}),
            good[2].clone(),
        ],
    ] {
        assert!(!f.accepts_log(log(&e)));
    }
    let mut good_extra = good.clone();
    good_extra.insert(
        0,
        json!({"reason":"compiler-message","message":{"level":"warning"}}),
    );
    good_extra.insert(0, json!({"reason":"build-script-executed"}));
    assert!(f.accepts_log(log(&good_extra)));
    // Accept Cargo's qualified package fragment and reversed binary order.
    let mut e = events();
    e.swap(0, 1);
    for item in &mut e[..2] {
        item["package_id"]=json!("path+file:///isolated/tools/long-run-supervisor-074#hydracache-long-run-supervisor-074@0.0.0");
    }
    assert!(f.accepts_log(log(&e)));
}

#[test]
fn cargo_raw_bounds_duplicates_and_nonterminated_records_refuse() {
    let mut f = Fixture::new();
    let good = log(&events());
    assert!(f.accepts_log(good.clone()));
    for raw in [
        vec![],
        good[..good.len() - 1].to_vec(),
        [b"\n".as_slice(), &good].concat(),
        vec![b' '; MAX_LOG_BYTES as usize + 1],
        vec![b' '; 1_048_578],
        [
            b"{\"reason\":\"build-finished\",\"reason\":\"build-finished\",\"success\":true}\n"
                .as_slice(),
            &good,
        ]
        .concat(),
    ] {
        assert!(!f.accepts_log(raw));
    }
    let raw = String::from_utf8(good).unwrap().replacen(
        "\"name\":",
        "\"name\":\"duplicate\",\"name\":",
        1,
    );
    assert!(!f.accepts_log(raw.into_bytes()));
}

#[test]
fn seeded_content_corruption_cannot_reuse_checked_receipt() {
    const SEED: u64 = 0x0074_b017;
    eprintln!("held-build content corruption seed={SEED:#x} iterations=128");
    let f = Fixture::new();
    let build =
        held::verify_receipt(&held::document_bytes(&f.receipt()).unwrap(), &f.checked()).unwrap();
    let mut state = SEED;
    for i in 0..128 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let mut bytes = if i % 2 == 0 {
            f.host.clone()
        } else {
            f.launcher.clone()
        };
        let pos = state as usize % bytes.len();
        bytes[pos] ^= 1 << ((state >> 32) % 8);
        let c = if i % 2 == 0 {
            held::Contents {
                original_host: &bytes,
                ..f.contents()
            }
        } else {
            held::Contents {
                held_launcher: &bytes,
                ..f.contents()
            }
        };
        assert!(
            build.verify_contents(&c).is_err(),
            "seed={SEED:#x} case={i}"
        );
    }
}

#[test]
fn legacy_policy_must_independently_authenticate_the_same_controller() {
    let mut f = Fixture::new();
    f.checked();
    let mut worker_policy: worker::BuilderPolicy = serde_json::from_slice(&f.worker_raw).unwrap();
    worker_policy.controller_key_hex =
        hex(SigningKey::from_bytes(&[20; 32]).verifying_key().as_bytes());
    f.worker_raw = worker::policy_bytes(&worker_policy).unwrap();
    let raw = held::document_bytes(&f.policy).unwrap();
    assert!(f.load(&raw, &hash(&raw)).is_err()); // both pins correct, controllers differ
}

#[test]
fn cargo_dependencies_are_allowed_but_valid_oversized_and_duplicate_records_refuse() {
    let mut f = Fixture::new();
    let mut e = events();
    let mut dependency = event("dependency", "lib.rs");
    dependency["target"]["kind"] = json!(["lib"]);
    dependency["target"]["crate_types"] = json!(["lib"]);
    dependency["executable"] = Value::Null;
    e.insert(0, dependency.clone());
    dependency["target"]["kind"] = json!(["custom-build"]);
    dependency["target"]["crate_types"] = json!(["bin"]);
    e.insert(0, dependency);
    assert!(f.accepts_log(log(&e)));
    let good = String::from_utf8(log(&events())).unwrap();
    for (from, to) in [
        (
            "\"reason\":\"compiler-artifact\"",
            "\"reason\":\"compiler-artifact\",\"reason\":\"compiler-artifact\"",
        ),
        ("\"test\":false", "\"test\":false,\"test\":false"),
        ("\"success\":true", "\"success\":true,\"success\":true"),
    ] {
        let bad = good.replacen(from, to, 1);
        assert_ne!(bad, good);
        assert!(!f.accepts_log(bad.into_bytes()));
    }
    let mut e = events();
    e.insert(0,json!({"reason":"compiler-message","message":{"level":"warning","rendered":"x".repeat(1_048_576)}}));
    assert!(!f.accepts_log(log(&e)));
    let mut e = events();
    e[2].as_object_mut().unwrap().remove("success");
    assert!(!f.accepts_log(log(&e)));
}

#[test]
fn valid_but_noncanonical_or_duplicate_documents_and_worker_signed_body_refuse() {
    let f = Fixture::new();
    let policy = f.checked();
    let receipt = f.receipt();
    let raw = held::document_bytes(&receipt).unwrap();
    held::verify_receipt(&raw, &policy).unwrap();
    let mut pretty = serde_json::to_vec_pretty(&receipt).unwrap();
    pretty.push(b'\n');
    assert!(held::verify_receipt(&pretty, &policy).is_err());
    let raw = String::from_utf8(raw).unwrap();
    let bad = raw.replacen(
        "\"authority_epoch\":7",
        "\"authority_epoch\":7,\"authority_epoch\":7",
        1,
    );
    assert_ne!(raw, bad);
    assert!(held::verify_receipt(bad.as_bytes(), &policy).is_err());
    let p = String::from_utf8(held::document_bytes(&f.policy).unwrap()).unwrap();
    let bad = p.replacen(
        "\"authority_epoch\":7",
        "\"authority_epoch\":7,\"authority_epoch\":7",
        1,
    );
    assert_ne!(p, bad);
    assert!(f.load(bad.as_bytes(), &hash(bad.as_bytes())).is_err());
    let mut receipt = serde_json::to_value(receipt).unwrap();
    let mut message = held::SIGNATURE_DOMAIN.to_vec();
    message.push(0);
    message.extend(held::document_bytes(&receipt["statement"]).unwrap());
    receipt["signature_hex"] = json!(hex(&f.worker.sign(&message).to_bytes()));
    assert!(held::verify_receipt(&held::document_bytes(&receipt).unwrap(), &policy).is_err());
    assert!(serde_json::from_str::<
        hydracache_long_run_supervisor_074::diagnostic_artifacts::SignedBuild,
    >(&raw)
    .is_err());
}
