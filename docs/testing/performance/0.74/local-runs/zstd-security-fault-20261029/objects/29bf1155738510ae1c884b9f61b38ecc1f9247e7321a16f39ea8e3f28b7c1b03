//! This RSA private test fixture is publicly committed, never a production key.
#[path = "diagnostic_controller_fence_preflight_tests.rs"]
mod fence_preflight_tests;
#[path = "diagnostic_oidc_roster_tests.rs"]
mod roster_tests;
use super::profile::Profile;
use super::*;
use crate::diagnostic_ipc::DiagnosticRequest;
use crate::diagnostic_worker_policy::trusted_input::controller_input::tests::Model;
use crate::diagnostic_worker_policy::trusted_input::tests::{
    controller_worker_fixture, kernel_packet_bytes,
};
use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    Engine,
};
use serde_json::{json, Value};
use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};

const BUILDER_POLICY: &[u8] =
    include_bytes!("../../../docs/testing/performance/0.74/diagnostic-builder-public-policy.json");
const RECEIPT: &[u8] = include_bytes!("../../../docs/testing/performance/0.74/local-runs/diagnostic-builder-hosted-37923423889/build-receipt-v1.json");
fn signer() -> ring::signature::RsaKeyPair {
    ring::signature::RsaKeyPair::from_pkcs8(
        &STANDARD
            .decode(include_str!("../tests/fixtures/public-synthetic-oidc-rsa-2048.pk8.b64").trim())
            .unwrap(),
    )
    .unwrap()
}
fn request(model: &Model) -> DiagnosticRequest {
    let mut r = model.request();
    r.identity.binary_sha256 =
        "bda1c094f3de7c6d42ec45c757838c7a41a8ff1d63a025c661af84455169a101".into();
    r.identity.build_provenance_sha256 = crate::sha256_hex(RECEIPT);
    r
}
fn line(v: &impl serde::Serialize) -> Vec<u8> {
    let mut b = crate::canonical_json(v).unwrap();
    b.push(b'\n');
    b
}
fn policy(r: &DiagnosticRequest) -> Profile {
    let pair = signer();
    let key = ring::signature::RsaPublicKeyComponents::<Vec<u8>>::from(pair.public());
    Profile {
        schema_version: profile::SCHEMA.into(),
        repository_id: r.controller.repository_id,
        issuer: profile::ISSUER.into(),
        workflow_ref: profile::WORKFLOW.into(),
        workflow_sha: "1".repeat(40),
        environment: profile::ENVIRONMENT.into(),
        subject: "synthetic-public-fixture-not-github-enrollment".into(),
        snapshot_not_before: r.issued_at_unix_seconds - 1,
        snapshot_expires_at: r.expires_at_unix_seconds + 600,
        key: profile::Key {
            kid: "synthetic-oidc-only".into(),
            kty: "RSA".into(),
            alg: "RS256".into(),
            usage: "sig".into(),
            n: URL_SAFE_NO_PAD.encode(key.n),
            e: URL_SAFE_NO_PAD.encode(key.e),
        },
    }
}
fn claims(r: &DiagnosticRequest, p: &Profile) -> Value {
    let hash = crate::sha256_hex(&crate::canonical_json(r).unwrap());
    json!({"iss":profile::ISSUER,"aud":format!("urn:hydracache:diagnostic:074:request:{hash}"),"sub":p.subject,
        "repository":"javaquasar/hydracache","repository_id":r.controller.repository_id.to_string(),
        "actor_id":r.controller.actor_id.to_string(),"run_id":r.controller.run_id.to_string(),"run_attempt":r.controller.run_attempt.to_string(),
        "workflow_ref":p.workflow_ref,"workflow_sha":p.workflow_sha,"environment":p.environment,
        "event_name":"workflow_dispatch","runner_environment":"github-hosted","jti":"synthetic-test-token",
        "nbf":r.issued_at_unix_seconds,"iat":r.issued_at_unix_seconds,"exp":r.expires_at_unix_seconds,
        "head_ref":"","base_ref":""})
}
fn token(header: &[u8], claims: &[u8]) -> Vec<u8> {
    let input = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(header),
        URL_SAFE_NO_PAD.encode(claims)
    );
    let pair = signer();
    let mut signature = vec![0; pair.public().modulus_len()];
    pair.sign(
        &ring::signature::RSA_PKCS1_SHA256,
        &ring::rand::SystemRandom::new(),
        input.as_bytes(),
        &mut signature,
    )
    .unwrap();
    format!("{input}.{}", URL_SAFE_NO_PAD.encode(signature)).into_bytes()
}
fn header(p: &Profile) -> Vec<u8> {
    serde_json::to_vec(&json!({"alg":"RS256","typ":"JWT","kid":p.key.kid})).unwrap()
}
fn check(p: &Profile, t: &[u8], r: &DiagnosticRequest, now: u64) -> Result<String, OidcError> {
    profile::verify(
        p,
        t,
        r,
        &crate::sha256_hex(&crate::canonical_json(r).unwrap()),
        now,
    )
}

