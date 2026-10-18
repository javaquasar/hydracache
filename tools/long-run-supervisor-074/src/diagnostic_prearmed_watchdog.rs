//! Private pre-armed mechanics, NOT original admission or whole-cgroup custody.
//! No public constructor, executable, evidence sink, unlock or completion API.
use super::{alarm, refused};
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::fs::MetadataExt;

const PENDING: u8 = 1;
const ENROLL: u8 = 2;
const ARMED: u8 = 3;
const REFUSED: u8 = 4;
const REVOKING: u8 = 5;
const MAGIC: &[u8; 8] = b"HCPG0741";
const MAX_RIGHTS: usize = 32;

fn packet(kind: u8) -> [u8; 16] {
    let mut bytes = [0; 16];
    bytes[..8].copy_from_slice(MAGIC);
    bytes[8] = kind;
    bytes
}
fn matches_packet(bytes: &[u8; 16], len: isize, flags: i32, kind: u8) -> bool {
    len == 16 && flags & !libc::MSG_CMSG_CLOEXEC == 0 && *bytes == packet(kind)
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct Origin {
    pid: i32,
    tid: i64,
    uid: u32,
    gid: u32,
    euid: u32,
    egid: u32,
}
impl Origin {
    fn current() -> Self {
        unsafe {
            Self {
                pid: libc::getpid(),
                tid: libc::syscall(libc::SYS_gettid),
                uid: libc::getuid(),
                gid: libc::getgid(),
                euid: libc::geteuid(),
                egid: libc::getegid(),
            }
        }
    }
}
struct Received {
    bytes: [u8; 16],
    len: isize,
    flags: i32,
    rights: [RawFd; MAX_RIGHTS],
    count: usize,
    credentials: Option<libc::ucred>,
    shape: bool,
}
impl Received {
    fn close(&mut self) {
        for fd in self.rights.iter_mut().take(self.count) {
            if *fd >= 0 {
                unsafe { libc::close(*fd) };
                *fd = -1;
            }
        }
    }
    fn valid(&self, kind: u8, pid: i32, uid: u32, gid: u32, rights: usize) -> bool {
        self.shape
            && matches_packet(&self.bytes, self.len, self.flags, kind)
            && self.count == rights
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
fn receive(socket: RawFd) -> Received {
    let mut received = Received {
        bytes: [0; 16],
        len: -1,
        flags: 0,
        rights: [-1; MAX_RIGHTS],
        count: 0,
        credentials: None,
        shape: true,
    };
    let mut control = [0usize; 16];
    let mut iov = libc::iovec {
        iov_base: received.bytes.as_mut_ptr().cast(),
        iov_len: 16,
    };
    let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
    msg.msg_iov = &mut iov;
    msg.msg_iovlen = 1;
    msg.msg_control = control.as_mut_ptr().cast();
    msg.msg_controllen = std::mem::size_of_val(&control);
    received.len = unsafe {
        libc::recvmsg(
            socket,
            &mut msg,
            libc::MSG_DONTWAIT | libc::MSG_CMSG_CLOEXEC | libc::MSG_TRUNC,
        )
    };
    received.flags = msg.msg_flags;
    if received.len < 0 {
        received.shape = false;
        return received;
    }
    unsafe {
        let mut header = libc::CMSG_FIRSTHDR(&msg);
        while !header.is_null() {
            let base = libc::CMSG_LEN(0) as usize;
            let offset = header as usize - msg.msg_control as usize;
            if offset > msg.msg_controllen
                || (*header).cmsg_len < base
                || (*header).cmsg_len > msg.msg_controllen - offset
            {
                received.shape = false;
                break;
            }
            let size = (*header).cmsg_len - base;
            if (*header).cmsg_level == libc::SOL_SOCKET && (*header).cmsg_type == libc::SCM_RIGHTS {
                if !size.is_multiple_of(std::mem::size_of::<RawFd>()) {
                    received.shape = false;
                }
                for i in 0..size / std::mem::size_of::<RawFd>() {
                    let fd = libc::CMSG_DATA(header)
                        .cast::<RawFd>()
                        .add(i)
                        .read_unaligned();
                    if received.count < MAX_RIGHTS {
                        received.rights[received.count] = fd;
                        received.count += 1;
                    } else {
                        libc::close(fd);
                        received.shape = false;
                    }
                }
            } else if (*header).cmsg_level == libc::SOL_SOCKET
                && (*header).cmsg_type == libc::SCM_CREDENTIALS
                && size == std::mem::size_of::<libc::ucred>()
                && received.credentials.is_none()
            {
                received.credentials = Some(
                    libc::CMSG_DATA(header)
                        .cast::<libc::ucred>()
                        .read_unaligned(),
                );
            } else {
                received.shape = false;
            }
            header = libc::CMSG_NXTHDR(&msg, header);
        }
    }
    received
}
fn send(socket: RawFd, kind: u8, right: Option<RawFd>) -> bool {
    let bytes = packet(kind);
    let mut iov = libc::iovec {
        iov_base: bytes.as_ptr().cast_mut().cast(),
        iov_len: bytes.len(),
    };
    let mut control = [0usize; 4];
    let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
    msg.msg_iov = &mut iov;
    msg.msg_iovlen = 1;
    if let Some(fd) = right {
        msg.msg_control = control.as_mut_ptr().cast();
        msg.msg_controllen =
            unsafe { libc::CMSG_SPACE(std::mem::size_of::<RawFd>() as _) } as usize;
        unsafe {
            let header = libc::CMSG_FIRSTHDR(&msg);
            (*header).cmsg_level = libc::SOL_SOCKET;
            (*header).cmsg_type = libc::SCM_RIGHTS;
            (*header).cmsg_len = libc::CMSG_LEN(std::mem::size_of::<RawFd>() as _) as usize;
            libc::CMSG_DATA(header).cast::<RawFd>().write_unaligned(fd);
        }
    }
    unsafe { libc::sendmsg(socket, &msg, libc::MSG_NOSIGNAL | libc::MSG_DONTWAIT) == 16 }
}
fn pair() -> io::Result<[File; 2]> {
    let mut fds = [-1; 2];
    if unsafe {
        libc::socketpair(
            libc::AF_UNIX,
            libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
            0,
            fds.as_mut_ptr(),
        )
    } != 0
    {
        return Err(io::Error::last_os_error());
    }
    let pair = unsafe { [File::from_raw_fd(fds[0]), File::from_raw_fd(fds[1])] };
    let on = 1i32;
    for file in &pair {
        if unsafe {
            libc::setsockopt(
                file.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PASSCRED,
                (&on as *const i32).cast(),
                std::mem::size_of_val(&on) as _,
            )
        } != 0
        {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(pair)
}
fn number(bytes: &[u8], key: &[u8]) -> Option<i32> {
    let mut value = None;
    for line in bytes.split(|b| *b == b'\n') {
        if let Some(digits) = line.strip_prefix(key) {
            if value.is_some() || digits.is_empty() {
                return None;
            }
            let mut n = 0i32;
            for byte in digits {
                if !byte.is_ascii_digit() {
                    return None;
                }
                n = n.checked_mul(10)?.checked_add((byte - b'0') as i32)?;
            }
            if n <= 0 {
                return None;
            }
            value = Some(n);
        }
    }
    value
}
// Stack-only procfs inspection. It is local mechanics, not a trusted original
// procfs/namespace admission. A final signal-0 check rejects a dead/reused PID.
fn proc_number(prefix: &[u8], id: i32, suffix: &[u8], key: &[u8]) -> Option<i32> {
    if id < 0 || prefix.len() + suffix.len() + 11 > 64 {
        return None;
    }
    let mut path = [0u8; 64];
    let mut digits = [0u8; 10];
    let mut n = id as u32;
    let mut count = 0;
    loop {
        digits[count] = b'0' + (n % 10) as u8;
        count += 1;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    path[..prefix.len()].copy_from_slice(prefix);
    for i in 0..count {
        path[prefix.len() + i] = digits[count - i - 1];
    }
    let end = prefix.len() + count;
    path[end..end + suffix.len()].copy_from_slice(suffix);
    let fd = unsafe {
        libc::syscall(
            libc::SYS_openat,
            libc::AT_FDCWD,
            path.as_ptr(),
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            0u32,
        )
    };
    if fd < 0 {
        return None;
    }
    let mut buffer = [0u8; 4097];
    let mut used = 0;
    while used < buffer.len() {
        let read = unsafe {
            libc::read(
                fd as RawFd,
                buffer.as_mut_ptr().add(used).cast(),
                buffer.len() - used,
            )
        };
        if read == 0 {
            break;
        }
        if read < 0 {
            unsafe { libc::close(fd as RawFd) };
            return None;
        }
        used += read as usize;
    }
    if unsafe { libc::close(fd as RawFd) } != 0 || used > 4096 {
        return None;
    }
    number(&buffer[..used], key)
}
fn live_child(fd: RawFd, owner: i32) -> bool {
    let Some(pid) = proc_number(b"/proc/self/fdinfo/", fd, b"", b"Pid:\t") else {
        return false;
    };
    pid != owner
        && proc_number(b"/proc/", pid, b"/status", b"PPid:\t") == Some(owner)
        && unsafe { libc::syscall(libc::SYS_pidfd_send_signal, fd, 0i32, 0usize, 0u32) } == 0
}
fn park() -> ! {
    loop {
        unsafe { libc::poll(std::ptr::null_mut(), 0, -1) };
    }
}
fn isolate(keep: &[RawFd]) -> bool {
    let mut first = 0u32;
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
fn terminal(fd: RawFd) -> bool {
    let mut poll = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    unsafe { libc::poll(&mut poll, 1, 0) != 0 }
}
fn end(peer: RawFd, target: Option<RawFd>) -> ! {
    let kind = if let Some(fd) = target {
        // Attempt only; never claim all writers or even terminal here.
        let result =
            unsafe { libc::syscall(libc::SYS_pidfd_send_signal, fd, libc::SIGKILL, 0usize, 0u32) };
        if result == 0 || unsafe { *libc::__errno_location() } == libc::ESRCH {
            REVOKING
        } else {
            REFUSED
        }
    } else {
        REFUSED
    };
    send(peer, kind, None);
    // Close queued rights too; no undisclosed writable aliases remain queued.
    unsafe { libc::close(peer) };
    park()
}
fn child(
    keep: &[RawFd],
    peer: RawFd,
    timer: RawFd,
    owner_pin: RawFd,
    origin: Origin,
    stop_before_ack: bool,
) -> ! {
    if !isolate(keep) || terminal(timer) || terminal(owner_pin) {
        end(peer, None);
    }
    if !send(peer, PENDING, None) {
        end(peer, None);
    }
    let mut target = None;
    let mut poll = [
        libc::pollfd {
            fd: timer,
            events: libc::POLLIN,
            revents: 0,
        },
        libc::pollfd {
            fd: owner_pin,
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
            events: libc::POLLIN,
            revents: 0,
        },
    ];
    loop {
        let n = unsafe { libc::poll(poll.as_mut_ptr(), 4, -1) };
        if n < 0 && unsafe { *libc::__errno_location() } == libc::EINTR {
            continue;
        }
        if n < 0 || poll[0].revents != 0 || poll[1].revents != 0 || poll[3].revents != 0 {
            end(peer, target);
        }
        if poll[2].revents != libc::POLLIN || target.is_some() {
            end(peer, target);
        }
        let mut message = receive(peer);
        let valid = message.valid(ENROLL, origin.pid, origin.uid, origin.gid, 1);
        if !valid {
            message.close();
            end(peer, None);
        }
        let fd = message.rights[0];
        if !live_child(fd, origin.pid) {
            message.close();
            end(peer, None);
        }
        // Retain before ACK; every subsequent refusal owns this exact target.
        target = Some(fd);
        message.rights[0] = -1;
        message.close();
        poll[3].fd = fd;
        if stop_before_ack {
            unsafe { libc::syscall(libc::SYS_kill, libc::getpid(), libc::SIGSTOP) };
        }
        if terminal(timer) || terminal(owner_pin) || terminal(peer) {
            end(peer, target);
        }
        if !send(peer, ARMED, None) {
            end(peer, target);
        }
    }
}
struct Watchdog {
    pid: i32,
    pinned: File,
    peer: File,
    alarm: File,
    origin: Origin,
    pending: bool,
    consumed: bool,
    armed: bool,
    refused: bool,
}
impl Watchdog {
    fn spawn(fence: File, deadline: u64, #[cfg(test)] stop_before_ack: bool) -> io::Result<Self> {
        let metadata = fence.metadata()?;
        let flags = unsafe { libc::fcntl(fence.as_raw_fd(), libc::F_GETFL) };
        if !metadata.is_file()
            || metadata.nlink() != 1
            || metadata.len() != 0
            || metadata.mode() & 0o7777 != 0o600
            || metadata.uid() != unsafe { libc::geteuid() }
            || flags < 0
            || flags & libc::O_ACCMODE != libc::O_RDWR
            || flags & (libc::O_APPEND | libc::O_PATH) != 0
            || unsafe { libc::flock(fence.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0
        {
            return Err(refused());
        }
        let origin = Origin::current();
        let pin = unsafe { libc::syscall(libc::SYS_pidfd_open, origin.pid, 0u32) };
        if pin < 0 {
            return Err(io::Error::last_os_error());
        }
        let owner = unsafe { File::from_raw_fd(pin as RawFd) };
        let timer = alarm(deadline)?;
        let [peer, send] = pair()?;
        let mut keep = [
            fence.as_raw_fd(),
            timer.as_raw_fd(),
            owner.as_raw_fd(),
            send.as_raw_fd(),
        ];
        keep.sort_unstable();
        let mut pinned = -1i32;
        let mut args = [0u64; 11];
        args[0] = libc::CLONE_PIDFD as u64;
        args[1] = (&mut pinned as *mut RawFd) as u64;
        args[4] = libc::SIGCHLD as u64;
        let blocked = u64::MAX;
        let mut previous = 0u64;
        if unsafe {
            libc::syscall(
                libc::SYS_rt_sigprocmask,
                libc::SIG_SETMASK,
                &blocked,
                &mut previous,
                8usize,
            )
        } != 0
        {
            return Err(io::Error::last_os_error());
        }
        let pid = unsafe {
            libc::syscall(
                libc::SYS_clone3,
                args.as_ptr(),
                std::mem::size_of_val(&args),
            )
        };
        if pid == 0 {
            #[cfg(not(test))]
            let stop_before_ack = false;
            child(
                &keep,
                send.as_raw_fd(),
                timer.as_raw_fd(),
                owner.as_raw_fd(),
                origin,
                stop_before_ack,
            );
        }
        let creation_error = (pid < 0).then(io::Error::last_os_error);
        let pin = (pid > 0).then(|| unsafe { File::from_raw_fd(pinned) });
        if unsafe {
            libc::syscall(
                libc::SYS_rt_sigprocmask,
                libc::SIG_SETMASK,
                &previous,
                0usize,
                8usize,
            )
        } != 0
        {
            return Err(io::Error::last_os_error());
        }
        if let Some(error) = creation_error {
            return Err(error);
        }
        // File close only: child retains the exclusive fence's OFD. No unlock.
        drop(fence);
        drop(send);
        drop(owner);
        Ok(Self {
            pid: pid as i32,
            pinned: pin.ok_or_else(refused)?,
            peer,
            alarm: timer,
            origin,
            pending: false,
            consumed: false,
            armed: false,
            refused: false,
        })
    }
    fn notice(&self, kind: u8) -> io::Result<()> {
        let mut poll = [
            libc::pollfd {
                fd: self.alarm.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: self.pinned.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: self.peer.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
        ];
        if unsafe { libc::poll(poll.as_mut_ptr(), 3, -1) } < 0
            || poll[..2].iter().any(|p| p.revents != 0)
            || poll[2].revents & libc::POLLIN == 0
        {
            return Err(refused());
        }
        let mut message = receive(self.peer.as_raw_fd());
        let valid = message.valid(kind, self.pid, self.origin.uid, self.origin.gid, 0);
        message.close();
        if !valid {
            return Err(refused());
        }
        Ok(())
    }
    fn pending(&mut self) -> io::Result<()> {
        if self.refused || self.pending || Origin::current() != self.origin {
            self.refused = true;
            return Err(refused());
        }
        if let Err(e) = self.notice(PENDING) {
            self.refused = true;
            return Err(e);
        }
        self.pending = true;
        Ok(())
    }
    fn enroll_once(&mut self, target: &File) -> io::Result<()> {
        if self.refused || !self.pending || self.consumed || Origin::current() != self.origin {
            self.refused = true;
            return Err(refused());
        }
        // Before even a failed send. No reset, unknown-ACK retry or renewal.
        self.consumed = true;
        if !send(self.peer.as_raw_fd(), ENROLL, Some(target.as_raw_fd())) {
            self.refused = true;
            return Err(refused());
        }
        if let Err(e) = self.notice(ARMED) {
            self.refused = true;
            return Err(e);
        }
        self.armed = true;
        Ok(())
    }
}

#[cfg(test)]
#[path = "diagnostic_prearmed_watchdog_tests.rs"]
mod tests;
