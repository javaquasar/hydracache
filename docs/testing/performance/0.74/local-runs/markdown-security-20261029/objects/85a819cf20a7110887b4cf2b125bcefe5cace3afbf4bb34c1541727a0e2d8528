//! Local synthetic configured rosters only, never actual GitHub trust selection.
use super::super::profile::{TrustProfile, MAX_ROSTER_BYTES, MAX_ROSTER_KEYS};
use super::*;

fn roster(r: &DiagnosticRequest, count: usize, signing_position: usize) -> Value {
    let mut p = serde_json::to_value(policy(r)).unwrap();
    let key = p.as_object_mut().unwrap().remove("key").unwrap();
    let keys: Vec<_> = (0..count)
        .map(|i| {
            let mut k = key.clone();
            k["kid"] = json!(format!("synthetic-{i:02}"));
            if i != signing_position {
                let mut n = URL_SAFE_NO_PAD.decode(k["n"].as_str().unwrap()).unwrap();
                n[1] ^= (i + 1) as u8;
                k["n"] = json!(URL_SAFE_NO_PAD.encode(n));
            }
            k
        })
        .collect();
    p["schema_version"] = json!("diagnostic-oidc-trust-input-074-v2");
    p["keys"] = json!(keys);
    p
}
fn roster_token(r: &DiagnosticRequest, kid: &str) -> Vec<u8> {
    token(
        &serde_json::to_vec(&json!({"alg":"RS256","typ":"JWT","kid":kid})).unwrap(),
        &serde_json::to_vec(&claims(r, &policy(r))).unwrap(),
    )
}
fn roster_check(
    p: &TrustProfile,
    t: &[u8],
    r: &DiagnosticRequest,
    now: u64,
) -> Result<String, OidcError> {
    profile::verify(
        p,
        t,
        r,
        &crate::sha256_hex(&crate::canonical_json(r).unwrap()),
        now,
    )
}

#[test]
fn oidc_roster_v2_membership_each_position_v1_compatibility_and_no_try_all() {
    let r = request(&Model::new());
    let v1 = TrustProfile::decode(&line(&policy(&r))).unwrap();
    let t = token(
        &header(&policy(&r)),
        &serde_json::to_vec(&claims(&r, &policy(&r))).unwrap(),
    );
    roster_check(&v1, &t, &r, r.issued_at_unix_seconds).unwrap();
    assert!(Profile::decode(&line(&roster(&r, 1, 0))).is_err());
    for count in [1, 4, MAX_ROSTER_KEYS] {
        for position in 0..count {
            let p = TrustProfile::decode(&line(&roster(&r, count, position))).unwrap();
            for kid_position in 0..count {
                let result = roster_check(
                    &p,
                    &roster_token(&r, &format!("synthetic-{kid_position:02}")),
                    &r,
                    r.issued_at_unix_seconds,
                );
                if kid_position == position {
                    result.unwrap();
                } else {
                    assert!(matches!(result, Err(OidcError::Signature)));
                }
            }
            assert!(roster_check(
                &p,
                &roster_token(&r, "unknown"),
                &r,
                r.issued_at_unix_seconds
            )
            .is_err());
            assert!(roster_check(&p, &t, &r, r.issued_at_unix_seconds).is_err());
        }
    }
}

