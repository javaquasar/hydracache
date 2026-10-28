#![cfg(target_os = "linux")]

use hydracache_long_run_supervisor_074::protocol::MAX_PACKET_BYTES;
use hydracache_long_run_supervisor_074::unix_transport::{
    SeqpacketConnection, SeqpacketListener, TransportError,
};
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::thread;
use std::time::{Duration, Instant};

#[test]
fn diagnostic_kernel_receive_uses_message_sender_and_kernel_time() {
    let temporary = tempfile::tempdir().unwrap();
    let socket = temporary.path().join("kernel.sock");
    let listener = SeqpacketListener::bind(&socket, 0o660, unsafe { libc::getegid() }).unwrap();
    let client = SeqpacketConnection::connect(&socket).unwrap();
    let mut receiver = listener
        .accept()
        .unwrap()
        .into_diagnostic_kernel_receiver()
        .unwrap();
    assert!(receiver.try_receive().unwrap().is_none());
    client.send_packet(b"\0kernel\xff").unwrap();
    let mut packet = receiver.try_receive().unwrap().unwrap();
    assert_eq!(packet.bytes(), b"\0kernel\xff");
    assert_eq!(packet.sender_pid(), std::process::id());
    assert!(packet.received_time().boottime_ns() > 0);
    assert!(packet.received_time().unix_seconds() > 0);
    assert_eq!(packet.received_time().boot_id().len(), 36);
    packet.revalidate_sender().unwrap();
}

#[test]
fn preserves_packet_boundaries_and_reports_exact_peer_credentials() {
    let temporary = tempfile::tempdir().unwrap();
    let socket = temporary.path().join("supervisor.sock");
    let listener = SeqpacketListener::bind(&socket, 0o660, unsafe { libc::getegid() }).unwrap();
    let server = thread::spawn(move || {
        let connection = listener.accept().unwrap();
        let credentials = connection.peer_credentials().unwrap();
        assert_eq!(credentials.pid, std::process::id());
        // SAFETY: geteuid/getegid take no arguments and have no failure sentinel.
        assert_eq!(credentials.uid, unsafe { libc::geteuid() });
        assert!(credentials.belongs_to_group(unsafe { libc::getegid() }));
        assert_eq!(connection.receive_packet().unwrap(), b"first");
        assert_eq!(connection.receive_packet().unwrap(), b"second");
        connection.send_packet(b"accepted").unwrap();
    });

    let client = SeqpacketConnection::connect(&socket).unwrap();
    client.send_packet(b"first").unwrap();
    client.send_packet(b"second").unwrap();
    assert_eq!(client.receive_packet().unwrap(), b"accepted");
    server.join().unwrap();
}

#[test]
fn enforces_packet_limit_socket_mode_and_exclusive_path() {
    let temporary = tempfile::tempdir().unwrap();
    let socket = temporary.path().join("supervisor.sock");
    let listener = SeqpacketListener::bind(&socket, 0o660, unsafe { libc::getegid() }).unwrap();
    assert_eq!(
        fs::metadata(&socket).unwrap().permissions().mode() & 0o777,
        0o660
    );
    assert_eq!(fs::metadata(&socket).unwrap().gid(), unsafe {
        libc::getegid()
    });
    assert!(matches!(
        SeqpacketListener::bind(&socket, 0o660, unsafe { libc::getegid() }),
        Err(TransportError::Path)
    ));

    let client = SeqpacketConnection::connect(&socket).unwrap();
    assert!(matches!(
        client.send_packet(&[]),
        Err(TransportError::PacketSize)
    ));
    assert!(matches!(
        client.send_packet(&vec![0; MAX_PACKET_BYTES + 1]),
        Err(TransportError::PacketSize)
    ));
    drop(client);
    drop(listener);
    assert!(!socket.exists());
}

#[test]
fn rejects_overlong_socket_path_before_creating_any_file() {
    let temporary = tempfile::tempdir().unwrap();
    let socket = temporary.path().join("x".repeat(200));
    assert!(matches!(
        SeqpacketListener::bind(&socket, 0o660, unsafe { libc::getegid() }),
        Err(TransportError::Path)
    ));
    assert!(!socket.exists());
}

#[test]
fn bounded_accept_returns_none_without_a_connection() {
    let temporary = tempfile::tempdir().unwrap();
    let socket = temporary.path().join("supervisor.sock");
    let listener = SeqpacketListener::bind(&socket, 0o660, unsafe { libc::getegid() }).unwrap();

    let started = Instant::now();
    assert!(listener
        .accept_timeout(Duration::from_millis(25))
        .unwrap()
        .is_none());
    assert!(started.elapsed() >= Duration::from_millis(20));
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[test]
fn bounded_accept_returns_a_ready_connection_without_waiting_for_deadline() {
    let temporary = tempfile::tempdir().unwrap();
    let socket = temporary.path().join("supervisor.sock");
    let listener = SeqpacketListener::bind(&socket, 0o660, unsafe { libc::getegid() }).unwrap();
    let client = SeqpacketConnection::connect(&socket).unwrap();
    client.send_packet(b"ready").unwrap();

    let started = Instant::now();
    let connection = listener
        .accept_timeout(Duration::from_secs(1))
        .unwrap()
        .unwrap();
    assert!(started.elapsed() < Duration::from_millis(250));
    assert_eq!(connection.receive_packet().unwrap(), b"ready");
}

#[test]
fn bounded_accept_rejects_zero_and_excessive_timeouts() {
    let temporary = tempfile::tempdir().unwrap();
    let socket = temporary.path().join("supervisor.sock");
    let listener = SeqpacketListener::bind(&socket, 0o660, unsafe { libc::getegid() }).unwrap();

    assert!(matches!(
        listener.accept_timeout(Duration::ZERO),
        Err(TransportError::AcceptTimeout)
    ));
    assert!(matches!(
        listener.accept_timeout(Duration::from_secs(61)),
        Err(TransportError::AcceptTimeout)
    ));
}

#[test]
fn bounded_receive_times_out_and_preserves_the_next_packet() {
    let temporary = tempfile::tempdir().unwrap();
    let socket = temporary.path().join("supervisor.sock");
    let listener = SeqpacketListener::bind(&socket, 0o660, unsafe { libc::getegid() }).unwrap();
    let client = SeqpacketConnection::connect(&socket).unwrap();
    let connection = listener.accept().unwrap();

    let started = Instant::now();
    assert!(connection
        .receive_packet_timeout(Duration::from_millis(25))
        .unwrap()
        .is_none());
    assert!(started.elapsed() >= Duration::from_millis(20));
    assert!(started.elapsed() < Duration::from_secs(1));

    client.send_packet(b"ready").unwrap();
    assert_eq!(
        connection
            .receive_packet_timeout(Duration::from_secs(1))
            .unwrap()
            .unwrap(),
        b"ready"
    );
}

#[test]
fn bounded_receive_rejects_zero_and_excessive_timeouts() {
    let temporary = tempfile::tempdir().unwrap();
    let socket = temporary.path().join("supervisor.sock");
    let listener = SeqpacketListener::bind(&socket, 0o660, unsafe { libc::getegid() }).unwrap();
    let _client = SeqpacketConnection::connect(&socket).unwrap();
    let connection = listener.accept().unwrap();

    assert!(matches!(
        connection.receive_packet_timeout(Duration::ZERO),
        Err(TransportError::ReceiveTimeout)
    ));
    assert!(matches!(
        connection.receive_packet_timeout(Duration::from_secs(61)),
        Err(TransportError::ReceiveTimeout)
    ));
}
