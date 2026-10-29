use ed25519_dalek::{Signer, SigningKey};
use hydracache_long_run_supervisor_074::diagnostic_signer_bootstrap_context::{
    AssertedBootstrapContext, SCHEMA,
};
use hydracache_long_run_supervisor_074::{
    diagnostic_artifacts::ArtifactDigest, diagnostic_signer_build::*,
};

#[path = "support/diagnostic_bootstrap_context.rs"]
mod context_fixture;

fn lf(value: &impl serde::Serialize) -> Vec<u8> {
    let mut raw = serde_json::to_vec(&serde_json::to_value(value).unwrap()).unwrap();
    raw.push(b'\n');
    raw
}
struct Fixture {
    key: SigningKey,
    statement: SignerBuildStatement,
    bytes: [Vec<u8>; 5],
}
impl Fixture {
    fn new() -> Self {
        let bytes = [
            b"not ELF".to_vec(),
            b"lock".to_vec(),
            b"log".to_vec(),
            b"opaque runtime".to_vec(),
            b"opaque context".to_vec(),
        ];
        let statement = SignerBuildStatement {
            schema_version: SCHEMA_BUILD.into(),
            repository_id: 1217101761,
            role: ROLE.into(),
            source_commit: "a".repeat(40),
            source_tree: "b".repeat(40),
            source_clean_before: true,
            source_clean_after: true,
            rustc_version: RUSTC_VERSION.into(),
            cargo_version: CARGO_VERSION.into(),
            target: "x86_64-unknown-linux-gnu".into(),
            profile: "release".into(),
            features: vec![],
            allocator: "System".into(),
            counting_allocator: false,
            build_command: BUILD_COMMAND.iter().map(|s| s.to_string()).collect(),
            binary: ArtifactDigest::of(&bytes[0]),
            root_lock: ArtifactDigest::of(&bytes[1]),
            build_log: ArtifactDigest::of(&bytes[2]),
            runtime_manifest: ArtifactDigest::of(&bytes[3]),
            bootstrap_context: ArtifactDigest::of(&bytes[4]),
        };
        let mut f = Self {
            key: SigningKey::from_bytes(&[95; 32]),
            statement,
            bytes,
        };
        f.set_raw(lf(&context_fixture::synthetic_context(&f.statement)));
        f
    }
    fn set_raw(&mut self, raw: Vec<u8>) {
        self.statement.bootstrap_context = ArtifactDigest::of(&raw);
        self.bytes[4] = raw;
    }
    fn selection(&self) -> AssertedSignerBuildSelection {
        AssertedSignerBuildSelection::new(self.key.verifying_key(), self.statement.clone()).unwrap()
    }
    fn contents(&self) -> SignerBuildContents<'_> {
        SignerBuildContents {
            binary: &self.bytes[0],
            root_lock: &self.bytes[1],
            build_log: &self.bytes[2],
            runtime_manifest: &self.bytes[3],
            bootstrap_context: &self.bytes[4],
        }
    }
    fn receipt(&self) -> Vec<u8> {
        let mut msg = SIGNATURE_DOMAIN.to_vec();
        msg.push(0);
        let statement = lf(&self.statement);
        msg.extend(&statement[..statement.len() - 1]);
        lf(&SignedSignerBuild {
            statement: self.statement.clone(),
            signature_hex: self
                .key
                .sign(&msg)
                .to_bytes()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
        })
    }
    fn refuse_only_stronger(&self) {
        let s = self.selection();
        let r = self.receipt();
        s.verify_supplied(&r, &self.contents()).unwrap();
        assert!(s
            .verify_supplied_with_bootstrap_context(&r, &self.contents())
            .is_err());
    }
}
const SCHEMA_BUILD: &str = hydracache_long_run_supervisor_074::diagnostic_signer_build::SCHEMA;

#[test]
fn fixed_bootstrap_schema_is_statement_data_not_authority() {
    assert_eq!(SCHEMA, "diagnostic-signer-bootstrap-context-074-v1");
    let _: Option<AssertedBootstrapContext> = None;
}

#[test]
fn matching_declared_context_checks_signature_and_all_content_without_enrollment() {
    let f = Fixture::new();
    let receipt = f.receipt();
    let m = f
        .selection()
        .verify_supplied_with_bootstrap_context(&receipt, &f.contents())
        .unwrap();
    assert_eq!(
        m.diagnostic_hashes(),
        (
            ArtifactDigest::of(&receipt).sha256.as_str(),
            f.statement.bootstrap_context.sha256.as_str()
        )
    );
    // Portable validation intentionally does not inspect ELF or acquire a launcher.
    assert_eq!(f.bytes[0], b"not ELF");
}

#[test]
fn every_binding_fixed_profile_and_false_declaration_is_checked_even_after_resigning() {
    let f = Fixture::new();
    let c: serde_json::Value = serde_json::from_slice(&f.bytes[4]).unwrap();
    for name in c.as_object().unwrap().keys() {
        let mut f = Fixture::new();
        let mut changed = c.clone();
        changed[name] = match name.as_str() {
            "launcher" => serde_json::to_value(&f.statement.binary).unwrap(),
            "workflow_sha" => serde_json::json!("0".repeat(40)),
            _ => match &c[name] {
                serde_json::Value::String(s) => serde_json::json!(format!("{s}x")),
                serde_json::Value::Bool(_) => serde_json::json!(true),
                serde_json::Value::Number(_) => serde_json::json!(0),
                serde_json::Value::Object(_) => {
                    serde_json::json!({"sha256":"d".repeat(64),"bytes":1})
                }
                _ => panic!("unexpected context field"),
            },
        };
        f.set_raw(lf(&changed));
        f.refuse_only_stronger();
    }
}

