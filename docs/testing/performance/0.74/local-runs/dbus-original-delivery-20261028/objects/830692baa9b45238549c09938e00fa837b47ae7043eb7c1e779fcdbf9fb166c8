//! External owning harness only. No production carrier or caller owner pidfd.
use super::*;
use std::fs::OpenOptions;
use std::io::Read;
use std::os::unix::fs::OpenOptionsExt;

fn packet(stage: u8) -> [u8; PACKET_BYTES] {
    let mut value = [0; PACKET_BYTES];
    value[..8].copy_from_slice(b"HCRGCUT1");
    value[8] = stage;
    value
}
fn selected() -> Option<u8> {
    if std::env::var("HC074_CREATOR_FIXTURE").as_deref() != Ok("local-only-v1") {
        return None;
    }
    std::env::var("HC074_CREATOR_CUT").ok()?.parse().ok()
}
fn carrier() -> RawFd {
    std::env::var("HC074_CREATOR_PEER")
        .unwrap()
        .parse()
        .unwrap()
}
pub(super) fn cut(pair: &RootPair<'_>, stage: u8) {
    if selected() != Some(stage) {
        return;
    }
    if stage == 5 || (stage == 1 && std::env::var("HC074_CREATOR_FAULT").as_deref() == Ok("0")) {
        // Real ACKs must be queued, not consumed, at this lost-ack boundary.
        // Wait only on the same existing alarm; never manufacture another timer.
        for guard in &pair.guards[if stage == 1 { 1 } else { 0 }..] {
            let mut fds = [
                libc::pollfd {
                    fd: guard.peer.as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                },
                libc::pollfd {
                    fd: pair.alarm.as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                },
                libc::pollfd {
                    fd: guard.pin.as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                },
            ];
            loop {
                let n = unsafe { libc::poll(fds.as_mut_ptr(), 3, -1) };
                if n < 0 && io::Error::last_os_error().raw_os_error() == Some(libc::EINTR) {
                    continue;
                }
                assert!(
                    n > 0
                        && fds[0].revents == libc::POLLIN
                        && fds[1].revents == 0
                        && fds[2].revents == 0
                );
                super::original::queued_ack_for_fixture(guard.peer.as_raw_fd(), &pair.frame.enroll);
                break;
            }
        }
    }
    assert!(raw::send(carrier(), &packet(stage), &[]));
    // Only the external owner may end this process. No simulated error/Drop.
    loop {
        unsafe { libc::pause() };
    }
}
fn inherited(variable: &str) -> File {
    let fd: i32 = std::env::var(variable).unwrap().parse().unwrap();
    assert!(fd > 2);
    assert_eq!(
        unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) },
        0
    );
    unsafe { File::from_raw_fd(fd) }
}
#[test]
#[ignore = "external local root creator SIGKILL harness only; no service or product"]
fn root_creator_victim() {
    assert_eq!(
        std::env::var("HC074_CREATOR_FIXTURE").as_deref(),
        Ok("local-only-v1")
    );
    assert_eq!(unsafe { libc::geteuid() }, 0);
    assert!(matches!(selected(), Some(1..=5)));
    let fence = inherited("HC074_CREATOR_FENCE");
    let alarm = inherited("HC074_CREATOR_ALARM");
    let _peer = inherited("HC074_CREATOR_PEER");
    let name = CString::new(std::env::var("HC074_CREATOR_ROOT").unwrap()).unwrap();
    assert!(name.to_bytes().starts_with(b"hc074-creator-") && !name.to_bytes().contains(&b'/'));
    let parent = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open("/sys/fs/cgroup")
        .unwrap();
    let mut root = PinnedRoot::pin_empty(parent.as_fd(), &name).unwrap();
    let binding = RootBindingClaims {
        attempt_sha256: [5; 32],
        original_context_sha256: [6; 32],
        root_device: root.identity.device,
        root_inode: root.identity.inode,
        deadline_boottime_ns: std::env::var("HC074_CREATOR_DEADLINE")
            .unwrap()
            .parse()
            .unwrap(),
    };
    let fault = std::env::var("HC074_CREATOR_FAULT")
        .unwrap()
        .parse()
        .unwrap();
    let mut pair =
        RootPair::spawn(fence.as_fd(), alarm.as_fd(), &mut root, binding, fault).unwrap();
    assert!(raw::send(
        carrier(),
        &packet(0),
        &pair.guards.each_ref().map(|g| g.pin.as_raw_fd())
    ));
    let mut go = [0];
    std::io::stdin().read_exact(&mut go).unwrap();
    assert_eq!(go, [b'G']);
    pair.enroll_once().unwrap();
    cut(&pair, 4);
    panic!("external harness did not select a real cut");
}
