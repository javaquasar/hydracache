use super::*;
#[path = "diagnostic_original_descriptor_holder_tests.rs"]
mod holder;

macro_rules! original_owner {
    ($drift:expr, |$held:ident, $paths:ident, $root:ident, $leaf:ident| $body:block) => {{
        let bundle = Bundle::new();
        with_original_at($drift, None, |original, $paths, receiver, sender, model| {
            with_owned_empty_fence_for_test(|fence, $root, $leaf| {
                let mut $held = accepted(original, fence, $leaf, receiver, sender, model)
                    .prepare_original_outputs_for_test(|_| Ok(()))
                    .unwrap()
                    .prepare_custodied_fixture_for_test(&bundle.path, |_, _| {})
                    .unwrap();
                $body
            });
        });
    }};
}
pub(super) use original_owner;
fn call_count(report: &trace::Report, stage: Stage) {
    assert_eq!(report.stages[stage as usize].authentication_calls, 34);
    assert_eq!(report.stages[stage as usize].signature_calls, 34);
    assert!(report.first_time.is_none());
}
fn healthy(samples: usize) {
    for sample in 0..samples {
        original_owner!(false, |held, _paths, root, leaf| {
            let before = records(leaf);
            let origins = held.origins_for_test();
            let (mut plan, report) = trace::capture(|| {
                trace::stage(Stage::Bind);
                held.plan_descriptors_fixture_for_test(|_| {}).unwrap()
            });
            call_count(&report, Stage::Bind);
            let (guardian, collector, writers, after) = plan.layout_for_test();
            assert_eq!(&after[..4], &origins[..4]);
            assert!(after[4] >= origins[4] && after[5] >= after[4]);
            assert!(writers
                .iter()
                .all(|fd| !guardian.contains(fd) && !collector.contains(fd)));
            assert!(collector.iter().all(|fd| guardian.contains(fd)));
            let (_, fds) = plan.inherited_for_test();
            for fd in &fds[4..6] {
                assert_eq!(
                    unsafe { libc::fcntl(*fd, libc::F_GETFL) } & libc::O_ACCMODE,
                    libc::O_RDWR
                );
                assert_eq!(unsafe { libc::lseek(*fd, 0, libc::SEEK_CUR) }, 0);
            }
            let (_, report) = trace::capture(|| {
                trace::stage(Stage::Checks);
                plan.revalidate_for_test().unwrap();
            });
            call_count(&report, Stage::Checks);
            let reobserved = plan.layout_for_test().3;
            assert_eq!(&reobserved[..4], &origins[..4]);
            assert!(reobserved[4] >= after[4] && reobserved[5] >= reobserved[4]);
            assert!(!held.is_refused());
            busy(root);
            assert_eq!(records(leaf), before);
            drop(held);
            retained_context_refuses_for_test(root);
            println!("HC074-ORIGINAL-DESCRIPTORS-CONTROL sample={sample} plan_ipc=34 validate_ipc=34 guardian=6 collector=4 excluded_writers=4 unchanged=true send=false");
        });
    }
}
#[test]
#[ignore = "manual nonroot original descriptor full-frame healthy screen; no execution"]
fn original_descriptor_plan_screen() {
    healthy(1);
}
#[test]
#[ignore = "manual nonroot original descriptor repeated full-frame controls; no execution"]
fn original_descriptor_plan_healthy() {
    healthy(3);
}

