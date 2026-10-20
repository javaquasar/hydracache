//! Actual holder in a private local cgroup mount, no manager send or product.
use super::*;
use crate::diagnostic_two_phase_launch::held_placement as wire;
use crate::diagnostic_worker_policy::trusted_input;
use ed25519_dalek::Signer;
use sha2::{Digest, Sha256};
use std::time::Instant;
use trusted_input::controller_input::held_runtime::late_join::live_fixture::Fixture;
use crate::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::time_observation as time;

#[path = "consuming_held_live_namespace.rs"]
mod namespace;

fn busy(campaigns: &Path) {
    assert!(matches!(
        crate::host_execution::lock_host_root(campaigns),
        Err(crate::host_execution::HostExecutionError::Busy)
    ));
}
fn sign(body: wire::PlacementBody, key: &ed25519_dalek::SigningKey) -> Vec<u8> {
    let signature = key.sign(&wire::placement_signing_message(&body).unwrap());
    let mut raw = crate::canonical_json(&wire::SignedPlacement {
        schema_version: wire::ENVELOPE_SCHEMA.into(),
        placement: body,
        signature_hex: crate::hex(&signature.to_bytes()),
    })
    .unwrap();
    raw.push(b'\n');
    raw
}
fn measured<T>(phase: &str, call: impl FnOnce() -> T) -> T {
    if std::env::var("HC074_CONSUMING_HELD_COST").as_deref() != Ok("1") {
        return call();
    }
    profiled(phase, call)
}
fn profiled<T>(phase: &str, call: impl FnOnce() -> T) -> T {
    fn cpu() -> i128 {
        let mut value: libc::timespec = unsafe { std::mem::zeroed() };
        assert_eq!(
            unsafe { libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, &mut value) },
            0
        );
        value.tv_sec as i128 * 1_000_000_000 + value.tv_nsec as i128
    }
    let wall = Instant::now();
    let before = cpu();
    // Preserve the same panic payload; record a failed phase rather than retry.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(call));
    let cpu_ns = cpu() - before;
    let wall_ns = wall.elapsed().as_nanos();
    println!("HC074-HELD-LIVE-COST phase={phase} wall_ns={wall_ns} process_cpu_ns={cpu_ns} returned={} includes_child_cpu=false guard_clock=false",result.is_ok());
    match result {
        Ok(value) => value,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

#[test]
fn consuming_held_cost_observer_preserves_single_call_result_and_panic_identity() {
    let calls = std::cell::Cell::new(0);
    assert_eq!(
        profiled("unit-result", || {
            calls.set(calls.get() + 1);
            Err::<(), _>(74)
        }),
        Err(74)
    );
    assert_eq!(calls.get(), 1);
    let marker = std::sync::Arc::new(75u32);
    let error = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        profiled("unit-panic", || {
            calls.set(calls.get() + 1);
            std::panic::panic_any(std::sync::Arc::clone(&marker));
        })
    }))
    .unwrap_err();
    assert_eq!(calls.get(), 2);
    assert!(std::sync::Arc::ptr_eq(
        &error.downcast::<std::sync::Arc<u32>>().unwrap(),
        &marker
    ));
}

