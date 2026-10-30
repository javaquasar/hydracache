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
    assert!(retry_marker(
        || Ok([format!("{GROUP_CHAIN}=").as_bytes(), b"\xff\0"].concat()),
        || Ok(true),
        17
    )
    .is_err());
}

fn process(pid: i32, parent: i32, group: i32, session: i32, birth: u64) -> Member {
    Member {
        pid,
        parent,
        group,
        session,
        birth,
        live: true,
    }
}
#[test]
fn proc_stat_parsing_handles_parentheses_zombies_and_exact_byte_and_field_bounds() {
    let mut fields = vec!["0"; 20];
    fields[0] = "S";
    fields[1] = "7";
    fields[2] = "8";
    fields[3] = "9";
    fields[19] = "1234";
    let stat = format!("42 (name with ) parentheses) {}", fields.join(" "));
    let member = parse_member(42, &stat).unwrap();
    assert_eq!(
        (
            member.pid,
            member.parent,
            member.group,
            member.session,
            member.birth,
            member.live
        ),
        (42, 7, 8, 9, 1234, true)
    );
    for state in ["Z", "X"] {
        fields[0] = state;
        assert!(
            !parse_member(42, &format!("42 (worker) {}", fields.join(" ")))
                .unwrap()
                .live
        );
    }
    for stat in [
        "not a stat".into(),
        "42 (worker) S 1".into(),
        "x".repeat(4097),
    ] {
        assert!(parse_member(42, &stat).is_err());
    }
    for index in [1, 2, 3, 19] {
        let mut invalid = fields.clone();
        invalid[index] = "invalid";
        assert!(parse_member(42, &format!("42 (worker) {}", invalid.join(" "))).is_err());
    }
}
#[test]
fn proc_inventory_is_bounded_and_only_skips_explicitly_transient_members() {
    let entries = ["self", "11", "12", "13", "14"]
        .into_iter()
        .map(|s| Ok(s.into()));
    let members = inventory_from(entries, |pid| match pid {
        11 => Ok(process(pid, 1, pid, pid, 3)),
        12 => Err(io::Error::from_raw_os_error(libc::ENOENT)),
        13 => Err(io::Error::from_raw_os_error(libc::ESRCH)),
        14 => Err(denied()),
        _ => panic!("non-PID entry must not be read"),
    })
    .unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].pid, 11);
    assert!(
        inventory_from([Err(io::Error::other("directory failure"))], |_| panic!(
            "invalid entry"
        ))
        .is_err()
    );
    assert!(inventory_from([Ok("11".into())], |_| Err(io::Error::other(
        "stat IO failure"
    )))
    .is_err());
    let entries = (0..65537).map(|_| Ok("non-process".into()));
    assert!(inventory_from(entries, |_| panic!("non-PID entry"))
        .err()
        .unwrap()
        .to_string()
        .contains("inventory bound"));
}
#[test]
fn inherited_sessions_require_the_live_owner_birth_and_exact_session_leader_relationship() {
    use std::os::unix::ffi::OsStringExt;
    assert_eq!(
        inherited_session_from(None, 30, |_| panic!("no marker needs no procfs")).unwrap(),
        None
    );
    for marker in [
        std::ffi::OsString::from_vec(vec![0xff]),
        "invalid".into(),
        "pid:5".into(),
    ] {
        assert!(inherited_session_from(Some(marker), 30, |_| panic!(
            "malformed owner must refuse before procfs"
        ))
        .is_err());
    }
    for mode in 0..5 {
        let mut owner = process(10, 1, 10, 1, 55);
        let mut leader = process(20, 10, 20, 20, 77);
        match mode {
            1 => owner.birth = 56,
            2 => owner.live = false,
            3 => leader.parent = 9,
            4 => owner.session = 20,
            _ => {}
        }
        let result = inherited_session_from(Some("10:55".into()), 30, |pid| match pid {
            10 => Ok(owner.clone()),
            20 => Ok(leader.clone()),
            30 => Ok(process(30, 20, 30, 20, 88)),
            _ => panic!("unexpected member"),
        });
        if mode == 0 {
            assert_eq!(result.unwrap(), Some(20));
        } else {
            assert!(result.unwrap_err().to_string().contains("foreign/stale"));
        }
    }
    assert!(
        inherited_session_from(Some("10:55".into()), 30, |_| Err(io::Error::other(
            "member unavailable"
        )))
        .is_err()
    );
}
