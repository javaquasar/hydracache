//! Stack and syscalls only after clone. No Rust IO errors, allocator or Drop.
use super::*;

pub(super) struct Received {
    pub bytes: [u8; PACKET_BYTES],
    pub len: isize,
    pub flags: i32,
    pub rights: [RawFd; 64],
    pub count: usize,
    pub credentials: Option<libc::ucred>,
    pub shape: bool,
}
impl Received {
    pub fn close(&mut self) {
        for fd in self.rights.iter_mut().take(self.count) {
            if *fd >= 0 {
                unsafe { libc::close(*fd) };
                *fd = -1;
            }
        }
    }
    pub fn matches(
        &self,
        bytes: &[u8; PACKET_BYTES],
        pid: i32,
        uid: u32,
        gid: u32,
        count: usize,
    ) -> bool {
        self.shape
            && self.len == PACKET_BYTES as isize
            && self.flags & !libc::MSG_CMSG_CLOEXEC == 0
            && self.bytes == *bytes
            && self.count == count
            && self
                .credentials
                .as_ref()
                .is_some_and(|c| c.pid == pid && c.uid == uid && c.gid == gid)
            && self
                .rights
                .iter()
                .take(self.count)
                .all(|fd| unsafe { libc::fcntl(*fd, libc::F_GETFD) } == libc::FD_CLOEXEC)
    }
}
pub(super) fn receive(socket: RawFd) -> Received {
    let mut result = Received {
        bytes: [0; PACKET_BYTES],
        len: -1,
        flags: 0,
        rights: [-1; 64],
        count: 0,
        credentials: None,
        shape: true,
    };
    let mut control = [0usize; 40];
    let mut iov = libc::iovec {
        iov_base: result.bytes.as_mut_ptr().cast(),
        iov_len: PACKET_BYTES,
    };
    let mut message: libc::msghdr = unsafe { std::mem::zeroed() };
    message.msg_iov = &mut iov;
    message.msg_iovlen = 1;
    message.msg_control = control.as_mut_ptr().cast();
    message.msg_controllen = std::mem::size_of_val(&control);
    result.len = unsafe {
        libc::recvmsg(
            socket,
            &mut message,
            libc::MSG_DONTWAIT | libc::MSG_CMSG_CLOEXEC | libc::MSG_TRUNC,
        )
    };
    result.flags = message.msg_flags;
    if result.len < 0 {
        result.shape = false;
        return result;
    }
    let mut rights_records = 0;
    unsafe {
        let mut header = libc::CMSG_FIRSTHDR(&message);
        while !header.is_null() {
            let base = libc::CMSG_LEN(0) as usize;
            let offset = header as usize - message.msg_control as usize;
            if offset > message.msg_controllen
                || (*header).cmsg_len < base
                || (*header).cmsg_len > message.msg_controllen - offset
            {
                result.shape = false;
                break;
            }
            let size = (*header).cmsg_len - base;
            if (*header).cmsg_level == libc::SOL_SOCKET && (*header).cmsg_type == libc::SCM_RIGHTS {
                rights_records += 1;
                if size == 0 || !size.is_multiple_of(std::mem::size_of::<RawFd>()) {
                    result.shape = false;
                }
                for index in 0..size / std::mem::size_of::<RawFd>() {
                    let fd = libc::CMSG_DATA(header)
                        .cast::<RawFd>()
                        .add(index)
                        .read_unaligned();
                    if fd < 0 {
                        result.shape = false;
                    } else if result.count < result.rights.len() {
                        result.rights[result.count] = fd;
                        result.count += 1;
                    } else {
                        libc::close(fd);
                        result.shape = false;
                    }
                }
            } else if (*header).cmsg_level == libc::SOL_SOCKET
                && (*header).cmsg_type == libc::SCM_CREDENTIALS
                && size == std::mem::size_of::<libc::ucred>()
                && result.credentials.is_none()
            {
                result.credentials = Some(
                    libc::CMSG_DATA(header)
                        .cast::<libc::ucred>()
                        .read_unaligned(),
                );
            } else {
                result.shape = false;
            }
            header = libc::CMSG_NXTHDR(&message, header);
        }
    }
    result.shape &= rights_records <= 1;
    result
}
pub(super) fn send(socket: RawFd, bytes: &[u8; PACKET_BYTES], rights: &[RawFd]) -> bool {
    if rights.len() > 253 {
        return false;
    }
    let mut control = [0usize; 132];
    let mut iov = libc::iovec {
        iov_base: bytes.as_ptr().cast_mut().cast(),
        iov_len: PACKET_BYTES,
    };
    let mut message: libc::msghdr = unsafe { std::mem::zeroed() };
    message.msg_iov = &mut iov;
    message.msg_iovlen = 1;
    if !rights.is_empty() {
        let size = std::mem::size_of_val(rights);
        message.msg_control = control.as_mut_ptr().cast();
        message.msg_controllen = unsafe { libc::CMSG_SPACE(size as _) } as usize;
        unsafe {
            let header = libc::CMSG_FIRSTHDR(&message);
            (*header).cmsg_level = libc::SOL_SOCKET;
            (*header).cmsg_type = libc::SCM_RIGHTS;
            (*header).cmsg_len = libc::CMSG_LEN(size as _) as usize;
            std::ptr::copy_nonoverlapping(
                rights.as_ptr().cast::<u8>(),
                libc::CMSG_DATA(header),
                size,
            );
        }
    }
    unsafe {
        libc::sendmsg(socket, &message, libc::MSG_DONTWAIT | libc::MSG_NOSIGNAL)
            == PACKET_BYTES as isize
    }
}
fn identity(fd: RawFd) -> Option<Stamp> {
    let mut fs: libc::statfs = unsafe { std::mem::zeroed() };
    let mut stat: libc::stat = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstatfs(fd, &mut fs) } != 0
        || fs.f_type != 0x63677270
        || unsafe { libc::fstat(fd, &mut stat) } != 0
        || stat.st_uid != 0
        || stat.st_gid != 0
        || stat.st_mode & 0o022 != 0
    {
        return None;
    }
    Some(Stamp {
        device: stat.st_dev,
        inode: stat.st_ino,
        uid: stat.st_uid,
        gid: stat.st_gid,
        mode: stat.st_mode,
    })
}
fn open(parent: RawFd, name: *const libc::c_char, directory: bool, write: bool) -> RawFd {
    unsafe {
        libc::openat(
            parent,
            name,
            libc::O_CLOEXEC
                | libc::O_NOFOLLOW
                | libc::O_NONBLOCK
                | if directory { libc::O_DIRECTORY } else { 0 }
                | if write {
                    libc::O_WRONLY
                } else {
                    libc::O_RDONLY
                },
        )
    }
}
fn access(fd: RawFd, mode: i32, directory: bool) -> bool {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    flags >= 0
        && unsafe { libc::fcntl(fd, libc::F_GETFD) } == libc::FD_CLOEXEC
        && flags & (libc::O_ACCMODE | libc::O_PATH | libc::O_APPEND | libc::O_ASYNC) == mode
        && flags & libc::O_NONBLOCK != 0
        && (flags & libc::O_DIRECTORY != 0) == directory
}
fn bytes(fd: RawFd, buffer: &mut [u8; 4097]) -> Option<usize> {
    let mut used = 0;
    loop {
        let n = unsafe {
            libc::pread(
                fd,
                buffer.as_mut_ptr().add(used).cast(),
                buffer.len() - used,
                used as _,
            )
        };
        if n == 0 {
            return Some(used);
        }
        if n < 0 {
            if unsafe { *libc::__errno_location() } == libc::EINTR {
                continue;
            }
            return None;
        }
        used += n as usize;
        if used == buffer.len() {
            return None;
        }
    }
}
pub(super) fn inspect(frame: &Snapshot, roles: [RawFd; 3], empty: bool) -> bool {
    let named = open(frame.parent, frame.name.as_ptr().cast(), true, false);
    let kind = open(roles[0], c"cgroup.type".as_ptr(), false, false);
    let procs = open(roles[0], c"cgroup.procs".as_ptr(), false, false);
    let kill = open(roles[0], c"cgroup.kill".as_ptr(), false, true);
    let events = open(roles[0], c"cgroup.events".as_ptr(), false, false);
    let files = [named, kind, procs, kill, events];
    let mut buffer = [0u8; 4097];
    let mut valid = files.iter().all(|fd| *fd >= 0)
        && access(roles[0], libc::O_RDONLY, true)
        && access(roles[1], libc::O_WRONLY, false)
        && access(roles[2], libc::O_RDONLY, false);
    for _ in 0..2 {
        valid &= identity(frame.parent) == Some(frame.parent_identity)
            && identity(named) == Some(frame.roles[0])
            && identity(kind) == Some(frame.kind)
            && identity(procs) == Some(frame.procs)
            && identity(kill) == Some(frame.roles[1])
            && identity(events) == Some(frame.roles[2]);
        for (fd, expected) in roles.into_iter().zip(frame.roles) {
            valid &= identity(fd) == Some(expected);
        }
        if valid {
            valid &= bytes(kind, &mut buffer).is_some_and(|n| &buffer[..n] == b"domain\n");
            valid &= bytes(procs, &mut buffer).is_some_and(|n| !empty || n == 0);
            valid &= bytes(roles[2], &mut buffer).is_some_and(|n| {
                let b = &buffer[..n];
                b == b"populated 0\nfrozen 0\n" || (!empty && b == b"populated 1\nfrozen 0\n")
            });
        }
        let closing = [
            open(frame.parent, frame.name.as_ptr().cast(), true, false),
            open(roles[0], c"cgroup.type".as_ptr(), false, false),
            open(roles[0], c"cgroup.procs".as_ptr(), false, false),
            open(roles[0], c"cgroup.kill".as_ptr(), false, true),
            open(roles[0], c"cgroup.events".as_ptr(), false, false),
        ];
        for (fd, stamp) in closing.into_iter().zip([
            frame.roles[0],
            frame.kind,
            frame.procs,
            frame.roles[1],
            frame.roles[2],
        ]) {
            valid &= identity(fd) == Some(stamp);
            if fd >= 0 {
                unsafe { libc::close(fd) };
            }
        }
    }
    for fd in files {
        if fd >= 0 {
            unsafe { libc::close(fd) };
        }
    }
    valid
}
pub(super) fn terminal(fd: RawFd) -> bool {
    let mut descriptor = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    unsafe { libc::poll(&mut descriptor, 1, 0) != 0 }
}
fn park() -> ! {
    loop {
        unsafe { libc::poll(std::ptr::null_mut(), 0, -1) };
    }
}
fn isolate(keep: &[RawFd; 5]) -> bool {
    let mut first = 0;
    for fd in keep {
        if first < *fd as u32
            && unsafe { libc::syscall(libc::SYS_close_range, first, *fd as u32 - 1, 0u32) } != 0
        {
            return false;
        }
        first = *fd as u32 + 1;
    }
    unsafe { libc::syscall(libc::SYS_close_range, first, u32::MAX, 0u32) == 0 }
}
fn revoke(peer: RawFd, frame: &Snapshot, roles: Option<[RawFd; 4]>, refused_frame: bool) -> ! {
    let mut reply = frame.enroll;
    reply[8] = if roles.is_some() && !refused_frame {
        4
    } else {
        3
    };
    let _ = send(peer, &reply, &[]);
    unsafe { libc::close(peer) }; // also closes every queued, undisclosed right
    let Some(roles) = roles else { park() };
    loop {
        // Acknowledge the event BEFORE kill, never clear a late-population
        // notification after kill and then sleep with an unrevoked writer.
        let mut buffer = [0u8; 4097];
        let observed = bytes(roles[3], &mut buffer);
        // The previously admitted kill FD is the obligation, even after later
        // named metadata drift. Never reopen or signal a replacement target.
        let result = unsafe { libc::pwrite(roles[1], b"1".as_ptr().cast(), 1, 0) };
        if result < 0 && unsafe { *libc::__errno_location() } == libc::EINTR {
            continue;
        }
        if result != 1 || observed.is_none() {
            park();
        }
        // Permanent revocation, including population after the first empty read.
        let mut event = libc::pollfd {
            fd: roles[3],
            events: libc::POLLPRI | libc::POLLERR,
            revents: 0,
        };
        loop {
            let n = unsafe { libc::poll(&mut event, 1, -1) };
            if n < 0 && unsafe { *libc::__errno_location() } == libc::EINTR {
                continue;
            }
            if n <= 0 || event.revents & (libc::POLLHUP | libc::POLLNVAL) != 0 {
                park();
            }
            break;
        }
    }
}
pub(super) fn child(
    keep: [RawFd; 5],
    peer: RawFd,
    timer: RawFd,
    owner: RawFd,
    origin: Origin,
    frame: Snapshot,
    stop_before_ack: bool,
) -> ! {
    if !isolate(&keep) || terminal(timer) || terminal(owner) {
        revoke(peer, &frame, None, true);
    }
    let mut poll = [
        libc::pollfd {
            fd: timer,
            events: libc::POLLIN,
            revents: 0,
        },
        libc::pollfd {
            fd: owner,
            events: libc::POLLIN,
            revents: 0,
        },
        libc::pollfd {
            fd: peer,
            events: libc::POLLIN,
            revents: 0,
        },
        libc::pollfd {
            fd: -1,
            events: libc::POLLPRI | libc::POLLERR,
            revents: 0,
        },
    ];
    let mut retained = None;
    loop {
        let n = unsafe { libc::poll(poll.as_mut_ptr(), 4, -1) };
        if n < 0 && unsafe { *libc::__errno_location() } == libc::EINTR {
            continue;
        }
        if n < 0 || poll[0].revents != 0 || poll[1].revents != 0 {
            revoke(peer, &frame, retained, false);
        }
        if let Some(roles) = retained {
            if poll[2].revents != 0 || !inspect(&frame, [roles[0], roles[1], roles[3]], false) {
                revoke(peer, &frame, retained, true);
            }
            continue;
        }
        if poll[2].revents != libc::POLLIN {
            revoke(peer, &frame, None, true);
        }
        let mut message = receive(peer);
        if !message.matches(&frame.enroll, origin.pid, origin.uid, origin.gid, 3) {
            message.close();
            revoke(peer, &frame, None, true);
        }
        let roles = [message.rights[0], message.rights[1], message.rights[2]];
        if !inspect(&frame, roles, true) {
            message.close();
            revoke(peer, &frame, None, true);
        }
        // A distinct OFD per guardian: consuming one kernfs notification must
        // never acknowledge the other guardian's event. Retain the original
        // received event descriptor as well; it remains identity custody only.
        let watch = open(roles[0], c"cgroup.events".as_ptr(), false, false);
        if watch < 0 || !inspect(&frame, [roles[0], roles[1], watch], true) {
            if watch >= 0 {
                unsafe { libc::close(watch) };
            }
            message.close();
            revoke(peer, &frame, None, true);
        }
        for fd in message.rights.iter_mut().take(3) {
            *fd = -1;
        }
        message.close();
        retained = Some([roles[0], roles[1], roles[2], watch]); // BEFORE ACK
        poll[3].fd = watch;
        if stop_before_ack {
            unsafe { libc::syscall(libc::SYS_kill, libc::getpid(), libc::SIGSTOP) };
        }
        if terminal(timer)
            || terminal(owner)
            || terminal(peer)
            || !inspect(&frame, [roles[0], roles[1], watch], true)
        {
            revoke(peer, &frame, retained, true);
        }
        let mut reply = frame.enroll;
        reply[8] = 2;
        if !send(peer, &reply, &[]) {
            revoke(peer, &frame, retained, true);
        }
    }
}
