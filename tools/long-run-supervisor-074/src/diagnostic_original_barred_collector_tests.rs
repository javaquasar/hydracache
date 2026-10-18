use super::*;

fn healthy(samples: usize) {
    for sample in 0..samples {
        original_owner!(false, |held, _paths, root, leaf| {
            let before = records(leaf);
            let origins = held.origins_for_test();
            let parent_mask = mask();
            let (mut joined, report) = trace::capture(|| {
                trace::stage(Stage::Bind);
                held.prepare_collector_fixture_for_test(0, |_| {}).unwrap()
            });
            call_count(&report, Stage::Bind);
            assert_eq!(mask(), parent_mask);
            joined.assert_original_for_test();
            assert_eq!(&joined.origins_for_test()[..4], &origins[..4]);
            let (_, report) = trace::capture(|| {
                trace::stage(Stage::Checks);
                joined.revalidate().unwrap();
            });
            call_count(&report, Stage::Checks);
            joined.assert_original_for_test();
            assert_eq!(&joined.origins_for_test()[..4], &origins[..4]);
            let pins = joined.pinned_for_test();
            busy(root);
            assert_eq!(records(leaf), before);
            drop(joined);
            for pinned in pins {
                terminal(&pinned);
            }
            busy(root);
            drop(held);
            drop(crate::host_execution::lock_host_root(root).unwrap());
            assert_eq!(records(leaf), before);
            retained_context_refuses_for_test(root);
            println!("HC074-BARRED-COLLECTOR-CONTROL sample={sample} create_ipc=34 validate_ipc=34 collector_fds=7 custody_fds=3 mask_exact=true unchanged=true start=false");
        });
    }
}
#[test]
#[ignore = "manual nonroot barred original collector healthy screen; no execution"]
fn original_barred_collector_screen() {
    healthy(1);
}
#[test]
#[ignore = "manual nonroot barred original collector repeated healthy controls; no execution"]
fn original_barred_collector_healthy() {
    healthy(3);
}
#[test]
#[ignore = "manual nonroot barred original collector live refusal matrix; no execution"]
fn original_barred_collector_refusals() {
    let seed = 0x0740_c118_u64;
    for case in [
        "clone-error",
        "restore-error",
        "trust",
        "output",
        "message",
        "custody-flags",
        "collector-kill",
        "custodian-kill",
        "final-expiry",
    ] {
        original_owner!(true, |held, paths, root, leaf| {
            let before = records(leaf);
            let parent_mask = mask();
            let fault = match case {
                "clone-error" => 1,
                "restore-error" => 2,
                _ => 0,
            };
            let mut failure = held
                .prepare_collector_fixture_for_test(fault, |joined| match case {
                    "trust" => fs::write(&paths.worker, b"barred collector trust drift").unwrap(),
                    "output" => fs::write(
                        output(leaf).join("collector-attempt-v1/collector-stderr.bin"),
                        b"unexpected",
                    )
                    .unwrap(),
                    other => joined.tamper_for_test(other),
                })
                .unwrap_err();
            assert_eq!(mask(), parent_mask);
            let reason = failure.reason_for_test();
            assert!(
                !reason.contains("Time"),
                "late time is not {case}: {reason}"
            );
            match case {
                "clone-error" => assert!(reason.contains("code: 11"), "{reason}"),
                "restore-error" => assert!(reason.contains("code: 5"), "{reason}"),
                "trust" => assert!(reason.contains("Input(Invalid)"), "{reason}"),
                "output" => assert!(reason.contains("Outputs(Invalid)"), "{reason}"),
                "message" => assert!(reason.contains("Binding"), "{reason}"),
                "custody-flags" => assert!(reason.contains("Storage(Document)"), "{reason}"),
                "collector-kill" | "custodian-kill" => assert_eq!(reason, "Child"),
                _ => assert_eq!(reason, "Original(Original(Expired))"),
            }
            assert!(failure.refused_for_test());
            assert_eq!(
                format!("{:?}", failure.retry_for_test().unwrap_err()),
                "Refused"
            );
            busy(root);
            drop(failure);
            busy(root);
            drop(held);
            drop(crate::host_execution::lock_host_root(root).unwrap());
            assert_eq!(records(leaf), before);
            retained_context_refuses_for_test(root);
            println!("HC074-BARRED-COLLECTOR-REFUSAL seed=0x{seed:08x} case={case} reason={reason} mask_exact=true sticky=true unchanged=true start=false");
        });
    }
}
#[test]
#[ignore = "manual nonroot barred original collector quarantine and exact SIGKILL matrix; no execution"]
fn original_barred_collector_process_matrix() {
    let seed = 0x0740_c218_u64;
    for repeat in 1..=3 {
        for case in ["expiry", "peer-activity", "peer-loss"] {
            original_owner!(false, |held, _paths, root, leaf| {
                let before = records(leaf);
                let joined = held.prepare_collector_fixture_for_test(0, |_| {}).unwrap();
                joined.assert_original_for_test();
                let mut children = joined.detach_for_test();
                drop(held);
                busy(root);
                children.quarantine_for_test(case);
                busy(root);
                children.kill_custody_for_test();
                busy(root);
                children.finish_for_test();
                drop(crate::host_execution::lock_host_root(root).unwrap());
                assert_eq!(records(leaf), before);
                retained_context_refuses_for_test(root);
                println!("HC074-BARRED-COLLECTOR-PROCESS seed=0x{seed:08x} repeat={repeat} case={case} collector_fds=3 io_closed=4 custody_killed_first=true fence_retained=true unchanged=true start=false");
            });
        }
    }
}
