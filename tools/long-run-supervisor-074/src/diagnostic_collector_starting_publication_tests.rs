use super::*;
use crate::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::{starting_record::*, progress::causal::start_consent::starting::*};
use std::os::unix::fs::{symlink, PermissionsExt};
#[path = "diagnostic_collector_starting_crash_tests.rs"]
mod crash_tests;

#[path = "diagnostic_original_collector_pipes_tests.rs"]
mod pipe_tests;

const STAGES: [CollectorStartingStage; 9] = [
    CollectorStartingStage::BeforeCreate,
    CollectorStartingStage::Created,
    CollectorStartingStage::CreateDirectorySynced,
    CollectorStartingStage::PrefixWritten,
    CollectorStartingStage::BytesWritten,
    CollectorStartingStage::Sealed,
    CollectorStartingStage::FileSynced,
    CollectorStartingStage::DirectorySynced,
    CollectorStartingStage::Acknowledging,
];

// Separate fixture: original packet/response/consent share an actual receiver.
// In a crash child the authority directory is owned by the parent, never adopted.
fn with_original_at(
    refused: bool,
    authority_path: Option<&std::path::Path>,
    run: impl FnOnce(
        &mut OidcControllerInput<'_, '_, '_, '_, '_, '_, '_, '_>,
        &Paths,
        &mut KernelReceiver,
        &OwnedFd,
        &Model,
    ),
) {
    let model = Model::new();
    let r = request(&model);
    let p = policy(&r);
    let t = token(&header(&p), &serde_json::to_vec(&claims(&r, &p)).unwrap());
    let (root, authority_root, mut source, mut authority, worker_bytes) =
        if let Some(path) = authority_path {
            let (root, source, authority, bytes) =
                crate::diagnostic_worker_policy::trusted_input::tests::controller_worker_fixture_at(
                    path,
                );
            (root, None, source, authority, bytes)
        } else {
            let (root, authority_root, source, authority, bytes) = controller_worker_fixture();
            (root, Some(authority_root), source, authority, bytes)
        };
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
        authority: authority_path
            .unwrap_or_else(|| authority_root.as_ref().unwrap().path())
            .into(),
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
}
fn accepted<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>(
    original: &'join mut OidcControllerInput<'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    fence: crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::OriginalEmptyReservationFence<'storage>,
    leaf: &std::path::Path,
    receiver: &mut KernelReceiver,
    sender: &OwnedFd,
    model: &Model,
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
    let challenge = reserved.issue_progress_challenge().unwrap();
    let nonce = crate::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::collector_challenge_nonce(challenge.challenge_to_sign()).unwrap();
    let mut response = request;
    response.operation = DiagnosticOperation::Heartbeat;
    response.expected_state_revision = 1;
    response.request_id = "f".repeat(64);
    response.nonce_sha256 = nonce;
    let causal = challenge
        .accept_response(receive(receiver, sender, &model.sign(response, &model.key)))
        .unwrap();
    let prepared = causal.prepare_framed_consent("1".repeat(64)).unwrap();
    let bytes = consent(prepared.consent_to_sign().clone(), &model.key);
    prepared
        .accept_consent(receive(receiver, sender, &bytes))
        .unwrap()
}

#[test]
#[ignore = "manual nonroot original Starting writes, interruption and live drift; no execution"]
fn collector_starting_publication_boundaries_and_live_refusal() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    // Each injected failure owns an independently fresh original chain.
    for at in STAGES {
        with_original_at(true, None, |original, _, receiver, sender, model| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let framed = accepted(original, fence, leaf, receiver, sender, model);
                let mut visited = Vec::new();
                let failure = framed
                    .publish_starting_with(|cut| {
                        visited.push(cut);
                        busy(root);
                        if cut == at {
                            return Err(CollectorStartingError::Interrupted(cut));
                        }
                        Ok(())
                    })
                    .unwrap_err();
                assert!(
                    matches!(failure.error(), CollectorStartingError::Interrupted(cut) if *cut == at)
                );
                assert_eq!(visited, STAGES[..visited.len()]);
                assert!(failure.is_refused());
                busy(root);
                drop(failure);
                retained_context_refuses_for_test(root);
                println!("HC074-STARTING-INTERRUPT cut={at:?} sticky=true fence_retained=true");
            });
        });
    }
    // External bytes, links, inventory, trust and concurrent writers cannot be adopted.
    for kind in 0..9 {
        with_original_at(true, None, |original, paths, receiver, sender, model| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let framed = accepted(original, fence, leaf, receiver, sender, model);
                let failure = framed
                    .publish_starting_with(|at| {
                        if at == CollectorStartingStage::Created {
                            let path = leaf.join(COLLECTOR_STARTING_NAME);
                            match kind {
                                0 => fs::write(&paths.worker, b"changed trust").unwrap(),
                                1 => fs::write(
                                    paths.authority.join("unexpected-starting-context"),
                                    b"invalid",
                                )
                                .unwrap(),
                                2 => fs::write(&path, b"partial external bytes").unwrap(),
                                3 => fs::set_permissions(&path, fs::Permissions::from_mode(0o400))
                                    .unwrap(),
                                4 => {
                                    fs::remove_file(&path).unwrap();
                                    symlink(RESERVED_NAME, &path).unwrap();
                                }
                                5 => fs::hard_link(&path, leaf.join("extra-link")).unwrap(),
                                6 => fs::rename(&path, leaf.join("renamed-starting")).unwrap(),
                                7 => {
                                    let path = path.clone();
                                    std::thread::spawn(move || {
                                        fs::write(path, b"concurrent drift").unwrap()
                                    })
                                    .join()
                                    .unwrap();
                                }
                                _ => fs::write(leaf.join("unexpected-file"), b"invalid").unwrap(),
                            }
                        }
                        Ok(())
                    })
                    .unwrap_err();
                assert!(failure.is_refused());
                busy(root);
                drop(failure);
                retained_context_refuses_for_test(root);
                println!("HC074-STARTING-DRIFT kind={kind} sticky=true fence_retained=true");
            });
        });
    }
    with_original_at(false, None, |original, _, receiver, sender, model| {
        with_owned_empty_fence_for_test(|fence, root, leaf| {
            let framed = accepted(original, fence, leaf, receiver, sender, model);
            let mut visited = Vec::new();
            let mut owner = framed
                .publish_starting_with(|at| {
                    visited.push(at);
                    Ok(())
                })
                .unwrap();
            assert_eq!(visited, STAGES);
            owner.revalidate().unwrap();
            busy(root);
            let raw = fs::read(leaf.join(COLLECTOR_STARTING_NAME)).unwrap();
            validate_retained_collector_starting(
                &raw,
                &fs::read(leaf.join(NAME)).unwrap(),
                &fs::read(leaf.join(RESERVED_NAME)).unwrap(),
            )
            .unwrap();
            assert_eq!(fs::read_dir(leaf).unwrap().count(), 3);
            drop(owner);
            retained_context_refuses_for_test(root);
            println!(
                "HC074-STARTING-CONTROL acknowledged=true ordered=true fsync=true start=false"
            );
        });
    });
}