#[test]
fn oidc_profile_original_jws_bytes_and_time_boundaries() {
    let r = request(&Model::new());
    let p = policy(&r);
    Profile::decode(&line(&p)).unwrap();
    let c = claims(&r, &p);
    for raw in [
        serde_json::to_vec(&c).unwrap(),
        serde_json::to_vec_pretty(&c).unwrap(),
    ] {
        let t = token(&header(&p), &raw);
        check(&p, &t, &r, r.issued_at_unix_seconds).unwrap();
        check(&p, &t, &r, r.expires_at_unix_seconds - 1).unwrap();
        assert!(check(&p, &t, &r, r.issued_at_unix_seconds - 1).is_err());
        assert!(check(&p, &t, &r, r.expires_at_unix_seconds).is_err());
        let mut stale = policy(&r);
        stale.snapshot_expires_at = r.issued_at_unix_seconds;
        assert!(check(&stale, &t, &r, r.issued_at_unix_seconds).is_err());
        stale.snapshot_not_before = r.issued_at_unix_seconds + 1;
        assert!(check(&stale, &t, &r, r.issued_at_unix_seconds).is_err());
    }
}
#[test]
fn oidc_profile_each_origin_time_type_and_duplicate_claim_refuses() {
    let r = request(&Model::new());
    let p = policy(&r);
    let original = claims(&r, &p);
    for name in original
        .as_object()
        .unwrap()
        .keys()
        .filter(|k| !matches!(k.as_str(), "head_ref" | "base_ref"))
    {
        for value in [Value::Null, json!([]), json!({}), json!(false), json!("")] {
            let mut c = original.clone();
            c[name] = value;
            assert!(
                check(
                    &p,
                    &token(&header(&p), &serde_json::to_vec(&c).unwrap()),
                    &r,
                    r.issued_at_unix_seconds
                )
                .is_err(),
                "{name}"
            );
        }
        let mut c = original.clone();
        c.as_object_mut().unwrap().remove(name);
        assert!(check(
            &p,
            &token(&header(&p), &serde_json::to_vec(&c).unwrap()),
            &r,
            r.issued_at_unix_seconds
        )
        .is_err());
    }
    for (field, value) in [
        ("run_id", json!("077")),
        ("exp", json!(r.issued_at_unix_seconds + 601)),
        ("iat", json!(r.issued_at_unix_seconds + 1)),
        ("nbf", json!(r.issued_at_unix_seconds + 1)),
        ("job_workflow_ref", json!("unreviewed")),
        ("job_workflow_sha", json!("1".repeat(40))),
    ] {
        let mut c = original.clone();
        c[field] = value;
        assert!(check(
            &p,
            &token(&header(&p), &serde_json::to_vec(&c).unwrap()),
            &r,
            r.issued_at_unix_seconds
        )
        .is_err());
    }
    let valid = serde_json::to_string(&original).unwrap();
    for suffix in [
        "\"x\":1,\"x\":2".to_string(),
        format!("\"\\u0069ss\":\"{}\"", profile::ISSUER),
        "\"x\":1.0".to_string(),
    ] {
        let raw = format!("{},{}{}", valid.trim_end_matches('}'), suffix, "}");
        assert!(check(
            &p,
            &token(&header(&p), raw.as_bytes()),
            &r,
            r.issued_at_unix_seconds
        )
        .is_err());
    }
}
#[test]
fn oidc_profile_key_header_and_compact_encoding_confusion_refuse() {
    let r = request(&Model::new());
    let p = policy(&r);
    let c = serde_json::to_vec(&claims(&r, &p)).unwrap();
    for (field, value) in [
        ("alg", "none"),
        ("alg", "HS256"),
        ("kid", "unknown"),
        ("typ", "other"),
        ("jku", "https://untrusted.invalid/keys"),
        ("crit", "ignored"),
        ("b64", "false"),
    ] {
        let mut h: Value = serde_json::from_slice(&header(&p)).unwrap();
        h[field] = json!(value);
        assert!(check(
            &p,
            &token(&serde_json::to_vec(&h).unwrap(), &c),
            &r,
            r.issued_at_unix_seconds
        )
        .is_err());
    }
    let t = token(&header(&p), &c);
    for bad in [
        vec![],
        vec![b'x'; MAX_TOKEN + 1],
        b"a.b.c.d".to_vec(),
        [t.clone(), b"=".to_vec()].concat(),
        b"a=.b.c".to_vec(),
        vec![255],
    ] {
        assert!(check(&p, &bad, &r, r.issued_at_unix_seconds).is_err());
    }
    let mut wrong = policy(&r);
    let mut n = URL_SAFE_NO_PAD.decode(&wrong.key.n).unwrap();
    n[1] ^= 1;
    wrong.key.n = URL_SAFE_NO_PAD.encode(n);
    assert!(matches!(
        check(&wrong, &t, &r, r.issued_at_unix_seconds),
        Err(OidcError::Signature)
    ));
    assert!(profile::verify(&p, &t, &r, &"0".repeat(64), r.issued_at_unix_seconds).is_err());
}
#[test]
fn oidc_profile_additional_scalar_claim_budget_and_duplicate_header_boundaries() {
    let r = request(&Model::new());
    let p = policy(&r);
    let mut c = claims(&r, &p);
    let remaining = 64 - c.as_object().unwrap().len();
    for i in 0..remaining {
        c[format!("extra{i}")] = json!(false);
    }
    check(
        &p,
        &token(&header(&p), &serde_json::to_vec(&c).unwrap()),
        &r,
        r.issued_at_unix_seconds,
    )
    .unwrap();
    c["extra_over_limit"] = json!(true);
    assert!(check(
        &p,
        &token(&header(&p), &serde_json::to_vec(&c).unwrap()),
        &r,
        r.issued_at_unix_seconds
    )
    .is_err());
    for (name, value, valid) in [
        ("k".repeat(64), "v".repeat(2048), true),
        ("k".repeat(65), "v".into(), false),
        ("k".into(), "v".repeat(2049), false),
        ("".into(), "v".into(), false),
        ("k".into(), "\0".into(), false),
    ] {
        let mut c = claims(&r, &p);
        c[name] = json!(value);
        assert_eq!(
            check(
                &p,
                &token(&header(&p), &serde_json::to_vec(&c).unwrap()),
                &r,
                r.issued_at_unix_seconds
            )
            .is_ok(),
            valid
        );
    }
    let payload = serde_json::to_vec(&claims(&r, &p)).unwrap();
    for raw in [
        b"{\"alg\":\"RS256\",\"alg\":\"RS256\",\"kid\":\"synthetic-oidc-only\",\"typ\":\"JWT\"}"
            .as_slice(),
        b"{\"alg\":\"RS256\",\"kid\":\"synthetic-oidc-only\"}".as_slice(),
    ] {
        assert!(check(&p, &token(raw, &payload), &r, r.issued_at_unix_seconds).is_err());
    }
}
#[test]
fn oidc_profile_canonical_schema_snapshot_and_rsa_key_bounds() {
    let r = request(&Model::new());
    let raw = line(&policy(&r));
    let original: Value = serde_json::from_slice(&raw).unwrap();
    for k in original.as_object().unwrap().keys() {
        let mut v = original.clone();
        v.as_object_mut().unwrap().remove(k);
        assert!(Profile::decode(&line(&v)).is_err());
        v = original.clone();
        v[k] = Value::Null;
        assert!(Profile::decode(&line(&v)).is_err());
    }
    for (k, v) in [
        ("schema_version", json!("future")),
        ("repository_id", json!(17)),
        ("issuer", json!("https://untrusted.invalid")),
        ("workflow_ref", json!("other")),
        ("workflow_sha", json!("f".repeat(39))),
        ("environment", json!("other")),
        ("subject", json!("")),
        ("snapshot_not_before", json!(0)),
        ("snapshot_expires_at", json!(0)),
        ("unknown", json!(true)),
    ] {
        let mut changed = original.clone();
        changed[k] = v;
        assert!(Profile::decode(&line(&changed)).is_err());
    }
    for (k, v) in [
        ("alg", json!("HS256")),
        ("kty", json!("oct")),
        ("use", json!("enc")),
        ("kid", json!("")),
        ("e", json!("Aw")),
        ("n", json!(URL_SAFE_NO_PAD.encode([0; 256]))),
        ("n", json!(URL_SAFE_NO_PAD.encode([255; 255]))),
        ("n", json!(URL_SAFE_NO_PAD.encode([255; 513]))),
        ("url", json!("untrusted")),
    ] {
        let mut changed = original.clone();
        changed["key"][k] = v;
        assert!(Profile::decode(&line(&changed)).is_err());
    }
    for bad in [
        vec![],
        vec![b'x'; 2049],
        raw[..raw.len() - 1].to_vec(),
        [raw, b" ".to_vec()].concat(),
    ] {
        assert!(Profile::decode(&bad).is_err());
    }
}

