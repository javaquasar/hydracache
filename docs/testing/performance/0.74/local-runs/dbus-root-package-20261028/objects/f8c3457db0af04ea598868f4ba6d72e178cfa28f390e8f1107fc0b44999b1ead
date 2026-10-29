//! Independent kernel mechanics, not an authenticated original-start capability.
//!
//! No public entry point. The future original join must move (not clone) the
//! fence and sinks, close every parent writer alias, and preserve live checks.
use std::fs::File;
use std::io::{self, Read};
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::fs::{FileTypeExt, MetadataExt};

const MAX_WRITERS: usize = 8;
const QUANTUM: usize = 8192;
const NS: u64 = 1_000_000_000;

struct Inputs {
    fence: File,
    readers: [File; 2],
    sinks: [File; 2],
    controller: File,
    writers: Vec<File>,
    // Selected original absolute BOOTTIME bound, never a relative duration.
    deadline: u64,
}
struct Guardian {
    pid: libc::pid_t,
    ready: File,
}
impl Guardian {
    fn notice(&self) -> io::Result<u8> {
        let mut byte = [0u8];
        let n = unsafe {
            libc::recv(
                self.ready.as_raw_fd(),
                byte.as_mut_ptr().cast(),
                1,
                libc::MSG_TRUNC,
            )
        };
        if n < 0 {
            return Err(io::Error::last_os_error());
        }
        if n != 1 || !(1..=10).contains(&byte[0]) {
            return Err(invalid());
        }
        Ok(byte[0])
    }
}

fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, "guardian descriptor binding")
}
fn now() -> io::Result<u64> {
    let mut t: libc::timespec = unsafe { std::mem::zeroed() };
    if unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut t) } != 0 {
        return Err(io::Error::last_os_error());
    }
    if t.tv_sec < 0 || !(0..NS as i64).contains(&t.tv_nsec) {
        return Err(invalid());
    }
    (t.tv_sec as u64)
        .checked_mul(NS)
        .and_then(|v| v.checked_add(t.tv_nsec as u64))
        .ok_or_else(invalid)
}
fn future(deadline: u64, observed: u64) -> io::Result<()> {
    if observed >= deadline || deadline > i64::MAX as u64 {
        return Err(invalid());
    }
    Ok(())
}
fn timer(deadline: u64) -> io::Result<File> {
    future(deadline, now()?)?;
    let fd = unsafe {
        libc::timerfd_create(libc::CLOCK_BOOTTIME, libc::TFD_CLOEXEC | libc::TFD_NONBLOCK)
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    let file = unsafe { File::from_raw_fd(fd) };
    let spec = libc::itimerspec {
        it_interval: libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        it_value: libc::timespec {
            tv_sec: (deadline / NS) as libc::time_t,
            tv_nsec: (deadline % NS) as libc::c_long,
        },
    };
    if unsafe { libc::timerfd_settime(fd, libc::TFD_TIMER_ABSTIME, &spec, std::ptr::null_mut()) }
        != 0
    {
        return Err(io::Error::last_os_error());
    }
    future(deadline, now()?)?;
    Ok(file)
}
fn pipe() -> io::Result<[File; 2]> {
    let mut fds = [-1; 2];
    if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC | libc::O_NONBLOCK) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { [File::from_raw_fd(fds[0]), File::from_raw_fd(fds[1])] })
}
fn notices() -> io::Result<[File; 2]> {
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
    Ok(unsafe { [File::from_raw_fd(fds[0]), File::from_raw_fd(fds[1])] })
}
fn validate(input: &Inputs) -> io::Result<()> {
    future(input.deadline, now()?)?;
    if input.writers.is_empty() || input.writers.len() > MAX_WRITERS {
        return Err(invalid());
    }
    let fence = input.fence.metadata()?;
    if !fence.is_file() || fence.nlink() != 1 || fence.len() != 0 {
        return Err(invalid());
    }
    let mut objects = vec![(fence.dev(), fence.ino())];
    for file in &input.readers {
        let m = file.metadata()?;
        let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) };
        if !m.file_type().is_fifo()
            || flags < 0
            || flags & (libc::O_ACCMODE | libc::O_NONBLOCK) != libc::O_NONBLOCK
            || objects.contains(&(m.dev(), m.ino()))
        {
            return Err(invalid());
        }
        objects.push((m.dev(), m.ino()));
    }
    for file in &input.sinks {
        let m = file.metadata()?;
        let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) };
        if !m.is_file()
            || m.nlink() != 1
            || m.len() != 0
            || flags < 0
            || flags & libc::O_ACCMODE != libc::O_WRONLY
            || flags & (libc::O_APPEND | libc::O_PATH) != 0
            || unsafe { libc::lseek(file.as_raw_fd(), 0, libc::SEEK_CUR) } != 0
            || objects.contains(&(m.dev(), m.ino()))
        {
            return Err(invalid());
        }
        objects.push((m.dev(), m.ino()));
    }
    // fdinfo distinguishes pidfds from arbitrary readable regular files. This
    // pre-fork check is mechanics, not authenticated process/cgroup enrollment.
    for file in std::iter::once(&input.controller).chain(&input.writers) {
        let mut raw = String::new();
        File::open(format!("/proc/thread-self/fdinfo/{}", file.as_raw_fd()))?
            .take(4097)
            .read_to_string(&mut raw)?;
        if raw.len() > 4096
            || raw.lines().filter(|l| l.starts_with("Pid:\t")).count() != 1
            || !raw.lines().any(|l| l.starts_with("NSpid:\t"))
        {
            return Err(invalid());
        }
    }
    Ok(())
}

