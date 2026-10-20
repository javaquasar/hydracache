//! Static invented claims only; no original owner or production trust is enrolled.
use super::*;
use held::role_binding as binding;
use hydracache_long_run_supervisor_074::{
    diagnostic_lease, diagnostic_start_consent,
    diagnostic_two_phase_launch::{self as launch, held_placement as placement, root_enrollment},
};
use sha2::{Digest, Sha256};

fn consent() -> diagnostic_start_consent::StartConsentBody {
    serde_json::from_value(json!({
        "schema_version":diagnostic_start_consent::BODY_SCHEMA,"action":"start-cell-one",
        "consent_request_id":"1".repeat(64),"reservation_request_id":"2".repeat(64),
        "controller_attempt_sha256":"3".repeat(64),"collector_reserved_sha256":"4".repeat(64),
        "reserved_state_sha256":"5".repeat(64),"challenge_nonce_sha256":"6".repeat(64),
        "authenticated_response_sha256":"7".repeat(64),"worker_policy_sha256":"8".repeat(64),
        "worker_policy_epoch":7,"collector_unit_sha256":"9".repeat(64),
        "controller":{"repository_id":worker::REPOSITORY_ID,"run_id":2,"run_attempt":3,"actor_id":4},
        "source_commit":diagnostic_lease::SOURCE_COMMIT,"preset":hydracache_long_run_supervisor_074::diagnostic_ipc::PRESET,
        "starting_state":{
            "identity":{"lease_id":"a".repeat(64),"boot_id":"00000000-0000-4000-8000-000000000074","binary_sha256":"b".repeat(64),"build_provenance_sha256":"c".repeat(64)},
            "revision":2,"stage":"starting","completed_cells":0,
            "reserved_monotonic_ns":1,"controller_monotonic_ns":3,"last_observed_monotonic_ns":4,
            "cell_started_monotonic_ns":4,"cgroup_inode":null,"reason":null,
            "cleanup_confirmed":false,"promotable":false,"admission_allowed":false},
        "issued_at_unix_seconds":100,"expires_at_unix_seconds":160
    })).unwrap()
}
fn sign_placement(f: &Fixture, p: &mut placement::SignedPlacement) {
    p.signature_hex = hex(&f
        .controller
        .sign(&placement::placement_signing_message(&p.placement).unwrap())
        .to_bytes());
}
fn body(f: &Fixture) -> binding::Body {
    let consent = consent();
    let artifacts = LaunchArtifacts {
        launcher_sha256: hash(&f.launcher),
        watchdog_sha256: hash(&f.host),
        collector_sha256: hash(&f.host),
    };
    let launch = launch::LaunchBody {
        schema_version: launch::BODY_SCHEMA.into(),
        action: "prepare-held-cell-one".into(),
        requirements_sha256: hash(&launch::launch_requirements(&consent, &artifacts).unwrap()),
        consent_claims: consent,
        artifacts,
    };
    let root = root_enrollment::RootBindingClaims {
        attempt_sha256: Sha256::digest(
            serde_json::to_vec(&(
                "hc074-original-root-attempt-v1",
                &launch.consent_claims.starting_state.identity,
            ))
            .unwrap(),
        )
        .into(),
        original_context_sha256: [75; 32],
        root_device: 3,
        root_inode: 6,
        deadline_boottime_ns: 10_000_000_002,
    };
    let p = placement::PlacementBody {
        schema_version: placement::BODY_SCHEMA.into(),
        action: "project-held-cell-one".into(),
        root_enrollment_hex: hex(&root_enrollment::PacketClaims {
            kind: root_enrollment::Kind::Enroll,
            binding: root,
        }
        .encode()
        .unwrap()),
        requirements_sha256: hash(&placement::placement_requirements(&launch, &root).unwrap()),
        launch_claims: launch,
    };
    let mut p = placement::SignedPlacement {
        schema_version: placement::ENVELOPE_SCHEMA.into(),
        placement: p,
        signature_hex: String::new(),
    };
    sign_placement(f, &mut p);
    binding::Body {
        schema_version: binding::BODY_SCHEMA.into(),
        action: "bind-held-build-to-placement".into(),
        placement: p,
        held_policy_sha256: hash(&held::document_bytes(&f.policy).unwrap()),
        held_receipt_sha256: hash(&held::document_bytes(&f.receipt()).unwrap()),
        authority_epoch: 7,
    }
}
fn signed_unchecked(f: &Fixture, b: binding::Body) -> Vec<u8> {
    let mut msg = binding::SIGNATURE_DOMAIN.to_vec();
    msg.push(0);
    msg.extend(held::document_bytes(&b).unwrap());
    held::document_bytes(&binding::SignedBinding {
        schema_version: binding::ENVELOPE_SCHEMA.into(),
        signature_hex: hex(&f.controller.sign(&msg).to_bytes()),
        binding: b,
    })
    .unwrap()
}
#[test]
fn signed_binding_checks_same_policy_receipt_epoch_roles_and_exact_placement_bytes() {
    let f = Fixture::new();
    let b = body(&f);
    let receipt = held::document_bytes(&f.receipt()).unwrap();
    let msg = binding::signing_message(&b, &f.checked(), &receipt).unwrap();
    let raw = signed_unchecked(&f, b.clone());
    let checked = binding::inspect_signature(&raw, &f.checked(), &receipt).unwrap();
    checked
        .matches_placement_bytes(&held::document_bytes(&b.placement).unwrap())
        .unwrap();
    checked.verify_contents(&f.contents()).unwrap();
    assert!(msg.starts_with(binding::SIGNATURE_DOMAIN));
}

