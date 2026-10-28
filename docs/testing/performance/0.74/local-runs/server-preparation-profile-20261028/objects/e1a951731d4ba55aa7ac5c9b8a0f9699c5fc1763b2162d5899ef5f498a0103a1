//! Real manager placement, readiness only. Never worker execution or completion.
use super::*;
#[path = "held_original_service_scope.rs"]
mod scope;

#[test]
#[ignore = "manual isolated WSL original owner service readiness; temporary diagnostic enrollment/account required"]
fn privileged_original_manager_service_once() {
    assert_eq!(
        std::env::var("HC074_ORIGINAL_SERVICE").as_deref(),
        Ok("native-once-v1")
    );
    assert_eq!(
        std::env::var("HC074_OWNED_ROOT_FIXTURE").as_deref(),
        Ok("local-only-v1")
    );
    assert_eq!(
        (unsafe { libc::geteuid() }, unsafe { libc::getegid() }),
        (0, 0)
    );
    assert_eq!(
        std::fs::read_to_string("/etc/hostname").unwrap().trim(),
        "hc074-native"
    );
    let (result, facts) = captured(run_original);
    println!("HC074-ORIGINAL-SERVICE-FIRST {facts}");
    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
}
fn run_original() {
    let model = trusted_input::controller_input::tests::privileged_fixture::new();
    let request = request(&model);
    let p = policy(&request);
    let token = token(
        &header(&p),
        &serde_json::to_vec(&claims(&request, &p)).unwrap(),
    );
    let (_worker_root, _authority_root, mut source, mut authority, worker_bytes) =
        trusted_input::tests::privileged_fixture::new();
    // All role preparation precedes original controller/causal/consent origins.
    let mut documents = Fixture::new(model.key.verifying_key(), &worker_bytes, BUILDER_POLICY);
    let bootstrap = documents.bootstrap_for_service();
    let bundle = bundle();
    let attempt: [u8; 32] = Sha256::digest(
        serde_json::to_vec(&("hc074-original-root-attempt-v1", &request.identity)).unwrap(),
    )
    .into();
    let scope = scope::Scope::create(attempt, &request.identity.lease_id);
    let mut cgroup = scope.cgroup();
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
            let prepared = measured("prepare-consent", || {
                causal.prepare_framed_consent("1".repeat(64)).unwrap()
            });
            let bytes = consent(prepared.consent_to_sign().clone(), &model.key);
            let accepted = measured("accept-consent", || {
                prepared.accept_consent(relay.receive(&bytes)).unwrap()
            });
            let outputs = measured("outputs", || accepted.prepare_original_outputs().unwrap());
            let frame = measured("enrollment-frame", || {
                outputs
                    .prepare_privileged_enrollment_for_test(&bundle.path().join("bundle"))
                    .unwrap()
            });
            assert_eq!(
                frame.offsets_for_test(),
                [300000000000, 60000000000, 10000000000]
            );
            let message = measured("fixed-manager-message", || {
                frame.prepare_fixed_manager_message().unwrap()
            });
            let mut owned = measured("owned-root-preparation", || {
                message.prepare_owned_root_guardians(&mut root).unwrap()
            });
            cgroup.guardians = Some(owned.fixture_guardians());
            let origins = owned.fixture_origins();
            let (mut body, binding) = owned.placement_for_fixture(identity);
            body.launch_claims.artifacts = documents.artifacts.clone();
            body.launch_claims.requirements_sha256 = crate::sha256_hex(
                &crate::diagnostic_two_phase_launch::launch_requirements(
                    &body.launch_claims.consent_claims,
                    &body.launch_claims.artifacts,
                )
                .unwrap(),
            );
            body.requirements_sha256 = crate::sha256_hex(
                &wire::placement_requirements(&body.launch_claims, &binding).unwrap(),
            );
            let placement = sign(body, &model.key);
            let v4 = documents.bind(serde_json::from_slice(&placement).unwrap(), &model.key);
            let [mut out, mut err, writer, other] = owned.fixture_streams();
            let custodians = owned.fixture_custodians();
            drop(writer);
            drop(other);
            measured("enrollment", || owned.enroll_once().unwrap());
            let placement = measured("placement", || {
                owned.bind_signed_placement(placement).unwrap()
            });
            let fixed = measured("fixed-slice", || placement.bind_fixed_slice().unwrap());
            let roles = documents.roles.take().unwrap();
            let trust = documents.input.take().unwrap();
            let held = fixed
                .join_held_fixture(
                    roles,
                    trust,
                    [
                        &documents.held_policy,
                        &documents.worker_policy,
                        &documents.receipt,
                        &v4,
                    ],
                )
                .unwrap();
            assert_eq!(held.fixture_origins(), origins);
            assert!(held.fixture_timer_live());
            busy(campaigns);
            let projected = held.project_held_unit().unwrap();
            let material = projected.materialize_held_unit().unwrap();
            let (image, policy, receipt) = bootstrap;
            let bootstrap = material.join_bootstrap(image, &policy, &receipt).unwrap();
            let installed = bootstrap.bind_installed_origin().unwrap();
            let entry = installed.materialize_installed_entry().unwrap();
            let mut delivery = entry.prepare_original_held_delivery().unwrap();
            println!("HC074-ORIGINAL-SERVICE prepared=true original_send=false");
            delivery.deliver_once().unwrap();
            println!("HC074-ORIGINAL-SERVICE original_send=true manager_ack=true");
            delivery.challenge_child_once().unwrap();
            // The owner's immutable absolute alarm bounds every fresh frame.
            // No retry of send or challenge, no renewed admission origin.
            while !delivery.try_child_ack().unwrap() {
                std::thread::yield_now();
            }
            println!("HC074-ORIGINAL-SERVICE readiness=true release=false completion=false");
            drop(delivery);
            scope.retire_service();
            cgroup.empty();
            for pin in &custodians {
                dead(pin);
            }
            assert!(drain(&mut out) && drain(&mut err));
            busy(campaigns);
            println!("HC074-ORIGINAL-SERVICE eof=true fence_retained=true custodians_terminal=true completion=false server=false product=false");
        },
    );
}
