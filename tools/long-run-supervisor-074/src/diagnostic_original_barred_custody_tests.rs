use super::*;
#[path = "diagnostic_original_barred_collector_tests.rs"]
mod collector_tests;
#[path = "diagnostic_original_custody_loss_tests.rs"]
mod loss_tests;
use std::fs::File;
fn mask() -> u64 {
    let mut value = 0u64;
    assert_eq!(
        unsafe {
            libc::syscall(
                libc::SYS_rt_sigprocmask,
                libc::SIG_SETMASK,
                0usize,
                &mut value,
                8usize,
            )
        },
        0
    );
    value
}
fn terminal(file: &File) {
    let mut poll = libc::pollfd {
        fd: file.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    assert_eq!(unsafe { libc::poll(&mut poll, 1, 0) }, 1);
    assert_ne!(poll.revents & libc::POLLIN, 0);
}
fn healthy(samples: usize) {
    for sample in 0..samples {
        original_owner!(false, |held, _paths, root, leaf| {
            let before = records(leaf);
            let origins = held.origins_for_test();
            let parent_mask = mask();
            let (mut barred, report) = trace::capture(|| {
                trace::stage(Stage::Bind);
                held.prepare_barred_fixture_for_test(0, |_| {}).unwrap()
            });
            assert_eq!(mask(), parent_mask);
            call_count(&report, Stage::Bind);
            barred.assert_original_for_test();
            let after = barred.origins_for_test();
            assert_eq!(&after[..4], &origins[..4]);
            assert!(after[4] >= origins[4] && after[5] >= after[4]);
            let (_, report) = trace::capture(|| {
                trace::stage(Stage::Checks);
                barred.revalidate().unwrap();
            });
            call_count(&report, Stage::Checks);
            barred.assert_original_for_test();
            let pinned = barred.pinned_for_test();
            busy(root);
            assert_eq!(records(leaf), before);
            drop(barred);
            terminal(&pinned);
            busy(root);
            drop(held);
            drop(crate::host_execution::lock_host_root(root).unwrap());
            assert_eq!(records(leaf), before);
            retained_context_refuses_for_test(root);
            println!("HC074-BARRED-CUSTODY-CONTROL sample={sample} create_ipc=34 validate_ipc=34 inherited=3 mask_exact=true unchanged=true start=false");
        });
    }
}
#[test]
#[ignore = "manual nonroot original autonomous barred custody healthy screen; no execution"]
fn original_barred_custody_screen() {
    healthy(1);
}
#[test]
#[ignore = "manual nonroot original autonomous barred custody repeated healthy controls; no execution"]
fn original_barred_custody_healthy() {
    healthy(3);
}
#[test]
#[ignore = "manual nonroot original barred custody live drift, creation errors and expiry; no execution"]
fn original_barred_custody_refusals() {
    let seed = 0x0740_bc18_u64;
    for case in [
        "clone-error",
        "restore-error",
        "trust",
        "output",
        "message",
        "custody-flags",
        "child-kill",
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
                .prepare_barred_fixture_for_test(fault, |barred| match case {
                    "trust" => fs::write(&paths.worker, b"barred trust drift").unwrap(),
                    "output" => fs::write(
                        output(leaf).join("collector-attempt-v1/collector-stderr.bin"),
                        b"unexpected",
                    )
                    .unwrap(),
                    "child-kill" => barred.kill_for_test(),
                    other => barred.tamper_for_test(other),
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
                "child-kill" => assert_eq!(reason, "Child"),
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
            println!("HC074-BARRED-CUSTODY-REFUSAL seed=0x{seed:08x} case={case} reason={reason} mask_exact=true sticky=true unchanged=true start=false");
        });
    }
}
#[test]
#[ignore = "manual nonroot original autonomous expiry, peer loss and exact custodian SIGKILL; no execution"]
fn original_barred_custody_process_matrix() {
    let seed = 0x0740_bd18_u64;
    for repeat in 1..=3 {
        for case in ["parent-alias-close", "autonomous-expiry", "peer-lost"] {
            original_owner!(false, |held, _paths, root, leaf| {
                let before = records(leaf);
                let barred = held.prepare_barred_fixture_for_test(0, |_| {}).unwrap();
                barred.assert_original_for_test();
                let fds = barred.inherited_for_test();
                let mut kernel = barred.detach_for_test();
                if case == "peer-lost" {
                    kernel.disconnect_for_test();
                }
                drop(held);
                for fd in fds {
                    assert_eq!(unsafe { libc::fcntl(fd, libc::F_GETFD) }, -1);
                }
                busy(root);
                if case == "autonomous-expiry" {
                    kernel.wait_expired_for_test();
                }
                kernel.assert_retained_for_test();
                busy(root);
                kernel.finish_for_test();
                drop(crate::host_execution::lock_host_root(root).unwrap());
                assert_eq!(records(leaf), before);
                retained_context_refuses_for_test(root);
                println!("HC074-BARRED-CUSTODY-PROCESS seed=0x{seed:08x} repeat={repeat} case={case} inherited=3 retained=true signal=9 unchanged=true start=false");
            });
        }
    }
    println!("HC074-BARRED-CUSTODY-PROCESS completed=true cases=9 product=false");
}