// Child code below uses only stack memory and async-signal-safe libc/syscalls.
// No Rust unwind, allocator, atfork callback, validation, logging or destructor
// may run after SYS_fork. This is Linux x86_64-only, not portable libc fork.
#[derive(Clone, Copy)]
struct Raw {
    fence: RawFd,
    readers: [RawFd; 2],
    sinks: [RawFd; 2],
    controller: RawFd,
    writers: [RawFd; MAX_WRITERS],
    count: usize,
    timer: RawFd,
    ready: RawFd,
    status: [RawFd; 2],
    limit: usize,
    #[cfg(test)]
    fault: u8,
}
fn errno() -> i32 {
    unsafe { *libc::__errno_location() }
}
fn park() -> ! {
    loop {
        unsafe { libc::poll(std::ptr::null_mut(), 0, -1) };
    }
}
fn kill_pinned(fd: RawFd) -> bool {
    if unsafe { libc::syscall(libc::SYS_pidfd_send_signal, fd, libc::SIGKILL, 0usize, 0u32) } == 0 {
        true
    } else {
        errno() == libc::ESRCH
    }
}
fn stop(raw: &Raw) -> bool {
    let mut ok = kill_pinned(raw.controller);
    for fd in raw.writers.iter().take(raw.count) {
        // Deliberately do not short-circuit later revocations after one error.
        ok = kill_pinned(*fd) && ok;
    }
    ok
}
#[repr(u8)]
#[derive(Clone, Copy)]
enum Failure {
    Isolation = 2,
    Observation = 3,
    Revocation = 4,
    Read = 5,
    Write = 6,
    Overflow = 7,
    Sync = 8,
    Close = 9,
    Expired = 10,
}
fn refused(raw: &Raw, reason: Failure) -> ! {
    stop(raw);
    let notice = [reason as u8];
    // Best effort only if the controller disconnected. The sticky obligation
    // never depends on delivery; an available observer gets an explicit cause.
    unsafe { libc::send(raw.ready, notice.as_ptr().cast(), 1, libc::MSG_NOSIGNAL) };
    // Keep the fence and evidence. Errors never grant success or retry.
    park()
}
fn terminal(fd: RawFd) -> i32 {
    let mut p = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    match unsafe { libc::poll(&mut p, 1, 0) } {
        0 if p.revents == 0 => 0,
        1 if p.revents == libc::POLLIN
            || p.revents == libc::POLLHUP
            || p.revents == libc::POLLIN | libc::POLLHUP =>
        {
            1
        }
        -1 if errno() == libc::EINTR => 0,
        _ => -1,
    }
}
fn releasable(writers_terminal: bool, eof: [bool; 2], refused: bool) -> bool {
    writers_terminal && eof == [true, true] && !refused
}
fn isolate(keep: &[RawFd]) -> bool {
    let mut first = 0u32;
    for fd in keep {
        let fd = *fd as u32;
        if first < fd && unsafe { libc::syscall(libc::SYS_close_range, first, fd - 1, 0u32) } != 0 {
            return false;
        }
        first = fd + 1;
    }
    unsafe { libc::syscall(libc::SYS_close_range, first, u32::MAX, 0u32) == 0 }
}
fn collector_refused(raw: &Raw, reason: Failure) -> ! {
    let byte = [reason as u8];
    unsafe { libc::send(raw.status[1], byte.as_ptr().cast(), 1, libc::MSG_NOSIGNAL) };
    park()
}
fn receive(fd: RawFd) -> i32 {
    let mut byte = [0u8];
    let n = unsafe { libc::recv(fd, byte.as_mut_ptr().cast(), 1, libc::MSG_TRUNC) };
    if n == 1 && byte[0] != 0 {
        byte[0] as i32
    } else if n < 0 && (errno() == libc::EAGAIN || errno() == libc::EINTR) {
        0
    } else {
        -1
    }
}
fn collect(raw: Raw, keep: &[RawFd]) -> ! {
    if !isolate(keep) {
        collector_refused(&raw, Failure::Isolation);
    }
    // No disk work until the guardian has pinned this child, shed its sink
    // aliases and released the one-shot private barrier.
    loop {
        match receive(raw.status[1]) {
            1 => break,
            0 => {
                unsafe { libc::poll(std::ptr::null_mut(), 0, 10) };
            }
            _ => collector_refused(&raw, Failure::Isolation),
        }
    }
    #[cfg(test)]
    if raw.fault == 5 {
        unsafe {
            libc::syscall(
                libc::SYS_kill,
                libc::syscall(libc::SYS_getpid),
                libc::SIGSTOP,
            )
        };
    }
    #[cfg(test)]
    if raw.fault == 1 {
        unsafe { libc::close(raw.sinks[0]) };
    }
    #[cfg(test)]
    if raw.fault == 4 {
        let limit = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        if unsafe { libc::setrlimit(libc::RLIMIT_FSIZE, &limit) } != 0 {
            collector_refused(&raw, Failure::Isolation);
        }
    }
    let mut eof = [false; 2];
    let mut bytes = [0usize; 2];
    let mut buffer = [0u8; QUANTUM];
    loop {
        let mut progressed = false;
        for i in 0..2 {
            if eof[i] {
                continue;
            }
            let n = unsafe { libc::read(raw.readers[i], buffer.as_mut_ptr().cast(), QUANTUM) };
            if n == 0 {
                eof[i] = true;
            } else if n < 0 {
                if errno() != libc::EAGAIN && errno() != libc::EINTR {
                    collector_refused(&raw, Failure::Read);
                }
            } else {
                progressed = true;
                let n = n as usize;
                if n > raw.limit - bytes[i] {
                    collector_refused(&raw, Failure::Overflow);
                }
                let mut offset = 0;
                while offset < n {
                    let written = unsafe {
                        libc::write(raw.sinks[i], buffer.as_ptr().add(offset).cast(), n - offset)
                    };
                    if written > 0 {
                        offset += written as usize;
                    } else if written == 0 || errno() != libc::EINTR {
                        collector_refused(&raw, Failure::Write);
                    }
                }
                bytes[i] += n;
            }
        }
        if eof == [true, true] {
            #[cfg(test)]
            if raw.fault == 2 {
                unsafe { libc::close(raw.sinks[0]) };
            }
            for fd in raw.sinks {
                if unsafe { libc::fsync(fd) } != 0 {
                    collector_refused(&raw, Failure::Sync);
                }
            }
            for fd in raw.sinks {
                if unsafe { libc::close(fd) } != 0 {
                    collector_refused(&raw, Failure::Close);
                }
            }
            let byte = [1u8];
            if unsafe { libc::send(raw.status[1], byte.as_ptr().cast(), 1, libc::MSG_NOSIGNAL) }
                != 1
            {
                park();
            }
            unsafe { libc::_exit(0) }
        }
        if !progressed {
            unsafe { libc::poll(std::ptr::null_mut(), 0, 10) };
        }
    }
}
fn guardian_refused(raw: &Raw, collector: RawFd, reason: Failure) -> ! {
    kill_pinned(collector);
    refused(raw, reason)
}
fn run(raw: Raw, keep: &[RawFd], collector_keep: &[RawFd], notify: bool) -> ! {
    // Block inherited userspace handlers before any raw-fork continuation.
    // Unblockable SIGKILL/SIGSTOP require external supervision/durable history.
    let blocked = u64::MAX;
    if unsafe {
        libc::syscall(
            libc::SYS_rt_sigprocmask,
            libc::SIG_SETMASK,
            &blocked,
            0usize,
            8usize,
        )
    } != 0
        || !isolate(keep)
    {
        refused(&raw, Failure::Isolation);
    }
    let child = unsafe { libc::syscall(libc::SYS_fork) };
    if child < 0 {
        refused(&raw, Failure::Isolation);
    }
    if child == 0 {
        collect(raw, collector_keep);
    }
    let collector = unsafe { libc::syscall(libc::SYS_pidfd_open, child, 0u32) };
    if collector < 0 {
        // The still-unreaped direct child cannot reuse its PID. It is behind
        // the private barrier and has never been allowed disk work.
        unsafe { libc::syscall(libc::SYS_kill, child, libc::SIGKILL) };
        refused(&raw, Failure::Isolation);
    }
    let collector = collector as RawFd;
    // The collector is still barred, so these non-last alias closes cannot
    // become disk flushes. The guardian owns NO regular-file output writer.
    for fd in raw.sinks {
        if unsafe { libc::close(fd) } != 0 {
            guardian_refused(&raw, collector, Failure::Close);
        }
    }
    if unsafe { libc::close(raw.status[1]) } != 0 {
        guardian_refused(&raw, collector, Failure::Close);
    }
    if terminal(raw.timer) != 0 {
        guardian_refused(&raw, collector, Failure::Expired);
    }
    let byte = [1u8];
    if unsafe { libc::send(raw.status[0], byte.as_ptr().cast(), 1, libc::MSG_NOSIGNAL) } != 1 {
        guardian_refused(&raw, collector, Failure::Isolation);
    }
    if notify {
        unsafe { libc::send(raw.ready, byte.as_ptr().cast(), 1, libc::MSG_NOSIGNAL) };
    }
    let mut stopping = false;
    let mut drained = false;
    loop {
        let alarm = terminal(raw.timer);
        let controller = terminal(raw.controller);
        if alarm < 0 || controller < 0 {
            guardian_refused(&raw, collector, Failure::Observation);
        }
        if alarm == 1 {
            guardian_refused(&raw, collector, Failure::Expired);
        }
        if !stopping && controller == 1 {
            stopping = true;
            if !stop(&raw) {
                guardian_refused(&raw, collector, Failure::Revocation);
            }
        }
        if !drained {
            match receive(raw.status[0]) {
                0 => (),
                1 => drained = true,
                2 => guardian_refused(&raw, collector, Failure::Isolation),
                5 => guardian_refused(&raw, collector, Failure::Read),
                6 => guardian_refused(&raw, collector, Failure::Write),
                7 => guardian_refused(&raw, collector, Failure::Overflow),
                8 => guardian_refused(&raw, collector, Failure::Sync),
                9 => guardian_refused(&raw, collector, Failure::Close),
                _ => guardian_refused(&raw, collector, Failure::Observation),
            }
        }
        let ended = terminal(collector);
        if ended < 0 || ended == 1 && !drained {
            guardian_refused(&raw, collector, Failure::Observation);
        }
        let mut writers_terminal = true;
        for fd in raw.writers.iter().take(raw.count) {
            match terminal(*fd) {
                1 => (),
                0 => writers_terminal = false,
                _ => guardian_refused(&raw, collector, Failure::Observation),
            }
        }
        if ended == 1 && drained && writers_terminal {
            let mut status = 0;
            if unsafe { libc::waitpid(child as libc::pid_t, &mut status, 0) }
                != child as libc::pid_t
                || !libc::WIFEXITED(status)
                || libc::WEXITSTATUS(status) != 0
            {
                guardian_refused(&raw, collector, Failure::Observation);
            }
            let mut eof = [false; 2];
            let mut byte = [0u8];
            for (i, fd) in raw.readers.iter().enumerate() {
                eof[i] = unsafe { libc::read(*fd, byte.as_mut_ptr().cast(), 1) } == 0;
            }
            if !releasable(writers_terminal, eof, false) {
                guardian_refused(&raw, collector, Failure::Observation);
            }
            // No completion, adoption or deletion. Only the private cessation
            // boundary permits closing this last fence owner.
            if unsafe { libc::close(raw.fence) } != 0 {
                park();
            }
            unsafe { libc::_exit(0) }
        }
        unsafe { libc::poll(std::ptr::null_mut(), 0, 10) };
    }
}
fn spawn(input: Inputs, notify: bool) -> io::Result<Guardian> {
    spawn_inner(
        input,
        notify,
        #[cfg(test)]
        0,
    )
}
fn spawn_inner(input: Inputs, notify: bool, #[cfg(test)] fault: u8) -> io::Result<Guardian> {
    validate(&input)?;
    let alarm = timer(input.deadline)?;
    let [ready, send] = notices()?;
    let status = notices()?;
    #[cfg(test)]
    if fault == 3 {
        // Guaranteed disconnected receiver before the child can send anything.
        if unsafe { libc::shutdown(ready.as_raw_fd(), libc::SHUT_RDWR) } != 0 {
            return Err(io::Error::last_os_error());
        }
    }
    let mut raw = Raw {
        fence: input.fence.as_raw_fd(),
        readers: input.readers.each_ref().map(AsRawFd::as_raw_fd),
        sinks: input.sinks.each_ref().map(AsRawFd::as_raw_fd),
        controller: input.controller.as_raw_fd(),
        writers: [-1; MAX_WRITERS],
        count: input.writers.len(),
        timer: alarm.as_raw_fd(),
        ready: send.as_raw_fd(),
        status: status.each_ref().map(AsRawFd::as_raw_fd),
        limit: crate::diagnostic_receipts::STREAM_BYTES,
        #[cfg(test)]
        fault,
    };
    for (slot, file) in raw.writers.iter_mut().zip(&input.writers) {
        *slot = file.as_raw_fd();
    }
    let mut keep = vec![raw.fence, raw.controller, raw.timer, raw.ready];
    keep.extend(raw.readers);
    keep.extend(raw.sinks);
    keep.extend(raw.status);
    keep.extend(raw.writers.iter().take(raw.count));
    keep.sort_unstable();
    if keep.windows(2).any(|p| p[0] == p[1]) || keep.iter().any(|fd| *fd < 0) {
        return Err(invalid());
    }
    let mut collector_keep = vec![
        raw.readers[0],
        raw.readers[1],
        raw.sinks[0],
        raw.sinks[1],
        raw.status[1],
    ];
    collector_keep.sort_unstable();
    // Block in the caller BEFORE fork: child-side blocking alone leaves an
    // inherited userspace-handler window. Other parent threads are unaffected.
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
    // No userspace atfork hooks; the child inherits the fully blocked mask.
    let pid = {
        #[cfg(test)]
        {
            if fault == 6 {
                // Parent-only deterministic syscall-failure seam; no process
                // limit or host-global resource mutation is used by tests.
                unsafe { *libc::__errno_location() = libc::EAGAIN };
                -1
            } else {
                unsafe { libc::syscall(libc::SYS_fork) }
            }
        }
        #[cfg(not(test))]
        {
            unsafe { libc::syscall(libc::SYS_fork) }
        }
    };
    if pid == 0 {
        run(raw, &keep, &collector_keep, notify);
    }
    let fork_error = (pid < 0).then(io::Error::last_os_error);
    // Parent-only path, including fork failure. Capture fork errno first.
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
    if let Some(error) = fork_error {
        return Err(error);
    }
    // File close only, NEVER flock(LOCK_UN): the child owns the same OFD now.
    drop(input);
    drop(alarm);
    drop(send);
    drop(status);
    Ok(Guardian {
        pid: pid as libc::pid_t,
        ready,
    })
}

#[cfg(test)]
#[path = "diagnostic_autonomous_watchdog_tests.rs"]
mod tests;
