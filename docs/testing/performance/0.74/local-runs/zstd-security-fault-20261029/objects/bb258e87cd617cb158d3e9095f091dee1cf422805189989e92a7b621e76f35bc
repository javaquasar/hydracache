//! No fixed/system bus is opened. Context methods below are compile-only.
use super::*;
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};

#[test]
fn complete_readonly_entrypoints_compile_without_context_stubs_or_host_io() {
    let _ = FixedManagerChannel::open_fixed;
    let _ = FixedManagerChannel::inspect;
    let _ = FixedManagerChannel::revalidate;
    let _ = FixedManagerChannel::observe_sandbox;
    assert_eq!(std::num::NonZeroUsize::new(16).unwrap().get(), 16);
}

#[test]
fn tokio_standard_handoff_keeps_fd_flags_and_original_socket_alias() {
    let runtime = Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();
    let (left, mut right) = std::os::unix::net::UnixStream::pair().unwrap();
    left.set_nonblocking(true).unwrap();
    right
        .set_read_timeout(Some(Duration::from_millis(250)))
        .unwrap();
    right
        .set_write_timeout(Some(Duration::from_millis(250)))
        .unwrap();
    let (mut standard, mut alias) = runtime.block_on(async {
        let stream = tokio::net::UnixStream::from_std(left).unwrap();
        let original = stream.as_raw_fd();
        // SAFETY: stream owns this live FD throughout both observations and
        // the duplication. The successful duplicate is immediately owned by File.
        let descriptor_flags = unsafe { libc::fcntl(original, libc::F_GETFD) };
        let status_flags = unsafe { libc::fcntl(original, libc::F_GETFL) };
        assert_eq!(descriptor_flags, libc::FD_CLOEXEC);
        assert_ne!(status_flags & libc::O_NONBLOCK, 0);
        let duplicated = unsafe { libc::fcntl(original, libc::F_DUPFD_CLOEXEC, 3) };
        assert!(duplicated >= 0);
        let alias = unsafe { File::from_raw_fd(duplicated) };
        let standard = io(stream.into_std()).unwrap();
        assert_eq!(standard.as_raw_fd(), original);
        assert_ne!(alias.as_raw_fd(), original);
        assert_eq!(
            unsafe { libc::fcntl(standard.as_raw_fd(), libc::F_GETFD) },
            descriptor_flags
        );
        assert_eq!(
            unsafe { libc::fcntl(standard.as_raw_fd(), libc::F_GETFL) },
            status_flags
        );
        (standard, alias)
    });
    standard.write_all(b"transferred").unwrap();
    let mut first = [0; 11];
    right.read_exact(&mut first).unwrap();
    assert_eq!(&first, b"transferred");
    right.write_all(b"74").unwrap();
    let mut response = [0; 2];
    standard.read_exact(&mut response).unwrap();
    assert_eq!(&response, b"74");
    drop(standard);
    alias.write_all(b"held").unwrap();
    let mut held = [0; 4];
    right.read_exact(&mut held).unwrap();
    assert_eq!(&held, b"held");
    drop(alias);
    assert_eq!(right.read(&mut [0; 1]).unwrap(), 0);
    println!(
        "HC074_FIXED_HANDOFF:same_fd=true:flags_equal=true:alias_survives=true:eof_after_last_owner=true"
    );
}
