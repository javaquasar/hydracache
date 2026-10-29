//! Ordering proof only: the unchanged holder is not a production guardian.
use super::super::*;
use super::{readable, Holder};
use std::os::fd::RawFd;

fn barred(child: &Holder) {
    child.assert_retained();
    // The structural B and optional A were consumed by spawn. No EOF command
    // has been sent: the unchanged raw continuation cannot read or sync sinks.
    assert_eq!(readable(&child.control, 0), 0);
}
fn aliases_closed(fds: [RawFd; 10]) {
    for fd in fds {
        assert_eq!(unsafe { libc::fcntl(fd, libc::F_GETFD) }, -1);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::EBADF)
        );
    }
}
fn healthy(samples: usize, missing_ack: bool) {
    let seed = 0x0740_b018_u64;
    for sample in 0..samples {
        for case in if missing_ack {
            &["normal", "ack-lost"][..]
        } else {
            &["normal"][..]
        } {
            original_owner!(false, |held, _paths, root, leaf| {
                let before = records(leaf);
                let mut plan = held.plan_descriptors_fixture_for_test(|_| {}).unwrap();
                let (keep, fds) = plan.inherited_for_test();
                let origins = plan.layout_for_test().3;
                let mut child = Holder::spawn(keep, fds, *case != "ack-lost");
                barred(&child);
                let (_, report) = trace::capture(|| {
                    trace::stage(Stage::Checks);
                    plan.revalidate_for_test().unwrap();
                });
                call_count(&report, Stage::Checks);
                let after = plan.layout_for_test().3;
                assert_eq!(&after[..4], &origins[..4]);
                assert!(after[4] >= origins[4] && after[5] >= after[4]);
                barred(&child);
                busy(root);
                assert_eq!(records(leaf), before);
                drop(held);
                aliases_closed(fds);
                busy(root);
                assert!(child.observe_eof(false));
                busy(root);
                child.finish(false);
                drop(crate::host_execution::lock_host_root(root).unwrap());
                assert_eq!(records(leaf), before);
                retained_context_refuses_for_test(root);
                println!("HC074-ORIGINAL-BARRIER-CONTROL seed=0x{seed:08x} sample={sample} case={case} frame_ipc=34 barred_during_frame=true eof=true unchanged=true send=false");
            });
        }
    }
}
#[test]
#[ignore = "manual nonroot post-fork original full-frame screen; no execution"]
fn original_barrier_validation_screen() {
    healthy(1, false);
}
#[test]
#[ignore = "manual nonroot repeated post-fork full frames and missing private ACK; no execution"]
fn original_barrier_validation_healthy() {
    healthy(3, true);
}
#[test]
#[ignore = "manual nonroot post-fork persistent drift and original expiry; no execution"]
fn original_barrier_validation_refusals() {
    let seed = 0x0740_b118_u64;
    for case in [
        "trust",
        "output",
        "writer",
        "message",
        "alarm",
        "custody-flags",
        "custody-object",
        "sink-offset",
        "final-expiry",
    ] {
        original_owner!(true, |held, paths, root, leaf| {
            let before = records(leaf);
            let mut plan = held.plan_descriptors_fixture_for_test(|_| {}).unwrap();
            let (keep, fds) = plan.inherited_for_test();
            let mut child = Holder::spawn(keep, fds, true);
            barred(&child);
            match case {
                "trust" => fs::write(&paths.worker, b"post-fork trust drift").unwrap(),
                "output" => fs::write(
                    output(leaf).join("collector-attempt-v1/collector-stderr.bin"),
                    b"unexpected",
                )
                .unwrap(),
                "sink-offset" => {
                    assert_eq!(unsafe { libc::lseek(fds[4], 17, libc::SEEK_SET) }, 17);
                }
                other => plan.tamper_for_test(other),
            }
            let reason = format!("{:?}", plan.revalidate_for_test().unwrap_err());
            assert!(
                !reason.contains("Time"),
                "late time is not {case}: {reason}"
            );
            match case {
                "trust" => assert!(reason.contains("Input(Invalid)"), "{reason}"),
                "output" => assert!(reason.contains("Outputs(Invalid)"), "{reason}"),
                "writer" => assert!(reason.contains("Invalid"), "{reason}"),
                "custody-flags" | "custody-object" => assert_eq!(reason, "Storage(Document)"),
                "final-expiry" => assert_eq!(reason, "Original(Expired)"),
                _ => assert!(reason.contains("Binding"), "{reason}"),
            }
            assert!(matches!(
                plan.revalidate_for_test(),
                Err(OriginalCustodyError::Refused)
            ));
            assert!(held.is_refused());
            barred(&child);
            busy(root);
            drop(held);
            aliases_closed(fds);
            barred(&child);
            busy(root);
            // No C/EOF command on any refusal, including after parent death.
            child.finish(true);
            drop(crate::host_execution::lock_host_root(root).unwrap());
            assert_eq!(records(leaf), before);
            retained_context_refuses_for_test(root);
            println!("HC074-ORIGINAL-BARRIER-REFUSAL seed=0x{seed:08x} case={case} reason={reason} sticky=true command=false retained=true unchanged=true send=false");
        });
    }
}
#[test]
#[ignore = "manual nonroot real holder SIGKILL around post-fork validation; no execution"]
fn original_barrier_validation_sigkill() {
    let seed = 0x0740_b218_u64;
    for repeat in 1..=3 {
        for cut in [
            "before-frame",
            "after-frame",
            "after-alias-close",
            "after-eof",
        ] {
            original_owner!(false, |held, _paths, root, leaf| {
                let before = records(leaf);
                let mut plan = held.plan_descriptors_fixture_for_test(|_| {}).unwrap();
                let (keep, fds) = plan.inherited_for_test();
                let mut child = Holder::spawn(keep, fds, true);
                barred(&child);
                let frame_ipc = if cut == "before-frame" {
                    0
                } else {
                    let (_, report) = trace::capture(|| {
                        trace::stage(Stage::Checks);
                        plan.revalidate_for_test().unwrap();
                    });
                    call_count(&report, Stage::Checks);
                    barred(&child);
                    34
                };
                let parent_retained = matches!(cut, "before-frame" | "after-frame");
                if parent_retained {
                    child.finish(true);
                    assert_eq!(readable(child.pidfd.as_ref().unwrap(), 0), 1);
                    // A formerly live frame cannot repair this terminal child.
                    busy(root);
                    assert_eq!(records(leaf), before);
                    drop(held);
                    aliases_closed(fds);
                } else {
                    drop(held);
                    aliases_closed(fds);
                    busy(root);
                    if cut == "after-eof" {
                        assert!(child.observe_eof(false));
                        busy(root);
                    } else {
                        barred(&child);
                    }
                    child.finish(true);
                    assert_eq!(readable(child.pidfd.as_ref().unwrap(), 0), 1);
                }
                drop(crate::host_execution::lock_host_root(root).unwrap());
                assert_eq!(records(leaf), before);
                retained_context_refuses_for_test(root);
                println!("HC074-ORIGINAL-BARRIER-SIGKILL seed=0x{seed:08x} repeat={repeat} cut={cut} frame_ipc={frame_ipc} parent_retained={parent_retained} signal=9 unchanged=true completion=false send=false");
            });
        }
    }
    println!("HC074-ORIGINAL-BARRIER-SIGKILL completed=true cases=12 product=false");
}
