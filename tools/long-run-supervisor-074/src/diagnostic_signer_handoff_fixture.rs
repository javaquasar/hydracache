//! Private synthetic IPC; only the original traced child advances this protocol.
use super::*;
use std::sync::OnceLock;
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
    pub(super) fn arg(self) -> &'static std::ffi::CStr {
        match self {
            Self::Good => c"good",
            Self::WrongReady => c"wrong-ready",
            Self::DuplicateReady => c"duplicate-ready",
            Self::Short => c"short",
            Self::Oversized => c"oversized",
            Self::WrongFd => c"wrong-fd",
            Self::Early => c"early",
            Self::Leak => c"leak",
            Self::NoAck => c"no-ack",
            Self::WrongAck => c"wrong-ack",
            Self::DuplicateAck => c"duplicate-ack",
            Self::Closed => c"closed",
            Self::Exit => c"exit",
            Self::Ancillary => c"ancillary",
            Self::Replace => c"replace",
            Self::Rodata => c"rodata",
        }
    }
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
pub(super) struct Channel {
    socket: File,
    inode: [u64; 2],
    pub(super) child_inode: [u64; 2],
    pub(super) child_closed: bool,
    step: Step,
    attempted: bool,
    sends: u32,
}
pub(super) fn pair(_: Case) -> io::Result<(Channel, File)> {
    let mut fds = [-1; 2];
    // SAFETY: fixed private unnamed sockets; initialized two-FD output.
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
    // SAFETY: both fresh descriptors transferred exactly once.
    let (socket, child) = unsafe { (File::from_raw_fd(fds[0]), File::from_raw_fd(fds[1])) };
    let inode = identity(&socket)?;
    let child_inode = identity(&child)?;
    let channel = Channel {
        socket,
        inode,
        child_inode,
        child_closed: false,
        step: Step::Fresh,
        attempted: false,
        sends: 0,
    };
    channel.check()?;
    Ok((channel, child))
}
impl Channel {
    pub(super) fn check(&self) -> io::Result<()> {
        // SAFETY: fixed queries on this privately owned endpoint.
        let (flags, fd_flags) = unsafe {
            (
                libc::fcntl(self.socket.as_raw_fd(), libc::F_GETFL),
                libc::fcntl(self.socket.as_raw_fd(), libc::F_GETFD),
            )
        };
        if identity(&self.socket)? != self.inode
            || flags < 0
            || flags & libc::O_NONBLOCK == 0
            || fd_flags < 0
            || fd_flags & libc::FD_CLOEXEC == 0
        {
            return Err(invalid());
        }
        for (option, expected) in [
            (libc::SO_TYPE, libc::SOCK_SEQPACKET),
            (libc::SO_DOMAIN, libc::AF_UNIX),
        ] {
            let mut value = 0i32;
            let mut size = std::mem::size_of_val(&value) as libc::socklen_t;
            // SAFETY: correctly sized initialized integer output.
            if unsafe {
                libc::getsockopt(
                    self.socket.as_raw_fd(),
                    libc::SOL_SOCKET,
                    option,
                    (&mut value as *mut i32).cast(),
                    &mut size,
                )
            } != 0
                || size as usize != std::mem::size_of_val(&value)
                || value != expected
            {
                return Err(invalid());
            }
        }
        Ok(())
    }
    fn receive(&self, expected: &[u8; 16]) -> io::Result<()> {
        let mut bytes = [0; 16];
        // SAFETY: bounded buffer, private endpoint; ancillary send syscalls cannot run.
        let n = unsafe {
            libc::recv(
                self.socket.as_raw_fd(),
                bytes.as_mut_ptr().cast(),
                bytes.len(),
                libc::MSG_DONTWAIT | libc::MSG_TRUNC,
            )
        };
        if n != 16 || &bytes != expected {
            return Err(invalid());
        }
        Ok(())
    }
    fn empty(&self) -> io::Result<()> {
        let mut bytes = [0u8; 16];
        // SAFETY: bounded nonblocking peek, no mutable child state.
        let n = unsafe {
            libc::recv(
                self.socket.as_raw_fd(),
                bytes.as_mut_ptr().cast(),
                bytes.len(),
                libc::MSG_PEEK | libc::MSG_DONTWAIT | libc::MSG_TRUNC,
            )
        };
        if n != -1 || io::Error::last_os_error().raw_os_error() != Some(libc::EAGAIN) {
            return Err(invalid());
        }
        Ok(())
    }
}
pub(super) fn root() -> io::Result<Root> {
    // SAFETY: nonroot fixture identity only.
    let (uid, gid) = unsafe { (libc::geteuid(), libc::getegid()) };
    if uid == 0 || gid == 0 {
        return Err(invalid());
    }
    static IMAGES: OnceLock<Result<[Vec<u8>; 4], String>> = OnceLock::new();
    let images = IMAGES.get_or_init(|| {
        let prepare = || -> io::Result<[Vec<u8>; 4]> {
            Ok([
                namespace::compile_helper(
                    include_str!("diagnostic_signer_handoff_helper.rs"),
                    "handoff",
                )?,
                super::super::read_bounded(&File::open("/lib64/ld-linux-x86-64.so.2")?)?,
                super::super::read_bounded(&File::open("/lib/x86_64-linux-gnu/libc.so.6")?)?,
                super::super::read_bounded(&File::open("/lib/x86_64-linux-gnu/libgcc_s.so.1")?)?,
            ])
        };
        prepare().map_err(|e| e.to_string())
    });
    Root::materialize(
        images.as_ref().map_err(|e| io::Error::other(e.clone()))?,
        uid,
        gid,
    )
}
pub(super) fn channel_operation(nr: u64) -> bool {
    [
        libc::SYS_sendto,
        libc::SYS_recvfrom,
        libc::SYS_sendmsg,
        libc::SYS_recvmsg,
        libc::SYS_sendmmsg,
        libc::SYS_recvmmsg,
        libc::SYS_write,
        libc::SYS_writev,
        libc::SYS_read,
        libc::SYS_readv,
    ]
    .contains(&(nr as i64))
}
fn check(owner: &mut RootedExec, step: Step) -> io::Result<(usize, usize)> {
    if owner.phase != Phase::Installed || owner.channel.as_ref().ok_or_else(invalid)?.step != step {
        return Err(invalid());
    }
    owner.context(false)?;
    maps::validate(
        &text(format!("/proc/{}/maps", owner.pid))?,
        &owner.root,
        true,
    )
}
fn entry(owner: &mut RootedExec, nr: i64, flags: i32) -> io::Result<usize> {
    match owner.event()? {
        Event::Entry {
            arch: ARCH,
            nr: actual,
            args,
        } if actual == nr as u64
            && args[0] == 3
            && args[2] == 16
            && args[3] == flags as u64
            && args[4] == 0
            && args[5] == 0 =>
        {
            usize::try_from(args[1]).map_err(|_| invalid())
        }
        _ => Err(invalid()),
    }
}
fn exit(owner: &mut RootedExec, value: i64) -> io::Result<()> {
    if owner.event()?
        != (Event::Exit {
            arch: ARCH,
            value,
            error: false,
        })
    {
        return Err(invalid());
    }
    Ok(())
}
fn send_from_child(owner: &mut RootedExec, step: Step, bytes: &[u8; 16]) -> io::Result<()> {
    let bounds = check(owner, step)?;
    let pointer = entry(owner, libc::SYS_sendto, libc::MSG_NOSIGNAL)?;
    if owner.memory(bounds, pointer, 16)? != bytes {
        return Err(invalid());
    }
    exit(owner, 16)?;
    if owner.memory(bounds, pointer, 16)? != bytes {
        return Err(invalid());
    }
    owner.channel.as_ref().ok_or_else(invalid)?.receive(bytes)?;
    check(owner, step)?;
    Ok(())
}
fn guarded(
    owner: &mut RootedExec,
    operation: impl FnOnce(&mut RootedExec) -> io::Result<()>,
) -> io::Result<()> {
    let result = operation(owner);
    if result.is_err() {
        if let Some(channel) = &mut owner.channel {
            channel.step = Step::Refused;
        }
        owner.refuse()?;
    }
    result
}
fn ready(owner: &mut RootedExec) -> io::Result<()> {
    guarded(owner, |owner| {
        send_from_child(owner, Step::Fresh, READY)?;
        owner.channel.as_mut().ok_or_else(invalid)?.step = Step::Ready;
        Ok(())
    })
}
fn deliver(owner: &mut RootedExec) -> io::Result<()> {
    guarded(owner, |owner| {
        let bounds = check(owner, Step::Ready)?;
        let pointer = entry(owner, libc::SYS_recvfrom, libc::MSG_TRUNC)?;
        owner.memory(bounds, pointer, 16)?;
        check(owner, Step::Ready)?;
        let channel = owner.channel.as_mut().ok_or_else(invalid)?;
        if channel.attempted || channel.sends != 0 {
            return Err(invalid());
        }
        channel.empty()?;
        // A failure after this latch is uncertain, never retryable.
        channel.attempted = true;
        // SAFETY: one fixed public dummy packet to the original private endpoint.
        let n = unsafe {
            libc::send(
                channel.socket.as_raw_fd(),
                DUMMY.as_ptr().cast(),
                DUMMY.len(),
                libc::MSG_NOSIGNAL | libc::MSG_DONTWAIT,
            )
        };
        if n != 16 {
            return Err(invalid());
        }
        channel.sends += 1;
        exit(owner, 16)?;
        if owner.memory(bounds, pointer, 16)? != DUMMY {
            return Err(invalid());
        }
        check(owner, Step::Ready)?;
        owner.channel.as_mut().ok_or_else(invalid)?.step = Step::Delivered;
        Ok(())
    })
}
fn complete(owner: &mut RootedExec) -> io::Result<()> {
    guarded(owner, |owner| {
        send_from_child(owner, Step::Delivered, ACK)?;
        owner.channel.as_mut().ok_or_else(invalid)?.step = Step::Acked;
        match owner.event()? {
            Event::Entry {
                arch: ARCH,
                nr,
                args,
            } if nr == libc::SYS_close as u64 && args[0] == 3 => {}
            _ => return Err(invalid()),
        }
        exit(owner, 0)?;
        owner.channel.as_mut().ok_or_else(invalid)?.child_closed = true;
        check(owner, Step::Acked)?;
        match owner.event()? {
            Event::Entry {
                arch: ARCH,
                nr,
                args,
            } if nr == libc::SYS_exit_group as u64 && args[0] == 0 => {}
            _ => return Err(invalid()),
        }
        owner.trace(libc::PTRACE_CONT, 0)?;
        if owner.wait()? != 0 {
            return Err(invalid());
        }
        owner.root.check()?;
        owner.phase = Phase::Terminal;
        owner.channel.as_mut().ok_or_else(invalid)?.step = Step::Done;
        Ok(())
    })
}
fn owner(case: Case) -> io::Result<RootedExec> {
    let mut owner = RootedExec::handoff(case)?;
    owner
        .enter()
        .map_err(|e| io::Error::other(format!("handoff enter: {e}")))?;
    owner
        .first_exec()
        .map_err(|e| io::Error::other(format!("handoff first exec: {e}")))?;
    owner
        .install()
        .map_err(|e| io::Error::other(format!("handoff install: {e}")))?;
    Ok(owner)
}
fn run(owner: &mut RootedExec) -> io::Result<()> {
    ready(owner)?;
    deliver(owner)?;
    complete(owner)
}
#[path = "diagnostic_signer_tracer_loss_fixture.rs"]
mod loss;
#[path = "diagnostic_signer_handoff_tests.rs"]
mod tests;
