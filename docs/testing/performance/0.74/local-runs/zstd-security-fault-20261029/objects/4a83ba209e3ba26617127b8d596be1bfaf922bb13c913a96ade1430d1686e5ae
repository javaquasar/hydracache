//! Opt-in combined-frame faults; neither child can execute or write output.
use super::*;
use crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::{inspect_empty_fence_at_for_test, with_owned_empty_fence_at_for_test};
use std::io::Read;
use std::os::fd::RawFd;
use std::os::unix::{fs::MetadataExt, net::UnixDatagram};
const ROOT: &str = "HC074_PRIVATE_ENROLLMENT_LOSS_ROOT";
const CUT: &str = "HC074_PRIVATE_ENROLLMENT_LOSS_CUT";
const AUTHORITY: &str = "HC074_PRIVATE_ENROLLMENT_LOSS_AUTHORITY";

#[test]
#[ignore = "manual nonroot combined-frame actual B loss; no execution"]
fn original_enrollment_lost_b_notice() {
    let seed = 0x0740_e318_u64;
    for repeat in 1..=3 {
        let bundle = Bundle::new();
        with_original_at(true, None, |original, _paths, receiver, sender, model| {
            with_owned_empty_fence_for_test(|fence, root, leaf| {
                let outputs = accepted(original, fence, leaf, receiver, sender, model)
                    .prepare_original_outputs_for_test(|_| Ok(()))
                    .unwrap();
                let before = records(leaf);
                let mut origins = None;
                let parent_mask = mask();
                let mut failure = outputs
                    .prepare_enrollment_fixture_for_test(&bundle.path, 3, |stage, joined| {
                        assert_eq!(stage, "retained", "lost B reached closing frame");
                        origins = Some(joined.origins_for_test());
                    })
                    .unwrap_err();
                assert_eq!(failure.reason_for_test(), "Expired");
                failure.assert_lost_b_for_test();
                assert_eq!(&failure.origins_for_test()[..4], &origins.unwrap()[..4]);
                assert_eq!(mask(), parent_mask);
                assert!(failure.refused_for_test());
                assert_eq!(
                    format!("{:?}", failure.retry_for_test().unwrap_err()),
                    "Refused"
                );
                busy(root);
                drop(failure);
                drop(crate::host_execution::lock_host_root(root).unwrap());
                assert_eq!(records(leaf), before);
                retained_context_refuses_for_test(root);
                println!("HC074-ENROLLMENT-LOSS-NOTICE seed=0x{seed:08x} repeat={repeat} reason=Expired barred_consumed=true expiry_notice=E origins_exact=true sticky=true unchanged=true start=false");
            });
        });
    }
}

// Fixed role order: collector then custodian. No original or peer FD export.
fn publish(socket: &UnixDatagram, pidfds: &[File; 2], raw: &[u8]) {
    #[repr(C)]
    struct Control([usize; 4]);
    let mut control = Control([0; 4]);
    let size = unsafe { libc::CMSG_SPACE((2 * std::mem::size_of::<RawFd>()) as u32) } as usize;
    assert!(size <= std::mem::size_of_val(&control));
    let mut iov = libc::iovec {
        iov_base: raw.as_ptr().cast_mut().cast(),
        iov_len: raw.len(),
    };
    let mut message: libc::msghdr = unsafe { std::mem::zeroed() };
    message.msg_iov = &mut iov;
    message.msg_iovlen = 1;
    message.msg_control = (&mut control as *mut Control).cast();
    message.msg_controllen = size;
    unsafe {
        let header = libc::CMSG_FIRSTHDR(&message);
        assert!(!header.is_null());
        (*header).cmsg_level = libc::SOL_SOCKET;
        (*header).cmsg_type = libc::SCM_RIGHTS;
        (*header).cmsg_len = libc::CMSG_LEN((2 * std::mem::size_of::<RawFd>()) as u32) as usize;
        let rights = [pidfds[0].as_raw_fd(), pidfds[1].as_raw_fd()];
        std::ptr::copy_nonoverlapping(rights.as_ptr(), libc::CMSG_DATA(header).cast::<RawFd>(), 2);
        assert_eq!(
            libc::sendmsg(socket.as_raw_fd(), &message, libc::MSG_NOSIGNAL),
            raw.len() as isize
        );
    }
    let mut command = [0u8; 1];
    std::io::stdin().read_exact(&mut command).unwrap();
    panic!("death fixture must be killed, never resumed: {command:?}");
}

