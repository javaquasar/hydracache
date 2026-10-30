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
fn member_liveness_refuses_unknown_state_and_accepts_only_observed_absence() {
    let mut value = process(11, 1, 11, 11, 3);
    assert!(member_live(Ok(value.clone())).unwrap());
    value.live = false;
    assert!(!member_live(Ok(value)).unwrap());
    for code in [libc::ENOENT, libc::ESRCH] {
        assert!(!member_live(Err(io::Error::from_raw_os_error(code))).unwrap());
    }
    for code in [libc::EACCES, libc::EIO] {
        assert_eq!(
            member_live(Err(io::Error::from_raw_os_error(code)))
                .unwrap_err()
                .raw_os_error(),
            Some(code)
        );
    }
    assert!(retry_marker(
        || Ok(format!("{GROUP_CHAIN}=invalid\0").into_bytes()),
        || panic!("a readable invalid marker does not need liveness"),
        17,
    )
    .is_err());
}

#[test]
fn descendant_procfs_exit_race_uses_the_same_member_lookup_without_retaining_private_environment() {
    let pid = std::process::id() as i32;
    // Assert only the byte bound; never print or persist process environment.
    assert!(read_environment(pid).unwrap().len() <= 1024 * 1024 + 1);
    assert!(read_environment(-1).is_err());
    let mut reads = 0;
    assert!(descendant_using(
        pid,
        17,
        |requested| {
            assert_eq!(requested, pid);
            reads += 1;
            if reads == 1 {
                Err(denied())
            } else {
                Ok(format!("{GROUP_CHAIN}=17\0").into_bytes())
            }
        },
        member
    )
    .unwrap());
    assert_eq!(reads, 2);
    assert!(!descendant_using(
        pid,
        17,
        |_| Err(denied()),
        |_| Err(io::Error::from_raw_os_error(libc::ESRCH))
    )
    .unwrap());
    assert!(descendant_using(
        pid,
        17,
        |_| Err(denied()),
        |_| Err(io::Error::other("member unreadable"))
    )
    .is_err());
}

