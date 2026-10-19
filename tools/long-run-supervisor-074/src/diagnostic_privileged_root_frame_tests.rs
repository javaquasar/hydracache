//! Real fixed manager and kernel root, synthetic issuers explicitly test-only.
use super::*;
use crate::diagnostic_two_phase_launch::root_enrollment::RootSource;
use std::os::fd::{AsFd, AsRawFd};
#[path = "diagnostic_privileged_root_cgroup_fixture.rs"]
mod cgroup;
#[path = "diagnostic_owned_root_creator_fixture.rs"]
mod creator;
#[path = "diagnostic_privileged_controller_relay.rs"]
mod relay;

fn bundle() -> tempfile::TempDir {
    let source = PathBuf::from(std::env::var_os("HC074_START_MATERIAL_BUNDLE").unwrap());
    assert!(source.is_absolute());
    assert_ne!(source, Path::new(crate::diagnostic_artifacts::INSTALL_ROOT));
    let root = crate::diagnostic_two_phase_launch::root_enrollment::privileged_fixture::new();
    let path = root.path().join("bundle");
    fs::create_dir(&path).unwrap();
    for name in [
        "timing-controls-074",
        "build-receipt-v1.json",
        "build-log.jsonl",
        "Cargo.lock.root",
        "Cargo.lock.observer",
        "embedded.json",
        "direct.json",
        "resp2.json",
        "resp3.json",
    ] {
        fs::copy(source.join(name), path.join(name)).unwrap();
        fs::set_permissions(
            path.join(name),
            fs::Permissions::from_mode(if name == "timing-controls-074" {
                0o555
            } else {
                0o444
            }),
        )
        .unwrap();
    }
    fs::set_permissions(&path, fs::Permissions::from_mode(0o555)).unwrap();
    root
}

