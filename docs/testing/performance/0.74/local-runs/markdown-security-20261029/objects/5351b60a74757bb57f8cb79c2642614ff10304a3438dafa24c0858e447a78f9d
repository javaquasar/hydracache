//! Numeric fixture attribution only; the original validation schedule is unchanged.
use super::*;
#[derive(Default)]
struct Timeline {
    issued: u64,
    points: Vec<(&'static str, u64)>,
}
fn boottime() -> u64 {
    let mut value = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    assert_eq!(
        unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut value) },
        0
    );
    u64::try_from(value.tv_sec).unwrap() * 1_000_000_000 + u64::try_from(value.tv_nsec).unwrap()
}
impl Timeline {
    fn mark(&mut self, name: &'static str) {
        self.points.push((name, boottime()));
    }
    fn challenge(&mut self, raw: &[u8]) {
        // Read-only numeric data from the same immutable canonical challenge.
        let now = boottime();
        let data: serde_json::Value = serde_json::from_slice(raw).unwrap();
        self.issued = data["issued_boottime_ns"].as_u64().unwrap();
        assert_ne!(self.issued, 0);
        self.points.push(("challenge-ready", now));
    }
    fn emit(&self, sample: usize) {
        for (name, now) in &self.points {
            println!("HC074-PREPARATION-POINT sample={sample} point={name} issued_ns={} now_ns={now} age_ns={}", self.issued, now.checked_sub(self.issued).unwrap());
        }
    }
}
fn accepted_profiled<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>(
    original: &'join mut OidcControllerInput<'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    fence: crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::OriginalEmptyReservationFence<'storage>,
    leaf: &std::path::Path,
    receiver: &mut KernelReceiver,
    sender: &OwnedFd,
    model: &Model,
    timeline: &mut Timeline,
) -> OriginalFramedConsent<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    let request = original.original.original.request.clone();
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
    trace::stage(Stage::Setup);
    let challenge = reserved.issue_progress_challenge().unwrap();
    timeline.challenge(challenge.challenge_to_sign());
    trace::stage(Stage::Fence);
    let nonce = crate::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::collector_challenge_nonce(challenge.challenge_to_sign()).unwrap();
    let mut response = request;
    response.operation = DiagnosticOperation::Heartbeat;
    response.expected_state_revision = 1;
    response.request_id = "f".repeat(64);
    response.nonce_sha256 = nonce;
    let causal = challenge
        .accept_response(receive(receiver, sender, &model.sign(response, &model.key)))
        .unwrap();
    timeline.mark("response-ready");
    trace::stage(Stage::Retain);
    let prepared = causal.prepare_framed_consent("1".repeat(64)).unwrap();
    timeline.mark("consent-prepared");
    trace::stage(Stage::Observe);
    let bytes = consent(prepared.consent_to_sign().clone(), &model.key);
    prepared
        .accept_consent(receive(receiver, sender, &bytes))
        .unwrap()
}

#[test]
#[ignore = "manual nonroot original preparation budget attribution; no dispatch or admission"]
fn original_preparation_budget_attribution() {
    for sample in 0..3 {
        let bundle = Bundle::new();
        with_original_at(false, None, |original, _paths, receiver, sender, model| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let mut timeline = Timeline::default();
                let mut rounds = Vec::new();
                let ((frame, probe), report) = trace::capture(|| {
                    trace::stage(Stage::Unassigned);
                    let framed = accepted_profiled(
                        original,
                        fence,
                        leaf,
                        receiver,
                        sender,
                        model,
                        &mut timeline,
                    );
                    timeline.mark("consent-ready");
                    trace::stage(Stage::Publish);
                    let outputs = framed
                        .prepare_original_outputs_for_test(|_| Ok(()))
                        .unwrap();
                    timeline.mark("outputs-ready");
                    trace::stage(Stage::Bind);
                    let mut frame = outputs
                        .prepare_enrollment_fixture_for_test(&bundle.path, 0, |_, _| {})
                        .unwrap();
                    timeline.mark("enrollment-ready");
                    let origins = frame.origins_for_test();
                    assert_eq!(origins[1], timeline.issued);
                    let before = records(leaf);
                    trace::stage(Stage::Checks);
                    let mut probe = frame.message_probe_for_test();
                    for pass in 0..2 {
                        let ((result, checks), round) = trace::capture(|| {
                            trace::stage(Stage::Checks);
                            frame.message_round_for_test(
                                &mut probe,
                                if pass == 0 { "ok" } else { "delivery" },
                            )
                        });
                        timeline.mark(if pass == 0 {
                            "manager-prepare-ready"
                        } else {
                            "manager-delivery-ready"
                        });
                        rounds.push((pass, checks, round));
                        result.unwrap();
                        assert_eq!(checks, 2);
                        frame.assert_original_for_test();
                        assert_eq!(&frame.origins_for_test()[..4], &origins[..4]);
                        assert_eq!(
                            frame.offsets_for_test(),
                            [300_000_000_000, 60_000_000_000, 10_000_000_000]
                        );
                        assert_eq!(records(leaf), before);
                    }
                    (frame, probe)
                });
                assert!(report.first_time.is_none() && report.time_refusals == 0);
                let stages = [
                    (Stage::Unassigned, "prechallenge-reservation"),
                    (Stage::Setup, "challenge-creation"),
                    (Stage::Fence, "response-acceptance"),
                    (Stage::Retain, "consent-preparation"),
                    (Stage::Observe, "consent-acceptance"),
                    (Stage::Publish, "joined-outputs"),
                    (Stage::Bind, "enrollment"),
                    (Stage::Checks, "manager-bookkeeping-inclusive"),
                ];
                let before = records(leaf);
                busy(root);
                drop(probe);
                busy(root);
                drop(frame);
                assert_eq!(records(leaf), before);
                retained_context_refuses_for_test(root);
                // Printing happens only after the original critical path and cleanup.
                timeline.emit(sample);
                for (stage, label) in stages {
                    let stats = report.stages[stage as usize];
                    assert_eq!(stats.authentication_calls, stats.signature_calls);
                    println!(
                        "HC074-PREPARATION-COST sample={sample} bucket={label} stats={stats:?}"
                    );
                }
                assert_eq!(report.stages[Stage::Bind as usize].authentication_calls, 34);
                for (pass, checks, round) in rounds {
                    let stats = round.stages[Stage::Checks as usize];
                    assert_eq!(stats.authentication_calls, 34);
                    assert_eq!(stats.signature_calls, 34);
                    assert!(round.first_time.is_none() && round.time_refusals == 0);
                    println!("HC074-PREPARATION-ROUND sample={sample} pass={pass} channel_checks={checks} stats={stats:?}");
                }
                println!("HC074-PREPARATION-CONTROL sample={sample} unchanged=true origins=true bounds=true rights=true test_channel_only=true start=false schedule_changed=false");
            });
        });
    }
}
