use super::*;
use std::os::fd::{FromRawFd, OwnedFd};

fn packet() -> KernelPacket {
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
    let mut receiver = KernelReceiver::new(unsafe { OwnedFd::from_raw_fd(fds[0]) }).unwrap();
    let sender = unsafe { OwnedFd::from_raw_fd(fds[1]) };
    assert_eq!(
        unsafe {
            libc::send(
                sender.as_raw_fd(),
                b"context".as_ptr().cast(),
                7,
                libc::MSG_NOSIGNAL,
            )
        },
        7
    );
    receiver.try_receive().unwrap().unwrap()
}
#[test]
fn manager_receiver_binding_retains_actual_origin_without_packet_or_claim_adoption() {
    let mut packet = packet();
    let origin = std::sync::Arc::clone(&packet.origin);
    let mut held = packet.retain_original_receiver_binding().unwrap();
    assert!(std::sync::Arc::ptr_eq(&origin, &held.origin));
    let files = held
        .origin
        .namespaces
        .each_ref()
        .map(|(file, _)| file.try_clone().unwrap());
    let boot = held.origin.boot.clone();
    drop(packet);
    held.require_context(&boot, files.each_ref()).unwrap();
    held.require_context(&boot, files.each_ref()).unwrap();
}
#[test]
fn manager_receiver_binding_wrong_boot_type_object_and_descriptor_latch() {
    for case in 0..5 {
        let mut packet = packet();
        let mut held = packet.retain_original_receiver_binding().unwrap();
        let mut files = held
            .origin
            .namespaces
            .each_ref()
            .map(|(file, _)| file.try_clone().unwrap());
        let good = held
            .origin
            .namespaces
            .each_ref()
            .map(|(file, _)| file.try_clone().unwrap());
        let boot = held.origin.boot.clone();
        let supplied_boot = if case == 0 { "different" } else { &boot };
        match case {
            1 => files.swap(0, 1),
            2 => files[1] = tempfile::tempfile().unwrap(),
            3 => assert_eq!(
                unsafe { libc::fcntl(files[2].as_raw_fd(), libc::F_SETFD, 0) },
                0
            ),
            4 => files[3] = files[2].try_clone().unwrap(),
            _ => (),
        }
        assert!(
            held.require_context(supplied_boot, files.each_ref())
                .is_err(),
            "case={case}"
        );
        assert!(matches!(
            held.require_context(&boot, good.each_ref()),
            Err(KernelError::Refused)
        ));
    }
}
#[test]
fn manager_receiver_binding_cannot_mint_from_refused_or_wrong_original_clock() {
    for clock in [false, true] {
        let mut packet = packet();
        if clock {
            packet.received.boot = "different".into();
        } else {
            packet.latch_refusal();
        }
        assert!(packet.retain_original_receiver_binding().is_err());
        assert!(packet.is_refused());
        assert!(matches!(
            packet.retain_original_receiver_binding(),
            Err(KernelError::Refused)
        ));
    }
}
