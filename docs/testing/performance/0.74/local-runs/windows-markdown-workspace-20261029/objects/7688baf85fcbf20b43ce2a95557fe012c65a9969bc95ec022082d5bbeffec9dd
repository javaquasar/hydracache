#![cfg(target_os = "linux")]

use hydracache_long_run_supervisor_074::systemd_notify::notify_path;
use std::os::unix::net::UnixDatagram;

#[test]
fn readiness_message_uses_one_bounded_unix_datagram() {
    let temporary = tempfile::tempdir().unwrap();
    let socket = temporary.path().join("notify.sock");
    let receiver = UnixDatagram::bind(&socket).unwrap();
    notify_path(&socket, b"READY=1\nSTATUS=test").unwrap();
    let mut buffer = [0_u8; 64];
    let received = receiver.recv(&mut buffer).unwrap();
    assert_eq!(&buffer[..received], b"READY=1\nSTATUS=test");
}

#[test]
fn empty_nul_and_overlong_notifications_are_rejected() {
    assert!(notify_path(std::path::Path::new(""), b"READY=1").is_err());
    assert!(notify_path(std::path::Path::new("/tmp/notify"), b"bad\0message").is_err());
    let long = format!("/tmp/{}", "x".repeat(200));
    assert!(notify_path(std::path::Path::new(&long), b"READY=1").is_err());
}