#[test]
#[ignore = "private subprocess for custody owner SIGKILL driver; no execution"]
fn original_enrollment_owner_victim() {
    let path = PathBuf::from(std::env::var_os(ROOT).expect("private driver root required"));
    assert_eq!(path.parent(), Some(Path::new("/tmp")));
    assert!(path
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("hc074-enrollment-loss-"));
    assert_eq!(fs::canonicalize(&path).unwrap(), path);
    for item in [&path, &path.join("context"), &path.join("origin")] {
        let stat = fs::symlink_metadata(item).unwrap();
        assert!(stat.is_dir() && !stat.file_type().is_symlink());
        assert_eq!(
            (stat.uid(), stat.gid(), stat.mode() & 0o7777),
            (
                unsafe { libc::geteuid() },
                unsafe { libc::getegid() },
                0o700
            )
        );
        assert_ne!(stat.uid(), 0);
    }
    assert_eq!(std::env::temp_dir(), path.join("context"));
    let cut = std::env::var(CUT).unwrap();
    if cut == "inspect" {
        inspect_empty_fence_at_for_test(&path.join("origin"), false);
        println!("HC074-ENROLLMENT-LOSS-REPLAY refused=true");
        return;
    }
    assert!(matches!(cut.as_str(), "before-frame" | "after-frame"));
    let authority = PathBuf::from(std::env::var_os(AUTHORITY).unwrap());
    assert_eq!(authority.parent(), Some(Path::new("/tmp")));
    assert_eq!(fs::canonicalize(&authority).unwrap(), authority);
    assert!(authority
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("hydracache-authority-"));
    let socket = UnixDatagram::unbound().unwrap();
    socket.connect(path.join("notice.sock")).unwrap();
    let bundle = Bundle::new();
    with_original_at(
        false,
        Some(&authority),
        |original, _, receiver, sender, model| {
            with_owned_empty_fence_at_for_test(&path.join("origin"), |fence, root, leaf| {
                let outputs = accepted(original, fence, leaf, receiver, sender, model)
                    .prepare_original_outputs_for_test(|_| Ok(()))
                    .unwrap();
                let before = records(leaf);
                let mut origins = None;
                let (_, report) = trace::capture(|| {
                    trace::stage(Stage::Bind);
                    outputs.prepare_enrollment_fixture_for_test(&bundle.path, 0, |stage, joined| {
                        if stage == "retained" { origins = Some(joined.origins_for_test()); }
                        if (cut == "before-frame" && stage == "enrolled")
                            || (cut == "after-frame" && stage == "closed") {
                            joined.assert_original_for_test();
                            assert_eq!(&joined.origins_for_test()[..4], &origins.unwrap()[..4]);
                            assert_eq!(records(leaf), before);
                            busy(root);
                            let raw = serde_json::to_vec(&serde_json::json!({
                                "schema": "hc074-enrollment-owner-test-v1", "owner": std::process::id(),
                                "roles": ["collector", "custodian"], "cut": cut,
                                "deadline": joined.deadline_for_test(), "origins": origins.unwrap(),
                                "frame_closed": stage == "closed",
                            })).unwrap();
                            assert!(raw.len() <= 2048);
                            publish(&socket, &joined.pinned_for_test(), &raw);
                        }
                    }).unwrap()
                });
                panic!("owner checkpoint returned unexpectedly: {report:?}");
            });
        },
    );
}