#[test]
#[ignore = "manual local root private-namespace actual consuming holder; no send or product"]
fn privileged_consuming_held_live_matrix() {
    assert_eq!(
        std::env::var("HC074_OWNED_ROOT_FIXTURE").as_deref(),
        Ok("local-only-v1")
    );
    assert_eq!(
        (unsafe { libc::geteuid() }, unsafe { libc::getegid() }),
        (0, 0)
    );
    const MODE: &str = "HC074_CONSUMING_HELD_LIVE_CASE";
    if let Ok(case) = std::env::var(MODE) {
        assert!(matches!(
            case.as_str(),
            "healthy" | "revalidate" | "role-before" | "role-after" | "trust-after"
        ));
        run(&case);
        return;
    }
    let test = std::thread::current().name().unwrap().to_owned();
    for case in [
        "healthy",
        "revalidate",
        "role-before",
        "role-after",
        "trust-after",
    ] {
        let result = Command::new(std::env::current_exe().unwrap())
            .args([
                &test,
                "--exact",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(MODE, case)
            .status()
            .unwrap();
        assert!(
            result.success(),
            "actual held case refused: {case}: {result}"
        );
    }
}
fn run(case: &str) {
    let (result, mut facts) = captured(|| run_original(case));
    facts["case"] = case.into();
    println!("HC074-HELD-LIVE-ORIGINAL {facts}");
    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
}
fn captured<T>(call: impl FnOnce() -> T) -> (std::thread::Result<T>, serde_json::Value) {
    let (((result, causal), age), kernel) =
        crate::diagnostic_kernel_admission::refusal_trace::capture(|| {
            time::closing::capture(|| {
                time::capture(|| std::panic::catch_unwind(std::panic::AssertUnwindSafe(call)))
            })
        });
    let kernel=kernel.map(|event|serde_json::json!({"site":event.site,"phase":event.phase,"file":event.file,"line":event.line,"clock":event.clock}));
    let facts = serde_json::json!({"returned":result.is_ok(),"causal":causal,"age":age,"kernel":kernel,"guard_clock_added":false,"deadline_changed":false,"server":false,"product":false});
    (result, facts)
}
#[test]
fn consuming_held_trace_keeps_first_original_refusal_and_same_panic_without_retry() {
    let calls = std::cell::Cell::new(0);
    let marker = std::sync::Arc::new(74u32);
    let (result, facts) = captured(|| {
        calls.set(calls.get() + 1);
        crate::diagnostic_kernel_admission::refusal_trace::record(
            "test-first",
            file!(),
            line!(),
            [1, 2, 3, 4, 1, 60],
        );
        crate::diagnostic_kernel_admission::refusal_trace::record(
            "test-later",
            file!(),
            line!(),
            [5; 6],
        );
        std::panic::panic_any(std::sync::Arc::clone(&marker));
    });
    assert_eq!(calls.get(), 1);
    assert_eq!(facts["kernel"]["site"], "test-first");
    assert!(std::sync::Arc::ptr_eq(
        &result
            .unwrap_err()
            .downcast::<std::sync::Arc<u32>>()
            .unwrap(),
        &marker
    ));
    let (value, facts) = captured(|| 74);
    assert_eq!(value.unwrap(), 74);
    assert!(facts["kernel"].is_null());
}
fn run_original(case: &str) {
    let _namespace = namespace::Private::new();
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
    let bundle = bundle();
    let attempt: [u8; 32] = Sha256::digest(
        serde_json::to_vec(&("hc074-original-root-attempt-v1", &request.identity)).unwrap(),
    )
    .into();
    let mut cgroup = namespace::attempt(attempt);
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
            owned.enroll_once().unwrap();
            let placement = measured("placement", || {
                owned.bind_signed_placement(placement).unwrap()
            });
            let fixed = measured("fixed-slice", || placement.bind_fixed_slice().unwrap());
            if case == "role-before" {
                assert_eq!(
                    unsafe { libc::fchmod(documents.alias.as_raw_fd(), 0o600) },
                    0
                );
            }
            let roles = documents.roles.take().unwrap();
            let trust = documents.input.take().unwrap();
            let result = measured("consuming", || {
                fixed.join_held_fixture(
                    roles,
                    trust,
                    [
                        &documents.held_policy,
                        &documents.worker_policy,
                        &documents.receipt,
                        &v4,
                    ],
                )
            });
            if case == "role-before" {
                let failure = result.err().expect("changed role must refuse");
                assert!(
                    format!("{:?}", failure.error()).starts_with("Roles("),
                    "{failure:?}"
                );
                assert!(failure.is_refused() && failure.fixture_timer_live());
                busy(campaigns);
                assert!(!drain(&mut out) && !drain(&mut err));
                drop(failure);
            } else {
                let mut held = result.unwrap();
                assert_eq!(held.fixture_origins(), origins);
                assert!(held.fixture_timer_live());
                busy(campaigns);
                if case == "revalidate" {
                    measured("revalidate", || held.revalidate().unwrap());
                    assert_eq!(held.fixture_origins(), origins);
                }
                if case == "role-after" || case == "trust-after" {
                    if case == "role-after" {
                        assert_eq!(
                            unsafe { libc::fchmod(documents.alias.as_raw_fd(), 0o600) },
                            0
                        );
                    } else {
                        documents.drift(false);
                    }
                    let error = held.revalidate().unwrap_err();
                    let reason = format!("{error:?}");
                    assert!(
                        reason.starts_with(if case == "role-after" {
                            "Roles("
                        } else {
                            "Trust("
                        }),
                        "{reason}"
                    );
                    assert!(held.fixture_timer_live());
                    busy(campaigns);
                    assert!(!drain(&mut out) && !drain(&mut err));
                    if case == "trust-after" {
                        documents.drift(true);
                    }
                    assert_eq!(format!("{:?}", held.revalidate().unwrap_err()), "Refused");
                }
                drop(held);
            }
            cgroup.empty();
            for pin in &custodians {
                dead(pin);
            }
            assert!(drain(&mut out) && drain(&mut err));
            busy(campaigns);
            println!("HC074-HELD-LIVE case={case} seed=0x074c051e actual_fixed=true actual_consuming=true original_bounds=true eof=true fence_retained=true guardians_terminal=true synthetic_images=true original_send=false completion=false server=false product=false");
        },
    );
}