#[test]
#[ignore = "manual local privileged original manager/root/dummy writer composition; no product or original send"]
fn privileged_original_root_manager_writer_screen() {
    run("retire");
}
#[test]
#[ignore = "manual local privileged original manager/root writer faults; no product or original send"]
fn privileged_original_root_manager_writer_matrix() {
    for case in [
        "retire",
        "guardian-dead",
        "guardian-stopped",
        "outside-alias",
        "trust-drift",
        "duplicate",
        "expiry",
    ] {
        run(case);
    }
}
fn run(case: &str) {
    run_inner(case, None);
}
#[test]
#[ignore = "external local privileged original manager/root creator SIGKILL only; no product or original send"]
fn privileged_original_root_manager_creator_victim() {
    let carrier = creator::Carrier::inherited();
    let case = if carrier.outside {
        "outside-alias"
    } else {
        "creator"
    };
    run_inner(case, Some(carrier));
}
fn run_inner(case: &str, mut carrier: Option<creator::Carrier>) {
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
            if let Some(carrier) = &mut carrier {
                carrier.preparing(&cgroup, &relay, campaigns);
            }
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
                [300_000_000_000, 60_000_000_000, 10_000_000_000]
            );
            let message = frame.prepare_fixed_manager_message().unwrap();
            let mut owned = message.prepare_owned_root_guardians(&mut root).unwrap();
            // Own exact cleanup handles before any possibly uncertain enrollment.
            cgroup.guardians = Some(owned.fixture_guardians());
            let origins = owned.fixture_origins();
            if carrier.is_some() {
                // Copy only the existing comparison's issued/now/boot values.
                // This neither samples another clock nor replaces its decision.
                let (result, report) = crate::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::time_observation::capture(|| owned.enroll_once());
                println!(
                    "HC074-OWNED-ENROLL success={} facts={}",
                    result.is_ok(),
                    serde_json::to_string(&report).unwrap()
                );
                result.unwrap();
            } else {
                owned.enroll_once().unwrap();
            }
            owned.revalidate().unwrap();
            assert_eq!(owned.fixture_origins(), origins);
            let [mut out, mut err, writer, other] = owned.fixture_streams();
            let custodians = owned.fixture_custodians();
            let outside = (case == "outside-alias").then(|| writer.try_clone().unwrap());
            cgroup.writer(
                false,
                writer.try_clone().unwrap(),
                other.try_clone().unwrap(),
            );
            cgroup.writer(
                true,
                writer.try_clone().unwrap(),
                other.try_clone().unwrap(),
            );
            // Distinct duplicated STDOUT/STDERR descriptions in a third writer.
            cgroup.writer(
                true,
                writer.try_clone().unwrap(),
                writer.try_clone().unwrap(),
            );
            drop(writer);
            drop(other);
            cgroup::ready(&mut out, 3);
            for writer in &mut cgroup.writers {
                writer.assert_live();
            }
            assert!(!cgroup::drain(&mut out));
            assert!(!cgroup::drain(&mut err));
            if let Some(carrier) = &carrier {
                let mut rights = vec![out.as_raw_fd(), err.as_raw_fd()];
                rights.extend(
                    cgroup
                        .guardians
                        .as_ref()
                        .unwrap()
                        .iter()
                        .map(AsRawFd::as_raw_fd),
                );
                rights.extend(custodians.iter().map(AsRawFd::as_raw_fd));
                rights.push(relay.creator_pin());
                rights.extend(cgroup.writers.iter().map(cgroup::Dummy::creator_pin));
                // Keep stdin writers in the external harness so creator death
                // cannot make cats exit naturally by EOF instead of revocation.
                rights.extend(cgroup.writers.iter().map(cgroup::Dummy::creator_input));
                if let Some(outside) = &outside {
                    rights.push(outside.as_raw_fd());
                }
                carrier.ready(origins, &rights);
            }
            match case {
                "guardian-dead" => cgroup.signal_guardian(0, libc::SIGKILL),
                "guardian-stopped" => cgroup.signal_guardian(0, libc::SIGSTOP),
                "trust-drift" => {
                    fs::write(
                        worker_root
                            .path()
                            .join("etc/hydracache-perf/worker-trust-074.json"),
                        b"root original drift\n",
                    )
                    .unwrap();
                    assert!(owned.revalidate().is_err());
                }
                "duplicate" => {
                    assert!(owned.enroll_once().is_err());
                }
                "expiry" => {
                    loop {
                        let mut t: libc::timespec = unsafe { std::mem::zeroed() };
                        assert_eq!(
                            unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut t) },
                            0
                        );
                        if t.tv_sec as u64 * 1_000_000_000 + t.tv_nsec as u64 >= origins.1[3] {
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(1));
                    }
                    for writer in &mut cgroup.writers {
                        writer.killed();
                    }
                    assert!(
                        !cgroup::drain(&mut out),
                        "live creator aliases still block EOF"
                    );
                    assert!(owned.revalidate().is_err());
                }
                _ => (),
            }
            // No alias-bearing message or frame is returned to a caller here.
            drop(owned);
            for writer in &mut cgroup.writers {
                writer.killed();
            }
            cgroup.empty();
            for pin in &custodians {
                cgroup::dead(pin);
            }
            assert!(cgroup::drain(&mut err));
            if let Some(outside) = outside {
                assert!(
                    !cgroup::drain(&mut out),
                    "outside alias must block cessation"
                );
                assert_eq!(
                    unsafe { libc::write(outside.as_raw_fd(), b"outside".as_ptr().cast(), 7) },
                    7
                );
                drop(outside);
            }
            assert!(cgroup::drain(&mut out));
            assert!(matches!(
                crate::host_execution::lock_host_root(campaigns),
                Err(crate::host_execution::HostExecutionError::Busy)
            ));
            println!("HC074-OWNED-ROOT case={case} seed=0x074c2024 fixed_manager=true original_owner_consumed=true actual_root=true actual_guardians=true dummy_writers=3 duplicated_writers=true custodians_dead=true drained_eof=true original_bounds=true fence_held=true test_issuers=true original_send=false completion=false product=false");
        },
    );
}
