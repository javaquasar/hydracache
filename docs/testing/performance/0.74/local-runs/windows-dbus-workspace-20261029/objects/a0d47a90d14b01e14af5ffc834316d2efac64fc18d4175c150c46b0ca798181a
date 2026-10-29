//! Parser/readiness units only, not an original-alarm or process-identity proof.
use super::*;
use std::io::Write;
fn event() -> File {
    let fd = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) };
    assert!(fd >= 0);
    unsafe { File::from_raw_fd(fd) }
}
fn fixture() -> (KernelCollector, File, File) {
    let mut fds = [-1; 2];
    assert_eq!(
        unsafe {
            libc::socketpair(
                libc::AF_UNIX,
                libc::SOCK_SEQPACKET | libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
                0,
                fds.as_mut_ptr(),
            )
        },
        0
    );
    // pid=0 makes cancellation inert. The synthetic event FD exercises only
    // poll precedence; manual tests use real atomically created child pidfds.
    (
        KernelCollector {
            pid: 0,
            pinned: event(),
            peer: unsafe { File::from_raw_fd(fds[0]) },
            child_peer: fds[1],
        },
        unsafe { File::from_raw_fd(fds[1]) },
        event(),
    )
}
fn send(peer: &File, bytes: &[u8]) {
    assert_eq!(
        unsafe {
            libc::send(
                peer.as_raw_fd(),
                bytes.as_ptr().cast(),
                bytes.len(),
                libc::MSG_NOSIGNAL,
            )
        },
        bytes.len() as isize
    );
}
#[test]
fn original_collector_notice_shape_and_duplicate_matrix() {
    for bytes in [b"".as_slice(), b"P", b"X", b"L", b"F", b"BB", b"B\0"] {
        for initial in [false, true] {
            let (kernel, peer, alarm) = fixture();
            send(&peer, bytes);
            assert!(matches!(
                kernel.observe(alarm.as_raw_fd(), initial),
                Err(BarredCustodyError::Child)
            ));
        }
    }
    let (kernel, peer, alarm) = fixture();
    send(&peer, b"B");
    kernel.observe(alarm.as_raw_fd(), true).unwrap();
    kernel.observe(alarm.as_raw_fd(), false).unwrap();
    send(&peer, b"B");
    assert!(matches!(
        kernel.observe(alarm.as_raw_fd(), false),
        Err(BarredCustodyError::Child)
    ));
    for initial in [false, true] {
        let (kernel, peer, alarm) = fixture();
        send(&peer, b"E");
        assert!(matches!(
            kernel.observe(alarm.as_raw_fd(), initial),
            Err(BarredCustodyError::Expired)
        ));
    }
    let (kernel, peer, alarm) = fixture();
    drop(peer);
    assert!(matches!(
        kernel.observe(alarm.as_raw_fd(), true),
        Err(BarredCustodyError::Child)
    ));
}
#[test]
fn original_collector_notice_alarm_precedes_queued_b() {
    let (kernel, peer, mut alarm) = fixture();
    send(&peer, b"B");
    alarm.write_all(&1u64.to_ne_bytes()).unwrap();
    assert!(matches!(
        kernel.observe(alarm.as_raw_fd(), true),
        Err(BarredCustodyError::Expired)
    ));
    let mut byte = 0u8;
    assert_eq!(
        unsafe {
            libc::recv(
                kernel.peer.as_raw_fd(),
                (&mut byte as *mut u8).cast(),
                1,
                libc::MSG_TRUNC,
            )
        },
        1
    );
    assert_eq!(byte, b'B');
}
#[test]
fn original_collector_notice_terminal_precedes_queued_b() {
    let (mut kernel, peer, alarm) = fixture();
    send(&peer, b"B");
    kernel.pinned.write_all(&1u64.to_ne_bytes()).unwrap();
    assert!(matches!(
        kernel.observe(alarm.as_raw_fd(), true),
        Err(BarredCustodyError::Child)
    ));
    assert!(matches!(kernel.alive(), Err(BarredCustodyError::Child)));
}
