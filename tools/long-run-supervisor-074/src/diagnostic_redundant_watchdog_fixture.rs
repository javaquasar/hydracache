//! Owned test carrier only; never an original execution or recovery interface.
use super::super::super::redundant::GuardPair;
use super::*;

fn publish_pair(
    socket: &UnixDatagram,
    pair: &GuardPair,
    target: &Target,
    case: &str,
    deadline: u64,
) {
    let guardians = pair.test_guardians();
    let mut rights = vec![
        guardians[0].pinned.as_raw_fd(),
        guardians[1].pinned.as_raw_fd(),
        target.pin.as_raw_fd(),
        target.reader.as_raw_fd(),
    ];
    if let Some(hidden) = &target.hidden {
        rights.push(hidden.as_raw_fd());
    }
    let raw = serde_json::to_vec(&serde_json::json!({
        "schema":"hc074-redundant-owner-test-v1", "case":case,
        "owner":std::process::id(), "deadline":deadline,
        "pending":guardians.iter().all(|g| g.pending), "armed":pair.test_state().1
    }))
    .unwrap();
    let mut control = [0usize; 8];
    let mut iov = libc::iovec {
        iov_base: raw.as_ptr().cast_mut().cast(),
        iov_len: raw.len(),
    };
    let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
    msg.msg_iov = &mut iov;
    msg.msg_iovlen = 1;
    msg.msg_control = control.as_mut_ptr().cast();
    msg.msg_controllen =
        unsafe { libc::CMSG_SPACE(std::mem::size_of_val(rights.as_slice()) as _) } as _;
    unsafe {
        let c = libc::CMSG_FIRSTHDR(&msg);
        (*c).cmsg_level = libc::SOL_SOCKET;
        (*c).cmsg_type = libc::SCM_RIGHTS;
        (*c).cmsg_len = libc::CMSG_LEN(std::mem::size_of_val(rights.as_slice()) as _) as _;
        std::ptr::copy_nonoverlapping(
            rights.as_ptr(),
            libc::CMSG_DATA(c).cast::<RawFd>(),
            rights.len(),
        );
        assert_eq!(
            libc::sendmsg(socket.as_raw_fd(), &msg, libc::MSG_NOSIGNAL),
            raw.len() as isize
        );
    }
}

#[test]
#[ignore = "private nonroot redundant-guardian driver; no original authority"]
fn redundant_watchdog_owner_victim() {
    let root =
        PathBuf::from(std::env::var_os("HC074_REDUNDANT_ROOT").expect("private driver required"));
    assert_eq!(root.parent(), Some(std::path::Path::new("/tmp")));
    assert!(root
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("hc074-redundant-"));
    assert_eq!(fs::canonicalize(&root).unwrap(), root);
    let meta = fs::symlink_metadata(&root).unwrap();
    assert!(meta.is_dir() && !meta.file_type().is_symlink());
    assert_eq!(meta.mode() & 0o7777, 0o700);
    assert_eq!(meta.uid(), unsafe { libc::getuid() });
    assert_ne!(meta.uid(), 0);
    let case = std::env::var("HC074_REDUNDANT_CASE").unwrap();
    assert!([
        "healthy-expiry",
        "owner-loss",
        "first-dead-owner-loss",
        "second-dead-owner-loss",
        "first-stopped-owner-loss",
        "second-stopped-owner-loss",
        "first-dead-expiry",
        "second-dead-expiry",
        "first-stopped-expiry",
        "second-stopped-expiry",
        "lost-second-ack-first-dead",
        "lost-second-ack-first-stopped",
        "partial-enrollment-loss",
        "duplicate-global",
        "hidden-writer-first-dead",
        "both-dead",
        "both-stopped"
    ]
    .contains(&case.as_str()));
    let early = case.starts_with("lost-second-ack") || case == "partial-enrollment-loss";
    let fence = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(root.join("fence"))
        .unwrap();
    let deadline = super::super::super::super::now().unwrap() + 10_000_000_000;
    let mut pair = GuardPair::spawn(fence, deadline, early).unwrap();
    let mut target = target(
        &pair.test_guardians()[0],
        case == "hidden-writer-first-dead",
    );
    if !early {
        pair.enroll_once(&target.pin).unwrap();
        pair.ready().unwrap();
        target.release.send_once().unwrap();
    }
    let socket = UnixDatagram::unbound().unwrap();
    socket.connect(root.join("notice.sock")).unwrap();
    publish_pair(&socket, &pair, &target, &case, deadline);
    target.hidden = None;
    command();
    if early {
        let result = pair.enroll_once(&target.pin);
        assert_eq!(
            case, "partial-enrollment-loss",
            "lost ACK owner must be killed"
        );
        assert!(result.is_err());
        assert_eq!(pair.test_state(), (true, false, true));
        assert!(pair.test_guardians()[0].armed);
        assert!(pair.test_guardians().iter().all(|g| g.consumed));
        assert!(pair.enroll_once(&target.pin).is_err());
        assert!(pair.ready().is_err());
    } else {
        assert_eq!(case, "duplicate-global");
        assert!(pair.enroll_once(&target.pin).is_err());
        assert_eq!(pair.test_state(), (true, true, true));
        assert!(pair.ready().is_err());
    }
    socket.send(b"D").unwrap();
    command();
    panic!("fixture must end by owned SIGKILL, never replay or unlock");
}
