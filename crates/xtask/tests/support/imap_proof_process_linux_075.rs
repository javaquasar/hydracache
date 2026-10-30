use super::*;
fn denied() -> io::Error {
    io::Error::from(io::ErrorKind::PermissionDenied)
}
#[test]
fn disappeared_procfs_entries_accept_enoent_and_esrch_but_not_live_access_failures() {
    for error in [libc::ENOENT, libc::ESRCH] {
        assert!(absent(&io::Error::from_raw_os_error(error)));
        assert!(!retry_marker(
            || Err(io::Error::from_raw_os_error(error)),
            || panic!("a vanished process needs no liveness retry"),
            17,
        )
        .unwrap());
    }
    for error in [libc::EACCES, libc::EIO, libc::EINVAL] {
        assert!(!absent(&io::Error::from_raw_os_error(error)));
    }
}
#[test]
fn transient_procfs_denial_retries_but_dead_owners_need_no_environment() {
    let mut calls = 0;
    assert!(retry_marker(
        || {
            calls += 1;
            if calls == 1 {
                Err(denied())
            } else {
                Ok(format!("{GROUP_CHAIN}=17,23\0").into_bytes())
            }
        },
        || Ok(true),
        17
    )
    .unwrap());
    assert_eq!(calls, 2);
    assert!(!retry_marker(|| Err(denied()), || Ok(false), 17).unwrap());
    assert!(!retry_marker(
        || Err(io::Error::from(io::ErrorKind::NotFound)),
        || panic!("absent process has no liveness check"),
        17
    )
    .unwrap());
}
#[test]
fn persistent_live_denial_and_unrelated_io_fail_loud() {
    let mut calls = 0;
    assert_eq!(
        retry_marker(
            || {
                calls += 1;
                Err(denied())
            },
            || Ok(true),
            17
        )
        .unwrap_err()
        .kind(),
        io::ErrorKind::PermissionDenied
    );
    assert_eq!(calls, 6);
    assert!(retry_marker(
        || Err(denied()),
        || Err(io::Error::other("liveness unknown")),
        17
    )
    .is_err());
    assert!(retry_marker(|| Err(io::Error::other("unexpected IO")), || Ok(true), 17).is_err());
}
#[test]
fn marker_parser_ignores_private_bytes_and_enforces_bounds() {
    use std::os::unix::ffi::OsStringExt;
    assert!(chain_environment(Err(std::env::VarError::NotUnicode(
        std::ffi::OsString::from_vec(vec![0xff])
    )))
    .is_err());
    assert!(chain_environment(Err(std::env::VarError::NotPresent))
        .unwrap()
        .is_empty());
    assert_eq!(chain_environment(Ok("17".into())).unwrap(), vec![17]);
    assert!(retry_marker(
        || Ok([
            b"PRIVATE=\xff\0".as_slice(),
            format!("{GROUP_CHAIN}=17\0").as_bytes()
        ]
        .concat()),
        || Ok(true),
        17
    )
    .unwrap());
    assert!(!retry_marker(|| Ok(b"PRIVATE=opaque\0".to_vec()), || Ok(true), 17).unwrap());
    for raw in ["-1", "0", "invalid", &"1,".repeat(1025)] {
        assert!(parse_chain(raw).is_err());
    }
    assert_eq!(parse_chain("").unwrap(), Vec::<i32>::new());
    assert!(retry_marker(|| Ok(vec![0; 1024 * 1024 + 1]), || Ok(true), 17).is_err());
}
