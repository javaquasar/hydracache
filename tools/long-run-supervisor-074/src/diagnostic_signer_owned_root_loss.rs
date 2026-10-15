//! Fixed normal-build stopped-creator-loss experiment, never authority.
use super::{
    bounded, file, isolation, open, refused, require, FixedProbe, Owner, Phase, Root, STOP,
};
use std::fs::File;
use std::io::{self, Read};
use std::os::fd::AsRawFd;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Spec {
    cut: u8,
    mode: u8,
}
impl Spec {
    fn parse(mode: &str) -> io::Result<Self> {
        let (cut, mode) = match mode {
            "root-loss-initial" => (0, 0),
            "root-loss-rooted" => (1, 0),
            "root-loss-exec" => (2, 0),
            "root-loss-loaded" => (3, 0),
            "root-loss-installed" => (4, 0),
            "root-loss-healthy-initial" => (0, 1),
            "root-loss-healthy-installed" => (4, 1),
            "root-loss-no-exitkill-installed" => (4, 2),
            _ => return Err(refused()),
        };
        Ok(Self { cut, mode })
    }
    fn frame(self, pid: i32, stage: u8) -> [u8; 16] {
        let mut raw = [0; 16];
        raw[..8].copy_from_slice(b"HC074RL1");
        raw[8..12].copy_from_slice(&pid.to_le_bytes());
        raw[12] = self.cut;
        raw[13] = self.mode;
        raw[14] = stage;
        raw
    }
    fn decode(self, raw: &[u8], stage: u8) -> io::Result<i32> {
        require(raw.len() == 16 && stage <= 1)?;
        let pid = i32::from_le_bytes(raw[8..12].try_into().map_err(|_| refused())?);
        require(pid > 0 && raw == self.frame(pid, stage))?;
        Ok(pid)
    }
}
fn poll(fd: &File, events: i16) -> io::Result<i16> {
    let mut item = libc::pollfd {
        fd: fd.as_raw_fd(),
        events,
        revents: 0,
    };
    require(
        unsafe { libc::poll(&mut item, 1, 5000) } == 1
            && item.revents & events != 0
            && item.revents & !(events | libc::POLLHUP) == 0,
    )?;
    Ok(item.revents)
}
fn nonterminal(fd: &File) -> io::Result<()> {
    let mut item = libc::pollfd {
        fd: fd.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    require(unsafe { libc::poll(&mut item, 1, 0) } == 0 && item.revents == 0)
}
struct Channel {
    file: File,
    sent: u8,
    received: u8,
    peer: i32,
}
impl Channel {
    fn new(file: File, peer: i32) -> Self {
        Self {
            file,
            peer,
            sent: 0,
            received: 0,
        }
    }
    fn send(&mut self, bytes: &[u8], fds: &[i32]) -> io::Result<()> {
        require(self.sent < 4 && [1, 16].contains(&bytes.len()) && [0, 2].contains(&fds.len()))?;
        self.sent += 1;
        poll(&self.file, libc::POLLOUT)?;
        let mut control = [0usize; 16];
        let mut iov = libc::iovec {
            iov_base: bytes.as_ptr().cast_mut().cast(),
            iov_len: bytes.len(),
        };
        let mut message: libc::msghdr = unsafe { std::mem::zeroed() };
        message.msg_iov = &mut iov;
        message.msg_iovlen = 1;
        if !fds.is_empty() {
            message.msg_control = control.as_mut_ptr().cast();
            message.msg_controllen =
                unsafe { libc::CMSG_SPACE(std::mem::size_of_val(fds) as u32) as usize };
            unsafe {
                let header = libc::CMSG_FIRSTHDR(&message);
                (*header).cmsg_level = libc::SOL_SOCKET;
                (*header).cmsg_type = libc::SCM_RIGHTS;
                (*header).cmsg_len = libc::CMSG_LEN(std::mem::size_of_val(fds) as u32) as usize;
                std::ptr::copy_nonoverlapping(
                    fds.as_ptr().cast::<u8>(),
                    libc::CMSG_DATA(header),
                    std::mem::size_of_val(fds),
                );
            }
        }
        require(
            unsafe {
                libc::sendmsg(
                    self.file.as_raw_fd(),
                    &message,
                    libc::MSG_NOSIGNAL | libc::MSG_DONTWAIT,
                )
            } == bytes.len() as isize,
        )
    }
    fn receive(&mut self, bytes: usize, descriptors: usize) -> io::Result<(Vec<u8>, Vec<File>)> {
        require(self.received < 4 && [1, 16].contains(&bytes) && [0, 2].contains(&descriptors))?;
        self.received += 1;
        poll(&self.file, libc::POLLIN)?;
        let mut raw = [0u8; 17];
        let mut control = [0usize; 16];
        let mut iov = libc::iovec {
            iov_base: raw.as_mut_ptr().cast(),
            iov_len: raw.len(),
        };
        let mut message: libc::msghdr = unsafe { std::mem::zeroed() };
        message.msg_iov = &mut iov;
        message.msg_iovlen = 1;
        message.msg_control = control.as_mut_ptr().cast();
        message.msg_controllen = 128;
        let got = unsafe {
            libc::recvmsg(
                self.file.as_raw_fd(),
                &mut message,
                libc::MSG_CMSG_CLOEXEC | libc::MSG_DONTWAIT,
            )
        };
        require(got >= 0)?;
        let mut fds = Vec::new();
        let mut credentials = None;
        let mut valid = message.msg_controllen <= std::mem::size_of_val(&control);
        // Collect every returned rights FD into RAII before rejecting the packet.
        let mut header = unsafe { libc::CMSG_FIRSTHDR(&message) };
        while !header.is_null() {
            let start = header as usize - control.as_ptr() as usize;
            let length = unsafe { (*header).cmsg_len };
            if length < unsafe { libc::CMSG_LEN(0) as usize }
                || start
                    .checked_add(length)
                    .is_none_or(|end| end > message.msg_controllen)
            {
                valid = false;
                break;
            }
            let data = unsafe { libc::CMSG_DATA(header) };
            let count = length - unsafe { libc::CMSG_LEN(0) as usize };
            match unsafe { ((*header).cmsg_level, (*header).cmsg_type) } {
                (libc::SOL_SOCKET, libc::SCM_RIGHTS) => {
                    valid &= count.is_multiple_of(std::mem::size_of::<i32>());
                    for offset in (0..count / 4).map(|i| i * 4) {
                        let fd =
                            unsafe { std::ptr::read_unaligned(data.add(offset).cast::<i32>()) };
                        fds.push(file(fd)?);
                    }
                }
                (libc::SOL_SOCKET, libc::SCM_CREDENTIALS)
                    if count == std::mem::size_of::<libc::ucred>() && credentials.is_none() =>
                {
                    credentials =
                        Some(unsafe { std::ptr::read_unaligned(data.cast::<libc::ucred>()) });
                }
                _ => valid = false,
            }
            header = unsafe { libc::CMSG_NXTHDR(&message, header) };
        }
        let cred = credentials.ok_or_else(refused)?;
        require(
            valid
                && message.msg_flags & (libc::MSG_TRUNC | libc::MSG_CTRUNC) == 0
                && got as usize == bytes
                && fds.len() == descriptors
                && cred.pid == self.peer
                && cred.uid == unsafe { libc::getuid() }
                && cred.gid == unsafe { libc::getgid() },
        )?;
        for fd in &fds {
            require(unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_GETFD) } == libc::FD_CLOEXEC)?;
        }
        Ok((raw[..got as usize].to_vec(), fds))
    }
    fn gate(&mut self, expected: u8) -> io::Result<()> {
        require(self.receive(1, 0)?.0 == [expected])
    }
}
fn pair() -> io::Result<[File; 2]> {
    let mut fds = [-1; 2];
    require(
        unsafe {
            libc::socketpair(
                libc::AF_UNIX,
                libc::SOCK_SEQPACKET | libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
                0,
                fds.as_mut_ptr(),
            )
        } == 0,
    )?;
    let pair = [file(fds[0])?, file(fds[1])?];
    let enabled = 1i32;
    for fd in &pair {
        require(
            unsafe {
                libc::setsockopt(
                    fd.as_raw_fd(),
                    libc::SOL_SOCKET,
                    libc::SO_PASSCRED,
                    (&enabled as *const i32).cast(),
                    4,
                )
            } == 0,
        )?;
    }
    Ok(pair)
}
fn signal(fd: &File) -> io::Result<()> {
    require(
        unsafe {
            libc::syscall(
                libc::SYS_pidfd_send_signal,
                fd.as_raw_fd(),
                libc::SIGKILL,
                std::ptr::null::<libc::siginfo_t>(),
                0u32,
            )
        } == 0,
    )
}
fn wait_info(fd: &File, flags: i32) -> io::Result<(i32, i32, i32)> {
    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
    if unsafe {
        libc::waitid(
            libc::P_PIDFD,
            fd.as_raw_fd() as u32,
            &mut info,
            libc::WEXITED | libc::WNOHANG | flags,
        )
    } != 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok((info.si_code, unsafe { info.si_status() }, unsafe {
        info.si_pid()
    }))
}
struct Process {
    pid: i32,
    fd: Option<File>,
    reaped: bool,
    direct: bool,
}
impl Process {
    fn direct(pid: i32) -> io::Result<Self> {
        require(pid > 0)?;
        let mut own = Self {
            pid,
            fd: None,
            reaped: false,
            direct: true,
        };
        own.fd = Some(file(unsafe {
            libc::syscall(libc::SYS_pidfd_open, pid, 0u32) as i32
        })?);
        Ok(own)
    }
    fn fd(&self) -> io::Result<&File> {
        self.fd.as_ref().ok_or_else(refused)
    }
    fn wait(&mut self, code: i32, status: i32) -> io::Result<()> {
        poll(self.fd()?, libc::POLLIN)?;
        let expected = (code, status, self.pid);
        require(wait_info(self.fd()?, libc::WNOWAIT)? == expected)?;
        require(wait_info(self.fd()?, 0)? == expected)?;
        self.reaped = true;
        require(wait_info(self.fd()?, 0).is_err_and(|e| e.raw_os_error() == Some(libc::ECHILD)))
    }
    fn cleanup(&mut self) -> io::Result<()> {
        if self.reaped {
            return Ok(());
        }
        if let Some(fd) = &self.fd {
            let _ = signal(fd);
        } else {
            require(self.direct)?;
            unsafe {
                libc::kill(self.pid, libc::SIGKILL);
            }
        }
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            let mut status = 0;
            let result =
                unsafe { libc::waitpid(self.pid, &mut status, libc::WNOHANG | libc::__WALL) };
            if result == self.pid && (libc::WIFEXITED(status) || libc::WIFSIGNALED(status)) {
                self.reaped = true;
                return Ok(());
            }
            if result < 0 {
                return Err(io::Error::last_os_error());
            }
            require(Instant::now() < until)?;
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}
fn arm(parent: i32) {
    // SAFETY: sole-main fixture fork child only; no authority or new process API.
    unsafe {
        let mut value = 0;
        if libc::prctl(
            libc::PR_SET_PDEATHSIG,
            libc::SIGKILL as usize,
            0usize,
            0usize,
            0usize,
        ) != 0
            || libc::prctl(libc::PR_GET_PDEATHSIG, &mut value, 0usize, 0usize, 0usize) != 0
            || value != libc::SIGKILL
            || libc::getppid() != parent
        {
            libc::_exit(81);
        }
    }
}
fn unrelated() -> io::Result<Process> {
    let parent = unsafe { libc::getpid() };
    let pid = unsafe { libc::fork() };
    if pid == 0 {
        arm(parent);
        unsafe {
            if libc::syscall(libc::SYS_close_range, 3u32, u32::MAX, 0u32) != 0 {
                libc::_exit(82);
            }
            loop {
                libc::pause();
            }
        }
    }
    Process::direct(pid)
}
struct Reaper(i32);
fn subreaper() -> io::Result<i32> {
    let mut value = -1i32;
    require(
        unsafe {
            libc::prctl(
                libc::PR_GET_CHILD_SUBREAPER,
                &mut value,
                0usize,
                0usize,
                0usize,
            )
        } == 0,
    )?;
    Ok(value)
}
impl Reaper {
    fn new() -> io::Result<Self> {
        let old = subreaper()?;
        require(old == 0)?;
        let guard = Self(old);
        require(
            unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1usize, 0usize, 0usize, 0usize) }
                == 0
                && subreaper()? == 1,
        )?;
        Ok(guard)
    }
    fn restore(&self) -> io::Result<()> {
        require(
            unsafe {
                libc::prctl(
                    libc::PR_SET_CHILD_SUBREAPER,
                    self.0 as usize,
                    0usize,
                    0usize,
                    0usize,
                )
            } == 0
                && subreaper()? == self.0,
        )
    }
}
impl Drop for Reaper {
    fn drop(&mut self) {
        if self.restore().is_err() {
            eprintln!("Local monitor subreaper restoration refused.");
        }
    }
}
struct Tree {
    root: Option<Root>,
    quiescent: bool,
}
impl Drop for Tree {
    fn drop(&mut self) {
        if !self.quiescent {
            if let Some(root) = &mut self.root {
                root.cleanup_attempted = true;
            }
        }
    }
}
fn scalar(raw: &str, key: &str) -> io::Result<i32> {
    let values: Vec<_> = raw
        .lines()
        .filter_map(|line| line.strip_prefix(key))
        .collect();
    require(values.len() == 1)?;
    let value = values[0].trim();
    require(!value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()))?;
    value.parse().map_err(|_| refused())
}
fn text(path: &str) -> io::Result<String> {
    bounded(open(path)?)
}
fn binding(creator: &Process, pid: i32, pidfd: &File, pipe: &File) -> io::Result<()> {
    nonterminal(creator.fd()?)?;
    nonterminal(pidfd)?;
    require(
        text(&format!(
            "/proc/{}/task/{}/children",
            creator.pid, creator.pid
        ))?
        .split_whitespace()
        .collect::<Vec<_>>()
            == [pid.to_string()],
    )?;
    let own = text(&format!("/proc/{}/status", creator.pid))?;
    require(
        scalar(&own, "Pid:")? == creator.pid
            && scalar(&own, "Tgid:")? == creator.pid
            && scalar(&own, "Threads:")? == 1
            && scalar(&own, "PPid:")? == unsafe { libc::getpid() },
    )?;
    let child = text(&format!("/proc/{pid}/status"))?;
    require(
        scalar(&child, "Pid:")? == pid
            && scalar(&child, "Tgid:")? == pid
            && scalar(&child, "PPid:")? == creator.pid
            && scalar(&child, "TracerPid:")? == creator.pid,
    )?;
    require(
        scalar(
            &text(&format!("/proc/self/fdinfo/{}", pidfd.as_raw_fd()))?,
            "Pid:",
        )? == pid,
    )?;
    // The child is still at its original pre-namespace stop. Open only a
    // metadata handle: opening a FIFO for reading could block or alter it.
    let path = std::ffi::CString::new(format!("/proc/{pid}/fd/3")).map_err(|_| refused())?;
    let writer = file(unsafe { libc::open(path.as_ptr(), libc::O_PATH | libc::O_CLOEXEC) })?;
    same_pipe(pipe, &writer)
}
fn same_pipe(pipe: &File, writer: &File) -> io::Result<()> {
    let flags = unsafe { libc::fcntl(pipe.as_raw_fd(), libc::F_GETFL) };
    use std::os::unix::fs::FileTypeExt;
    use std::os::unix::fs::MetadataExt;
    let read = pipe.metadata()?;
    let write = writer.metadata()?;
    require(
        read.file_type().is_fifo()
            && write.file_type().is_fifo()
            && read.dev() == write.dev()
            && read.ino() == write.ino()
            && flags >= 0
            && flags & (libc::O_ACCMODE | libc::O_NONBLOCK) == libc::O_NONBLOCK,
    )?;
    Ok(())
}
fn group_stop(owner: &mut Owner) -> io::Result<()> {
    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
    require(
        unsafe { libc::ptrace(libc::PTRACE_GETSIGINFO, owner.pid, 0usize, &mut info) } == 0
            && info.si_signo == libc::SIGSTOP,
    )?;
    owner.trace(libc::PTRACE_CONT, libc::SIGSTOP as usize)?;
    require(owner.wait()? == STOP)?;
    require(
        unsafe { libc::ptrace(libc::PTRACE_GETSIGINFO, owner.pid, 0usize, &mut info) } == -1
            && io::Error::last_os_error().raw_os_error() == Some(libc::EINVAL),
    )
}
fn creator(root: Root, endpoint: File, spec: Spec, parent: i32) -> ! {
    // Sole-main orchestration fork only. The worker raw fork path is unchanged.
    let mut owner = match Owner::build(root, Some(FixedProbe::Native)) {
        Ok(owner) => owner,
        Err(_) => unsafe { libc::_exit(91) },
    };
    let mut channel = Channel::new(endpoint, parent);
    let result = (|| -> io::Result<()> {
        channel.send(
            &spec.frame(owner.pid, 0),
            &[
                owner.pidfd.as_ref().ok_or_else(refused)?.as_raw_fd(),
                owner.pipe.as_raw_fd(),
            ],
        )?;
        channel.gate(b'G')?;
        if spec.cut >= 1 {
            owner.advance(Phase::Initial, Owner::enter)?;
        }
        if spec.cut >= 2 {
            owner.advance(Phase::Rooted, Owner::first_exec)?;
        }
        if spec.cut >= 3 {
            owner.advance(Phase::Exec, Owner::loaded)?;
        }
        if spec.cut >= 4 {
            owner.advance(Phase::Loaded, isolation::install)?;
        }
        if spec.cut == 4 && spec.mode != 1 {
            group_stop(&mut owner)?;
        }
        if spec.mode == 2 {
            owner.trace(
                libc::PTRACE_SETOPTIONS,
                (libc::PTRACE_O_TRACEEXEC | libc::PTRACE_O_TRACESYSGOOD) as usize,
            )?;
        }
        channel.send(&spec.frame(owner.pid, 1), &[])?;
        channel.gate(b'H')?;
        require(spec.mode == 1)?;
        owner.terminate()?;
        require(owner.terminal == Some(libc::SIGKILL))?;
        Ok(())
    })();
    // The monitor kept these same original root objects from BEFORE this fork.
    // It alone may clean after original creator/worker/stream quiescence.
    owner.root.cleanup_attempted = true;
    drop(owner);
    unsafe { libc::_exit(if result.is_ok() { 0 } else { 91 }) }
}
fn detached_stopped(raw: &str, monitor: i32) -> io::Result<bool> {
    require(scalar(raw, "PPid:")? == monitor && scalar(raw, "TracerPid:")? == 0)?;
    let states: Vec<_> = raw
        .lines()
        .filter_map(|line| line.strip_prefix("State:"))
        .collect();
    require(states.len() == 1)?;
    match states[0].trim() {
        "T (stopped)" => Ok(true),
        "R (running)" | "t (tracing stop)" => Ok(false),
        _ => Err(refused()),
    }
}
pub(super) fn run(mode: &str) -> io::Result<()> {
    let spec = Spec::parse(mode)?;
    let root = super::super::acquire()?;
    require(
        text(&format!("/proc/self/task/{}/children", unsafe {
            libc::getpid()
        }))?
        .trim()
        .is_empty(),
    )?;
    let reaper = Reaper::new()?;
    let mut tree = Tree {
        root: Some(root),
        quiescent: false,
    };
    let [reader, writer] = pair()?;
    let mut unrelated = unrelated()?;
    let parent = unsafe { libc::getpid() };
    let pid = unsafe { libc::fork() };
    if pid == 0 {
        arm(parent);
        drop(reader);
        creator(tree.root.take().unwrap(), writer, spec, parent);
    }
    let mut original = Process::direct(pid)?;
    drop(writer);
    let mut channel = Channel::new(reader, pid);
    let mut worker: Option<Process> = None;
    let result = (|| -> io::Result<()> {
        let (raw, mut fds) = channel.receive(16, 2)?;
        let child = spec.decode(&raw, 0)?;
        let mut pipe = fds.pop().ok_or_else(refused)?;
        let pidfd = fds.pop().ok_or_else(refused)?;
        binding(&original, child, &pidfd, &pipe)?;
        worker = Some(Process {
            pid: child,
            fd: Some(pidfd),
            reaped: false,
            direct: false,
        });
        channel.send(b"G", &[])?;
        let (cut, fds) = channel.receive(16, 0)?;
        require(fds.is_empty() && spec.decode(&cut, 1)? == child)?;
        let child = worker.as_mut().ok_or_else(refused)?;
        nonterminal(original.fd()?)?;
        nonterminal(child.fd()?)?;
        nonterminal(unrelated.fd()?)?;
        if spec.mode == 1 {
            channel.send(b"H", &[])?;
            original.wait(libc::CLD_EXITED, 0)?;
            poll(child.fd()?, libc::POLLIN)?;
            require(
                wait_info(child.fd()?, 0).is_err_and(|e| e.raw_os_error() == Some(libc::ECHILD)),
            )?;
            child.reaped = true;
        } else {
            signal(original.fd()?)?;
            original.wait(libc::CLD_KILLED, libc::SIGKILL)?;
            // Keep the peer endpoint until creator terminal/reap, never early EOF.
            if spec.mode == 2 {
                let until = Instant::now() + Duration::from_secs(5);
                loop {
                    nonterminal(child.fd()?)?;
                    if detached_stopped(&text(&format!("/proc/{}/status", child.pid))?, parent)? {
                        break;
                    }
                    require(Instant::now() < until)?;
                    std::thread::sleep(Duration::from_millis(1));
                }
                // Explicit negative cleanup, NEVER automatic-death evidence.
                signal(child.fd()?)?;
            }
            child.wait(libc::CLD_KILLED, libc::SIGKILL)?;
        }
        let mut extra = [0];
        require(pipe.read(&mut extra)? == 0)?;
        nonterminal(unrelated.fd()?)?;
        signal(unrelated.fd()?)?;
        unrelated.wait(libc::CLD_KILLED, libc::SIGKILL)?;
        tree.quiescent = true;
        let root = tree.root.as_mut().ok_or_else(refused)?;
        root.check()?;
        root.cleanup()?;
        reaper.restore()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = original.cleanup();
        if let Some(child) = &mut worker {
            let _ = child.cleanup();
        }
        let _ = unrelated.cleanup();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pipe() -> [File; 2] {
        let mut fds = [-1; 2];
        assert_eq!(
            unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC | libc::O_NONBLOCK) },
            0
        );
        [file(fds[0]).unwrap(), file(fds[1]).unwrap()]
    }
    fn channels() -> (Channel, Channel) {
        let [a, b] = pair().unwrap();
        let pid = unsafe { libc::getpid() };
        (Channel::new(a, pid), Channel::new(b, pid))
    }
    #[test]
    fn fixed_frames_reject_every_non_pid_bit_and_unregistered_modes() {
        for name in [
            "root-loss-initial",
            "root-loss-rooted",
            "root-loss-exec",
            "root-loss-loaded",
            "root-loss-installed",
            "root-loss-healthy-initial",
            "root-loss-healthy-installed",
            "root-loss-no-exitkill-installed",
        ] {
            let spec = Spec::parse(name).unwrap();
            for stage in 0..2 {
                let raw = spec.frame(123, stage);
                assert_eq!(spec.decode(&raw, stage).unwrap(), 123);
                for i in (0..8).chain(12..16) {
                    for bit in 0..8 {
                        let mut bad = raw;
                        bad[i] ^= 1 << bit;
                        assert!(spec.decode(&bad, stage).is_err());
                    }
                }
                for length in 0..16 {
                    assert!(spec.decode(&raw[..length], stage).is_err());
                }
                assert!(spec.decode(&[0; 17], stage).is_err());
                assert!(spec.decode(&spec.frame(0, stage), stage).is_err());
                assert!(spec.decode(&spec.frame(-1, stage), stage).is_err());
                assert!(spec.decode(&raw, 2).is_err());
            }
        }
        for name in [
            "",
            "root-loss-",
            "root-loss-running",
            "root-loss-pre-arm",
            "root-loss-installed-extra",
            "root-loss-healthy-loaded",
        ] {
            assert!(Spec::parse(name).is_err());
        }
    }
    #[test]
    fn channel_uses_kernel_credentials_cloexec_and_fixed_packet_budget() {
        let (mut a, mut b) = channels();
        let [reader, writer] = pipe();
        a.send(&[1; 16], &[reader.as_raw_fd(), writer.as_raw_fd()])
            .unwrap();
        let (raw, fds) = b.receive(16, 2).unwrap();
        assert_eq!(raw, [1; 16]);
        assert_eq!(fds.len(), 2);
        same_pipe(&fds[0], &fds[1]).unwrap();
        for fd in &fds {
            assert_eq!(
                unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_GETFD) },
                libc::FD_CLOEXEC
            );
        }
        for _ in 0..3 {
            a.send(b"G", &[]).unwrap();
            b.gate(b'G').unwrap();
        }
        assert!(a.send(b"G", &[]).is_err());
        assert!(b.receive(1, 0).is_err());
    }
    #[test]
    fn rejected_rights_are_closed_even_for_foreign_credentials_or_truncation() {
        for case in 0..4 {
            let (mut a, mut b) = channels();
            let [mut reader, writer] = pipe();
            a.send(&[2; 16], &[writer.as_raw_fd(), writer.as_raw_fd()])
                .unwrap();
            drop(writer);
            if case == 0 {
                b.peer += 1;
            }
            let expected_bytes = if case == 1 { 1 } else { 16 };
            let expected_fds = if case == 2 { 0 } else { 2 };
            if case == 3 {
                // A separate oversized datagram, after the rights datagram.
                let bytes = [0u8; 32];
                assert_eq!(
                    unsafe {
                        libc::send(
                            a.file.as_raw_fd(),
                            bytes.as_ptr().cast(),
                            bytes.len(),
                            libc::MSG_NOSIGNAL,
                        )
                    },
                    32
                );
                assert!(b.receive(expected_bytes, expected_fds).is_ok());
                assert!(b.receive(16, 0).is_err());
            } else {
                assert!(b.receive(expected_bytes, expected_fds).is_err());
            }
            // Any leaked received writer would yield EAGAIN, not EOF.
            assert_eq!(reader.read(&mut [0; 1]).unwrap(), 0);
        }
    }
    #[test]
    fn channel_rejects_missing_credentials_wrong_gate_and_malformed_requests() {
        let (mut a, mut b) = channels();
        a.send(b"H", &[]).unwrap();
        assert!(b.gate(b'G').is_err());
        assert!(a.send(&[], &[]).is_err());
        assert!(a.send(b"G", &[0]).is_err());
        assert!(b.receive(2, 0).is_err());
        let disabled = 0i32;
        assert_eq!(
            unsafe {
                libc::setsockopt(
                    b.file.as_raw_fd(),
                    libc::SOL_SOCKET,
                    libc::SO_PASSCRED,
                    (&disabled as *const i32).cast(),
                    4,
                )
            },
            0
        );
        a.send(b"G", &[]).unwrap();
        assert!(b.receive(1, 0).is_err());
    }
    #[test]
    fn original_pipe_identity_rejects_unrelated_pipe_and_wrong_access() {
        let [reader, writer] = pipe();
        let [other_reader, other_writer] = pipe();
        same_pipe(&reader, &writer).unwrap();
        assert!(same_pipe(&reader, &other_writer).is_err());
        assert!(same_pipe(&other_reader, &writer).is_err());
        assert!(same_pipe(&writer, &reader).is_err());
        assert!(same_pipe(&reader, &open("/dev/null").unwrap()).is_err());
        assert_eq!(
            unsafe { libc::fcntl(reader.as_raw_fd(), libc::F_SETFL, 0) },
            0
        );
        assert!(same_pipe(&reader, &writer).is_err());
    }
    #[test]
    fn truncated_ancillary_rights_close_every_delivered_or_truncated_writer() {
        let (a, mut b) = channels();
        let [mut reader, writer] = pipe();
        let rights = [writer.as_raw_fd(); 32];
        let mut control = [0usize; 32];
        let mut payload = [0u8; 16];
        let mut iov = libc::iovec {
            iov_base: payload.as_mut_ptr().cast(),
            iov_len: payload.len(),
        };
        let mut message: libc::msghdr = unsafe { std::mem::zeroed() };
        message.msg_iov = &mut iov;
        message.msg_iovlen = 1;
        message.msg_control = control.as_mut_ptr().cast();
        message.msg_controllen = unsafe { libc::CMSG_SPACE(128) as usize };
        unsafe {
            let header = libc::CMSG_FIRSTHDR(&message);
            (*header).cmsg_level = libc::SOL_SOCKET;
            (*header).cmsg_type = libc::SCM_RIGHTS;
            (*header).cmsg_len = libc::CMSG_LEN(128) as usize;
            std::ptr::copy_nonoverlapping(
                rights.as_ptr().cast::<u8>(),
                libc::CMSG_DATA(header),
                128,
            );
            assert_eq!(
                libc::sendmsg(a.file.as_raw_fd(), &message, libc::MSG_NOSIGNAL),
                16
            );
        }
        drop(writer);
        assert!(b.receive(16, 2).is_err());
        assert_eq!(reader.read(&mut [0; 1]).unwrap(), 0);
    }
    #[test]
    fn stopped_negative_requires_detachment_and_never_accepts_running_as_stopped() {
        let status = |state: &str, tracer: i32, parent: i32| {
            format!("PPid:\t{parent}\nTracerPid:\t{tracer}\nState:\t{state}\n")
        };
        assert!(detached_stopped(&status("T (stopped)", 0, 123), 123).unwrap());
        for state in ["R (running)", "t (tracing stop)"] {
            assert!(!detached_stopped(&status(state, 0, 123), 123).unwrap());
        }
        for state in ["S (sleeping)", "Z (zombie)", "T", ""] {
            assert!(detached_stopped(&status(state, 0, 123), 123).is_err());
        }
        assert!(detached_stopped(&status("T (stopped)", 1, 123), 123).is_err());
        assert!(detached_stopped(&status("T (stopped)", 0, 124), 123).is_err());
    }
    #[test]
    fn scalar_fields_refuse_duplicates_signs_overflow_and_non_decimal() {
        assert_eq!(scalar("Pid:\t123\n", "Pid:").unwrap(), 123);
        for raw in [
            "",
            "Pid:\t\n",
            "Pid: -1\n",
            "Pid: +1\n",
            "Pid: 0x12\n",
            "Pid: 2147483648\n",
            "Pid: 1\nPid: 1\n",
        ] {
            assert!(scalar(raw, "Pid:").is_err());
        }
    }
    #[test]
    fn tree_drop_preserves_original_names_without_quiescence() {
        for quiescent in [false, true] {
            let (root, temp) = super::super::tests::root();
            let dirs: Vec<_> = root
                .dirs
                .iter()
                .map(|node| node.file.try_clone().unwrap())
                .collect();
            drop(Tree {
                root: Some(root),
                quiescent,
            });
            assert_eq!(temp.path().join("root").exists(), !quiescent);
            // Unit-only outer TempDir owns the preserved synthetic tree. Retain
            // original directory handles for fixture teardown, never recapture.
            if !quiescent {
                for dir in dirs {
                    super::super::super::chmod(&dir, 0o700).unwrap();
                }
            }
        }
        assert!(Process::direct(0).is_err());
        assert!(Process::direct(-1).is_err());
    }
}
