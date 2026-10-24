//! Binary-private ordinary-root public dummy protocol, never signing authority.
use super::{
    file, isolation, maps, refused, require, FixedProbe, Owner, Phase, Stamp, EXEC, MARKER, STOP,
};
use isolation::Event;
use std::fs::File;
use std::io;
use std::os::fd::AsRawFd;

const READY: &[u8; 16] = b"HC074-READY-v001";
const DUMMY: &[u8; 16] = b"HC074-DUMMY-v001";
const ACK: &[u8; 16] = b"HC074-ACK---v001";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Case {
    Good,
    WrongReady,
    DuplicateReady,
    Short,
    Oversized,
    WrongFd,
    Early,
    Leak,
    NoAck,
    WrongAck,
    DuplicateAck,
    Closed,
    Exit,
    Ancillary,
    Replace,
    Rodata,
}
impl Case {
    fn parse(name: &str) -> io::Result<Self> {
        Ok(match name {
            "good" => Self::Good,
            "wrong-ready" => Self::WrongReady,
            "duplicate-ready" => Self::DuplicateReady,
            "short" => Self::Short,
            "oversized" => Self::Oversized,
            "wrong-fd" => Self::WrongFd,
            "early" => Self::Early,
            "leak" => Self::Leak,
            "no-ack" => Self::NoAck,
            "wrong-ack" => Self::WrongAck,
            "duplicate-ack" => Self::DuplicateAck,
            "closed" => Self::Closed,
            "exit" => Self::Exit,
            "ancillary" => Self::Ancillary,
            "replace" => Self::Replace,
            "rodata" => Self::Rodata,
            _ => return Err(refused()),
        })
    }
    pub(super) fn arg(self) -> &'static std::ffi::CStr {
        match self {
            Self::Good => c"--root-handoff-good",
            Self::WrongReady => c"--root-handoff-wrong-ready",
            Self::DuplicateReady => c"--root-handoff-duplicate-ready",
            Self::Short => c"--root-handoff-short",
            Self::Oversized => c"--root-handoff-oversized",
            Self::WrongFd => c"--root-handoff-wrong-fd",
            Self::Early => c"--root-handoff-early",
            Self::Leak => c"--root-handoff-leak",
            Self::NoAck => c"--root-handoff-no-ack",
            Self::WrongAck => c"--root-handoff-wrong-ack",
            Self::DuplicateAck => c"--root-handoff-duplicate-ack",
            Self::Closed => c"--root-handoff-closed",
            Self::Exit => c"--root-handoff-exit",
            Self::Ancillary => c"--root-handoff-ancillary",
            Self::Replace => c"--root-handoff-replace",
            Self::Rodata => c"--root-handoff-rodata",
        }
    }
    fn sent(self) -> u32 {
        u32::from(matches!(
            self,
            Self::Good | Self::NoAck | Self::WrongAck | Self::DuplicateAck
        ))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Run,
    Withhold,
    SendFail,
    Replay,
    KillInstalled,
    DropInstalled,
    KillReady,
    DropReady,
    KillDelivered,
    DropDelivered,
}
fn spec(mode: &str) -> io::Result<(Case, Action)> {
    let name = mode.strip_prefix("root-handoff-").ok_or_else(refused)?;
    let action = match name {
        "withhold" => Action::Withhold,
        "send-failure" => Action::SendFail,
        "replay" => Action::Replay,
        "kill-installed" => Action::KillInstalled,
        "drop-installed" => Action::DropInstalled,
        "kill-ready" => Action::KillReady,
        "drop-ready" => Action::DropReady,
        "kill-delivered" => Action::KillDelivered,
        "drop-delivered" => Action::DropDelivered,
        _ => return Ok((Case::parse(name)?, Action::Run)),
    };
    Ok((Case::Good, action))
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Fresh,
    Ready,
    Delivered,
    Acked,
    Done,
    Refused,
}
pub(super) struct State {
    original: Stamp,
    step: Step,
    attempted: bool,
    dispatched: bool,
    sends: u32,
    child_closed: bool,
    events: u8,
}
impl State {
    fn next_event(&mut self) -> io::Result<()> {
        require(self.events < 16)?;
        self.events += 1;
        Ok(())
    }
    pub(super) fn closed(&self) -> bool {
        self.child_closed
    }
    pub(super) fn check(&self, socket: &File) -> io::Result<()> {
        require(self.original.same_object(&Stamp::of(&socket.metadata()?)))?;
        socket_flags(socket)
    }
    fn attempt(&mut self) -> io::Result<()> {
        require(
            self.step == Step::Ready && !self.attempted && !self.dispatched && self.sends == 0,
        )?;
        self.attempted = true;
        Ok(())
    }
    fn dispatch(&mut self) -> io::Result<()> {
        require(self.step == Step::Ready && self.attempted && !self.dispatched && self.sends == 0)?;
        // This fact is terminal even if the actual send returns an error.
        self.dispatched = true;
        Ok(())
    }
    fn counts(&self, cut: u8) -> io::Result<()> {
        let (step, sends, attempted, dispatched, closed) = match cut {
            5 => (Step::Fresh, 0, false, false, false),
            6 | 7 => (Step::Ready, 0, false, false, false),
            8 => (Step::Delivered, 1, true, true, false),
            9 => (Step::Acked, 1, true, true, false),
            10 => (Step::Acked, 1, true, true, true),
            11 => (Step::Ready, 0, true, false, false),
            12 => (Step::Ready, 1, true, true, false),
            _ => return Err(refused()),
        };
        require(
            self.step == step
                && self.sends == sends
                && self.attempted == attempted
                && self.dispatched == dispatched
                && self.child_closed == closed,
        )
    }
}
pub(super) fn socket_flags(socket: &File) -> io::Result<()> {
    require(socket.metadata()?.file_type().is_socket())?;
    let flags = unsafe { libc::fcntl(socket.as_raw_fd(), libc::F_GETFL) };
    require(
        flags >= 0
            && flags & (libc::O_ACCMODE | libc::O_NONBLOCK) == (libc::O_RDWR | libc::O_NONBLOCK)
            && unsafe { libc::fcntl(socket.as_raw_fd(), libc::F_GETFD) } == libc::FD_CLOEXEC,
    )?;
    for (option, expected) in [
        (libc::SO_TYPE, libc::SOCK_SEQPACKET),
        (libc::SO_DOMAIN, libc::AF_UNIX),
    ] {
        let mut value = 0i32;
        let mut size = 4;
        require(
            unsafe {
                libc::getsockopt(
                    socket.as_raw_fd(),
                    libc::SOL_SOCKET,
                    option,
                    (&mut value as *mut i32).cast(),
                    &mut size,
                )
            } == 0
                && size == 4
                && value == expected,
        )?;
    }
    Ok(())
}
use std::os::unix::fs::FileTypeExt;
pub(super) type Transport = (File, File, Option<State>);
pub(super) fn transport(case: Option<Case>) -> io::Result<Transport> {
    let mut fds = [-1; 2];
    let result = if case.is_some() {
        unsafe {
            libc::socketpair(
                libc::AF_UNIX,
                libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
                0,
                fds.as_mut_ptr(),
            )
        }
    } else {
        unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC | libc::O_NONBLOCK) }
    };
    require(result == 0)?;
    let parent = file(fds[0])?;
    let child = file(fds[1])?;
    let state = if case.is_some() {
        socket_flags(&parent)?;
        socket_flags(&child)?;
        Some(State {
            original: Stamp::of(&parent.metadata()?),
            step: Step::Fresh,
            attempted: false,
            dispatched: false,
            sends: 0,
            child_closed: false,
            events: 0,
        })
    } else {
        None
    };
    Ok((parent, child, state))
}
fn event(owner: &mut Owner) -> io::Result<Event> {
    if owner.phase == Phase::Installed {
        let channel = owner.handoff.as_mut().ok_or_else(refused)?;
        channel.next_event()?;
    }
    owner.trace(libc::PTRACE_SYSCALL, 0)?;
    require(owner.wait()? == ((libc::SIGTRAP | 0x80) << 8) | 0x7f)?;
    info(owner)
}
fn info(owner: &Owner) -> io::Result<Event> {
    let mut info: libc::ptrace_syscall_info = unsafe { std::mem::zeroed() };
    let n = unsafe {
        libc::ptrace(
            libc::PTRACE_GET_SYSCALL_INFO,
            owner.pid,
            std::mem::size_of_val(&info),
            &mut info,
        )
    };
    require(n >= 0)?;
    isolation::decode(&info, n as usize)
}
fn returned(owner: &mut Owner, expected: i64) -> io::Result<()> {
    require(
        event(owner)?
            == Event::Exit {
                value: expected,
                error: false,
            },
    )
}
fn channel_operation(nr: u64) -> bool {
    [
        libc::SYS_sendto,
        libc::SYS_recvfrom,
        libc::SYS_sendmsg,
        libc::SYS_recvmsg,
        libc::SYS_sendmmsg,
        libc::SYS_recvmmsg,
        libc::SYS_read,
        libc::SYS_write,
        libc::SYS_readv,
        libc::SYS_writev,
        libc::SYS_close,
        libc::SYS_shutdown,
        libc::SYS_ioctl,
    ]
    .contains(&(nr as i64))
}
fn startup_allowed(nr: u64, args: [u64; 6]) -> bool {
    if nr == libc::SYS_prctl as u64 {
        return args[..5] == [libc::PR_GET_NO_NEW_PRIVS as u64, 0, 0, 0, 0]
            || (args[0] == libc::PR_GET_PDEATHSIG as u64 && args[1] != 0 && args[2..5] == [0; 3]);
    }
    if args[0] == 3
        && (channel_operation(nr)
            || (nr == libc::SYS_fcntl as u64
                && ![libc::F_GETFD as u64, libc::F_GETFL as u64].contains(&args[1])))
    {
        return false;
    }
    [
        libc::SYS_brk,
        libc::SYS_mmap,
        libc::SYS_mprotect,
        libc::SYS_munmap,
        libc::SYS_access,
        libc::SYS_openat,
        libc::SYS_newfstatat,
        libc::SYS_read,
        libc::SYS_fstat,
        libc::SYS_pread64,
        libc::SYS_close,
        libc::SYS_arch_prctl,
        libc::SYS_set_tid_address,
        libc::SYS_set_robust_list,
        libc::SYS_rseq,
        libc::SYS_prlimit64,
        libc::SYS_poll,
        libc::SYS_rt_sigaction,
        libc::SYS_getrandom,
        libc::SYS_sigaltstack,
        libc::SYS_getuid,
        libc::SYS_getgid,
        libc::SYS_geteuid,
        libc::SYS_getegid,
        libc::SYS_capget,
        libc::SYS_fcntl,
        libc::SYS_write,
        libc::SYS_gettid,
        libc::SYS_futex,
        libc::SYS_clock_gettime,
        libc::SYS_lseek,
        libc::SYS_readlink,
        libc::SYS_readlinkat,
        libc::SYS_getcwd,
        libc::SYS_statx,
    ]
    .contains(&(nr as i64))
}
pub(super) fn loaded(owner: &mut Owner) -> io::Result<()> {
    require(owner.phase == Phase::Exec && owner.handoff.is_some())?;
    // The first syscall-stop after an exec event may be exec's exit.
    let mut next = event(owner)?;
    if next
        == (Event::Exit {
            value: 0,
            error: false,
        })
    {
        next = event(owner)?;
    }
    let mut marker = false;
    let mut killed = false;
    for _ in 0..256 {
        let Event::Entry { nr, args } = next else {
            return Err(refused());
        };
        if nr == libc::SYS_write as u64 && args[0] == 3 {
            require(!marker && args[2] == 16)?;
            owner.stack = Some(maps::validate(&owner.text("maps")?, &owner.root, true)?);
            require(isolation::memory(owner, args[1] as usize, 16)? == MARKER)?;
            returned(owner, 16)?;
            marker = true;
        } else if marker {
            if nr == libc::SYS_getpid as u64 && !killed {
                returned(owner, owner.pid as i64)?;
            } else {
                require(
                    nr == libc::SYS_kill as u64
                        && !killed
                        && args[0] == owner.pid as u64
                        && args[1] == libc::SIGSTOP as u64,
                )?;
                returned(owner, 0)?;
                killed = true;
            }
        } else {
            require(startup_allowed(nr, args))?;
            require(matches!(event(owner)?, Event::Exit { .. }))?;
        }
        owner.trace(libc::PTRACE_SYSCALL, 0)?;
        let status = owner.wait()?;
        if status == STOP {
            return require(marker && killed);
        }
        require(status != EXEC && status == ((libc::SIGTRAP | 0x80) << 8) | 0x7f)?;
        next = info(owner)?;
    }
    Err(refused())
}
fn state(owner: &mut Owner, expected: Step) -> io::Result<()> {
    require(
        owner.phase == Phase::Installed
            && owner.handoff.as_ref().ok_or_else(refused)?.step == expected,
    )?;
    owner
        .handoff
        .as_ref()
        .ok_or_else(refused)?
        .check(&owner.pipe)?;
    let mut item = libc::pollfd {
        fd: owner.pidfd.as_ref().ok_or_else(refused)?.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    require(unsafe { libc::poll(&mut item, 1, 0) } == 0 && item.revents == 0)
}
fn context(owner: &mut Owner) -> io::Result<()> {
    owner.context(false)?;
    require(Some(maps::validate(&owner.text("maps")?, &owner.root, true)?) == owner.stack)
}
fn empty(socket: &File) -> io::Result<()> {
    let mut bytes = [0u8; 16];
    require(
        unsafe {
            libc::recv(
                socket.as_raw_fd(),
                bytes.as_mut_ptr().cast(),
                16,
                libc::MSG_PEEK | libc::MSG_DONTWAIT | libc::MSG_TRUNC,
            )
        } == -1
            && io::Error::last_os_error().raw_os_error() == Some(libc::EAGAIN),
    )
}
fn receive(socket: &File, expected: &[u8; 16]) -> io::Result<()> {
    let mut bytes = [0; 16];
    require(
        unsafe {
            libc::recv(
                socket.as_raw_fd(),
                bytes.as_mut_ptr().cast(),
                16,
                libc::MSG_DONTWAIT | libc::MSG_TRUNC,
            )
        } == 16
            && &bytes == expected,
    )
}
fn entry(owner: &mut Owner, nr: i64, flags: i32) -> io::Result<usize> {
    entry_pointer(event(owner)?, nr, flags)
}
fn entry_pointer(event: Event, nr: i64, flags: i32) -> io::Result<usize> {
    let Event::Entry { nr: actual, args } = event else {
        return Err(refused());
    };
    require(
        actual == nr as u64
            && args[0] == 3
            && args[2] == 16
            && args[3] == flags as u64
            && args[4..] == [0; 2],
    )?;
    usize::try_from(args[1]).map_err(|_| refused())
}
fn outgoing(owner: &mut Owner, step: Step, bytes: &[u8; 16]) -> io::Result<()> {
    state(owner, step)?;
    let pointer = entry(owner, libc::SYS_sendto, libc::MSG_NOSIGNAL)?;
    require(isolation::memory(owner, pointer, 16)? == bytes)?;
    returned(owner, 16)?;
    require(isolation::memory(owner, pointer, 16)? == bytes)?;
    receive(&owner.pipe, bytes)?;
    state(owner, step)
}
fn ready(owner: &mut Owner) -> io::Result<()> {
    outgoing(owner, Step::Fresh, READY)?;
    owner.handoff.as_mut().ok_or_else(refused)?.step = Step::Ready;
    Ok(())
}
fn awaiting_read(owner: &mut Owner) -> io::Result<usize> {
    state(owner, Step::Ready)?;
    let pointer = entry(owner, libc::SYS_recvfrom, libc::MSG_TRUNC)?;
    isolation::memory(owner, pointer, 16)?;
    state(owner, Step::Ready)?;
    context(owner)?;
    empty(&owner.pipe)?;
    Ok(pointer)
}
fn deliver(owner: &mut Owner, failed_send: bool) -> io::Result<()> {
    let pointer = awaiting_read(owner)?;
    send_at_read(owner, pointer, failed_send)
}
fn send_at_read(owner: &mut Owner, pointer: usize, failed_send: bool) -> io::Result<()> {
    let channel = owner.handoff.as_mut().ok_or_else(refused)?;
    channel.attempt()?;
    send_latched(owner, failed_send)?;
    receive_at_read(owner, pointer)
}
fn send_latched(owner: &mut Owner, failed_send: bool) -> io::Result<()> {
    state(owner, Step::Ready)?;
    owner.handoff.as_mut().ok_or_else(refused)?.dispatch()?;
    if failed_send {
        require(unsafe { libc::shutdown(owner.pipe.as_raw_fd(), libc::SHUT_WR) } == 0)?;
    }
    let n = unsafe {
        libc::send(
            owner.pipe.as_raw_fd(),
            DUMMY.as_ptr().cast(),
            16,
            libc::MSG_NOSIGNAL | libc::MSG_DONTWAIT,
        )
    };
    require(n == 16)?;
    owner.handoff.as_mut().ok_or_else(refused)?.sends += 1;
    Ok(())
}
fn receive_at_read(owner: &mut Owner, pointer: usize) -> io::Result<()> {
    state(owner, Step::Ready)?;
    let channel = owner.handoff.as_ref().ok_or_else(refused)?;
    require(channel.attempted && channel.dispatched && channel.sends == 1)?;
    returned(owner, 16)?;
    require(isolation::memory(owner, pointer, 16)? == DUMMY)?;
    state(owner, Step::Ready)?;
    owner.handoff.as_mut().ok_or_else(refused)?.step = Step::Delivered;
    Ok(())
}
fn complete(owner: &mut Owner) -> io::Result<()> {
    acknowledge(owner)?;
    close(owner)?;
    finish(owner)
}
fn acknowledge(owner: &mut Owner) -> io::Result<()> {
    outgoing(owner, Step::Delivered, ACK)?;
    owner.handoff.as_mut().ok_or_else(refused)?.step = Step::Acked;
    Ok(())
}
fn close(owner: &mut Owner) -> io::Result<()> {
    state(owner, Step::Acked)?;
    require(!owner.handoff.as_ref().ok_or_else(refused)?.closed())?;
    require(
        matches!(event(owner)?, Event::Entry { nr, args } if nr == libc::SYS_close as u64 && args[0] == 3),
    )?;
    returned(owner, 0)?;
    owner.handoff.as_mut().ok_or_else(refused)?.child_closed = true;
    state(owner, Step::Acked)?;
    context(owner)?;
    Ok(())
}
fn finish(owner: &mut Owner) -> io::Result<()> {
    state(owner, Step::Acked)?;
    require(owner.handoff.as_ref().ok_or_else(refused)?.closed())?;
    require(
        matches!(event(owner)?, Event::Entry { nr, args } if nr == libc::SYS_exit_group as u64 && args[0] == 0),
    )?;
    owner.trace(libc::PTRACE_CONT, 0)?;
    require(owner.wait()? == 0)?;
    owner.handoff.as_mut().ok_or_else(refused)?.step = Step::Done;
    owner.phase = Phase::Terminal;
    Ok(())
}
// A private continuation is obtained only by observing the original trace.
// In particular Receive holds the exact original stack pointer, not caller input.
pub(super) enum Checkpoint {
    Installed,
    Ready,
    Receive(usize),
    Latched(usize),
    Sent(usize),
    Delivered,
    Acked,
    Closed,
}
pub(super) fn pause(owner: &mut Owner, cut: u8) -> io::Result<Checkpoint> {
    require((5..=12).contains(&cut))?;
    state(owner, Step::Fresh)?;
    if cut == 5 {
        return Ok(Checkpoint::Installed);
    }
    ready(owner)?;
    if cut == 6 {
        return Ok(Checkpoint::Ready);
    }
    if cut == 7 {
        return Ok(Checkpoint::Receive(awaiting_read(owner)?));
    }
    if matches!(cut, 11 | 12) {
        let pointer = awaiting_read(owner)?;
        owner.handoff.as_mut().ok_or_else(refused)?.attempt()?;
        if cut == 11 {
            return Ok(Checkpoint::Latched(pointer));
        }
        send_latched(owner, false)?;
        return Ok(Checkpoint::Sent(pointer));
    }
    deliver(owner, false)?;
    if cut == 8 {
        return Ok(Checkpoint::Delivered);
    }
    acknowledge(owner)?;
    if cut == 9 {
        return Ok(Checkpoint::Acked);
    }
    close(owner)?;
    Ok(Checkpoint::Closed)
}
pub(super) fn counts(owner: &Owner, cut: u8) -> io::Result<()> {
    owner.handoff.as_ref().ok_or_else(refused)?.counts(cut)
}
pub(super) fn resume(owner: &mut Owner, checkpoint: Checkpoint) -> io::Result<()> {
    match checkpoint {
        Checkpoint::Installed => {
            ready(owner)?;
            deliver(owner, false)?;
            complete(owner)?;
        }
        Checkpoint::Ready => {
            deliver(owner, false)?;
            complete(owner)?;
        }
        Checkpoint::Receive(pointer) => {
            send_at_read(owner, pointer, false)?;
            complete(owner)?;
        }
        Checkpoint::Latched(pointer) => {
            send_latched(owner, false)?;
            receive_at_read(owner, pointer)?;
            complete(owner)?;
        }
        Checkpoint::Sent(pointer) => {
            receive_at_read(owner, pointer)?;
            complete(owner)?;
        }
        Checkpoint::Delivered => complete(owner)?,
        Checkpoint::Acked => {
            close(owner)?;
            finish(owner)?;
        }
        Checkpoint::Closed => finish(owner)?,
    }
    owner.terminate()?;
    let channel = owner.handoff.as_ref().ok_or_else(refused)?;
    require(
        owner.phase == Phase::Terminal
            && owner.terminal == Some(0)
            && channel.step == Step::Done
            && channel.attempted
            && channel.dispatched
            && channel.sends == 1,
    )
}
pub(super) fn send_window_terminal(socket: &File, cut: u8, mode: u8) -> io::Result<()> {
    match (cut, mode) {
        (11, 0 | 1 | 3) | (12, 1) => {}
        (12, 0) => {
            // Only this original sent-but-unread loss must discard pending input.
            // ECONNRESET is a required transport observation, never EOF proof.
            let mut bytes = [0; 17];
            require(
                unsafe {
                    libc::recv(
                        socket.as_raw_fd(),
                        bytes.as_mut_ptr().cast(),
                        bytes.len(),
                        libc::MSG_DONTWAIT | libc::MSG_TRUNC,
                    )
                } == -1
                    && io::Error::last_os_error().raw_os_error() == Some(libc::ECONNRESET),
            )?;
        }
        (12, 3) => receive(socket, ACK)?,
        _ => return Err(refused()),
    }
    let mut extra = [0; 17];
    require(
        unsafe {
            libc::recv(
                socket.as_raw_fd(),
                extra.as_mut_ptr().cast(),
                extra.len(),
                libc::MSG_PEEK | libc::MSG_DONTWAIT | libc::MSG_TRUNC,
            )
        } == 0,
    )?;
    drain_socket(socket)
}
pub(super) fn drain(owner: &Owner) -> io::Result<()> {
    drain_socket(&owner.pipe)
}
pub(super) fn drain_socket(socket: &File) -> io::Result<()> {
    let mut total = 0;
    let mut eof = false;
    // Exact remaining diagnostic messages are bounded, even on refusal.
    for _ in 0..5 {
        let mut bytes = [0; 17];
        let n = unsafe {
            libc::recv(
                socket.as_raw_fd(),
                bytes.as_mut_ptr().cast(),
                bytes.len(),
                libc::MSG_DONTWAIT | libc::MSG_TRUNC,
            )
        };
        require(n >= 0)?;
        if n == 0 {
            // Zero-length records are not EOF on a live seqpacket endpoint.
            let mut item = libc::pollfd {
                fd: socket.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            require(
                unsafe { libc::poll(&mut item, 1, 0) } == 1
                    && item.revents & libc::POLLHUP != 0
                    && item.revents & !(libc::POLLHUP | libc::POLLIN) == 0,
            )?;
            eof = true;
            continue;
        }
        require(!eof)?;
        total += n;
        require(total <= 48)?;
    }
    // Fixed observed writer can have at most four records including markers;
    // five receives must cross EOF, even if a zero record preceded HUP.
    require(eof)
}
fn cleanup(mut owner: Owner, ordinary_drop: bool) -> io::Result<()> {
    let pid = owner.pid;
    let pidfd = owner.pidfd.as_ref().ok_or_else(refused)?.try_clone()?;
    let parent = owner.root.parent.try_clone()?;
    let name = owner.root.name.clone();
    if ordinary_drop {
        owner.root.check()?;
    }
    if !ordinary_drop {
        owner.terminate()?;
        owner.root.check()?;
        owner.root.cleanup()?;
    }
    drop(owner);
    let mut ready = libc::pollfd {
        fd: pidfd.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    require(
        unsafe { libc::poll(&mut ready, 1, 0) } == 1
            && ready.revents & libc::POLLIN != 0
            && ready.revents & !(libc::POLLIN | libc::POLLHUP) == 0,
    )?;
    let mut status = 0;
    require(
        unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG | libc::__WALL) } == -1
            && io::Error::last_os_error().raw_os_error() == Some(libc::ECHILD),
    )?;
    let mut stat: libc::stat = unsafe { std::mem::zeroed() };
    require(
        unsafe {
            libc::fstatat(
                parent.as_raw_fd(),
                name.as_ptr(),
                &mut stat,
                libc::AT_SYMLINK_NOFOLLOW,
            )
        } == -1
            && io::Error::last_os_error().raw_os_error() == Some(libc::ENOENT),
    )
}

pub(super) fn run(mode: &str) -> io::Result<()> {
    let (case, action) = spec(mode)?;
    let mut owner = Owner::build_profile(
        super::super::acquire()?,
        Some(FixedProbe::Native),
        Some(case),
    )?;
    let prepared = (|| {
        owner.advance(Phase::Initial, Owner::enter)?;
        owner.advance(Phase::Rooted, Owner::first_exec)?;
        owner.advance(Phase::Exec, Owner::loaded)?;
        owner.advance(Phase::Loaded, isolation::install)
    })();
    if matches!(case, Case::Early | Case::Leak) {
        require(prepared.is_err())?;
        return cleanup(owner, false);
    }
    prepared?;
    if matches!(action, Action::KillInstalled | Action::DropInstalled) {
        return cleanup(owner, action == Action::DropInstalled);
    }
    let result = (|| {
        ready(&mut owner)?;
        if matches!(action, Action::KillReady | Action::DropReady) {
            return Ok(());
        }
        if action == Action::Withhold {
            awaiting_read(&mut owner)?;
            return Ok(());
        }
        if action == Action::Replay {
            return ready(&mut owner);
        }
        deliver(&mut owner, action == Action::SendFail)?;
        if matches!(action, Action::KillDelivered | Action::DropDelivered) {
            return Ok(());
        }
        complete(&mut owner)
    })();
    let channel = owner.handoff.as_mut().ok_or_else(refused)?;
    let sent = if action == Action::Run {
        case.sent()
    } else if matches!(action, Action::KillDelivered | Action::DropDelivered) {
        1
    } else {
        0
    };
    require(
        channel.sends == sent && channel.attempted == (sent == 1 || action == Action::SendFail),
    )?;
    let expected_error = (action == Action::Run && case != Case::Good)
        || matches!(action, Action::Replay | Action::SendFail);
    require(result.is_err() == expected_error)?;
    if result.is_err() {
        channel.step = Step::Refused;
        owner.phase = Phase::Refused;
    }
    cleanup(
        owner,
        matches!(action, Action::DropReady | Action::DropDelivered),
    )
}
pub(super) fn child(mode: &str) -> ! {
    let case = match mode
        .strip_prefix("--root-handoff-")
        .and_then(|s| Case::parse(s).ok())
    {
        Some(case) => case,
        None => unsafe { libc::_exit(92) },
    };
    let mut ready = [0; 17];
    ready[..16].copy_from_slice(READY);
    let mut received = [0; 16];
    let mut ack = *ACK;
    if case == Case::WrongReady {
        ready[0] ^= 1;
    }
    if case == Case::WrongAck {
        ack[0] ^= 1;
    }
    if super::child_assertions(true).is_err() {
        unsafe { libc::_exit(92) }
    }
    unsafe {
        if case == Case::Early {
            libc::syscall(
                libc::SYS_sendto,
                3i32,
                ready.as_ptr(),
                16usize,
                libc::MSG_NOSIGNAL,
                0usize,
                0usize,
            );
        }
        if case == Case::Leak {
            libc::dup2(0, 5);
        }
    }
    if super::child_loaded(true).is_err() {
        unsafe { libc::_exit(92) }
    }
    isolation::install_child();
    let length = if case == Case::Short {
        15
    } else if case == Case::Oversized {
        17
    } else {
        16
    };
    let fd = if case == Case::WrongFd { 1 } else { 3 };
    unsafe {
        if case == Case::Closed {
            libc::close(3);
        }
        if case == Case::Exit {
            libc::_exit(72);
        }
        if case == Case::Ancillary {
            libc::syscall(libc::SYS_sendmsg, 3i32, 0usize, 0i32);
        }
        if case == Case::Replace {
            libc::dup2(0, 3);
        }
        let pointer = if case == Case::Rodata {
            READY.as_ptr()
        } else {
            ready.as_ptr()
        };
        if libc::syscall(
            libc::SYS_sendto,
            fd,
            pointer,
            length as usize,
            libc::MSG_NOSIGNAL,
            0usize,
            0usize,
        ) != length
        {
            libc::_exit(73);
        }
        if case == Case::DuplicateReady {
            libc::syscall(
                libc::SYS_sendto,
                3i32,
                ready.as_ptr(),
                16usize,
                libc::MSG_NOSIGNAL,
                0usize,
                0usize,
            );
        }
        if libc::syscall(
            libc::SYS_recvfrom,
            3i32,
            received.as_mut_ptr(),
            16usize,
            libc::MSG_TRUNC,
            0usize,
            0usize,
        ) != 16
            || &received != DUMMY
        {
            libc::_exit(74);
        }
        if case == Case::NoAck {
            libc::_exit(75);
        }
        if libc::syscall(
            libc::SYS_sendto,
            3i32,
            ack.as_ptr(),
            16usize,
            libc::MSG_NOSIGNAL,
            0usize,
            0usize,
        ) != 16
        {
            libc::_exit(76);
        }
        if case == Case::DuplicateAck {
            libc::syscall(
                libc::SYS_sendto,
                3i32,
                ack.as_ptr(),
                16usize,
                libc::MSG_NOSIGNAL,
                0usize,
                0usize,
            );
        }
        if libc::close(3) != 0 {
            libc::_exit(77);
        }
        libc::_exit(0);
    }
}

#[cfg(test)]
#[path = "diagnostic_signer_owned_root_handoff_tests.rs"]
mod tests;
