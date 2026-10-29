//! Private local fixture carrier. Never runtime delivery or production enrollment.
use super::*;
use std::fs::File;
use std::os::fd::{FromRawFd, RawFd};
use std::os::unix::fs::MetadataExt;
pub(super) struct Carrier {
    peer: File,
    preparation: Option<serde_json::Value>,
    pub(super) outside: bool,
}
impl Carrier {
    pub(super) fn inherited() -> Self {
        assert_eq!(
            std::env::var("HC074_OWNED_ROOT_FIXTURE").as_deref(),
            Ok("local-only-v1")
        );
        assert_eq!(
            (unsafe { libc::geteuid() }, unsafe { libc::getegid() }),
            (0, 0)
        );
        let fd: RawFd = std::env::var("HC074_OWNED_CREATOR_PEER")
            .unwrap()
            .parse()
            .unwrap();
        assert!(fd > 2);
        for (option, expected) in [
            (libc::SO_DOMAIN, libc::AF_UNIX),
            (libc::SO_TYPE, libc::SOCK_SEQPACKET),
        ] {
            let mut value: libc::c_int = 0;
            let mut size = std::mem::size_of_val(&value) as libc::socklen_t;
            assert_eq!(
                unsafe {
                    libc::getsockopt(
                        fd,
                        libc::SOL_SOCKET,
                        option,
                        (&mut value as *mut libc::c_int).cast(),
                        &mut size,
                    )
                },
                0
            );
            assert_eq!(value, expected);
        }
        assert_eq!(
            unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) },
            0
        );
        let case = std::env::var("HC074_OWNED_CREATOR_CASE").unwrap();
        assert!([
            "owner",
            "first-dead",
            "second-dead",
            "first-stopped",
            "second-stopped",
            "outside-alias"
        ]
        .contains(&case.as_str()));
        Self {
            peer: unsafe { File::from_raw_fd(fd) },
            preparation: None,
            outside: case == "outside-alias",
        }
    }
    fn send(&self, value: &serde_json::Value, rights: &[RawFd]) {
        let bytes = serde_json::to_vec(value).unwrap();
        assert!(!bytes.is_empty() && bytes.len() < 16384 && rights.len() <= 14);
        let mut vector = libc::iovec {
            iov_base: bytes.as_ptr().cast_mut().cast(),
            iov_len: bytes.len(),
        };
        // usize alignment covers cmsghdr; exact CMSG_LEN excludes padding.
        let mut control = [0usize; 16];
        let mut message: libc::msghdr = unsafe { std::mem::zeroed() };
        message.msg_iov = &mut vector;
        message.msg_iovlen = 1;
        if !rights.is_empty() {
            let size = std::mem::size_of_val(rights) as u32;
            message.msg_control = control.as_mut_ptr().cast();
            message.msg_controllen = unsafe { libc::CMSG_SPACE(size) } as usize;
            assert!(message.msg_controllen <= std::mem::size_of_val(&control));
            unsafe {
                let header = libc::CMSG_FIRSTHDR(&message);
                (*header).cmsg_level = libc::SOL_SOCKET;
                (*header).cmsg_type = libc::SCM_RIGHTS;
                (*header).cmsg_len = libc::CMSG_LEN(size) as usize;
                std::ptr::copy_nonoverlapping(
                    rights.as_ptr(),
                    libc::CMSG_DATA(header).cast(),
                    rights.len(),
                );
            }
        }
        assert_eq!(
            unsafe { libc::sendmsg(self.peer.as_raw_fd(), &message, libc::MSG_NOSIGNAL) },
            bytes.len() as isize
        );
    }
    pub(super) fn preparing(
        &mut self,
        cgroup: &cgroup::Cgroup,
        relay: &relay::Relay,
        campaigns: &Path,
    ) {
        assert!(self.preparation.is_none());
        let info = fs::symlink_metadata(cgroup.creator_path()).unwrap();
        let scopes =
            crate::diagnostic_two_phase_launch::root_enrollment::privileged_fixture::scopes();
        assert_eq!(scopes.len(), 6);
        let value = serde_json::json!({"schema":"hc074-owned-root-creator-v1", "phase":"preparing", "root":cgroup.creator_path(), "root_device":info.dev(), "root_inode":info.ino(), "fence":campaigns.join(".host-execution.lock"), "scopes":scopes, "relay":relay.creator_scope(), "test_issuers":true, "original_send":false, "completion":false, "product":false});
        self.send(&value, &[]);
        self.preparation = Some(value);
    }
    pub(super) fn ready(&self, origins: ([u64; 4], [u64; 4]), rights: &[RawFd]) -> ! {
        assert_eq!(rights.len(), if self.outside { 14 } else { 13 });
        let mut value = self.preparation.as_ref().unwrap().clone();
        value["phase"] = serde_json::json!("ready");
        value["origins"] = serde_json::json!(origins.0);
        value["bounds"] = serde_json::json!(origins.1);
        self.send(&value, rights);
        // A separate subreaper must SIGKILL the exact creator. No Drop shortcut.
        loop {
            unsafe { libc::pause() };
        }
    }
}