#[test]
fn session_planning_preserves_root_and_nested_ownership_without_spawning() {
    let mut command = Command::new("unit-never-spawned");
    command.env(GROUP_CHAIN, "stale");
    configure_session(
        &mut command,
        None,
        || Ok(process(11, 1, 11, 11, 42)),
        || panic!("root must not inherit a group chain"),
        17,
    )
    .unwrap();
    let values = command
        .get_envs()
        .map(|(k, v)| {
            (
                k.to_string_lossy().into_owned(),
                v.map(|s| s.to_string_lossy().into_owned()),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(values[SESSION_OWNER].as_deref(), Some("11:42"));
    assert_eq!(values[GROUP_CHAIN], None);
    assert!(configure_session(
        &mut command,
        None,
        || Err(denied()),
        || panic!("root chain"),
        17
    )
    .is_err());
    for (parents, expected) in [(vec![], "17"), (vec![7], "7,17"), (vec![7, 17], "7,17")] {
        let mut command = Command::new("unit-never-spawned");
        configure_session(
            &mut command,
            Some(7),
            || panic!("nested owner must not be replaced"),
            || Ok(parents),
            17,
        )
        .unwrap();
        let marker = command
            .get_envs()
            .find(|(key, _)| *key == std::ffi::OsStr::new(GROUP_CHAIN))
            .unwrap()
            .1
            .unwrap();
        assert_eq!(marker, std::ffi::OsStr::new(expected));
    }
    assert!(configure_session(
        &mut command,
        Some(7),
        || panic!("nested owner"),
        || Err(denied()),
        17
    )
    .is_err());
    assert_eq!(child_session(None, 23), Some(23));
    assert_eq!(child_session(Some(7), 23), None);
}

#[test]
fn child_setup_checks_each_syscall_result_without_forking_or_setting_limits() {
    for root in [false, true] {
        let mut session_calls = 0;
        let mut limit_calls = 0;
        child_setup(
            root,
            123,
            || {
                session_calls += 1;
                23
            },
            |limit| {
                limit_calls += 1;
                assert_eq!(limit.rlim_cur, 123);
                assert_eq!(limit.rlim_max, 123);
                0
            },
        )
        .unwrap();
        assert_eq!(session_calls, usize::from(root));
        assert_eq!(limit_calls, 1);
    }
    assert!(child_setup(
        true,
        123,
        || -1,
        |_| panic!("setsid failure must stop setup")
    )
    .is_err());
    assert!(child_setup(
        false,
        123,
        || panic!("nested worker must retain its session"),
        |_| -1
    )
    .is_err());
}

#[test]
fn root_reaping_waits_for_the_entire_owned_tree_to_drain() {
    use std::os::unix::process::ExitStatusExt;
    assert!(
        reap_after_drain(1, || panic!("live descendants must retain the root zombie"))
            .unwrap()
            .is_none()
    );
    assert!(reap_after_drain(0, || Ok(None)).unwrap().is_none());
    assert!(reap_after_drain(0, || Ok(Some(ExitStatus::from_raw(0))))
        .unwrap()
        .unwrap()
        .success());
    assert_eq!(
        reap_after_drain(0, || Err(denied())).unwrap_err().kind(),
        io::ErrorKind::PermissionDenied
    );
}

#[test]
fn managed_nested_leaf_executes_and_reaps_one_finite_owned_shell() {
    let mut command = Command::new("/bin/sh");
    command.args(["-c", "printf 'owned-shell-complete'"]);
    let result = execute(
        command,
        &[],
        ProcessBounds {
            timeout: Duration::from_secs(10),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result.stdout, b"owned-shell-complete");
    assert!(result.direct_child_reaped);
    assert_eq!(result.remaining_live_members, 0);
    println!("owned-leaf-complete");
}

#[test]
fn real_cooperative_nested_controller_reaps_both_finite_owned_roots() {
    let mut command = Command::new(std::env::current_exe().unwrap());
    let (_, module) = module_path!().split_once("::").unwrap();
    command.args([
        "--exact",
        &format!("{module}::managed_nested_leaf_executes_and_reaps_one_finite_owned_shell"),
        "--nocapture",
    ]);
    let result = execute(
        command,
        &[],
        ProcessBounds {
            timeout: Duration::from_secs(15),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(String::from_utf8(result.stdout)
        .unwrap()
        .contains("owned-leaf-complete"));
    assert!(result.direct_child_reaped);
    assert_eq!(result.remaining_live_members, 0);
}

#[test]
fn ownership_inventory_distinguishes_root_nested_foreign_and_unreadable_members() {
    // This model owns no processes. Suppress Drop unconditionally, including
    // during a failed assertion, so no test PID/group can receive a real signal.
    let tree = std::mem::ManuallyDrop::new(Tree {
        group: 17,
        session: Some(17),
        nested_session: Some(7),
        terminated: false,
        finished: false,
    });
    let mut dead = process(1, 0, 17, 17, 1);
    dead.live = false;
    let members = vec![
        dead,
        process(2, 0, 22, 17, 1),
        process(3, 0, 17, 22, 1),
        process(4, 0, 22, 7, 1),
        process(5, 0, 22, 7, 1),
        process(6, 0, 22, 22, 1),
    ];
    let owned = tree
        .owned_members_using(
            || Ok(members),
            |pid, group| {
                assert_eq!(group, 17);
                match pid {
                    4 => Ok(true),
                    5 => Ok(false),
                    _ => panic!("foreign member must not be inspected"),
                }
            },
        )
        .unwrap();
    assert_eq!(
        owned.iter().map(|m| m.pid).collect::<Vec<_>>(),
        vec![2, 3, 4]
    );
    for code in [libc::ENOENT, libc::ESRCH] {
        assert!(tree
            .owned_members_using(
                || Ok(vec![process(4, 0, 22, 7, 1)]),
                |_, _| Err(io::Error::from_raw_os_error(code)),
            )
            .unwrap()
            .is_empty());
    }
    assert!(tree
        .owned_members_using(|| Ok(vec![process(4, 0, 22, 7, 1)]), |_, _| Err(denied()),)
        .is_err());
    assert!(tree
        .owned_members_using(
            || Err(io::Error::other("inventory unreadable")),
            |_, _| panic!("inventory failed"),
        )
        .is_err());
}

#[test]
fn termination_requires_observed_ownership_and_distinguishes_absence_from_signal_failure() {
    fn model() -> std::mem::ManuallyDrop<Tree> {
        std::mem::ManuallyDrop::new(Tree {
            group: 17,
            session: Some(17),
            nested_session: None,
            terminated: false,
            finished: false,
        })
    }
    fn failure(code: i32) -> i32 {
        // Model the documented syscall result on this test thread only.
        // No real PID is passed to any signal or ownership operation.
        unsafe {
            *libc::__errno_location() = code;
        }
        -1
    }
    let mut tree = model();
    let mut groups = Vec::new();
    tree.terminate_using(
        |_| Ok(vec![process(1, 0, 23, 17, 1), process(2, 0, 23, 17, 1)]),
        |group, signal| {
            assert_eq!(signal, libc::SIGKILL);
            groups.push(group);
            0
        },
    )
    .unwrap();
    assert_eq!(groups, vec![-17, -23]);
    assert!(tree.terminated);
    let mut tree = model();
    tree.terminate_using(
        |_| Ok(vec![process(1, 0, 23, 17, 1)]),
        |_, _| failure(libc::ESRCH),
    )
    .unwrap();
    assert!(tree.terminated);
    for root_failure in [false, true] {
        let mut tree = model();
        let mut calls = 0;
        let error = tree
            .terminate_using(
                |_| Ok(vec![process(1, 0, 23, 17, 1)]),
                |group, _| {
                    calls += 1;
                    if root_failure || group == -23 {
                        failure(libc::EPERM)
                    } else {
                        0
                    }
                },
            )
            .unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::EPERM));
        assert_eq!(calls, if root_failure { 1 } else { 2 });
        assert!(!tree.terminated);
    }
    let mut tree = model();
    assert!(tree
        .terminate_using(
            |_| Ok(vec![process(1, 0, 0, 17, 1)]),
            |group, _| {
                assert_eq!(group, -17);
                0
            }
        )
        .unwrap_err()
        .to_string()
        .contains("invalid owned process group"));
    assert!(!tree.terminated);
    assert!(tree
        .terminate_using(
            |_| Err(denied()),
            |_, _| panic!("unreadable ownership must not signal")
        )
        .is_err());
    tree.finished = true;
    tree.terminate_using(
        |_| panic!("finished tree needs no inventory"),
        |_, _| panic!("finished tree must not signal"),
    )
    .unwrap();
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
