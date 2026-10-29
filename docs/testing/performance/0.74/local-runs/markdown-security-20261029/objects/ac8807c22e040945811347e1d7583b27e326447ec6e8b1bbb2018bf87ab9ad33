//! Private ordinary-process transfer, NOT a raw post-clone guardian.
//! Allocations/destructors here must never run in an inherited guardian child.
//! No ACK emitter, timer, fence, original-owner admission or execution API.
use super::super::{inspect_packet, Kind, PacketClaims, RootBindingClaims, PACKET_BYTES};
use super::*;
use std::os::fd::RawFd;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Creator {
    pid: i32,
    tid: i64,
    uid: u32,
    gid: u32,
    euid: u32,
    egid: u32,
}
impl Creator {
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
    fn matches(&self, credentials: &libc::ucred) -> bool {
        credentials.pid == self.pid && credentials.uid == self.uid && credentials.gid == self.gid
    }
}
fn socket_pair(creator: Creator) -> io::Result<[File; 2]> {
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
    for file in &pair {
        let on = 1i32;
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
        let mut peer: libc::ucred = unsafe { std::mem::zeroed() };
        let mut size = std::mem::size_of_val(&peer) as libc::socklen_t;
        if unsafe {
            libc::getsockopt(
                file.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                (&mut peer as *mut libc::ucred).cast(),
                &mut size,
            )
        } != 0
            || size as usize != std::mem::size_of_val(&peer)
            || peer.pid != creator.pid
            || peer.uid != creator.euid
            || peer.gid != creator.egid
        {
            return Err(refused());
        }
    }
    Ok(pair)
}
fn send(socket: &File, bytes: &[u8], rights: &[RawFd]) -> io::Result<()> {
    if rights.len() > 253 {
        return Err(refused());
    }
    let mut control = [0usize; 132];
    let mut iov = libc::iovec {
        iov_base: bytes.as_ptr().cast_mut().cast(),
        iov_len: bytes.len(),
    };
    let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
    msg.msg_iov = &mut iov;
    msg.msg_iovlen = 1;
    if !rights.is_empty() {
        let size = std::mem::size_of_val(rights);
        msg.msg_control = control.as_mut_ptr().cast();
        msg.msg_controllen = unsafe { libc::CMSG_SPACE(size as _) } as usize;
        unsafe {
            let header = libc::CMSG_FIRSTHDR(&msg);
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
    let result = unsafe {
        libc::sendmsg(
            socket.as_raw_fd(),
            &msg,
            libc::MSG_DONTWAIT | libc::MSG_NOSIGNAL,
        )
    };
    if result < 0 {
        return Err(io::Error::last_os_error());
    }
    if result as usize != bytes.len() {
        return Err(refused());
    }
    Ok(())
}
struct Frame {
    bytes: [u8; PACKET_BYTES],
    credentials: libc::ucred,
    rights: Vec<File>,
}
fn receive(socket: &File) -> io::Result<Frame> {
    let mut bytes = [0u8; PACKET_BYTES];
    // Intentionally bounded below SCM_MAX_FD; MSG_CTRUNC must refuse, while
    // every installed prefix right is still adopted and closed on that error.
    let mut control = [0usize; 32];
    let mut iov = libc::iovec {
        iov_base: bytes.as_mut_ptr().cast(),
        iov_len: bytes.len(),
    };
    let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
    msg.msg_iov = &mut iov;
    msg.msg_iovlen = 1;
    msg.msg_control = control.as_mut_ptr().cast();
    msg.msg_controllen = std::mem::size_of_val(&control);
    let len = unsafe {
        libc::recvmsg(
            socket.as_raw_fd(),
            &mut msg,
            libc::MSG_DONTWAIT | libc::MSG_CMSG_CLOEXEC | libc::MSG_TRUNC,
        )
    };
    if len < 0 {
        return Err(io::Error::last_os_error());
    }
    let mut credentials = None;
    let mut rights = Vec::new();
    let mut right_records = 0;
    let mut shape = len as usize == PACKET_BYTES && msg.msg_flags & !libc::MSG_CMSG_CLOEXEC == 0;
    unsafe {
        let mut header = libc::CMSG_FIRSTHDR(&msg);
        while !header.is_null() {
            let base = libc::CMSG_LEN(0) as usize;
            let offset = header as usize - msg.msg_control as usize;
            if offset > msg.msg_controllen
                || (*header).cmsg_len < base
                || (*header).cmsg_len > msg.msg_controllen - offset
            {
                shape = false;
                break;
            }
            let size = (*header).cmsg_len - base;
            if (*header).cmsg_level == libc::SOL_SOCKET && (*header).cmsg_type == libc::SCM_RIGHTS {
                right_records += 1;
                shape &= size > 0 && size.is_multiple_of(std::mem::size_of::<RawFd>());
                for i in 0..size / std::mem::size_of::<RawFd>() {
                    let fd = libc::CMSG_DATA(header)
                        .cast::<RawFd>()
                        .add(i)
                        .read_unaligned();
                    if fd < 0 {
                        shape = false;
                    } else {
                        rights.push(File::from_raw_fd(fd));
                    }
                }
            } else if (*header).cmsg_level == libc::SOL_SOCKET
                && (*header).cmsg_type == libc::SCM_CREDENTIALS
                && size == std::mem::size_of::<libc::ucred>()
                && credentials.is_none()
            {
                credentials = Some(
                    libc::CMSG_DATA(header)
                        .cast::<libc::ucred>()
                        .read_unaligned(),
                );
            } else {
                shape = false;
            }
            header = libc::CMSG_NXTHDR(&msg, header);
        }
    }
    if !shape || right_records != 1 || rights.len() != 3 {
        return Err(refused()); // RAII closes ALL installed rights, not just accepted roles.
    }
    Ok(Frame {
        bytes,
        credentials: credentials.ok_or_else(refused)?,
        rights,
    })
}
fn access(file: &File, mode: i32, directory: bool) -> io::Result<()> {
    let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) };
    let fd_flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFD) };
    if flags < 0
        || fd_flags != libc::FD_CLOEXEC
        || flags & (libc::O_ACCMODE | libc::O_PATH | libc::O_APPEND | libc::O_ASYNC) != mode
        || flags & libc::O_NONBLOCK == 0
        || (flags & libc::O_DIRECTORY != 0) != directory
    {
        return Err(refused());
    }
    Ok(())
}
/// Retained transport result only. There is deliberately no kill or ACK method.
struct TransferredRoot {
    directory: File,
    kill: File,
    events: File,
    identities: [Stamp; 3],
}
impl TransferredRoot {
    fn inspect_empty(&self) -> io::Result<()> {
        access(&self.directory, libc::O_RDONLY, true)?;
        access(&self.kill, libc::O_WRONLY, false)?;
        access(&self.events, libc::O_RDONLY, false)?;
        if !self.directory.metadata()?.is_dir()
            || !self.kill.metadata()?.is_file()
            || !self.events.metadata()?.is_file()
        {
            return Err(refused());
        }
        let kind = Document::open(&self.directory, c"cgroup.type", false)?;
        let procs = Document::open(&self.directory, c"cgroup.procs", false)?;
        for _ in 0..2 {
            for (file, identity) in [&self.directory, &self.kill, &self.events]
                .into_iter()
                .zip(self.identities)
            {
                if stamp(file)? != identity || identity.device != self.identities[0].device {
                    return Err(refused());
                }
            }
            for (name, write, expected) in [
                (c"cgroup.kill", true, self.identities[1]),
                (c"cgroup.events", false, self.identities[2]),
            ] {
                if stamp(&child(self.directory.as_fd(), name, false, write)?)? != expected {
                    return Err(refused());
                }
            }
            kind.revalidate(&self.directory)?;
            procs.revalidate(&self.directory)?;
            if bounded(&kind.file)? != b"domain\n"
                || !bounded(&procs.file)?.is_empty()
                || events(&bounded(&self.events)?)?
                    != (Observation {
                        populated: false,
                        frozen: false,
                    })
            {
                return Err(refused());
            }
        }
        Ok(())
    }
}
struct Sender<'a> {
    socket: File,
    creator: Creator,
    expected: RootBindingClaims,
    root: &'a mut PinnedRoot,
    consumed: bool,
}
struct Inbox {
    socket: File,
    creator: Creator,
    expected: RootBindingClaims,
    consumed: bool,
}
fn pair(root: &mut PinnedRoot, expected: RootBindingClaims) -> io::Result<(Sender<'_>, Inbox)> {
    PacketClaims {
        kind: Kind::Enroll,
        binding: expected,
    }
    .encode()
    .map_err(|_| refused())?;
    let observation = root.observe()?;
    if expected.root_device != root.identity.device
        || expected.root_inode != root.identity.inode
        || observation.populated
        || observation.frozen
    {
        return Err(refused());
    }
    let creator = Creator::current();
    let [sender, receiver] = socket_pair(creator)?;
    Ok((
        Sender {
            socket: sender,
            creator,
            expected,
            root,
            consumed: false,
        },
        Inbox {
            socket: receiver,
            creator,
            expected,
            consumed: false,
        },
    ))
}
impl Sender<'_> {
    fn send_once(&mut self) -> io::Result<()> {
        if self.consumed {
            return Err(refused());
        }
        self.consumed = true; // uncertainty BEFORE validation and the first syscall
        if Creator::current() != self.creator {
            return Err(refused());
        }
        let observation = self.root.observe()?;
        if observation.populated || observation.frozen {
            return Err(refused());
        }
        let bytes = PacketClaims {
            kind: Kind::Enroll,
            binding: self.expected,
        }
        .encode()
        .map_err(|_| refused())?;
        let sent = send(
            &self.socket,
            &bytes,
            &[
                self.root.directory.as_raw_fd(),
                self.root.kill.file.as_raw_fd(),
                self.root.events.file.as_raw_fd(),
            ],
        );
        let closing = self.root.observe()?;
        if Creator::current() != self.creator || closing.populated || closing.frozen {
            return Err(refused());
        }
        sent
    }
}
impl Inbox {
    fn receive_once(&mut self) -> io::Result<TransferredRoot> {
        if self.consumed {
            return Err(refused());
        }
        self.consumed = true; // no retry on EAGAIN, peer death or invalid input
        let frame = receive(&self.socket)?;
        if !self.creator.matches(&frame.credentials)
            || inspect_packet(&frame.bytes)
                != Ok(PacketClaims {
                    kind: Kind::Enroll,
                    binding: self.expected,
                })
        {
            return Err(refused());
        }
        let mut rights = frame.rights.into_iter();
        let directory = rights.next().ok_or_else(refused)?;
        let kill = rights.next().ok_or_else(refused)?;
        let events = rights.next().ok_or_else(refused)?;
        let identities = [stamp(&directory)?, stamp(&kill)?, stamp(&events)?];
        if identities[0].device != self.expected.root_device
            || identities[0].inode != self.expected.root_inode
        {
            return Err(refused());
        }
        let root = TransferredRoot {
            directory,
            kill,
            events,
            identities,
        };
        root.inspect_empty()?;
        Ok(root)
    }
}

#[cfg(test)]
#[path = "diagnostic_cgroup_root_transport_tests.rs"]
mod tests;
