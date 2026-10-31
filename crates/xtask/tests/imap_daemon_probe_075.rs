use xtask::imap_daemon_probe_075::{binary_digest, http_response, verify_exchange};

#[test]
fn malformed_or_ambiguous_http_evidence_is_refused() {
    for raw in [
        b"incomplete".as_slice(),
        b"HTTP/2 200 OK\r\nContent-Length: 0\r\n\r\n",
        b"HTTP/1.1 invalid OK\r\nContent-Length: 0\r\n\r\n",
        b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\n",
        b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nContent-Length: 0\r\n\r\n",
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n",
        b"HTTP/1.1 200 OK\r\nmissing-colon\r\n\r\n",
        b"HTTP/1.1 200 OK\r\nContent-Length: nope\r\n\r\n",
        b"HTTP/1.1 200 OK\r\n\r\n",
    ] {
        assert!(http_response(raw).is_err(), "{raw:?}");
    }
    assert_eq!(
        http_response(b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\n\r\nok").unwrap(),
        (200, b"ok".as_slice())
    );
    assert!(http_response(&vec![0; 65537]).is_err());
    assert!(verify_exchange(
        0,
        b"HTTP/1.1 200 OK\r\nContent-Length: 15\r\n\r\n{\"ready\":false}"
    )
    .is_err());
    assert!(verify_exchange(1, b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n").is_err());
    assert!(verify_exchange(3, b"$-1\r\n+OK\r\n").is_err());
    assert!(verify_exchange(4, b"-ERR\r\n").is_err());
    assert!(verify_exchange(99, b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n").is_err());
}

#[test]
fn daemon_binary_identity_is_content_bound_and_rejects_missing_or_non_files() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("binary");
    assert!(binary_digest(&file).is_err());
    assert!(binary_digest(temp.path()).is_err());
    std::fs::write(&file, []).unwrap();
    assert!(binary_digest(&file).is_err());
    std::fs::write(&file, [0, 255]).unwrap();
    let original = binary_digest(&file).unwrap();
    std::fs::write(&file, [0, 254]).unwrap();
    assert_ne!(binary_digest(&file).unwrap(), original);
}
