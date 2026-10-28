use super::*;
use crate::diagnostic_ipc::validation_trace::{self as trace, Stage};
fn boottime() -> u64 {
    let mut t: libc::timespec = unsafe { std::mem::zeroed() };
    assert_eq!(
        unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut t) },
        0
    );
    (t.tv_sec as u64) * 1_000_000_000 + t.tv_nsec as u64
}

#[test]
#[ignore = "manual nonroot publication-free original consent budget attribution"]
fn validation_frame_publication_budget() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    for sample in 0..3 {
        let (_, report) = trace::capture(|| {
            trace::stage(Stage::Setup);
            with_progress_original(true, |original, _, receiver, sender, model| {
                let request = original.original.original.request.clone();
                with_owned_empty_fence_for_test(|fence, root, leaf| {
                    let attempt = original
                        .bind_common_empty_fence(fence)
                        .unwrap()
                        .retain_request()
                        .unwrap();
                    let request_raw = fs::read(leaf.join(NAME)).unwrap();
                    let mut observed = attempt.into_observed_reservation();
                    let state = observed.state_to_sign().unwrap();
                    let reserved = observed
                        .publish_reserved(&signed(&body(&request, &state, &request_raw)))
                        .unwrap();
                    let reserved_raw = fs::read(leaf.join(RESERVED_NAME)).unwrap();
                    let challenge = reserved.issue_progress_challenge().unwrap();
                    let nonce = crate::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::collector_challenge_nonce(challenge.challenge_to_sign()).unwrap();
                    let mut response = request.clone();
                    response.operation = DiagnosticOperation::Heartbeat;
                    response.expected_state_revision = 1;
                    response.request_id = "f".repeat(64);
                    response.nonce_sha256 = nonce;
                    let packet = receive(receiver, sender, &model.sign(response, &model.key));
                    let received = packet.received_time().boottime_ns();
                    let causal = challenge.accept_response(packet).unwrap();
                    let prepared = causal.prepare_start_consent("1".repeat(64)).unwrap();
                    let signed_consent = consent(prepared.consent_to_sign().clone(), &model.key);
                    let packet = receive(receiver, sender, &signed_consent);
                    let mut accepted = prepared
                        .accept_consent(packet)
                        .unwrap()
                        .into_validation_frame()
                        .unwrap();
                    let started = boottime();
                    let mut completed = 0;
                    trace::stage(Stage::Checks);
                    // Isolate an expected time refusal from the fixture's outer healthy setup.
                    let (_, refusal) = crate::diagnostic_kernel_admission::refusal_trace::capture(
                        || {
                            for check in 0..19 {
                                let result = accepted.revalidate();
                                let now = boottime();
                                println!("HC074-FRAME-BUDGET sample={sample} check={check} elapsed_ns={} reserved_age_ns={} heartbeat_age_ns={} result={result:?}",
                                now - started, now - state.reserved_monotonic_ns, now - received);
                                if result.is_err() {
                                    assert!(accepted.is_refused());
                                    assert!(matches!(
                                        accepted.revalidate(),
                                        Err(OriginalStartConsentError::Refused)
                                    ));
                                    break;
                                }
                                completed += 1;
                            }
                        },
                    );
                    println!("HC074-FRAME-BUDGET-SUMMARY sample={sample} completed={completed} requested=19 publication_free=true admitted={} first_kernel_refusal={refusal:?}", completed == 19);
                    assert_eq!(fs::read(leaf.join(NAME)).unwrap(), request_raw);
                    assert_eq!(fs::read(leaf.join(RESERVED_NAME)).unwrap(), reserved_raw);
                    assert_eq!(fs::read_dir(leaf).unwrap().count(), 2);
                    busy(root);
                    if !accepted.is_refused() {
                        fs::remove_file(leaf.join(RESERVED_NAME)).unwrap();
                        assert!(accepted.revalidate().is_err());
                    }
                    assert!(accepted.is_refused());
                    busy(root);
                    drop(accepted);
                    drop(crate::host_execution::lock_host_root(root).unwrap());
                    retained_context_refuses_for_test(root);
                });
            });
        });
        // These counters cover diagnostic IPC authentication, not every crypto primitive.
        println!("HC074-FRAME-BUDGET-COUNTERS sample={sample} ipc_trace={report:?}");
    }
}