#[test]
fn oidc_roster_v2_shape_types_order_and_duplicate_material_refuse() {
    let r = request(&Model::new());
    let original = roster(&r, 4, 2);
    for field in original.as_object().unwrap().keys() {
        for replacement in [Value::Null, json!([]), json!({}), json!(false)] {
            let mut p = original.clone();
            p[field] = replacement;
            assert!(TrustProfile::decode(&line(&p)).is_err(), "{field}");
        }
        let mut p = original.clone();
        p.as_object_mut().unwrap().remove(field);
        assert!(TrustProfile::decode(&line(&p)).is_err());
    }
    for case in 0..11 {
        let mut p = original.clone();
        match case {
            0 => p["keys"] = json!([]),
            1 => p = roster(&r, MAX_ROSTER_KEYS + 1, 0),
            2 => p["keys"].as_array_mut().unwrap().swap(0, 1),
            3 => p["keys"][1]["kid"] = p["keys"][0]["kid"].clone(),
            4 => p["keys"][1]["n"] = p["keys"][0]["n"].clone(),
            5 => p["key"] = p["keys"][0].clone(),
            6 => p["schema_version"] = json!(profile::SCHEMA),
            7 => p["schema_version"] = json!("diagnostic-oidc-trust-input-074-v3"),
            8 => p["keys"][0]["jku"] = json!("https://foreign.invalid/keys"),
            9 => p["keys"][0]["kid"] = json!(""),
            _ => p["keys"][0]["kid"] = json!("x".repeat(129)),
        }
        assert!(TrustProfile::decode(&line(&p)).is_err(), "case={case}");
    }
    for (field, value) in [
        ("issuer", json!("https://foreign.invalid")),
        ("workflow_sha", json!("A".repeat(40))),
        ("workflow_ref", json!("other")),
        ("environment", json!("other")),
        ("subject", json!("")),
        ("repository_id", json!(17)),
        ("snapshot_not_before", json!(0)),
        ("snapshot_expires_at", json!(0)),
    ] {
        let mut p = original.clone();
        p[field] = value;
        assert!(TrustProfile::decode(&line(&p)).is_err(), "{field}");
    }
    for field in ["kid", "kty", "alg", "use", "n", "e"] {
        let mut p = original.clone();
        p["keys"][0].as_object_mut().unwrap().remove(field);
        assert!(TrustProfile::decode(&line(&p)).is_err());
        p = original.clone();
        p["keys"][0][field] = Value::Null;
        assert!(TrustProfile::decode(&line(&p)).is_err());
    }
}

#[test]
fn oidc_roster_v2_document_rsa_and_canonical_bounds() {
    let r = request(&Model::new());
    let original = roster(&r, MAX_ROSTER_KEYS, 0);
    let raw = line(&original);
    assert!(raw.len() > 2048 && raw.len() < MAX_ROSTER_BYTES);
    TrustProfile::decode(&raw).unwrap();
    for bad in [
        vec![],
        vec![b' '; MAX_ROSTER_BYTES + 1],
        raw[..raw.len() - 1].to_vec(),
        serde_json::to_vec_pretty(&original).unwrap(),
        [raw.clone(), b"\n".to_vec()].concat(),
        [raw[..raw.len() - 2].to_vec(), b",\"keys\":[]}\n".to_vec()].concat(),
    ] {
        assert!(TrustProfile::decode(&bad).is_err());
    }
    for bytes in [256, 512] {
        let mut p = roster(&r, 1, 0);
        p["keys"][0]["n"] = json!(URL_SAFE_NO_PAD.encode(vec![255; bytes]));
        TrustProfile::decode(&line(&p)).unwrap();
    }
    for value in [
        URL_SAFE_NO_PAD.encode(vec![255; 255]),
        URL_SAFE_NO_PAD.encode(vec![255; 513]),
        URL_SAFE_NO_PAD.encode(vec![0; 256]),
        URL_SAFE_NO_PAD.encode([vec![0], vec![255; 256]].concat()),
        format!("{}=", original["keys"][0]["n"].as_str().unwrap()),
    ] {
        let mut p = original.clone();
        p["keys"][0]["n"] = json!(value);
        assert!(TrustProfile::decode(&line(&p)).is_err());
    }
    for (field, value) in [
        ("kty", "oct"),
        ("alg", "HS256"),
        ("use", "enc"),
        ("e", "Aw"),
    ] {
        let mut p = original.clone();
        p["keys"][0][field] = json!(value);
        assert!(TrustProfile::decode(&line(&p)).is_err());
    }
    // Adding JSON whitespace never enlarges a v1 compatibility window.
    let mut v1 = line(&policy(&r));
    v1.splice(v1.len() - 1..v1.len() - 1, vec![b' '; 2048]);
    assert!(TrustProfile::decode(&v1).is_err());
}

