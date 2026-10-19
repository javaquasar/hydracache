//! Private local pair mechanics; never original admission or completion.
use super::super::{Kind, PacketClaims, RootBindingClaims, PACKET_BYTES};
use super::*;
use std::os::fd::RawFd;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
#[derive(Clone, Copy)]
struct Snapshot {
    parent: RawFd,
    parent_identity: Stamp,
    name: [u8; 129],
    roles: [Stamp; 3],
    kind: Stamp,
    procs: Stamp,
    enroll: [u8; PACKET_BYTES],
}
impl Snapshot {
    fn capture(root: &mut PinnedRoot, binding: RootBindingClaims) -> io::Result<Self> {
        let state = root.observe()?;
        if state.populated
            || state.frozen
            || binding.root_device != root.identity.device
            || binding.root_inode != root.identity.inode
        {
            return Err(refused());
        }
        let mut name = [0; 129];
        let bytes = root.name.to_bytes();
        if bytes.len() > 128 {
            return Err(refused());
        }
        name[..bytes.len()].copy_from_slice(bytes);
        Ok(Self {
            parent: root.parent.as_raw_fd(),
            parent_identity: root.parent_identity,
            name,
            roles: [root.identity, root.kill.identity, root.events.identity],
            kind: root.kind.identity,
            procs: root.procs.identity,
            enroll: PacketClaims {
                kind: Kind::Enroll,
                binding,
            }
            .encode()
            .map_err(|_| refused())?,
        })
    }
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
    for socket in &pair {
        let on = 1i32;
        if unsafe {
            libc::setsockopt(
                socket.as_raw_fd(),
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
fn keep(mut fds: [RawFd; 5]) -> io::Result<[RawFd; 5]> {
    fds.sort_unstable();
    if fds[0] < 0 || fds.windows(2).any(|w| w[0] == w[1]) {
        return Err(refused());
    }
    Ok(fds)
}
fn boottime() -> io::Result<u64> {
    let mut t: libc::timespec = unsafe { std::mem::zeroed() };
    if unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut t) } != 0
        || t.tv_sec < 0
        || t.tv_nsec < 0
    {
        return Err(refused());
    }
    (t.tv_sec as u64)
        .checked_mul(1_000_000_000)
        .and_then(|n| n.checked_add(t.tv_nsec as u64))
        .ok_or_else(refused)
}
fn inspect_timer(timer: &File, deadline: u64) -> io::Result<()> {
    let before = boottime()?;
    let information = std::fs::read_to_string(format!("/proc/self/fdinfo/{}", timer.as_raw_fd()))?;
    let after = boottime()?;
    if information.len() > 4096
        || !information
            .lines()
            .filter(|l| l.starts_with("clockid:"))
            .eq(["clockid: 7"])
        || !information
            .lines()
            .filter(|l| l.starts_with("settime flags:"))
            .eq(["settime flags: 01"])
        || !information
            .lines()
            .filter(|l| l.starts_with("it_interval:"))
            .eq(["it_interval: (0, 0)"])
        || unsafe { libc::fcntl(timer.as_raw_fd(), libc::F_GETFD) } != libc::FD_CLOEXEC
    {
        return Err(refused());
    }
    // fdinfo reads remaining/interval without forwarding an expired periodic
    // timer. Even a gettime syscall can forward one; do not call it at all.
    let remaining = remaining_from_info(&information).ok_or_else(refused)?;
    let low = before.checked_add(remaining).ok_or_else(refused)?;
    let high = after.checked_add(remaining).ok_or_else(refused)?;
    if remaining == 0 || deadline < low || deadline > high || raw::terminal(timer.as_raw_fd()) {
        return Err(refused());
    }
    Ok(())
}
fn remaining_from_info(information: &str) -> Option<u64> {
    let mut lines = information.lines().filter(|l| l.starts_with("it_value:"));
    let value = lines
        .next()?
        .strip_prefix("it_value: (")?
        .strip_suffix(')')?;
    if lines.next().is_some() {
        return None;
    }
    let (seconds, nanos) = value.split_once(", ")?;
    let number = |s: &str| -> Option<u64> {
        if s.is_empty()
            || (s.len() > 1 && s.starts_with('0'))
            || !s.bytes().all(|b| b.is_ascii_digit())
        {
            return None;
        }
        s.parse().ok()
    };
    let seconds = number(seconds)?;
    let nanos = number(nanos)?;
    if nanos >= 1_000_000_000 {
        return None;
    }
    seconds.checked_mul(1_000_000_000)?.checked_add(nanos)
}
struct Guard {
    pid: i32,
    pin: File,
    peer: File,
    consumed: bool,
}
impl Guard {
    fn cancel_pending(&mut self) -> io::Result<()> {
        if self.pid == 0 {
            return Ok(());
        }
        loop {
            let result = unsafe {
                libc::syscall(
                    libc::SYS_pidfd_send_signal,
                    self.pin.as_raw_fd(),
                    libc::SIGKILL,
                    0usize,
                    0u32,
                )
            };
            if result == 0 || unsafe { *libc::__errno_location() } == libc::ESRCH {
                break;
            }
            if unsafe { *libc::__errno_location() } != libc::EINTR {
                return Err(io::Error::last_os_error());
            }
        }
        loop {
            let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
            if unsafe {
                libc::waitid(
                    libc::P_PIDFD,
                    self.pin.as_raw_fd() as _,
                    &mut info,
                    libc::WEXITED,
                )
            } == 0
            {
                self.pid = 0;
                return Ok(());
            }
            if unsafe { *libc::__errno_location() } != libc::EINTR {
                return Err(io::Error::last_os_error());
            }
        }
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        // Before any send there is no target. Afterwards, peer close asks the
        // independent guardian to revoke; it must NOT be killed by parent Drop.
        if !self.consumed {
            let _ = self.cancel_pending();
        }
    }
}
fn spawn_one(
    fence: BorrowedFd<'_>,
    timer: &File,
    origin: Origin,
    frame: Snapshot,
    #[cfg(test)] fault: u8,
) -> io::Result<Guard> {
    let pin = unsafe { libc::syscall(libc::SYS_pidfd_open, origin.pid, 0u32) };
    if pin < 0 {
        return Err(io::Error::last_os_error());
    }
    let owner = unsafe { File::from_raw_fd(pin as _) };
    let [peer, child_peer] = pair()?;
    let keep = keep([
        fence.as_raw_fd(),
        timer.as_raw_fd(),
        owner.as_raw_fd(),
        child_peer.as_raw_fd(),
        frame.parent,
    ])?;
    let mut child_pin = -1i32;
    let mut args = [0u64; 11];
    args[0] = libc::CLONE_PIDFD as u64;
    args[1] = (&mut child_pin as *mut RawFd) as u64;
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
    let clone_child = || unsafe {
        libc::syscall(
            libc::SYS_clone3,
            args.as_ptr(),
            std::mem::size_of_val(&args),
        )
    };
    #[cfg(test)]
    let pid = if fault == 2 {
        unsafe { *libc::__errno_location() = libc::EAGAIN };
        -1
    } else {
        clone_child()
    };
    #[cfg(not(test))]
    let pid = clone_child();
    if pid == 0 {
        #[cfg(test)]
        let stop_before_ack = fault == 1;
        #[cfg(not(test))]
        let stop_before_ack = false;
        raw::child(
            keep,
            child_peer.as_raw_fd(),
            timer.as_raw_fd(),
            owner.as_raw_fd(),
            origin,
            frame,
            stop_before_ack,
        );
    }
    let error = (pid < 0).then(io::Error::last_os_error);
    // Own cancellation before restoring signals. A newly created child cannot
    // yet have received a root; neither failing restore nor second clone leaks it.
    let owned = (pid > 0).then(|| Guard {
        pid: pid as _,
        pin: unsafe { File::from_raw_fd(child_pin) },
        peer,
        consumed: false,
    });
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
    #[cfg(test)]
    if fault == 3 {
        return Err(io::Error::from_raw_os_error(libc::EIO));
    }
    if let Some(error) = error {
        return Err(error);
    }
    owned.ok_or_else(refused)
}
pub(super) struct RootPair<'a> {
    guards: [Guard; 2],
    root: &'a mut PinnedRoot,
    alarm: File,
    origin: Origin,
    frame: Snapshot,
    consumed: bool,
    armed: bool,
    refused: bool,
}
impl<'a> RootPair<'a> {
    pub(super) fn spawn(
        fence: BorrowedFd<'_>,
        alarm: BorrowedFd<'_>,
        root: &'a mut PinnedRoot,
        binding: RootBindingClaims,
        #[cfg(test)] fault: u8,
    ) -> io::Result<Self> {
        let origin = Origin::current();
        let fence_file = File::from(fence.try_clone_to_owned()?);
        let m = fence_file.metadata()?;
        let flags = unsafe { libc::fcntl(fence_file.as_raw_fd(), libc::F_GETFL) };
        let fence_info =
            std::fs::read_to_string(format!("/proc/self/fdinfo/{}", fence_file.as_raw_fd()))?;
        if !m.is_file()
            || m.len() != 0
            || m.nlink() != 1
            || m.mode() & 0o7777 != 0o600
            || m.uid() != origin.euid
            || m.gid() != origin.egid
            || flags < 0
            || flags & libc::O_ACCMODE != libc::O_RDWR
            || flags & (libc::O_PATH | libc::O_APPEND) != 0
            || fence_info.len() > 4096
            || fence_info
                .lines()
                .filter(|l| l.starts_with("lock:") && l.contains("FLOCK") && l.contains("WRITE"))
                .count()
                != 1
        {
            return Err(refused());
        }
        let alarm = File::from(alarm.try_clone_to_owned()?);
        inspect_timer(&alarm, binding.deadline_boottime_ns)?;
        let frame = Snapshot::capture(root, binding)?;
        let first = spawn_one(
            fence_file.as_fd(),
            &alarm,
            origin,
            frame,
            #[cfg(test)]
            {
                match fault {
                    1 => 1,
                    3 => 2,
                    5 => 3,
                    _ => 0,
                }
            },
        )?;
        let second = spawn_one(
            fence_file.as_fd(),
            &alarm,
            origin,
            frame,
            #[cfg(test)]
            {
                match fault {
                    2 => 1,
                    4 => 2,
                    6 => 3,
                    _ => 0,
                }
            },
        )?;
        let result = Self {
            guards: [first, second],
            root,
            alarm,
            origin,
            frame,
            consumed: false,
            armed: false,
            refused: false,
        };
        result.live()?;
        Ok(result)
    }
    fn live(&self) -> io::Result<()> {
        if Origin::current() != self.origin
            || raw::terminal(self.alarm.as_raw_fd())
            || self.guards.iter().any(|g| {
                g.pid == 0 || raw::terminal(g.pin.as_raw_fd()) || raw::terminal(g.peer.as_raw_fd())
            })
        {
            return Err(refused());
        }
        Ok(())
    }
    fn fail(&mut self) {
        self.refused = true;
        for g in &self.guards {
            unsafe { libc::shutdown(g.peer.as_raw_fd(), libc::SHUT_RDWR) };
        }
    }
    fn notice(&self, index: usize) -> io::Result<()> {
        let g = &self.guards[index];
        let mut poll = [
            libc::pollfd {
                fd: self.alarm.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: g.pin.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: g.peer.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
        ];
        loop {
            let n = unsafe { libc::poll(poll.as_mut_ptr(), 3, -1) };
            if n < 0 && unsafe { *libc::__errno_location() } == libc::EINTR {
                continue;
            }
            if n <= 0
                || poll[0].revents != 0
                || poll[1].revents != 0
                || poll[2].revents != libc::POLLIN
            {
                return Err(refused());
            }
            break;
        }
        let mut message = raw::receive(g.peer.as_raw_fd());
        let mut expected = self.frame.enroll;
        expected[8] = 2;
        let valid = message.matches(&expected, g.pid, self.origin.uid, self.origin.gid, 0);
        message.close();
        if !valid {
            return Err(refused());
        }
        Ok(())
    }
    pub(super) fn enroll_once(&mut self) -> io::Result<()> {
        if self.consumed || self.refused {
            self.fail();
            return Err(refused());
        }
        self.consumed = true;
        let result = self.submit_and_ack();
        if result.is_err() {
            self.fail();
        }
        result
    }
    fn submit_and_ack(&mut self) -> io::Result<()> {
        self.live()?;
        let observed = self.root.observe()?;
        if observed.populated || observed.frozen {
            return Err(refused());
        }
        // Consume BOTH before even the first send; never cancel a possible root
        // custodian in Drop, including failed-send and partial-enrollment paths.
        for guard in &mut self.guards {
            guard.consumed = true;
        }
        let rights = [
            self.root.directory.as_raw_fd(),
            self.root.kill.file.as_raw_fd(),
            self.root.events.file.as_raw_fd(),
        ];
        for guard in &self.guards {
            if !raw::send(guard.peer.as_raw_fd(), &self.frame.enroll, &rights) {
                return Err(refused());
            }
        }
        // Both requests precede waiting for either ACK. One stopped child must
        // not prevent the other from retaining a root and its independent alarm.
        #[cfg(test)]
        creator::cut(self, 5);
        self.notice(0)?;
        #[cfg(test)]
        creator::cut(self, 1);
        self.notice(1)?;
        #[cfg(test)]
        creator::cut(self, 2);
        self.live()?;
        let closing = self.root.observe()?;
        if closing.populated || closing.frozen {
            return Err(refused());
        }
        self.armed = true;
        #[cfg(test)]
        creator::cut(self, 3);
        Ok(())
    }
    #[cfg(test)]
    pub(super) fn signal_for_fixture(&self, index: usize, signal: i32) {
        assert_eq!(
            unsafe {
                libc::syscall(
                    libc::SYS_pidfd_send_signal,
                    self.guards[index].pin.as_raw_fd(),
                    signal,
                    0usize,
                    0u32,
                )
            },
            0
        );
    }
    #[cfg(test)]
    pub(super) fn cleanup_for_fixture(&mut self) {
        for guard in &mut self.guards {
            guard.cancel_pending().unwrap();
        }
    }
    #[cfg(test)]
    pub(super) fn state_for_fixture(&self) -> (bool, bool, bool) {
        (self.consumed, self.armed, self.refused)
    }
    #[cfg(test)]
    pub(super) fn disconnect_for_fixture(&mut self) {
        self.fail();
    }
    #[cfg(test)]
    pub(super) fn pids_for_fixture(&self) -> [i32; 2] {
        self.guards.each_ref().map(|g| g.pid)
    }
    #[cfg(test)]
    pub(super) fn terminal_for_fixture(&self, index: usize) -> bool {
        raw::terminal(self.guards[index].pin.as_raw_fd())
    }
}

#[path = "diagnostic_root_guardian_raw.rs"]
mod raw;
#[cfg(test)]
#[path = "diagnostic_root_guardian_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "diagnostic_root_creator_tests.rs"]
mod creator;

#[path = "diagnostic_root_guardian_original_kernel.rs"]
pub(crate) mod original;