fn receipt(f: &Fixture) -> Vec<u8> {
    held::document_bytes(&f.receipt()).unwrap()
}
fn accepts(f: &Fixture, b: binding::Body) -> bool {
    binding::inspect_signature(&signed_unchecked(f, b), &f.checked(), &receipt(f)).is_ok()
}
fn refresh(f: &Fixture, p: &mut placement::SignedPlacement) {
    let raw: Vec<_> = p
        .placement
        .root_enrollment_hex
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect();
    let root = root_enrollment::inspect_packet(&raw).unwrap().binding;
    p.placement.launch_claims.requirements_sha256 = hash(
        &launch::launch_requirements(
            &p.placement.launch_claims.consent_claims,
            &p.placement.launch_claims.artifacts,
        )
        .unwrap(),
    );
    p.placement.requirements_sha256 =
        hash(&placement::placement_requirements(&p.placement.launch_claims, &root).unwrap());
    sign_placement(f, p);
}
#[test]
fn every_unsigned_outer_and_nested_binding_mutation_refuses() {
    let f = Fixture::new();
    let raw = signed_unchecked(&f, body(&f));
    for case in 0..9 {
        let mut doc: Value = serde_json::from_slice(&raw).unwrap();
        match case {
            0 => doc["schema_version"] = json!("future"),
            1 => doc["binding"]["action"] = json!("start"),
            2 => doc["binding"]["held_policy_sha256"] = json!("d".repeat(64)),
            3 => doc["binding"]["held_receipt_sha256"] = json!("e".repeat(64)),
            4 => doc["binding"]["authority_epoch"] = json!(8),
            5 => doc["binding"]["placement"]["signature_hex"] = json!("00".repeat(64)),
            6 => {
                doc["binding"]["placement"]["placement"]["root_enrollment_hex"] =
                    json!("00".repeat(root_enrollment::PACKET_BYTES))
            }
            7 => {
                doc["binding"]["placement"]["placement"]["launch_claims"]["artifacts"]
                    ["launcher_sha256"] = json!("f".repeat(64))
            }
            _ => doc["signature_hex"] = json!("00".repeat(64)),
        }
        assert!(
            binding::inspect_signature(
                &held::document_bytes(&doc).unwrap(),
                &f.checked(),
                &receipt(&f)
            )
            .is_err(),
            "{case}"
        );
    }
}
#[test]
fn wrong_controller_or_builder_key_at_either_signature_refuses() {
    let f = Fixture::new();
    let b = body(&f);
    let message = binding::signing_message(&b, &f.checked(), &receipt(&f)).unwrap();
    for key in [&f.builder, &f.worker, &SigningKey::from_bytes(&[21; 32])] {
        let doc = binding::SignedBinding {
            schema_version: binding::ENVELOPE_SCHEMA.into(),
            binding: b.clone(),
            signature_hex: hex(&key.sign(&message).to_bytes()),
        };
        assert!(binding::inspect_signature(
            &held::document_bytes(&doc).unwrap(),
            &f.checked(),
            &receipt(&f)
        )
        .is_err());
        let mut bad = b.clone();
        bad.placement.signature_hex = hex(&key
            .sign(&placement::placement_signing_message(&bad.placement.placement).unwrap())
            .to_bytes());
        assert!(!accepts(&f, bad));
    }
}
#[test]
fn valid_but_different_receipt_or_independently_checked_policy_refuses() {
    let f = Fixture::new();
    let b = body(&f);
    let raw = signed_unchecked(&f, b.clone());
    let mut other = Fixture::new();
    other.log = log(&[
        json!({"reason":"build-script-executed"}),
        events()[0].clone(),
        events()[1].clone(),
        events()[2].clone(),
    ]);
    let other_receipt = receipt(&other);
    assert_ne!(other_receipt, receipt(&f));
    assert!(held::verify_receipt(&other_receipt, &f.checked()).is_ok());
    assert!(binding::inspect_signature(&raw, &f.checked(), &other_receipt).is_err());
    other.policy.source.tree = "c".repeat(40);
    assert!(binding::inspect_signature(&raw, &other.checked(), &receipt(&other)).is_err());
    let mut bad = b;
    bad.held_policy_sha256 = hash(&held::document_bytes(&other.policy).unwrap());
    assert!(
        binding::inspect_signature(&signed_unchecked(&f, bad), &other.checked(), &receipt(&f))
            .is_err()
    );
}
#[test]
fn resigned_epoch_repository_and_digest_substitution_refuses() {
    let f = Fixture::new();
    for case in 0..7 {
        let mut b = body(&f);
        match case {
            0 => b.authority_epoch = 0,
            1 => b.authority_epoch = 8,
            2 => {
                b.placement
                    .placement
                    .launch_claims
                    .consent_claims
                    .worker_policy_epoch = 8;
                refresh(&f, &mut b.placement);
            }
            3 => {
                b.placement
                    .placement
                    .launch_claims
                    .consent_claims
                    .controller
                    .repository_id = 1;
                refresh(&f, &mut b.placement);
            }
            4 => b.held_policy_sha256 = "d".repeat(64),
            5 => b.held_receipt_sha256 = "e".repeat(64),
            _ => b.held_policy_sha256 = "A".repeat(64),
        }
        assert!(binding::signing_message(&b, &f.checked(), &receipt(&f)).is_err());
        assert!(!accepts(&f, b), "{case}");
    }
}
#[test]
fn resigned_each_role_mismatch_refuses_even_with_valid_old_placement() {
    let f = Fixture::new();
    for case in 0..3 {
        let mut b = body(&f);
        let artifacts = &mut b.placement.placement.launch_claims.artifacts;
        match case {
            0 => artifacts.launcher_sha256 = "d".repeat(64),
            1 => artifacts.watchdog_sha256 = "e".repeat(64),
            _ => artifacts.collector_sha256 = "f".repeat(64),
        }
        refresh(&f, &mut b.placement);
        assert!(placement::inspect_placement_signature(
            &held::document_bytes(&b.placement).unwrap(),
            &f.controller.verifying_key()
        )
        .is_ok());
        assert!(!accepts(&f, b));
    }
}
#[test]
fn another_valid_attempt_cannot_match_the_original_placement_bytes() {
    let f = Fixture::new();
    let original = body(&f);
    let mut next = original.clone();
    next.placement
        .placement
        .launch_claims
        .consent_claims
        .starting_state
        .identity
        .lease_id = "d".repeat(64);
    let mut packet = root_enrollment::inspect_packet(
        &next
            .placement
            .placement
            .root_enrollment_hex
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect::<Vec<_>>(),
    )
    .unwrap();
    packet.binding.attempt_sha256 = Sha256::digest(
        serde_json::to_vec(&(
            "hc074-original-root-attempt-v1",
            &next
                .placement
                .placement
                .launch_claims
                .consent_claims
                .starting_state
                .identity,
        ))
        .unwrap(),
    )
    .into();
    next.placement.placement.root_enrollment_hex = hex(&packet.encode().unwrap());
    next.placement
        .placement
        .launch_claims
        .consent_claims
        .consent_request_id = "d".repeat(64);
    refresh(&f, &mut next.placement);
    let checked =
        binding::inspect_signature(&signed_unchecked(&f, next), &f.checked(), &receipt(&f))
            .unwrap();
    assert!(checked
        .matches_placement_bytes(&held::document_bytes(&original.placement).unwrap())
        .is_err());
}
#[test]
fn canonical_unknown_duplicate_nested_and_signature_encoding_are_strict() {
    let f = Fixture::new();
    let raw = signed_unchecked(&f, body(&f));
    let mut extra = raw.clone();
    extra.push(b'\n');
    assert!(binding::inspect_signature(&extra, &f.checked(), &receipt(&f)).is_err());
    for case in 0..6 {
        let mut doc: Value = serde_json::from_slice(&raw).unwrap();
        match case {
            0 => doc["unknown"] = json!(1),
            1 => doc["binding"]["unknown"] = json!(1),
            2 => doc["binding"]["placement"]["unknown"] = json!(1),
            3 => doc["binding"]["placement"]["placement"]["unknown"] = json!(1),
            4 => {
                doc["signature_hex"] = json!(doc["signature_hex"].as_str().unwrap().to_uppercase())
            }
            _ => doc["signature_hex"] = json!("00"),
        }
        assert!(binding::inspect_signature(
            &held::document_bytes(&doc).unwrap(),
            &f.checked(),
            &receipt(&f)
        )
        .is_err());
    }
    let text = String::from_utf8(raw).unwrap();
    for field in ["\"authority_epoch\":7", "\"worker_policy_epoch\":7"] {
        let bad = text.replacen(field, &format!("{field},{field}"), 1);
        assert_ne!(bad, text);
        assert!(binding::inspect_signature(bad.as_bytes(), &f.checked(), &receipt(&f)).is_err());
    }
}
#[test]
fn all_document_receipt_and_signing_budgets_refuse() {
    let f = Fixture::new();
    let b = body(&f);
    let raw = signed_unchecked(&f, b.clone());
    for bytes in [
        Vec::new(),
        vec![b' '; binding::MAX_ENVELOPE_BYTES + 1],
        b"{}\n".to_vec(),
    ] {
        assert!(binding::inspect_signature(&bytes, &f.checked(), &receipt(&f)).is_err());
    }
    for bytes in [Vec::new(), vec![b' '; 65537]] {
        assert!(binding::inspect_signature(&raw, &f.checked(), &bytes).is_err());
    }
    let mut large = b;
    large.placement.signature_hex = "0".repeat(binding::MAX_BODY_BYTES + 1);
    assert!(binding::signing_message(&large, &f.checked(), &receipt(&f)).is_err());
}
#[test]
fn seeded_unsigned_bit_corruption_never_reuses_inspection() {
    let f = Fixture::new();
    let raw = signed_unchecked(&f, body(&f));
    let policy = f.checked();
    let receipt = receipt(&f);
    let mut seed = 0x0074_b14du64;
    eprintln!("held_role_binding seed=0x{seed:08x} corruptions=64");
    for index in 0..64 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mut bad = raw.clone();
        let at = (seed as usize) % bad.len();
        bad[at] ^= 1 << ((seed >> 32) % 8);
        assert!(
            binding::inspect_signature(&bad, &policy, &receipt).is_err(),
            "{index}"
        );
    }
}
#[test]
fn checked_output_rejects_changed_contents_and_nonexact_placement_bytes() {
    let f = Fixture::new();
    let b = body(&f);
    let checked =
        binding::inspect_signature(&signed_unchecked(&f, b.clone()), &f.checked(), &receipt(&f))
            .unwrap();
    let mut bad = held::document_bytes(&b.placement).unwrap();
    bad.pop();
    assert!(checked.matches_placement_bytes(&bad).is_err());
    let mut changed = Fixture::new();
    changed.host[127] ^= 1;
    assert!(checked.verify_contents(&changed.contents()).is_err());
}
#[test]
fn new_domain_and_envelope_do_not_upgrade_any_old_signature() {
    let f = Fixture::new();
    let b = body(&f);
    let raw = signed_unchecked(&f, b.clone());
    assert_ne!(binding::SIGNATURE_DOMAIN, held::SIGNATURE_DOMAIN);
    assert_ne!(binding::SIGNATURE_DOMAIN, placement::PLACEMENT_DOMAIN);
    assert_ne!(binding::SIGNATURE_DOMAIN, launch::SIGNATURE_DOMAIN);
    assert!(placement::inspect_placement_signature(&raw, &f.controller.verifying_key()).is_err());
    assert!(held::verify_receipt(&raw, &f.checked()).is_err());
    assert!(binding::inspect_signature(
        &held::document_bytes(&b.placement).unwrap(),
        &f.checked(),
        &receipt(&f)
    )
    .is_err());
    let mut doc: binding::SignedBinding = serde_json::from_slice(&raw).unwrap();
    doc.signature_hex = b.placement.signature_hex;
    assert!(binding::inspect_signature(
        &held::document_bytes(&doc).unwrap(),
        &f.checked(),
        &receipt(&f)
    )
    .is_err());
}
