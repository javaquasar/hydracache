//! Real local manager/frame/root, synthetic issuers, no held-unit send or product.
use super::*;
use crate::diagnostic_two_phase_launch::held_placement as wire;
use crate::diagnostic_two_phase_launch::root_enrollment::{Kind, PacketClaims};
use ed25519_dalek::{Signer, SigningKey};
use crate::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::time_observation;

fn busy(campaigns: &Path) {
    assert!(matches!(
        crate::host_execution::lock_host_root(campaigns),
        Err(crate::host_execution::HostExecutionError::Busy)
    ));
}

fn sign(body: wire::PlacementBody, key: &SigningKey) -> Vec<u8> {
    let signature = key.sign(&wire::placement_signing_message(&body).unwrap());
    let mut raw = crate::canonical_json(&wire::SignedPlacement {
        schema_version: wire::ENVELOPE_SCHEMA.into(),
        placement: body,
        signature_hex: signature
            .to_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
    })
    .unwrap();
    raw.push(b'\n');
    raw
}
#[test]
#[ignore = "manual local signed placement/original owner/root join; no original send or product"]
fn privileged_owned_placement_live_join() {
    for case in [
        "healthy",
        "revalidate",
        "unarmed",
        "wrong-key",
        "different-consent",
        "foreign-root",
        "trust-drift",
    ] {
        run(case);
    }
}
fn run(case: &str) {
    use crate::diagnostic_worker_policy::trusted_input;
    let model = trusted_input::controller_input::tests::privileged_fixture::new();
    let request = request(&model);
    let p = policy(&request);
    let token = token(
        &header(&p),
        &serde_json::to_vec(&claims(&request, &p)).unwrap(),
    );
    let (worker_root, _authority_root, mut source, mut authority, worker_bytes) =
        trusted_input::tests::privileged_fixture::new();
    let bundle = bundle();
    let mut cgroup = cgroup::Cgroup::new();
    let mut root = RootSource::pin(cgroup.parent.as_fd(), &cgroup.name).unwrap();
    let identity = root.identity();
    let mut relay = relay::Relay::new();
    let mut packet = relay.receive(&model.sign(request.clone(), &model.key));
    let mut worker = source
        .verify_kernel_policy(&worker_bytes, &mut packet, &mut authority)
        .unwrap();
    let mut controller = model.input();
    let mut oidc = TrustModel::new(&request);
    let mut original = controller.authenticate_controller(&mut worker).unwrap();
    let mut build = original
        .bind_reviewed_build(BUILDER_POLICY, RECEIPT)
        .unwrap();
    let mut original = build.bind_oidc(&mut oidc.trust, &token).unwrap();
    crate::diagnostic_output_preparation::fixed_intent_storage::privileged_fixture::with_fence(
        |fence, campaigns, leaf| {
            let attempt = original
                .bind_common_empty_fence(fence)
                .unwrap()
                .retain_request()
                .unwrap();
            let raw = fs::read(leaf.join(NAME)).unwrap();
            let mut observed = attempt.into_observed_reservation();
            let state = observed.state_to_sign().unwrap();
            let reserved = observed
                .publish_reserved(&signed(&body(&request, &state, &raw)))
                .unwrap();
            let challenge = reserved.issue_progress_challenge().unwrap();
            let nonce=crate::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::collector_challenge_nonce(challenge.challenge_to_sign()).unwrap();
            let mut response = request.clone();
            response.operation = DiagnosticOperation::Heartbeat;
            response.expected_state_revision = 1;
            response.request_id = "f".repeat(64);
            response.nonce_sha256 = nonce;
            let causal = challenge
                .accept_response(relay.receive(&model.sign(response, &model.key)))
                .unwrap();
            let prepared = causal.prepare_framed_consent("1".repeat(64)).unwrap();
            let bytes = consent(prepared.consent_to_sign().clone(), &model.key);
            let accepted = prepared.accept_consent(relay.receive(&bytes)).unwrap();
            let outputs = accepted.prepare_original_outputs().unwrap();
            let frame = outputs
                .prepare_privileged_enrollment_for_test(&bundle.path().join("bundle"))
                .unwrap();
            assert_eq!(
                frame.offsets_for_test(),
                [300000000000, 60000000000, 10000000000]
            );
            let message = frame.prepare_fixed_manager_message().unwrap();
            let mut owned = message.prepare_owned_root_guardians(&mut root).unwrap();
            cgroup.guardians = Some(owned.fixture_guardians());
            let origins = owned.fixture_origins();
            let (mut body, mut binding) = owned.placement_for_fixture(identity);
            if case != "unarmed" {
                owned.enroll_once().unwrap();
            }
            if case == "different-consent" {
                body.launch_claims.consent_claims.worker_policy_epoch += 1;
            }
            if case == "foreign-root" {
                binding.root_inode += 1;
                body.root_enrollment_hex = PacketClaims {
                    kind: Kind::Enroll,
                    binding,
                }
                .encode()
                .unwrap()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            }
            let launch = &mut body.launch_claims;
            launch.requirements_sha256 = crate::sha256_hex(
                &crate::diagnostic_two_phase_launch::launch_requirements(
                    &launch.consent_claims,
                    &launch.artifacts,
                )
                .unwrap(),
            );
            body.requirements_sha256 =
                crate::sha256_hex(&wire::placement_requirements(launch, &binding).unwrap());
            let wrong = SigningKey::from_bytes(&[75; 32]);
            let raw = sign(
                body,
                if case == "wrong-key" {
                    &wrong
                } else {
                    &model.key
                },
            );
            if case == "trust-drift" {
                fs::write(
                    worker_root
                        .path()
                        .join("etc/hydracache-perf/worker-trust-074.json"),
                    b"owned placement live trust drift\n",
                )
                .unwrap();
            }
            let [mut out, mut err, writer, other] = owned.fixture_streams();
            let custodians = owned.fixture_custodians();
            drop(writer);
            drop(other);
            let (result, facts) = time_observation::capture(|| {
                let result = owned.bind_signed_placement(raw);
                if case == "healthy" || case == "revalidate" {
                    let mut joined = result.unwrap();
                    assert_eq!(joined.placement_origins_for_fixture(), origins);
                    if case == "revalidate" {
                        joined.revalidate().unwrap();
                        assert_eq!(joined.placement_origins_for_fixture(), origins);
                    }
                    assert!(joined.placement_timer_live_for_fixture());
                    busy(campaigns);
                    drop(joined);
                    true
                } else {
                    let error = result.err().expect("negative must refuse");
                    let reason = format!("{error:?}");
                    assert!(
                        !reason.contains("Time") && !reason.contains("Expired"),
                        "{case}: {reason}"
                    );
                    assert!(
                        error.placement_timer_live_for_fixture(),
                        "{case}: original timer must still be live"
                    );
                    // Retained failed custody, not a fabricated Armed keeper.
                    busy(campaigns);
                    assert!(!cgroup::drain(&mut out));
                    assert!(!cgroup::drain(&mut err));
                    drop(error);
                    false
                }
            });
            let facts = serde_json::to_value(facts).unwrap();
            assert!(facts["first_refusal"].is_null(), "{case}: {facts}");
            cgroup.empty();
            for pin in &custodians {
                cgroup::dead(pin);
            }
            assert!(cgroup::drain(&mut out));
            assert!(cgroup::drain(&mut err));
            let armed = case != "unarmed";
            if armed {
                busy(campaigns);
            } else {
                // All writers/custodians are already stopped. A raw flock is
                // not permission to replay the retained incomplete attempt.
                drop(crate::host_execution::lock_host_root(campaigns).unwrap());
                crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::retained_context_refuses_for_test(campaigns);
            }
            println!("HC074-OWNED-PLACEMENT-LIVE case={case} accepted={result} seed=0x074e6a1d fixed_manager=true actual_root=true original_bounds=true eof=true fence_held_while_owned=true fence_held_after_drop={armed} retained_replay_refused={} timer_live_before_drop=true test_issuers=true actual_systemd_placement=false original_send=false completion=false product=false facts={facts}", !armed);
        },
    );
}
