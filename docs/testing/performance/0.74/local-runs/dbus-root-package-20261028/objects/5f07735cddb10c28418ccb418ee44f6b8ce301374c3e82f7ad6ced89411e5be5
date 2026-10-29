use super::placement as api;
use super::*;
use crate::diagnostic_start_consent::StartConsentBody;
use crate::diagnostic_two_phase_launch::held_placement as wire;
use crate::diagnostic_two_phase_launch::root_enrollment::{Kind, PacketClaims};
use crate::diagnostic_two_phase_launch::{LaunchArtifacts, LaunchBody};
use ed25519_dalek::{Signer, SigningKey};
use std::cell::RefCell;
use std::rc::Rc;

fn consent() -> StartConsentBody {
    serde_json::from_value(serde_json::json!({
        "schema_version":crate::diagnostic_start_consent::BODY_SCHEMA,"action":"start-cell-one",
        "consent_request_id":"1".repeat(64),"reservation_request_id":"2".repeat(64),
        "controller_attempt_sha256":"3".repeat(64),"collector_reserved_sha256":"4".repeat(64),
        "reserved_state_sha256":"5".repeat(64),"challenge_nonce_sha256":"6".repeat(64),
        "authenticated_response_sha256":"7".repeat(64),"worker_policy_sha256":"8".repeat(64),
        "worker_policy_epoch":1,"collector_unit_sha256":"9".repeat(64),
        "controller":{"repository_id":1,"run_id":2,"run_attempt":3,"actor_id":4},
        "source_commit":crate::diagnostic_lease::SOURCE_COMMIT,"preset":crate::diagnostic_ipc::PRESET,
        "starting_state":{
            "identity":{"lease_id":"a".repeat(64),"boot_id":"00000000-0000-4000-8000-000000000074","binary_sha256":"b".repeat(64),"build_provenance_sha256":"c".repeat(64)},
            "revision":2,"stage":"starting","completed_cells":0,
            "reserved_monotonic_ns":1,"controller_monotonic_ns":3,"last_observed_monotonic_ns":4,
            "cell_started_monotonic_ns":4,"cgroup_inode":null,"reason":null,
            "cleanup_confirmed":false,"promotable":false,"admission_allowed":false},
        "issued_at_unix_seconds":100,"expires_at_unix_seconds":160
    })).unwrap()
}
fn key() -> SigningKey {
    SigningKey::from_bytes(&[74; 32])
}
fn binding(consent: &StartConsentBody) -> Binding {
    Binding {
        source: Source {
            owner: current_owner(),
            state: consent.starting_state.clone(),
            origins: [1, 2, 3, 4],
            bounds: [300000000001, 60000000004, 10000000002, 10000000002],
            descriptors: std::array::from_fn(|i| i as i32 + 3),
            guardians: [30, 31],
            names: (":1.74".into(), ":1.75".into()),
            serial: NonZeroU32::new(7).unwrap(),
        },
        addressed: [70, 71],
    }
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn envelope(consent: &StartConsentBody, root: RootBindingClaims, key: &SigningKey) -> Vec<u8> {
    let artifacts = LaunchArtifacts {
        launcher_sha256: "d".repeat(64),
        watchdog_sha256: "e".repeat(64),
        collector_sha256: "f".repeat(64),
    };
    let launch = LaunchBody {
        schema_version: crate::diagnostic_two_phase_launch::BODY_SCHEMA.into(),
        action: "prepare-held-cell-one".into(),
        requirements_sha256: crate::sha256_hex(
            &crate::diagnostic_two_phase_launch::launch_requirements(consent, &artifacts).unwrap(),
        ),
        consent_claims: consent.clone(),
        artifacts,
    };
    let body = wire::PlacementBody {
        schema_version: wire::BODY_SCHEMA.into(),
        action: "project-held-cell-one".into(),
        root_enrollment_hex: hex(&PacketClaims {
            kind: Kind::Enroll,
            binding: root,
        }
        .encode()
        .unwrap()),
        requirements_sha256: crate::sha256_hex(
            &wire::placement_requirements(&launch, &root).unwrap(),
        ),
        launch_claims: launch,
    };
    let signature = key.sign(&wire::placement_signing_message(&body).unwrap());
    let mut bytes = crate::canonical_json(&wire::SignedPlacement {
        schema_version: wire::ENVELOPE_SCHEMA.into(),
        placement: body,
        signature_hex: hex(&signature.to_bytes()),
    })
    .unwrap();
    bytes.push(b'\n');
    bytes
}
struct Owner {
    binding: Binding,
    consent: StartConsentBody,
    refused: bool,
    rounds: u8,
    auths: std::cell::Cell<u8>,
    fail_round: bool,
    fail_auth: u8,
    closing: Option<&'static str>,
    trace: Rc<RefCell<Vec<&'static str>>>,
}
impl RootOwner for Owner {
    fn full_round(&mut self) -> Result<(), PendingError> {
        self.trace.borrow_mut().push("full-frame");
        self.rounds += 1;
        if self.refused || self.fail_round {
            return Err(MessageError::Refused.into());
        }
        if let Some(case) = self.closing {
            self.binding.tamper_for_test(case)
        }
        Ok(())
    }
    fn descriptors(&self) -> Result<[BorrowedFd<'_>; 2], PendingError> {
        panic!("no descriptor extraction")
    }
    fn binding(&self, identity: [u64; 2]) -> Result<RootBindingClaims, PendingError> {
        derive(&self.binding, identity)
    }
    fn refuse(&mut self) {
        self.refused = true
    }
}
impl api::PlacementOwner for Owner {
    fn inspect(&self, raw: &[u8], root: RootBindingClaims) -> Result<(), PendingError> {
        self.trace.borrow_mut().push("signed-constraints");
        self.auths.set(self.auths.get() + 1);
        if self.auths.get() == self.fail_auth {
            return Err(MessageError::Binding.into());
        }
        api::authenticate(raw, &key().verifying_key(), &self.consent, root)
    }
}
struct Controls {
    root: RootBindingClaims,
    refused: bool,
    armed: bool,
    observations: u8,
    fail: u8,
    closing_drift: bool,
    trace: Rc<RefCell<Vec<&'static str>>>,
}
impl RootControls for Controls {
    fn observe(&mut self) -> Result<(), PendingError> {
        self.trace.borrow_mut().push("kernel");
        self.observations += 1;
        if self.refused || self.observations == self.fail {
            return Err(MessageError::Refused.into());
        }
        if self.closing_drift && self.observations == 3 {
            self.root.root_inode += 1
        }
        Ok(())
    }
    fn enroll_once(&mut self) -> Result<(), PendingError> {
        panic!("binding must never reenroll")
    }
    fn refuse(&mut self) {
        self.refused = true
    }
}
impl api::PlacementControls for Controls {
    fn binding_for_placement(&mut self) -> Result<RootBindingClaims, PendingError> {
        self.observe()?;
        if !self.armed {
            return Err(MessageError::Refused.into());
        }
        Ok(self.root)
    }
}
fn fixture() -> (Owner, Controls, Vec<u8>) {
    let consent = consent();
    let binding = binding(&consent);
    let root = derive(&binding, [40, 41]).unwrap();
    let raw = envelope(&consent, root, &key());
    let trace = Rc::default();
    (
        Owner {
            binding,
            consent,
            refused: false,
            rounds: 0,
            auths: 0.into(),
            fail_round: false,
            fail_auth: 0,
            closing: None,
            trace: Rc::clone(&trace),
        },
        Controls {
            root,
            refused: false,
            armed: true,
            observations: 0,
            fail: 0,
            closing_drift: false,
            trace,
        },
        raw,
    )
}
#[test]
fn owned_placement_healthy_uses_one_unchanged_full_round_and_no_reenrollment() {
    let (mut owner, mut controls, raw) = fixture();
    let retained = api::placement_join(&mut owner, &mut controls, &raw, None).unwrap();
    assert_eq!(owner.rounds, 1);
    assert_eq!(owner.auths.get(), 2);
    assert_eq!(
        *owner.trace.borrow(),
        [
            "kernel",
            "kernel",
            "signed-constraints",
            "full-frame",
            "signed-constraints",
            "kernel",
            "kernel"
        ]
    );
    api::placement_join(&mut owner, &mut controls, &raw, Some(&retained)).unwrap();
    assert_eq!(owner.rounds, 2);
    assert_eq!(owner.auths.get(), 4);
}
#[test]
fn owned_placement_all_opening_closing_and_authentication_failures_poison_both() {
    for case in 0..8 {
        let (mut owner, mut controls, raw) = fixture();
        match case {
            0..=3 => controls.fail = case + 1,
            4 => owner.fail_round = true,
            5..=6 => owner.fail_auth = case - 4,
            _ => controls.closing_drift = true,
        }
        assert!(api::placement_join(&mut owner, &mut controls, &raw, None).is_err());
        assert!(owner.refused && controls.refused);
        let rounds = owner.rounds;
        assert!(api::placement_join(&mut owner, &mut controls, &raw, None).is_err());
        assert_eq!(owner.rounds, rounds);
    }
}
#[test]
fn owned_placement_every_original_context_drift_refuses_before_and_after_full_round() {
    for closing in [false, true] {
        for case in [
            "owner", "identity", "state", "origin", "bounds", "deadline", "fence", "alarm",
            "guardian", "names", "serial", "alias",
        ] {
            let (mut owner, mut controls, raw) = fixture();
            if closing {
                owner.closing = Some(case)
            } else {
                owner.binding.tamper_for_test(case)
            }
            assert!(
                api::placement_join(&mut owner, &mut controls, &raw, None).is_err(),
                "{closing} {case}"
            );
            assert!(owner.refused && controls.refused);
        }
    }
}
fn leaves(value: &serde_json::Value, path: String, paths: &mut Vec<String>) {
    if let serde_json::Value::Object(map) = value {
        for (name, value) in map {
            leaves(value, format!("{path}/{name}"), paths)
        }
    } else {
        paths.push(path)
    }
}
#[test]
fn owned_placement_all_36_original_consent_leaves_are_exact() {
    let (owner, controls, raw) = fixture();
    let value = serde_json::to_value(&owner.consent).unwrap();
    let mut paths = Vec::new();
    leaves(&value, String::new(), &mut paths);
    assert_eq!(paths.len(), 36);
    for path in paths {
        let mut changed = value.clone();
        let leaf = changed.pointer_mut(&path).unwrap();
        *leaf = match leaf {
            serde_json::Value::String(s) => serde_json::Value::String(if s == "starting" {
                "reserved".into()
            } else {
                format!("{s}a")
            }),
            serde_json::Value::Number(n) => serde_json::json!(n.as_u64().unwrap() + 1),
            serde_json::Value::Bool(b) => serde_json::json!(!*b),
            serde_json::Value::Null => {
                if path.ends_with("reason") {
                    serde_json::json!("controller_lost")
                } else {
                    serde_json::json!(1)
                }
            }
            _ => unreachable!(),
        };
        let expected: StartConsentBody = serde_json::from_value(changed).unwrap();
        assert!(
            api::authenticate(&raw, &key().verifying_key(), &expected, controls.root).is_err(),
            "{path}"
        );
    }
}
#[test]
fn owned_placement_wrong_key_budget_unarmed_uncertain_and_refused_refuse() {
    for case in 0..6 {
        let (mut owner, mut controls, mut raw) = fixture();
        match case {
            0 => {
                raw = envelope(
                    &owner.consent,
                    controls.root,
                    &SigningKey::from_bytes(&[75; 32]),
                )
            }
            1 => raw = vec![b'x'; wire::MAX_ENVELOPE_BYTES + 1],
            2 => controls.armed = false,
            3 => controls.refused = true,
            4 => owner.refused = true,
            _ => raw.pop().map(|_| ()).unwrap(),
        }
        assert!(api::placement_join(&mut owner, &mut controls, &raw, None).is_err());
        assert!(owner.refused && controls.refused);
    }
}
#[test]
fn owned_placement_all_root_fields_and_immutable_snapshot_are_exact() {
    for case in 0..6 {
        let (mut owner, mut controls, raw) = fixture();
        let retained = api::placement_join(&mut owner, &mut controls, &raw, None).unwrap();
        match case {
            0 => controls.root.attempt_sha256[0] ^= 1,
            1 => controls.root.original_context_sha256[0] ^= 1,
            2 => controls.root.root_device += 1,
            3 => controls.root.root_inode += 1,
            4 => controls.root.deadline_boottime_ns += 1,
            _ => {
                owner.consent.worker_policy_epoch += 1;
            }
        }
        assert!(api::placement_join(&mut owner, &mut controls, &raw, Some(&retained)).is_err());
        assert!(owner.refused && controls.refused);
    }
}
#[test]
fn owned_placement_seeded_constraints_cannot_swap_signed_bytes_after_binding() {
    let mut seed = 0x074e_6a1d_u64;
    for _ in 0..128 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let (mut owner, mut controls, raw) = fixture();
        let retained = api::placement_join(&mut owner, &mut controls, &raw, None).unwrap();
        let mut doc: wire::SignedPlacement = serde_json::from_slice(&raw).unwrap();
        doc.placement.launch_claims.artifacts.launcher_sha256 = format!("{seed:064x}");
        let launch = &mut doc.placement.launch_claims;
        launch.requirements_sha256 = crate::sha256_hex(
            &crate::diagnostic_two_phase_launch::launch_requirements(
                &launch.consent_claims,
                &launch.artifacts,
            )
            .unwrap(),
        );
        doc.placement.requirements_sha256 =
            crate::sha256_hex(&wire::placement_requirements(launch, &controls.root).unwrap());
        doc.signature_hex = hex(&key()
            .sign(&wire::placement_signing_message(&doc.placement).unwrap())
            .to_bytes());
        let mut changed = crate::canonical_json(&doc).unwrap();
        changed.push(b'\n');
        assert!(api::placement_join(&mut owner, &mut controls, &changed, Some(&retained)).is_err());
    }
    println!("HC074-OWNED-PLACEMENT seed=0x074e6a1d identities=128 execution=false");
}
#[test]
fn owned_placement_constructor_consumes_original_and_retains_only_revalidation() {
    fn signature(
        value: OwnedRootGuardians<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
        raw: Vec<u8>,
    ) {
        if let Ok(mut joined) = value.bind_signed_placement(raw) {
            let _ = joined.revalidate();
        }
    }
    let _contract = signature;
}