#[test]
fn oidc_roster_v2_origin_header_and_exclusive_windows_are_unchanged() {
    let r = request(&Model::new());
    let original = roster(&r, 4, 1);
    let p = TrustProfile::decode(&line(&original)).unwrap();
    let t = roster_token(&r, "synthetic-01");
    roster_check(&p, &t, &r, r.issued_at_unix_seconds).unwrap();
    roster_check(&p, &t, &r, r.expires_at_unix_seconds - 1).unwrap();
    for now in [r.issued_at_unix_seconds - 1, r.expires_at_unix_seconds] {
        assert!(roster_check(&p, &t, &r, now).is_err());
    }
    for (field, value) in [
        ("snapshot_not_before", r.issued_at_unix_seconds + 1),
        ("snapshot_expires_at", r.issued_at_unix_seconds),
    ] {
        let mut changed = original.clone();
        changed[field] = json!(value);
        let p = TrustProfile::decode(&line(&changed)).unwrap();
        assert!(roster_check(&p, &t, &r, r.issued_at_unix_seconds).is_err());
    }
    for (field, value) in [
        ("aud", json!("other")),
        ("run_id", json!("099")),
        ("actor_id", json!("17")),
        ("workflow_sha", json!("2".repeat(40))),
        ("job_workflow_ref", json!("other")),
        ("exp", json!(r.issued_at_unix_seconds + 601)),
    ] {
        let mut c = claims(&r, &policy(&r));
        c[field] = value;
        let h = json!({"alg":"RS256","typ":"JWT","kid":"synthetic-01"});
        assert!(roster_check(
            &p,
            &token(
                &serde_json::to_vec(&h).unwrap(),
                &serde_json::to_vec(&c).unwrap()
            ),
            &r,
            r.issued_at_unix_seconds
        )
        .is_err());
    }
    for (field, value) in [
        ("alg", "HS256"),
        ("jku", "https://foreign.invalid"),
        ("jwk", "embedded"),
        ("kid", "unknown"),
    ] {
        let mut h = json!({"alg":"RS256","typ":"JWT","kid":"synthetic-01"});
        h[field] = json!(value);
        assert!(roster_check(
            &p,
            &token(
                &serde_json::to_vec(&h).unwrap(),
                &serde_json::to_vec(&claims(&r, &policy(&r))).unwrap()
            ),
            &r,
            r.issued_at_unix_seconds
        )
        .is_err());
    }
}

