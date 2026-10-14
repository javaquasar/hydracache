//! Test-only syscall/memory observation for an owned single-thread fork helper.
use std::fs::File;
use std::io;
use std::marker::PhantomData;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::rc::Rc;
use std::time::{Duration, Instant};

const ARCH: u32 = 0xc000_003e;
const I386: u32 = 0x4000_0003;
const X32: u32 = 0x4000_0000;
const COUNT: usize = 34;
const DEADLINE: Duration = Duration::from_secs(5);
const SYSCALL_STOP: i32 = ((libc::SIGTRAP | 0x80) << 8) | 0x7f;
const DENIED: [u32; 9] = [
    libc::SYS_execve as u32,
    libc::SYS_execveat as u32,
    libc::SYS_fork as u32,
    libc::SYS_vfork as u32,
    libc::SYS_clone as u32,
    libc::SYS_clone3 as u32,
    libc::SYS_ptrace as u32,
    libc::SYS_seccomp as u32,
    libc::SYS_prctl as u32,
];
fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "owned seccomp fixture refused")
}
fn peek_word(word: libc::c_long, errno: i32) -> io::Result<libc::c_long> {
    if word == -1 && errno != 0 {
        Err(io::Error::from_raw_os_error(errno))
    } else {
        Ok(word)
    }
}
const fn ins(code: u16, jt: u8, jf: u8, k: u32) -> libc::sock_filter {
    libc::sock_filter { code, jt, jf, k }
}
fn policy() -> [libc::sock_filter; COUNT] {
    let mut out = [ins(0, 0, 0, 0); COUNT];
    out[..6].copy_from_slice(&[
        ins(0x20, 0, 0, 4),
        ins(0x15, 1, 0, ARCH),
        ins(0x06, 0, 0, libc::SECCOMP_RET_KILL_PROCESS),
        ins(0x20, 0, 0, 0),
        ins(0x45, 0, 1, X32),
        ins(0x06, 0, 0, libc::SECCOMP_RET_KILL_PROCESS),
    ]);
    for (index, nr) in DENIED[..8].iter().enumerate() {
        out[6 + index * 2] = ins(0x15, 0, 1, *nr);
        out[7 + index * 2] = ins(0x06, 0, 0, libc::SECCOMP_RET_ERRNO | libc::EPERM as u32);
    }
    out[22..].copy_from_slice(&[
        ins(0x15, 0, 10, libc::SYS_prctl as u32),
        ins(0x20, 0, 0, 16),
        ins(0x15, 0, 7, libc::PR_SET_DUMPABLE as u32),
        ins(0x20, 0, 0, 20),
        ins(0x15, 0, 5, 0),
        ins(0x20, 0, 0, 24),
        ins(0x15, 0, 3, 0),
        ins(0x20, 0, 0, 28),
        ins(0x15, 0, 1, 0),
        ins(0x06, 0, 0, libc::SECCOMP_RET_ALLOW),
        ins(0x06, 0, 0, libc::SECCOMP_RET_ERRNO | libc::EPERM as u32),
        ins(0x06, 0, 0, libc::SECCOMP_RET_ALLOW),
    ]);
    out
}
fn encoded(program: &[libc::sock_filter]) -> Vec<u8> {
    program
        .iter()
        .flat_map(|i| {
            let code = i.code.to_le_bytes();
            let k = i.k.to_le_bytes();
            [code[0], code[1], i.jt, i.jf, k[0], k[1], k[2], k[3]]
        })
        .collect()
}
#[derive(Clone, Copy)]
enum Case {
    ExecSelf,
    ExecOther,
    ExecFd,
    Denied(u32),
    X32,
    I386,
    RestoreDumpable,
}
#[derive(Clone, Copy)]
enum Fault {
    None,
    Flags,
    Header,
    Policy,
    Length,
    Pointer,
}
#[derive(Debug, PartialEq, Eq)]
enum Event {
    Entry { arch: u32, nr: u64, args: [u64; 6] },
    Exit { arch: u32, value: i64, error: bool },
}