#[test]
#[ignore = "manual nonroot original descriptor draft drift and sticky refusal; no execution"]
fn original_descriptor_plan_refusals() {
    let seed = 0x0740_d018_u64;
    for case in [
        "trust",
        "output",
        "writer",
        "custody-flags",
        "message",
        "alarm",
        "preexisting-offset",
    ] {
        original_owner!(true, |held, paths, root, leaf| {
            let before = records(leaf);
            if case == "preexisting-offset" {
                held.shift_original_sink_for_test();
            }
            let result = held.plan_descriptors_fixture_for_test(|owner| match case {
                "preexisting-offset" => (),
                "trust" => fs::write(&paths.worker, b"descriptor trust drift").unwrap(),
                "output" => fs::write(
                    output(leaf).join("collector-attempt-v1/collector-stderr.bin"),
                    b"unexpected",
                )
                .unwrap(),
                other => owner.tamper_for_test(other),
            });
            let error = match result {
                Ok(_) => panic!("draft drift accepted"),
                Err(error) => error,
            };
            let reason = format!("{error:?}");
            assert!(
                !reason.contains("Time"),
                "late time is not {case}: {reason}"
            );
            match case {
                "trust" => assert!(reason.contains("Input(Invalid)"), "{reason}"),
                "output" => assert!(reason.contains("Outputs(Invalid)"), "{reason}"),
                "writer" => assert!(reason.contains("Invalid"), "{reason}"),
                "custody-flags" => assert_eq!(reason, "Storage(Document)"),
                _ => assert!(reason.contains("Binding"), "{reason}"),
            }
            assert!(held.is_refused());
            assert!(matches!(
                held.plan_descriptors_fixture_for_test(|_| {}),
                Err(OriginalCustodyError::Refused)
            ));
            busy(root);
            assert_eq!(records(leaf), before);
            drop(held);
            retained_context_refuses_for_test(root);
            println!("HC074-ORIGINAL-DESCRIPTORS-REFUSAL seed=0x{seed:08x} case={case} sticky=true held=true unchanged=true send=false");
        });
    }
}
#[test]
#[ignore = "manual nonroot original descriptor later drift and final absolute expiry; no execution"]
fn original_descriptor_plan_later_refusals() {
    for case in ["trust", "custody-object", "final-expiry"] {
        original_owner!(true, |held, paths, root, leaf| {
            let before = records(leaf);
            let mut plan = held.plan_descriptors_fixture_for_test(|_| {}).unwrap();
            if case == "trust" {
                fs::write(&paths.worker, b"later descriptor trust drift").unwrap();
            } else {
                plan.tamper_for_test(case);
            }
            let reason = format!("{:?}", plan.revalidate_for_test().unwrap_err());
            assert!(!reason.contains("Time"), "{reason}");
            match case {
                "trust" => assert!(reason.contains("Input(Invalid)"), "{reason}"),
                "custody-object" => assert_eq!(reason, "Storage(Document)"),
                _ => assert_eq!(reason, "Original(Expired)"),
            }
            assert!(matches!(
                plan.revalidate_for_test(),
                Err(OriginalCustodyError::Refused)
            ));
            assert!(held.is_refused());
            busy(root);
            assert_eq!(records(leaf), before);
            drop(held);
            retained_context_refuses_for_test(root);
            println!("HC074-ORIGINAL-DESCRIPTORS-LATER case={case} sticky=true held=true unchanged=true send=false");
        });
    }
}

#[test]
#[ignore = "manual nonroot original descriptor allowlist fork, actual EOF and holder SIGKILL; no execution"]
fn original_descriptor_holder_matrix() {
    let seed = 0x0740_d118_u64;
    for repeat in 1..=3 {
        for case in ["normal", "ack-lost", "holder-kill", "hidden-writer"] {
            original_owner!(false, |held, _paths, root, leaf| {
                let before = records(leaf);
                let plan = held.plan_descriptors_fixture_for_test(|_| {}).unwrap();
                let (keep, fds) = plan.inherited_for_test();
                let hidden = (case == "hidden-writer").then(|| plan.hidden_writer_for_test());
                let mut child = holder::Holder::spawn(keep, fds, case != "ack-lost");
                busy(root);
                drop(held);
                for fd in fds {
                    assert_eq!(unsafe { libc::fcntl(fd, libc::F_GETFD) }, -1);
                }
                busy(root);
                let eof = if case == "holder-kill" {
                    false
                } else {
                    child.observe_eof(case == "hidden-writer")
                };
                if case == "hidden-writer" {
                    drop(hidden);
                    child.assert_retained();
                }
                busy(root);
                child.finish(case == "holder-kill" || case == "hidden-writer");
                drop(crate::host_execution::lock_host_root(root).unwrap());
                assert_eq!(records(leaf), before);
                retained_context_refuses_for_test(root);
                println!("HC074-ORIGINAL-DESCRIPTOR-HOLDER seed=0x{seed:08x} repeat={repeat} case={case} eof={eof} retained=true unchanged=true send=false");
            });
        }
    }
    println!("HC074-ORIGINAL-DESCRIPTOR-HOLDER completed=true cases=12 product=false");
}