fn roster_binding(case: Option<u8>) {
    let model = Model::new();
    let r = request(&model);
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
    fs::write(&path, line(&roster(&r, MAX_ROSTER_KEYS, 4))).unwrap();
    oidc.trust.source = Source::open(oidc.root.path(), unsafe { libc::geteuid() }).unwrap();
    {
        let mut original = controller.authenticate_controller(&mut worker).unwrap();
        let mut build = original
            .bind_reviewed_build(BUILDER_POLICY, RECEIPT)
            .unwrap();
        let mut caller = roster_token(&r, "synthetic-04");
        if case == Some(0) {
            caller = roster_token(&r, "unknown");
            assert!(build.bind_oidc(&mut oidc.trust, &caller).is_err());
            assert!(build
                .bind_oidc(&mut oidc.trust, &roster_token(&r, "synthetic-04"))
                .is_err());
        } else {
            let mut guard = build.bind_oidc(&mut oidc.trust, &caller).unwrap();
            caller.fill(0);
            guard.revalidate().unwrap();
            if let Some(case) = case {
                match case {
                    1 => fs::write(&path, line(&roster(&r, 4, 1))).unwrap(),
                    2 => {
                        fs::rename(&path, path.with_extension("old")).unwrap();
                        fs::write(&path, line(&roster(&r, MAX_ROSTER_KEYS, 4))).unwrap();
                    }
                    3 => fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap(),
                    4 => guard.original.original.worker.latch_refusal(),
                    _ => fs::write(&path, line(&policy(&r))).unwrap(),
                }
                assert!(guard.revalidate().is_err());
                fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
                fs::write(&path, line(&roster(&r, MAX_ROSTER_KEYS, 4))).unwrap();
                assert!(matches!(guard.revalidate(), Err(OidcError::Refused)));
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
fn oidc_roster_v2_original_binding_constructor_and_rotation_cross_latch() {
    roster_binding(None);
    for case in 0..6 {
        roster_binding(Some(case));
    }
}

#[test]
fn oidc_roster_v2_source_metadata_bounds_and_nofollow() {
    let r = request(&Model::new());
    for case in 0..6 {
        let m = TrustModel::new(&r);
        let path = m.path();
        fs::write(&path, line(&roster(&r, MAX_ROSTER_KEYS, 0))).unwrap();
        match case {
            0 => fs::set_permissions(&path, fs::Permissions::from_mode(0o666)).unwrap(),
            1 => {
                fs::rename(&path, path.with_extension("old")).unwrap();
                symlink(path.with_extension("old"), &path).unwrap();
            }
            2 => fs::hard_link(&path, path.with_extension("link")).unwrap(),
            3 => fs::write(&path, vec![0; MAX_ROSTER_BYTES + 1]).unwrap(),
            4 => fs::write(&path, []).unwrap(),
            _ => fs::set_permissions(m.root.path().join("etc"), fs::Permissions::from_mode(0o777))
                .unwrap(),
        }
        assert!(Source::open(m.root.path(), unsafe { libc::geteuid() }).is_err());
    }
}

#[test]
fn oidc_roster_v2_snapshot_read_budget_is_original_and_other_sources_stay_small() {
    use crate::diagnostic_worker_policy::trusted_input::{bounded_read_limit, Snapshot};
    use std::fs::File;
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("public-synthetic-only");
    let uid = unsafe { libc::geteuid() };
    fs::write(&path, vec![1; MAX_ROSTER_BYTES]).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(Snapshot::new(File::open(&path).unwrap(), uid).is_err());
    Snapshot::new_with_limit(File::open(&path).unwrap(), uid, MAX_ROSTER_BYTES).unwrap();
    fs::write(&path, vec![1; MAX_ROSTER_BYTES + 1]).unwrap();
    assert!(Snapshot::new_with_limit(File::open(&path).unwrap(), uid, MAX_ROSTER_BYTES).is_err());
    fs::write(&path, b"synthetic").unwrap();
    let original = Snapshot::new(File::open(&path).unwrap(), uid).unwrap();
    let changed_limit =
        Snapshot::new_with_limit(File::open(&path).unwrap(), uid, MAX_ROSTER_BYTES).unwrap();
    assert!(original.same(&changed_limit).is_err());
    for limit in [0, 2049, usize::MAX] {
        assert!(Snapshot::new_with_limit(File::open(&path).unwrap(), uid, limit).is_err());
    }
    for length in [1, MAX_ROSTER_BYTES, MAX_ROSTER_BYTES + 1] {
        let read = bounded_read_limit(
            |buffer, offset| {
                if offset as usize == length {
                    return Ok(0);
                }
                buffer[0] = 1;
                Ok(1)
            },
            MAX_ROSTER_BYTES,
        );
        assert_eq!(read.is_ok(), length <= MAX_ROSTER_BYTES);
    }
    assert!(bounded_read_limit(|_, _| Ok(usize::MAX), MAX_ROSTER_BYTES).is_err());
    assert!(bounded_read_limit(
        |_, _| Err(std::io::ErrorKind::Interrupted.into()),
        MAX_ROSTER_BYTES
    )
    .is_err());
}

#[test]
fn oidc_roster_v2_seeded_membership_mutations_and_independent_bindings() {
    let r = request(&Model::new());
    let mut seed = 0x07402a11_u64;
    for _ in 0..64 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let selected = seed as usize % MAX_ROSTER_KEYS;
        let p = TrustProfile::decode(&line(&roster(&r, MAX_ROSTER_KEYS, selected))).unwrap();
        let mut t = roster_token(&r, &format!("synthetic-{selected:02}"));
        roster_check(&p, &t, &r, r.issued_at_unix_seconds).unwrap();
        let text = std::str::from_utf8(&t).unwrap();
        let parts: Vec<_> = text.split('.').collect();
        let mut signature = URL_SAFE_NO_PAD.decode(parts[2]).unwrap();
        let index = seed as usize % signature.len();
        signature[index] ^= 1;
        t = format!(
            "{}.{}.{}",
            parts[0],
            parts[1],
            URL_SAFE_NO_PAD.encode(signature)
        )
        .into_bytes();
        assert!(matches!(
            roster_check(&p, &t, &r, r.issued_at_unix_seconds),
            Err(OidcError::Signature)
        ));
    }
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| scope.spawn(|| roster_binding(None)))
            .collect();
        for h in handles {
            h.join().unwrap();
        }
    });
    println!("seed=0x07402a11 membership_mutations=64 independent_bindings=4 actual_issuer_enrolled=false");
}