fn decode(info: &libc::ptrace_syscall_info, bytes: usize) -> io::Result<Event> {
    if bytes > std::mem::size_of::<libc::ptrace_syscall_info>() || info.flags != 0 {
        return Err(invalid());
    }
    match info.op {
        1 if bytes >= 80 => {
            // SAFETY: kernel identifies entry and returned all entry fields.
            let entry = unsafe { info.u.entry };
            Ok(Event::Entry {
                arch: info.arch,
                nr: entry.nr,
                args: entry.args,
            })
        }
        2 if bytes >= 33 => {
            // SAFETY: kernel identifies exit and returned sval plus is_error.
            let exit = unsafe { info.u.exit };
            if exit.is_error > 1 {
                return Err(invalid());
            }
            Ok(Event::Exit {
                arch: info.arch,
                value: exit.sval,
                error: exit.is_error == 1,
            })
        }
        _ => Err(invalid()),
    }
}
struct Owner {
    pid: libc::pid_t,
    pidfd: Option<OwnedFd>,
    reaped: bool,
    _thread: PhantomData<Rc<()>>,
}
impl Owner {
    fn wait(&mut self) -> io::Result<i32> {
        let deadline = Instant::now() + DEADLINE;
        loop {
            if self.reaped {
                return Err(invalid());
            }
            let mut status = 0;
            // SAFETY: exact original child, initialized status storage, no wait-any.
            let pid = unsafe { libc::waitpid(self.pid, &mut status, libc::WNOHANG | libc::__WALL) };
            if pid == self.pid {
                self.reaped = libc::WIFEXITED(status) || libc::WIFSIGNALED(status);
                return Ok(status);
            }
            if pid < 0 && io::Error::last_os_error().raw_os_error() != Some(libc::EINTR) {
                return Err(io::Error::last_os_error());
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "owned syscall fixture stalled",
                ));
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    fn trace(&self, operation: u32, data: usize) -> io::Result<()> {
        // SAFETY: original stopped tracee only; scalar option/continuation data.
        if unsafe { libc::ptrace(operation, self.pid, 0usize, data) } < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
    fn event(&mut self) -> io::Result<Event> {
        self.trace(libc::PTRACE_SYSCALL, 0)?;
        if self.wait()? != SYSCALL_STOP {
            return Err(invalid());
        }
        self.info()
    }
    fn info(&self) -> io::Result<Event> {
        // SAFETY: all-zero integral/union storage is valid; no references inside.
        let mut info: libc::ptrace_syscall_info = unsafe { std::mem::zeroed() };
        // SAFETY: exact owned stopped child and sized, initialized output buffer.
        let size = unsafe {
            libc::ptrace(
                libc::PTRACE_GET_SYSCALL_INFO,
                self.pid,
                std::mem::size_of_val(&info),
                &mut info,
            )
        };
        if size < 0 {
            return Err(io::Error::last_os_error());
        }
        decode(&info, size as usize)
    }
    fn read(&self, address: usize, count: usize) -> io::Result<Vec<u8>> {
        if address == 0 || count == 0 || count > COUNT * 8 {
            return Err(invalid());
        }
        let end = address.checked_add(count).ok_or_else(invalid)?;
        let start = address & !7;
        let mut out = Vec::with_capacity(count + 14);
        let mut cursor = start;
        while cursor < end {
            // SAFETY: libc errno TLS slot belongs to the reading tracer thread.
            unsafe {
                *libc::__errno_location() = 0;
            }
            // SAFETY: bounded read at a preselected original address on our stopped child.
            let word = unsafe { libc::ptrace(libc::PTRACE_PEEKDATA, self.pid, cursor, 0usize) };
            // SAFETY: read same tracer-thread errno; -1 can be valid data.
            let errno = unsafe { *libc::__errno_location() };
            out.extend_from_slice(&peek_word(word, errno)?.to_ne_bytes());
            cursor = cursor.checked_add(8).ok_or_else(invalid)?;
        }
        let offset = address - start;
        Ok(out[offset..offset + count].to_vec())
    }
    fn memory(&self, header: usize, program: usize, expected: &[u8]) -> io::Result<()> {
        let raw = self.read(header, 16)?;
        // Do not interpret C header padding; only declared len and pointer fields.
        if u16::from_le_bytes(raw[..2].try_into().unwrap()) as usize != COUNT
            || u64::from_le_bytes(raw[8..16].try_into().unwrap()) != program as u64
            || self.read(program, COUNT * 8)? != expected
        {
            return Err(invalid());
        }
        Ok(())
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        if self.reaped {
            return;
        }
        // SAFETY: only our own still-unreaped child; pidfd after acquisition.
        unsafe {
            if let Some(fd) = &self.pidfd {
                libc::syscall(
                    libc::SYS_pidfd_send_signal,
                    fd.as_raw_fd(),
                    libc::SIGKILL,
                    0usize,
                    0u32,
                );
            } else {
                libc::kill(self.pid, libc::SIGKILL);
            }
        }
        if let Err(error) = self.wait() {
            eprintln!("owned seccomp fixture cleanup failed: {error}");
        }
    }
}
fn probe(case: Case, fault: Fault) -> io::Result<()> {
    // SAFETY: no-argument effective UID query; this cohort never runs as root.
    if unsafe { libc::geteuid() } == 0 {
        return Err(invalid());
    }
    let executable = File::open("/bin/true")?;
    let fd = executable.as_raw_fd();
    if fd < 3 {
        return Err(invalid());
    }
    let mut program = policy();
    if matches!(fault, Fault::Policy) {
        program[COUNT - 1].k = libc::SECCOMP_RET_KILL_PROCESS;
    }
    let mut other_program = program;
    let mut header = libc::sock_fprog {
        len: COUNT as u16,
        filter: program.as_mut_ptr(),
    };
    if matches!(fault, Fault::Length) {
        header.len = 1;
    }
    if matches!(fault, Fault::Pointer) {
        header.filter = other_program.as_mut_ptr();
    }
    let foreign = libc::sock_fprog {
        len: COUNT as u16,
        filter: program.as_mut_ptr(),
    };
    let header_address = &header as *const _ as usize;
    let program_address = program.as_ptr() as usize;
    let supplied = match fault {
        Fault::Header => &foreign,
        _ => &header,
    };
    let flags = if matches!(fault, Fault::Flags) {
        0
    } else {
        libc::SECCOMP_FILTER_FLAG_TSYNC
    };
    let nr = match case {
        Case::ExecSelf | Case::ExecOther => libc::SYS_execve as u32,
        Case::ExecFd => libc::SYS_execveat as u32,
        Case::Denied(nr) if DENIED.contains(&nr) => nr,
        Case::Denied(_) => return Err(invalid()),
        Case::X32 => libc::SYS_execve as u32 | X32,
        Case::I386 => 11,
        Case::RestoreDumpable => libc::SYS_prctl as u32,
    };
    // SAFETY: all inputs prepared. child() never allocates/locks/unwinds after fork.
    let pid = unsafe { libc::fork() };
    if pid < 0 {
        return Err(io::Error::last_os_error());
    }
    if pid == 0 {
        child(case, nr, fd, supplied, flags);
    }
    let mut owner = Owner {
        pid,
        pidfd: None,
        reaped: false,
        _thread: PhantomData,
    };
    // SAFETY: original direct fork child, not yet waited/reaped; no adoption.
    let handle = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0u32) };
    if handle < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: successful syscall created one unowned descriptor.
    owner.pidfd = Some(unsafe { OwnedFd::from_raw_fd(handle as i32) });
    if owner.wait()? != (libc::SIGSTOP << 8) | 0x7f {
        return Err(invalid());
    }
    owner.trace(
        libc::PTRACE_SETOPTIONS,
        (libc::PTRACE_O_TRACESYSGOOD | libc::PTRACE_O_TRACEEXEC | libc::PTRACE_O_EXITKILL) as usize,
    )?;
    let install_entry = owner.event()?;
    match install_entry {
        Event::Entry {
            arch: ARCH,
            nr,
            args,
        } if nr == libc::SYS_seccomp as u64
            && args[0] == libc::SECCOMP_SET_MODE_FILTER as u64
            && args[1] == libc::SECCOMP_FILTER_FLAG_TSYNC
            && args[2] == header_address as u64 => {}
        _ => return Err(invalid()),
    }
    let expected = encoded(&policy());
    owner.memory(header_address, program_address, &expected)?;
    if owner.event()?
        != (Event::Exit {
            arch: ARCH,
            value: 0,
            error: false,
        })
    {
        return Err(invalid());
    }
    owner.memory(header_address, program_address, &expected)?;
    match owner.event()? {
        Event::Entry {
            arch: ARCH,
            nr,
            args,
        } if nr == libc::SYS_prctl as u64
            && args[0] == libc::PR_SET_DUMPABLE as u64
            && args[1] == 0 => {}
        _ => return Err(invalid()),
    }
    if owner.event()?
        != (Event::Exit {
            arch: ARCH,
            value: 0,
            error: false,
        })
    {
        return Err(invalid());
    }
    let wanted_arch = if matches!(case, Case::I386) {
        I386
    } else {
        ARCH
    };
    match owner.event()? {
        Event::Entry {
            arch, nr: observed, ..
        } if arch == wanted_arch && observed == nr as u64 => {}
        _ => return Err(invalid()),
    }
    if matches!(case, Case::X32 | Case::I386) {
        owner.trace(libc::PTRACE_SYSCALL, 0)?;
        let status = owner.wait()?;
        if status == SYSCALL_STOP {
            let event = owner.info()?;
            if event
                != (Event::Exit {
                    arch: wanted_arch,
                    value: nr as i64,
                    error: false,
                })
            {
                return Err(invalid());
            }
            owner.trace(libc::PTRACE_SYSCALL, 0)?;
            let terminal = owner.wait()?;
            if libc::WIFSIGNALED(terminal) && libc::WTERMSIG(terminal) == libc::SIGSYS {
                return Ok(());
            }
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("owned ABI terminal after exact exit refused: {terminal:#x}"),
            ));
        }
        if !libc::WIFSIGNALED(status) || libc::WTERMSIG(status) != libc::SIGSYS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("owned alternate ABI terminal refused: nr={nr:#x} status={status:#x}"),
            ));
        }
    } else {
        if owner.event()?
            != (Event::Exit {
                arch: ARCH,
                value: -(libc::EPERM as i64),
                error: true,
            })
        {
            return Err(invalid());
        }
        match owner.event()? {
            Event::Entry {
                arch: ARCH,
                nr,
                args,
            } if nr == libc::SYS_exit_group as u64 && args[0] == 0 => {}
            _ => return Err(invalid()),
        }
        owner.trace(libc::PTRACE_SYSCALL, 0)?;
        if owner.wait()? != 0 {
            return Err(invalid());
        }
    }
    Ok(())
}
fn child(case: Case, nr: u32, fd: i32, header: &libc::sock_fprog, flags: u64) -> ! {
    // SAFETY: fixed prepared inputs; only libc/core/asm until _exit. No allocation,
    // IO, locks, Rust destructors or arbitrary helper/credential execution.
    unsafe {
        let keep = fd as u32;
        let core_limit = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        if (keep > 3 && libc::syscall(libc::SYS_close_range, 3u32, keep - 1, 0u32) < 0)
            || libc::syscall(libc::SYS_close_range, keep + 1, u32::MAX, 0u32) < 0
            || libc::setrlimit(libc::RLIMIT_CORE, &core_limit) < 0
            || libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) < 0
            || libc::ptrace(libc::PTRACE_TRACEME, 0, 0usize, 0usize) < 0
            || libc::kill(libc::getpid(), libc::SIGSTOP) < 0
        {
            libc::_exit(70);
        }
        if libc::syscall(
            libc::SYS_seccomp,
            libc::SECCOMP_SET_MODE_FILTER,
            flags,
            header,
        ) != 0
        {
            libc::_exit(71);
        }
        if libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0) < 0 {
            libc::_exit(74);
        }
        let argv = [c"owned-seccomp-probe".as_ptr(), std::ptr::null()];
        let env = [std::ptr::null::<libc::c_char>()];
        let result = match case {
            Case::ExecSelf => libc::syscall(
                nr as libc::c_long,
                c"/proc/self/exe".as_ptr(),
                argv.as_ptr(),
                env.as_ptr(),
            ),
            Case::ExecOther => libc::syscall(
                nr as libc::c_long,
                c"/bin/true".as_ptr(),
                argv.as_ptr(),
                env.as_ptr(),
            ),
            Case::ExecFd => libc::syscall(
                nr as libc::c_long,
                fd,
                c"".as_ptr(),
                argv.as_ptr(),
                env.as_ptr(),
                libc::AT_EMPTY_PATH,
            ),
            Case::X32 => libc::syscall(nr as libc::c_long, 0usize, 0usize, 0usize),
            Case::I386 => {
                core::arch::asm!("push rbx", "xor ebx, ebx", "xor ecx, ecx", "xor edx, edx", "mov eax, 11", "int 0x80", "pop rbx", lateout("eax") _, lateout("ecx") _, lateout("edx") _);
                libc::_exit(72);
            }
            Case::RestoreDumpable => libc::syscall(
                nr as libc::c_long,
                libc::PR_SET_DUMPABLE as u64,
                1u64,
                0u64,
                0u64,
                0u64,
            ),
            Case::Denied(_) => libc::syscall(
                nr as libc::c_long,
                0usize,
                0usize,
                0usize,
                0usize,
                0usize,
                0usize,
            ),
        };
        libc::_exit(
            if result == -1 && *libc::__errno_location() == libc::EPERM {
                0
            } else {
                73
            },
        );
    }
}

#[path = "diagnostic_signer_seccomp_fixture_tests.rs"]
mod tests;
