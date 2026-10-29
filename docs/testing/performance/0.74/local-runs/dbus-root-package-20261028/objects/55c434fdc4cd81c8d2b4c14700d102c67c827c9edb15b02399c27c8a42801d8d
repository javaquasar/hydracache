use super::*;
use crate::diagnostic_two_phase_launch::held_placement as placement;
use crate::diagnostic_two_phase_launch::root_enrollment::{
    Kind, PacketClaims, RootBindingClaims, PACKET_BYTES,
};

fn root(launch: &LaunchBody) -> RootBindingClaims {
    use sha2::{Digest, Sha256};
    RootBindingClaims {
        attempt_sha256: Sha256::digest(
            serde_json::to_vec(&(
                "hc074-original-root-attempt-v1",
                &launch.consent_claims.starting_state.identity,
            ))
            .unwrap(),
        )
        .into(),
        original_context_sha256: [74; 32],
        root_device: 23,
        root_inode: 74,
        deadline_boottime_ns: 12_000_000_001,
    }
}
fn body() -> placement::PlacementBody {
    let launch = fixture();
    let root = root(&launch);
    placement::PlacementBody {
        schema_version: placement::BODY_SCHEMA.into(),
        action: "project-held-cell-one".into(),
        root_enrollment_hex: hex(&PacketClaims {
            kind: Kind::Enroll,
            binding: root,
        }
        .encode()
        .unwrap()),
        requirements_sha256: sha256_hex(
            &placement::placement_requirements(&launch, &root).unwrap(),
        ),
        launch_claims: launch,
    }
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn envelope(body: placement::PlacementBody) -> Vec<u8> {
    let signature = key().sign(&placement::placement_signing_message(&body).unwrap());
    line(&placement::SignedPlacement {
        schema_version: placement::ENVELOPE_SCHEMA.into(),
        placement: body,
        signature_hex: hex(&signature.to_bytes()),
    })
    .unwrap()
}
fn inspect(raw: &[u8]) -> Result<placement::PlacementBody, placement::PlacementError> {
    placement::inspect_placement_signature(raw, &key().verifying_key())
}
#[test]
fn held_placement_roundtrip_exact_root_and_data_only_scope() {
    let body = body();
    let raw = envelope(body.clone());
    let result = inspect(&raw).unwrap();
    assert_eq!(line(&result).unwrap(), line(&body).unwrap());
    let doc: Value = serde_json::from_slice(
        &placement::placement_requirements(&body.launch_claims, &root(&body.launch_claims))
            .unwrap(),
    )
    .unwrap();
    let attempt = hex(&root(&body.launch_claims).attempt_sha256);
    let slice = format!("system-hc074{attempt}.slice");
    let root_path = format!("/system.slice/{slice}");
    let unit =
        crate::diagnostic_lease::cell_intent(&body.launch_claims.consent_claims.starting_state)
            .unwrap()
            .unit_name;
    assert_eq!(doc["placement"]["slice_unit_name"], slice);
    assert_eq!(doc["placement"]["slice_parent_unit_name"], "system.slice");
    assert_eq!(doc["placement"]["expected_root_control_group"], root_path);
    assert_eq!(doc["placement"]["held_unit_name"], unit);
    assert_eq!(
        doc["placement"]["expected_held_control_group"],
        format!("{root_path}/{unit}")
    );
    assert_eq!(doc["root_enrollment_hex"], body.root_enrollment_hex);
    assert_eq!(
        doc["absolute_deadline_boottime_ns"],
        root(&body.launch_claims).deadline_boottime_ns
    );
    for field in [
        "root_is_enrolled",
        "held_unit_is_projected",
        "original_send_allowed",
        "barrier_release_allowed",
        "completion_allowed",
        "fence_release_allowed",
    ] {
        assert_eq!(doc["admission"][field], false, "{field}");
    }
    assert_eq!(doc["renewal"], false);
}
#[test]
fn held_placement_seeded_names_and_identity_binding_are_deterministic() {
    let mut seed = 0x074e_5a1c_u64;
    let mut names = std::collections::BTreeSet::new();
    for _ in 0..128 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut launch = fixture();
        launch.consent_claims.starting_state.identity.lease_id = format!("{seed:064x}");
        launch.requirements_sha256 =
            sha256_hex(&launch_requirements(&launch.consent_claims, &launch.artifacts).unwrap());
        let root = root(&launch);
        let bytes = placement::placement_requirements(&launch, &root).unwrap();
        assert_eq!(
            bytes,
            placement::placement_requirements(&launch, &root).unwrap()
        );
        let doc: Value = serde_json::from_slice(&bytes).unwrap();
        let name = doc["placement"]["slice_unit_name"].as_str().unwrap();
        assert!(name.starts_with("system-hc074") && name.ends_with(".slice"));
        assert!(name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"-.".contains(&b)));
        assert_eq!(name.bytes().filter(|b| *b == b'-').count(), 1);
        assert!(names.insert(name.to_owned()));
        let mut foreign = root;
        foreign.attempt_sha256[0] ^= 1;
        assert!(placement::placement_requirements(&launch, &foreign).is_err());
    }
    println!("HC074-HELD-PLACEMENT seed=0x074e5a1c identities=128 runtime_admission=false");
}
#[test]
fn held_placement_all_packet_kinds_zero_fields_and_foreign_attempt_refuse() {
    let launch = fixture();
    let original = root(&launch);
    for case in 0..7 {
        let mut changed = original;
        match case {
            0 => changed.attempt_sha256 = [0; 32],
            1 => changed.original_context_sha256 = [0; 32],
            2 => changed.root_device = 0,
            3 => changed.root_inode = 0,
            4 => changed.deadline_boottime_ns = 0,
            5 => changed.deadline_boottime_ns = i64::MAX as u64 + 1,
            _ => changed.attempt_sha256[0] ^= 1,
        }
        assert!(placement::placement_requirements(&launch, &changed).is_err());
    }
    for kind in [Kind::Armed, Kind::Refused, Kind::Revoking] {
        let mut body = body();
        body.root_enrollment_hex = hex(&PacketClaims {
            kind,
            binding: original,
        }
        .encode()
        .unwrap());
        assert!(placement::placement_signing_message(&body).is_err());
    }
}
#[test]
fn held_placement_changed_signed_fields_digest_and_key_refuse() {
    let raw = envelope(body());
    for pointer in [
        "/schema_version",
        "/signature_hex",
        "/placement/schema_version",
        "/placement/action",
        "/placement/requirements_sha256",
        "/placement/root_enrollment_hex",
        "/placement/launch_claims/requirements_sha256",
        "/placement/launch_claims/artifacts/launcher_sha256",
        "/placement/launch_claims/consent_claims/collector_unit_sha256",
        "/placement/launch_claims/consent_claims/starting_state/identity/lease_id",
    ] {
        let mut doc: Value = serde_json::from_slice(&raw).unwrap();
        let value = doc.pointer_mut(pointer).unwrap();
        let mut text = value.as_str().unwrap().as_bytes().to_vec();
        text[0] = if text[0] == b'a' { b'b' } else { b'a' };
        *value = Value::String(String::from_utf8(text).unwrap());
        assert!(inspect(&line(&doc).unwrap()).is_err(), "{pointer}");
    }
    assert!(placement::inspect_placement_signature(
        &raw,
        &SigningKey::from_bytes(&[75; 32]).verifying_key()
    )
    .is_err());
    let mut changed = body();
    let mut bytes = PacketClaims {
        kind: Kind::Enroll,
        binding: root(&changed.launch_claims),
    }
    .encode()
    .unwrap();
    for index in 0..PACKET_BYTES {
        bytes[index] ^= 1;
        changed.root_enrollment_hex = hex(&bytes);
        assert!(
            placement::placement_signing_message(&changed).is_err(),
            "byte {index}"
        );
        bytes[index] ^= 1;
    }
}
#[test]
fn held_placement_unknown_duplicate_noncanonical_budget_and_hex_refuse() {
    let raw = envelope(body());
    for pointer in ["", "/placement", "/placement/launch_claims"] {
        let mut doc: Value = serde_json::from_slice(&raw).unwrap();
        doc.pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("future".into(), json!(true));
        assert!(inspect(&line(&doc).unwrap()).is_err());
    }
    let duplicate = format!(
        "{{\"schema_version\":\"{}\",{}",
        placement::ENVELOPE_SCHEMA,
        &String::from_utf8(raw.clone()).unwrap()[1..]
    );
    for bytes in [
        Vec::new(),
        vec![b'x'; placement::MAX_ENVELOPE_BYTES + 1],
        raw[..raw.len() - 1].to_vec(),
        [raw.clone(), b"\n".to_vec()].concat(),
        duplicate.into_bytes(),
        serde_json::to_vec_pretty(&serde_json::from_slice::<Value>(&raw).unwrap()).unwrap(),
    ] {
        assert!(inspect(&bytes).is_err());
    }
    for text in [
        "".into(),
        "0".repeat(207),
        "0".repeat(209),
        "A".repeat(208),
        "g".repeat(208),
        "é".repeat(104),
    ] {
        let mut changed = body();
        changed.root_enrollment_hex = text;
        assert!(placement::placement_signing_message(&changed).is_err());
    }
}
#[test]
fn held_placement_versions_and_signature_domains_do_not_upgrade_old_claims() {
    let legacy = line(&signed(fixture())).unwrap();
    assert!(inspect(&legacy).is_err());
    let raw = envelope(body());
    assert!(super::inspect(&raw).is_err());
    let body = body();
    let mut wrong = placement::PLACEMENT_DOMAIN.to_vec();
    wrong.push(0);
    wrong.extend(line(&body).unwrap());
    assert_eq!(wrong, placement::placement_signing_message(&body).unwrap());
    for domain in [
        SIGNATURE_DOMAIN,
        crate::diagnostic_start_consent::SIGNATURE_DOMAIN,
    ] {
        let mut message = domain.to_vec();
        message.push(0);
        message.extend(line(&body).unwrap());
        let signature = key().sign(&message);
        let envelope = line(&placement::SignedPlacement {
            schema_version: placement::ENVELOPE_SCHEMA.into(),
            placement: body.clone(),
            signature_hex: hex(&signature.to_bytes()),
        })
        .unwrap();
        assert!(inspect(&envelope).is_err());
    }
}
