use super::*;
use crate::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::{OriginalPipeError, PipePreparationStage};

#[test]
#[ignore = "manual nonroot original pipe ownership under acknowledged Starting; no execution"]
fn original_collector_pipes_joined_fault_and_live_refusal() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    let cuts = [
        PipePreparationStage::BeforeCreate,
        PipePreparationStage::StdoutCreated,
        PipePreparationStage::StdoutReady,
        PipePreparationStage::StderrCreated,
        PipePreparationStage::StderrReady,
        PipePreparationStage::Acknowledging,
    ];
    for (index, cut) in cuts.into_iter().enumerate() {
        with_original_at(true, None, |original, _, receiver, sender, model| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let starting = accepted(original, fence, leaf, receiver, sender, model)
                    .publish_starting()
                    .unwrap();
                let raw = fs::read(leaf.join(COLLECTOR_STARTING_NAME)).unwrap();
                let mut visited = Vec::new();
                let failure = starting
                    .prepare_original_pipes_with(|at| {
                        visited.push(at);
                        if at == cut {
                            return Err(OriginalPipeError::Interrupted(at));
                        }
                        Ok(())
                    })
                    .unwrap_err();
                assert!(
                    matches!(failure.error(), OriginalPipeError::Interrupted(at) if *at == cut)
                );
                assert_eq!(visited, cuts[..=index]);
                assert_eq!(
                    failure.held_pipe_count_for_test(),
                    [0, 1, 1, 2, 2, 2][index]
                );
                assert!(failure.is_refused());
                busy(root);
                assert_eq!(fs::read(leaf.join(COLLECTOR_STARTING_NAME)).unwrap(), raw);
                assert_eq!(fs::read_dir(leaf).unwrap().count(), 3);
                drop(failure);
                retained_context_refuses_for_test(root);
                println!("HC074-PIPE-INTERRUPT cut={cut:?} held=true refused=true unchanged=true");
            });
        });
    }
    for kind in 0..8 {
        with_original_at(true, None, |original, _, receiver, sender, model| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let starting = accepted(original, fence, leaf, receiver, sender, model)
                    .publish_starting()
                    .unwrap();
                let mut owner = starting.prepare_original_pipes().unwrap();
                owner.corrupt_for_test(kind);
                let result = owner.revalidate();
                assert!(
                    matches!(result, Err(OriginalPipeError::Invalid)),
                    "{result:?}"
                );
                assert!(owner.is_refused());
                assert!(matches!(
                    owner.revalidate(),
                    Err(OriginalPipeError::Refused)
                ));
                busy(root);
                assert_eq!(fs::read_dir(leaf).unwrap().count(), 3);
                drop(owner);
                retained_context_refuses_for_test(root);
                println!("HC074-PIPE-DRIFT kind={kind} held=true refused=true");
            });
        });
    }
    for concurrent in [false, true] {
        with_original_at(true, None, |original, paths, receiver, sender, model| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let starting = accepted(original, fence, leaf, receiver, sender, model)
                    .publish_starting()
                    .unwrap();
                let mut owner = starting.prepare_original_pipes().unwrap();
                let result = if concurrent {
                    owner.concurrent_drift_for_test()
                } else {
                    fs::write(&paths.worker, b"changed pipe-preparation trust").unwrap();
                    owner.revalidate()
                };
                if concurrent {
                    assert!(
                        matches!(result, Err(OriginalPipeError::Invalid)),
                        "{result:?}"
                    );
                } else {
                    assert!(
                        matches!(result, Err(OriginalPipeError::Original(_))),
                        "{result:?}"
                    );
                }
                assert!(owner.is_refused());
                busy(root);
                drop(owner);
                retained_context_refuses_for_test(root);
                println!("HC074-PIPE-LIVE concurrent={concurrent} held=true refused=true");
            });
        });
    }
    with_original_at(true, None, |original, paths, receiver, sender, model| {
        with_owned_empty_fence_for_test(|fence, root, leaf| {
            let starting = accepted(original, fence, leaf, receiver, sender, model)
                .publish_starting()
                .unwrap();
            let failure = starting
                .prepare_original_pipes_with(|at| {
                    if at == PipePreparationStage::Acknowledging {
                        fs::write(&paths.worker, b"pipe acknowledgement trust drift").unwrap();
                    }
                    Ok(())
                })
                .unwrap_err();
            assert!(matches!(failure.error(), OriginalPipeError::Original(_)));
            assert!(failure.is_refused());
            assert_eq!(failure.held_pipe_count_for_test(), 2);
            busy(root);
            drop(failure);
            retained_context_refuses_for_test(root);
            println!("HC074-PIPE-ACK-DRIFT closing=true held=true refused=true");
        });
    });
    with_original_at(true, None, |original, paths, receiver, sender, model| {
        with_owned_empty_fence_for_test(|fence, root, leaf| {
            let mut starting = accepted(original, fence, leaf, receiver, sender, model)
                .publish_starting()
                .unwrap();
            fs::write(&paths.worker, b"pre-refused original Starting").unwrap();
            assert!(starting.revalidate().is_err());
            let failure = starting.prepare_original_pipes().unwrap_err();
            assert!(failure.is_refused());
            assert_eq!(failure.held_pipe_count_for_test(), 0);
            busy(root);
            drop(failure);
            retained_context_refuses_for_test(root);
            println!("HC074-PIPE-PRE-REFUSED allocated=0 held=true refused=true");
        });
    });
    with_original_at(false, None, |original, _, receiver, sender, model| {
        with_owned_empty_fence_for_test(|fence, root, leaf| {
            let starting = accepted(original, fence, leaf, receiver, sender, model)
                .publish_starting()
                .unwrap();
            let mut owner = starting.prepare_original_pipes().unwrap();
            owner.revalidate().unwrap();
            assert!(!owner.is_refused());
            busy(root);
            owner.drop_eof_for_test();
            retained_context_refuses_for_test(root);
            println!("HC074-PIPE-CONTROL original=true private=true drop_eof=true execution=false");
        });
    });
}
