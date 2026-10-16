use super::*;
#[path = "diagnostic_collector_causal_tests.rs"]
mod causal_tests;
#[path = "diagnostic_validation_cost_tests.rs"]
mod validation_cost_tests;
use crate::diagnostic_ipc::DiagnosticOperation;
use crate::diagnostic_kernel_admission::{KernelPacket, KernelReceiver};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

// Separate original-message fixture; no change to prior crash proofs or constructors.
fn pair() -> (KernelReceiver, OwnedFd) {
    let mut fds = [-1; 2];
    assert_eq!(
        unsafe {
            libc::socketpair(
                libc::AF_UNIX,
                libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC,
                0,
                fds.as_mut_ptr(),
            )
        },
        0
    );
    (
        KernelReceiver::new(unsafe { OwnedFd::from_raw_fd(fds[0]) }).unwrap(),
        unsafe { OwnedFd::from_raw_fd(fds[1]) },
    )
}
fn receive(receiver: &mut KernelReceiver, sender: &OwnedFd, bytes: &[u8]) -> KernelPacket {
    assert_eq!(
        unsafe {
            libc::send(
                sender.as_raw_fd(),
                bytes.as_ptr().cast(),
                bytes.len(),
                libc::MSG_NOSIGNAL,
            )
        },
        bytes.len() as isize
    );
    receiver.try_receive().unwrap().unwrap()
}
fn with_progress_original(
    refused: bool,
    run: impl FnOnce(
        &mut OidcControllerInput<'_, '_, '_, '_, '_, '_, '_, '_>,
        &Paths,
        &mut KernelReceiver,
        &OwnedFd,
        &Model,
    ),
) {
    let (_, event) = crate::diagnostic_kernel_admission::refusal_trace::capture(|| {
        let model = Model::new();
        let r = request(&model);
        let p = policy(&r);
        let t = token(&header(&p), &serde_json::to_vec(&claims(&r, &p)).unwrap());
        let (root, authority_root, mut source, mut authority, worker_bytes) =
            controller_worker_fixture();
        let (mut receiver, sender) = pair();
        let mut packet = receive(&mut receiver, &sender, &model.sign(r.clone(), &model.key));
        let mut worker = source
            .verify_kernel_policy(&worker_bytes, &mut packet, &mut authority)
            .unwrap();
        let mut controller = model.input();
        let mut oidc = TrustModel::new(&r);
        let paths = Paths {
            oidc: oidc.path(),
            worker: root
                .path()
                .join("etc/hydracache-perf/worker-trust-074.json"),
            authority: authority_root.path().into(),
        };
        {
            let mut original = controller.authenticate_controller(&mut worker).unwrap();
            let mut build = original
                .bind_reviewed_build(BUILDER_POLICY, RECEIPT)
                .unwrap();
            let mut guard = build.bind_oidc(&mut oidc.trust, &t).unwrap();
            run(&mut guard, &paths, &mut receiver, &sender, &model);
            assert_eq!(guard.is_refused(), refused);
        }
        assert_eq!(oidc.trust.is_refused(), refused);
        assert_eq!(controller.is_refused(), refused);
        assert_eq!(worker.is_refused(), refused);
        drop(worker);
        assert_eq!(source.is_refused(), refused);
        assert_eq!(authority.is_failed(), refused);
        assert_eq!(packet.is_refused(), refused);
    });
    // No generic Invalid is expected in the full positive/composition cases below.
    assert!(event.is_none(), "unexpected kernel refusal: {event:?}");
}

#[test]
fn collector_progress_retains_both_owners_without_mutation_and_cross_refuses() {
    use crate::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::CollectorProgressError;
    // Healthy, authenticated wrong controller, configured-source drift, pre-refused owner.
    for case in 0..4 {
        with_progress_original(true, |original, paths, receiver, sender, model| {
            let request = original.original.original.request.clone();
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let attempt = original
                    .bind_common_empty_fence(fence)
                    .unwrap()
                    .retain_request()
                    .unwrap();
                let retained = fs::read(leaf.join(NAME)).unwrap();
                let mut observed = attempt.into_observed_reservation();
                let state = observed.state_to_sign().unwrap();
                let mut read = observed
                    .publish_reserved(&signed(&body(&request, &state, &retained)))
                    .unwrap();
                let raw = fs::read(leaf.join(RESERVED_NAME)).unwrap();
                let mut next = request.clone();
                next.operation = DiagnosticOperation::Heartbeat;
                next.expected_state_revision = 1;
                next.request_id = "f".repeat(64);
                next.nonce_sha256 = "0".repeat(64);
                if case == 1 {
                    next.controller.run_id += 1;
                }
                let packet = receive(receiver, sender, &model.sign(next, &model.key));
                if case == 2 {
                    let saved = fs::read(&paths.worker).unwrap();
                    fs::write(&paths.worker, &saved).unwrap();
                }
                if case == 3 {
                    fs::remove_file(leaf.join(RESERVED_NAME)).unwrap();
                    assert!(read.revalidate().is_err());
                }
                let result = read.bind_authenticated_progress(packet);
                if case == 0 {
                    let mut progress = result.unwrap();
                    assert!(!progress.is_refused());
                    assert!(!format!("{progress:?}").contains("request_id"));
                    assert_eq!(fs::read(leaf.join(NAME)).unwrap(), retained);
                    assert_eq!(fs::read(leaf.join(RESERVED_NAME)).unwrap(), raw);
                    assert_eq!(fs::read_dir(leaf).unwrap().count(), 2);
                    busy(root);
                    // A later owned-object mismatch must latch every retained original input.
                    fs::remove_file(leaf.join(RESERVED_NAME)).unwrap();
                    assert!(matches!(
                        progress.revalidate(),
                        Err(CollectorProgressError::Reservation(
                            CollectorReservedError::Storage(_)
                        ))
                    ));
                    assert!(progress.is_refused());
                    assert!(matches!(
                        progress.revalidate(),
                        Err(CollectorProgressError::Refused)
                    ));
                    busy(root);
                    drop(progress);
                } else {
                    let failure = result.unwrap_err();
                    if case == 1 {
                        assert!(matches!(failure.error(), CollectorProgressError::Binding));
                    }
                    if case == 3 {
                        assert!(matches!(failure.error(), CollectorProgressError::Refused));
                    }
                    assert!(failure.is_refused());
                    assert_eq!(fs::read(leaf.join(NAME)).unwrap(), retained);
                    if case != 3 {
                        assert_eq!(fs::read(leaf.join(RESERVED_NAME)).unwrap(), raw);
                    }
                    busy(root);
                    drop(failure);
                }
                drop(crate::host_execution::lock_host_root(root).unwrap());
                retained_context_refuses_for_test(root);
                println!("collector_progress composition_case={case} preflight_writes=false start=false fence_retained=true");
            });
        });
    }
}

#[test]
fn collector_progress_api_shape() {
    fn probe(
        owner: OriginalCollectorReservation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
        packet: crate::diagnostic_kernel_admission::KernelPacket,
    ) {
        let _ = owner.bind_authenticated_progress(packet);
    }
    let _ = probe;
}