#[test]
fn canonical_schema_unknown_duplicate_missing_types_and_budget_refuse() {
    let f = Fixture::new();
    let c: serde_json::Value = serde_json::from_slice(&f.bytes[4]).unwrap();
    let mut unknown = c.clone();
    unknown["admission_allowed"] = serde_json::json!(false);
    let mut missing = c.clone();
    missing.as_object_mut().unwrap().remove("launcher");
    let mut numeric = c.clone();
    numeric["build_has_oidc_authority"] = serde_json::json!(0);
    let raw = std::str::from_utf8(&f.bytes[4]).unwrap();
    for bytes in [
        b"opaque context".to_vec(),
        lf(&unknown),
        lf(&missing),
        lf(&numeric),
        serde_json::to_vec_pretty(&c).unwrap(),
        f.bytes[4][..f.bytes[4].len() - 1].to_vec(),
        [f.bytes[4].as_slice(), b"\n"].concat(),
        raw.replace('\n', "\r\n").into_bytes(),
        raw.replacen(
            "\"event_name\":",
            "\"event_name\":\"workflow_dispatch\",\"event_name\":",
            1,
        )
        .into_bytes(),
        raw.replacen("\"sha256\":", "\"sha256\":\"d\",\"sha256\":", 1)
            .into_bytes(),
        vec![b' '; MAX_CONTEXT_BYTES as usize],
    ] {
        let mut f = Fixture::new();
        f.set_raw(bytes);
        f.refuse_only_stronger();
    }
    let mut f = Fixture::new();
    f.set_raw(vec![0; MAX_CONTEXT_BYTES as usize + 1]);
    assert!(AssertedSignerBuildSelection::new(f.key.verifying_key(), f.statement).is_err());
}

#[test]
fn launcher_and_workflow_pins_are_bounded_assertions_not_discovered_authority() {
    for field in ["source_commit", "source_tree"] {
        let mut f = Fixture::new();
        if field == "source_commit" {
            f.statement.source_commit = "0".repeat(40);
        } else {
            f.statement.source_tree = "0".repeat(40);
        }
        f.set_raw(lf(&context_fixture::synthetic_context(&f.statement)));
        f.refuse_only_stronger();
    }
    for value in [
        serde_json::json!({"sha256":"INVALID","bytes":1}),
        serde_json::json!({"sha256":"0".repeat(64),"bytes":1}),
        serde_json::json!({"sha256":"a".repeat(64),"bytes":0}),
        serde_json::json!({"sha256":"a".repeat(64),"bytes":MAX_BINARY_BYTES+1}),
    ] {
        let mut f = Fixture::new();
        let mut c: serde_json::Value = serde_json::from_slice(&f.bytes[4]).unwrap();
        c["launcher"] = value;
        f.set_raw(lf(&c));
        f.refuse_only_stronger();
    }
    let mut f = Fixture::new();
    let old = f.selection();
    let mut c: serde_json::Value = serde_json::from_slice(&f.bytes[4]).unwrap();
    c["launcher"] = serde_json::to_value(ArtifactDigest::of(b"another asserted launcher")).unwrap();
    c["workflow_sha"] = serde_json::json!("d".repeat(40));
    f.set_raw(lf(&c));
    assert!(old
        .verify_supplied_with_bootstrap_context(&f.receipt(), &f.contents())
        .is_err());
    // Explicitly selecting another full expected statement may pass syntax;
    // this model must not be confused with independent pin enrollment.
    f.selection()
        .verify_supplied_with_bootstrap_context(&f.receipt(), &f.contents())
        .unwrap();
}

#[test]
fn wrong_receipt_key_signature_and_unbound_content_cannot_gain_stronger_result() {
    let mut f = Fixture::new();
    let selection = f.selection();
    let raw = f.receipt();
    f.key = SigningKey::from_bytes(&[96; 32]);
    assert!(selection
        .verify_supplied_with_bootstrap_context(&f.receipt(), &f.contents())
        .is_err());
    for index in 0..5 {
        f.bytes[index][0] ^= 1;
        assert!(selection
            .verify_supplied_with_bootstrap_context(&raw, &f.contents())
            .is_err());
        f.bytes[index][0] ^= 1;
    }
}

#[test]
fn seeded_signed_context_mutations_and_four_independent_models() {
    let mut seed = 0x0740_c078u64;
    for _ in 0..64 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut f = Fixture::new();
        let mut c = context_fixture::synthetic_context(&f.statement);
        let mut schema = c.schema_version.into_bytes();
        let index = seed as usize % schema.len();
        schema[index] ^= 1;
        c.schema_version = String::from_utf8(schema).unwrap();
        f.set_raw(lf(&c));
        f.refuse_only_stronger();
    }
    let handles: Vec<_> = (0..4)
        .map(|_| {
            std::thread::spawn(|| {
                let f = Fixture::new();
                f.selection()
                    .verify_supplied_with_bootstrap_context(&f.receipt(), &f.contents())
                    .unwrap();
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
    println!("seed=0x0740c078 resigned_context_mutations=64 independent_models=4 actual_launcher_or_job_enrolled=false");
}