struct TrustModel {
    root: tempfile::TempDir,
    trust: OidcTrustInput,
}
impl TrustModel {
    fn new(r: &DiagnosticRequest) -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        fs::create_dir(root.path().join("etc")).unwrap();
        fs::create_dir(root.path().join("etc/hydracache-perf")).unwrap();
        let path = root.path().join("etc/hydracache-perf").join(FILE);
        fs::write(&path, line(&policy(r))).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
        let trust = OidcTrustInput {
            source: Source::open(root.path(), unsafe { libc::geteuid() }).unwrap(),
            refused: false,
            _original_thread: PhantomData,
        };
        Self { root, trust }
    }
    fn path(&self) -> PathBuf {
        self.root.path().join("etc/hydracache-perf").join(FILE)
    }
}
fn binding(case: Option<u8>) {
    let model = Model::new();
    let r = request(&model);
    let t = token(
        &header(&policy(&r)),
        &serde_json::to_vec(&claims(&r, &policy(&r))).unwrap(),
    );
    let bytes = model.sign(r.clone(), &model.key);
    let (_root, _authority_root, mut source, mut authority, worker_bytes) =
        controller_worker_fixture();
    let mut packet = kernel_packet_bytes(&bytes);
    let mut worker = source
        .verify_kernel_policy(&worker_bytes, &mut packet, &mut authority)
        .unwrap();
    let mut controller = model.input();
    let mut oidc = TrustModel::new(&r);
    let path = oidc.path();
    {
        let mut original = controller.authenticate_controller(&mut worker).unwrap();
        let mut build = original
            .bind_reviewed_build(BUILDER_POLICY, RECEIPT)
            .unwrap();
        let mut caller = t;
        let mut guard = build.bind_oidc(&mut oidc.trust, &caller).unwrap();
        caller.fill(0);
        guard.revalidate().unwrap();
        if let Some(case) = case {
            match case {
                0 => guard.token[10] ^= 1,
                1 => guard.token_digest = "0".repeat(64),
                2 => fs::write(&path, b"drift\n").unwrap(),
                3 => {
                    fs::rename(&path, path.with_extension("old")).unwrap();
                    fs::write(&path, line(&policy(&r))).unwrap();
                }
                4 => fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap(),
                5 => guard.original.original.request.controller.run_id += 1,
                6 => guard.original.original.worker.latch_refusal(),
                _ => guard.trust.refused = true,
            }
            assert!(guard.revalidate().is_err());
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
            fs::write(&path, line(&policy(&r))).unwrap();
            assert!(matches!(guard.revalidate(), Err(OidcError::Refused)));
        } else {
            for _ in 0..3 {
                guard.revalidate().unwrap();
            }
        }
    }
    let refused = case.is_some();
    assert_eq!(oidc.trust.is_refused(), refused);
    assert_eq!(controller.is_refused(), refused);
    assert_eq!(worker.is_refused(), refused);
    drop(worker);
    assert_eq!(source.is_refused(), refused);
    assert_eq!(authority.is_failed(), refused);
    assert_eq!(packet.is_refused(), refused);
}
#[test]
fn oidc_original_binding_owned_bytes_healthy_drop_and_sticky_faults() {
    binding(None);
    for case in 0..8 {
        binding(Some(case));
    }
}
#[test]
fn oidc_constructor_bad_token_source_and_prior_refusal_cross_latch_all_owners() {
    for case in 0..6 {
        let model = Model::new();
        let r = request(&model);
        let p = policy(&r);
        let mut t = token(&header(&p), &serde_json::to_vec(&claims(&r, &p)).unwrap());
        let bytes = model.sign(r.clone(), &model.key);
        let (_root, _authority_root, mut source, mut authority, worker_bytes) =
            controller_worker_fixture();
        let mut packet = kernel_packet_bytes(&bytes);
        let mut worker = source
            .verify_kernel_policy(&worker_bytes, &mut packet, &mut authority)
            .unwrap();
        let mut controller = model.input();
        let mut oidc = TrustModel::new(&r);
        {
            let mut original = controller.authenticate_controller(&mut worker).unwrap();
            let mut build = original
                .bind_reviewed_build(BUILDER_POLICY, RECEIPT)
                .unwrap();
            match case {
                0 => t[10] ^= 1,
                1 => t.clear(),
                2 => t = vec![0; MAX_TOKEN + 1],
                3 => oidc.trust.refused = true,
                4 => fs::write(oidc.path(), b"changed\n").unwrap(),
                _ => build.original.worker.latch_refusal(),
            }
            assert!(build.bind_oidc(&mut oidc.trust, &t).is_err());
            assert!(build.is_refused() && oidc.trust.is_refused());
            let valid = token(&header(&p), &serde_json::to_vec(&claims(&r, &p)).unwrap());
            assert!(build.bind_oidc(&mut oidc.trust, &valid).is_err());
        }
        assert!(controller.is_refused() && worker.is_refused());
        drop(worker);
        assert!(source.is_refused() && authority.is_failed() && packet.is_refused());
    }
}
#[test]
fn oidc_source_nofollow_modes_links_bounds_and_fixed_nonroot_refusal() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    assert!(matches!(open_fixed_oidc_trust(), Err(OidcError::Invalid)));
    assert_eq!(
        Path::new("/")
            .join("etc")
            .join("hydracache-perf")
            .join(FILE),
        Path::new(FIXED_OIDC_PATH)
    );
    for case in 0..6 {
        let r = request(&Model::new());
        let m = TrustModel::new(&r);
        let path = m.path();
        match case {
            0 => fs::set_permissions(&path, fs::Permissions::from_mode(0o666)).unwrap(),
            1 => {
                fs::rename(&path, path.with_extension("old")).unwrap();
                symlink(path.with_extension("old"), &path).unwrap();
            }
            2 => fs::hard_link(&path, path.with_extension("link")).unwrap(),
            3 => fs::write(&path, []).unwrap(),
            4 => fs::write(&path, vec![0; 2049]).unwrap(),
            _ => fs::set_permissions(m.root.path().join("etc"), fs::Permissions::from_mode(0o777))
                .unwrap(),
        }
        assert!(Source::open(m.root.path(), unsafe { libc::geteuid() }).is_err());
    }
}
#[test]
fn oidc_seeded_signature_mutations_and_independent_bindings() {
    let r = request(&Model::new());
    let p = policy(&r);
    let t = token(&header(&p), &serde_json::to_vec(&claims(&r, &p)).unwrap());
    let text = String::from_utf8(t).unwrap();
    let parts: Vec<_> = text.split('.').collect();
    let signature = URL_SAFE_NO_PAD.decode(parts[2]).unwrap();
    let mut seed = 0x07552074_u64;
    for _ in 0..32 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut bad = signature.clone();
        bad[(seed as usize) % signature.len()] ^= 1;
        let changed = format!("{}.{}.{}", parts[0], parts[1], URL_SAFE_NO_PAD.encode(bad));
        assert!(matches!(
            check(&p, changed.as_bytes(), &r, r.issued_at_unix_seconds),
            Err(OidcError::Signature)
        ));
    }
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4).map(|_| scope.spawn(|| binding(None))).collect();
        for h in handles {
            h.join().unwrap();
        }
    });
    println!("seed=0x07552074 signature_mutations=32 independent_bindings=4 real_github_issuer_enrollment=false");
}
