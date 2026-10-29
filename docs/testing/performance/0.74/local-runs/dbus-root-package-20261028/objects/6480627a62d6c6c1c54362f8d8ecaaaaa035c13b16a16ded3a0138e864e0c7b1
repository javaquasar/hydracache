//! Observation only: no write, new authority clock or deadline adjustment.
use super::*;
use crate::diagnostic_ipc::validation_trace::{self as trace, Stage};

fn boottime() -> u64 {
    let mut time: libc::timespec = unsafe { std::mem::zeroed() };
    assert_eq!(
        unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut time) },
        0
    );
    (time.tv_sec as u64) * 1_000_000_000 + time.tv_nsec as u64
}

fn cpu() -> (u128, u128) {
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    assert_eq!(
        unsafe { libc::getrusage(libc::RUSAGE_THREAD, &mut usage) },
        0
    );
    let micros = |t: libc::timeval| {
        u128::try_from(t.tv_sec).unwrap() * 1_000_000 + u128::try_from(t.tv_usec).unwrap()
    };
    (micros(usage.ru_utime), micros(usage.ru_stime))
}

fn inclusive(sample: u8, owner: &str, mut check: impl FnMut()) {
    for repetition in 0..3 {
        let before = cpu();
        let start = boottime();
        let (_, report) = trace::capture(|| {
            trace::stage(Stage::Checks);
            check();
        });
        let elapsed = boottime() - start;
        let after = cpu();
        let stats = report.stages[Stage::Checks as usize];
        assert!(report.first_time.is_none());
        println!("HC074-CONSENT-OWNER sample={sample} owner={owner} repetition={repetition} inclusive_wall_ns={elapsed} user_us={} system_us={} ipc_authentication_calls={} ipc_signature_calls={} ipc_signature_ns={}",
            after.0.checked_sub(before.0).unwrap(), after.1.checked_sub(before.1).unwrap(),
            stats.authentication_calls, stats.signature_calls, stats.signature_ns);
    }
}

#[test]
#[ignore = "manual nonroot inclusive original validation owner attribution"]
fn original_consent_validation_owner_attribution() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    for sample in 0..3 {
        with_progress_original(false, |original, _, _, _, _| {
            inclusive(sample, "kernel-worker", || {
                original.original.original.worker.revalidate().unwrap()
            });
            inclusive(sample, "signed-controller", || {
                original.original.original.revalidate().unwrap()
            });
            inclusive(sample, "reviewed-build", || {
                original.original.revalidate().unwrap()
            });
            inclusive(sample, "oidc-controller", || original.revalidate().unwrap());
            assert!(!original.is_refused());
        });
    }
    println!("HC074-CONSENT-OWNER-SCOPE samples=3 repetitions=3 owners=4 inclusive=true mutation=false start=false");
}

#[test]
#[ignore = "manual nonroot publication-free original consent budget attribution"]
fn original_consent_publication_budget() {
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
                    let mut accepted = prepared.accept_consent(packet).unwrap();
                    let started = boottime();
                    let mut completed = 0;
                    trace::stage(Stage::Checks);
                    // Isolate an expected time refusal from the fixture's outer healthy setup.
                    let (_, refusal) = crate::diagnostic_kernel_admission::refusal_trace::capture(
                        || {
                            for check in 0..19 {
                                let result = accepted.revalidate();
                                let now = boottime();
                                println!("HC074-CONSENT-BUDGET sample={sample} check={check} elapsed_ns={} reserved_age_ns={} heartbeat_age_ns={} result={result:?}",
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
                    println!("HC074-CONSENT-BUDGET-SUMMARY sample={sample} completed={completed} requested=19 publication_free=true admitted={} first_kernel_refusal={refusal:?}", completed == 19);
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
        println!("HC074-CONSENT-BUDGET-COUNTERS sample={sample} ipc_trace={report:?}");
    }
}
