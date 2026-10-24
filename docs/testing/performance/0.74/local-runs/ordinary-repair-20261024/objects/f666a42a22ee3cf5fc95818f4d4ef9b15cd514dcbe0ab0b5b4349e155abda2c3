//! Effect-local interruption, concurrent live drift and unchanged real expiry.
use super::*;

#[test]
#[ignore = "manual nonroot J1 fresh-chain exact expiry-site attribution; no execution"]
fn original_output_join_expiry_site_attribution() {
    require_original_progress_expiry();
}

fn require_original_progress_expiry() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    with_original_at(true, None, |original, _, receiver, sender, model| {
        with_owned_empty_fence_for_test(|fence, root, leaf| {
            let framed = accepted(original, fence, leaf, receiver, sender, model);
            let mut waited = false;
            let (result, event) =
                crate::diagnostic_kernel_admission::refusal_trace::capture(|| {
                    crate::diagnostic_kernel_admission::refusal_trace::phase("output-join-expiry");
                    framed.prepare_original_outputs_for_test(|at| {
                        if at == OutputJoinStage::Starting(CollectorStartingStage::BeforeCreate) {
                            std::thread::sleep(std::time::Duration::from_secs(
                                crate::diagnostic_lease::CONTROLLER_LOSS_SECONDS,
                            ));
                            waited = true;
                        }
                        Ok(())
                    })
                });
            let failure = result.unwrap_err();
            assert!(waited);
            assert_eq!(
                format!("{:?}", failure.error()),
                "Original(Original(Original(Causal(Progress(Input(Kernel(Invalid)))))))"
            );
            let event = event.expect("exact original expiry site required");
            assert_eq!(event.site, "progress-age");
            assert_eq!(event.phase, "output-join-expiry");
            assert!(event.file.ends_with("diagnostic_kernel_progress.rs"));
            let [received, after, now, zero_a, zero_b, limit] = event.clock;
            assert!(received > after && now >= received);
            assert_eq!((zero_a, zero_b), (0, 0));
            assert_eq!(
                limit,
                crate::diagnostic_lease::CONTROLLER_LOSS_SECONDS * 1_000_000_000
            );
            assert!(now - received >= limit);
            assert!(failure.is_refused());
            assert_eq!(fs::read_dir(leaf).unwrap().count(), 2);
            assert!(!output(leaf).exists());
            busy(root);
            drop(failure);
            retained_context_refuses_for_test(root);
            println!("HC074-OUTPUT-JOIN-EXPIRY-ATTRIBUTION event={event:?} exact=true renewed=false created=false");
        });
    });
}

#[test]
#[ignore = "manual nonroot J1 Starting/pipe interruption and every-effect live drift"]
fn original_output_join_all_effects_live_refusal() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    let cuts = joined_cuts();
    assert_eq!(cuts.len(), 25);
    for cut in cuts.iter().copied().take(15) {
        with_original_at(true, None, |original, _, receiver, sender, model| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let framed = accepted(original, fence, leaf, receiver, sender, model);
                let request = fs::read(leaf.join(NAME)).unwrap();
                let reserved = fs::read(leaf.join(RESERVED_NAME)).unwrap();
                let mut visited = Vec::new();
                let failure = framed
                    .prepare_original_outputs_for_test(|at| {
                        visited.push(at);
                        if at == cut {
                            return Err(OriginalSinkError::JoinedInterrupted(at));
                        }
                        Ok(())
                    })
                    .unwrap_err();
                assert!(
                    matches!(failure.error(), OriginalSinkError::JoinedInterrupted(at) if *at == cut),
                    "{failure:?}"
                );
                assert_eq!(visited, cuts[..visited.len()]);
                assert!(failure.is_refused());
                assert!(!output(leaf).exists());
                assert_eq!(fs::read(leaf.join(NAME)).unwrap(), request);
                assert_eq!(fs::read(leaf.join(RESERVED_NAME)).unwrap(), reserved);
                let created =
                    cut != OutputJoinStage::Starting(CollectorStartingStage::BeforeCreate);
                assert_eq!(
                    fs::read_dir(leaf).unwrap().count(),
                    2 + usize::from(created)
                );
                busy(root);
                drop(failure);
                retained_context_refuses_for_test(root);
                println!("HC074-OUTPUT-JOIN-INTERRUPT cut={cut:?} sticky=true retained=true no_output=true");
            });
        });
    }
    let mut order = cuts;
    let mut seed = 0x0740_51e1_u64;
    for i in (1..order.len()).rev() {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        order.swap(i, seed as usize % (i + 1));
    }
    for (index, cut) in order.into_iter().enumerate() {
        with_original_at(true, None, |original, paths, receiver, sender, model| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let framed = accepted(original, fence, leaf, receiver, sender, model);
                let mut injected = false;
                let failure = framed
                    .prepare_original_outputs_for_test(|at| {
                        if at == cut {
                            let path = paths.worker.clone();
                            if index % 2 == 0 {
                                std::thread::spawn(move || {
                                    fs::write(path, b"concurrent original trust drift").unwrap()
                                })
                                .join()
                                .unwrap();
                            } else {
                                fs::write(path, b"original trust drift").unwrap();
                            }
                            injected = true;
                        }
                        Ok(())
                    })
                    .unwrap_err();
                assert!(injected, "{cut:?} {failure:?}");
                assert!(
                    matches!(failure.error(), OriginalSinkError::Original(_)),
                    "{cut:?} {failure:?}"
                );
                let reason = format!("{:?}", failure.error());
                assert!(
                    reason.contains("Input(Invalid)")
                        && !reason.contains("Kernel(")
                        && !reason.contains("Time"),
                    "{cut:?} {reason}"
                );
                assert!(failure.is_refused());
                busy(root);
                drop(failure);
                retained_context_refuses_for_test(root);
                println!("HC074-OUTPUT-JOIN-LIVE seed=0x074051e1 cut={cut:?} concurrent={} exact_invalid=true closing=true retained=true", index % 2 == 0);
            });
        });
    }
    require_original_progress_expiry();
    println!("HC074-OUTPUT-JOIN-EXPIRY original_progress=true site=progress-age renewed=false created=false");
}
