//! Private process-mutating entry; only the terminating fixed probe can call it.
use super::{program, FixedProbe};
use std::collections::BTreeSet;
use std::fs::{File, OpenOptions};
use std::io::{self, Read};
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;

fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "fixed child isolation refused")
}
fn proc_file(path: &str, directory: bool) -> io::Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(
            libc::O_NOFOLLOW
                | libc::O_CLOEXEC
                | libc::O_NONBLOCK
                | if directory { libc::O_DIRECTORY } else { 0 },
        )
        .open(path)?;
    // SAFETY: fixed filesystem query on one owned descriptor and initialized output.
    let mut fs: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstatfs(file.as_raw_fd(), &mut fs) } != 0
        || fs.f_type as u64 != libc::PROC_SUPER_MAGIC as u64
    {
        return Err(invalid());
    }
    Ok(file)
}
fn field<'a>(raw: &'a str, name: &str) -> io::Result<&'a str> {
    let mut values = raw.lines().filter_map(|line| line.strip_prefix(name));
    let value = values.next().ok_or_else(invalid)?;
    if values.next().is_some() {
        return Err(invalid());
    }
    Ok(value.trim())
}
fn number(raw: &str, name: &str) -> io::Result<u64> {
    let value = field(raw, name)?;
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid());
    }
    value.parse().map_err(|_| invalid())
}
struct Status {
    uid: u32,
    gid: u32,
    nnp: u64,
    filters: u64,
    mode: u64,
}
fn status() -> io::Result<Status> {
    let mut bytes = Vec::new();
    proc_file("/proc/thread-self/status", false)?
        .take(65537)
        .read_to_end(&mut bytes)?;
    if bytes.is_empty() || bytes.len() > 65536 {
        return Err(invalid());
    }
    let raw = std::str::from_utf8(&bytes).map_err(|_| invalid())?;
    // SAFETY: local numeric ID queries only, never host/account enrollment.
    let (uid, gid) = unsafe { (libc::getuid(), libc::getgid()) };
    if unsafe { libc::geteuid() } != uid || unsafe { libc::getegid() } != gid {
        return Err(invalid());
    }
    decode_status(raw, uid, gid)
}
fn decode_status(raw: &str, uid: u32, gid: u32) -> io::Result<Status> {
    if raw.is_empty() || raw.len() > 65536 || uid == 0 || gid == 0 || number(raw, "Threads:")? != 1
    {
        return Err(invalid());
    }
    for (name, expected) in [("Uid:", uid), ("Gid:", gid)] {
        let ids = field(raw, name)?
            .split_whitespace()
            .map(str::parse::<u32>)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| invalid())?;
        if ids.len() != 4 || ids.iter().any(|id| *id != expected) {
            return Err(invalid());
        }
    }
    for name in ["CapInh:", "CapPrm:", "CapEff:", "CapAmb:"] {
        if u64::from_str_radix(field(raw, name)?, 16).map_err(|_| invalid())? != 0 {
            return Err(invalid());
        }
    }
    let s = Status {
        uid,
        gid,
        nnp: number(raw, "NoNewPrivs:")?,
        filters: number(raw, "Seccomp_filters:")?,
        mode: number(raw, "Seccomp:")?,
    };
    if s.nnp > 1
        || s.filters > 65
        || !((s.mode == 0 && s.filters == 0) || (s.mode == 2 && s.filters > 0))
    {
        return Err(invalid());
    }
    Ok(s)
}
fn records(
    raw: &[u8],
    names: &mut BTreeSet<u32>,
    dots: &mut BTreeSet<&'static str>,
) -> io::Result<()> {
    if raw.len() > 512 {
        return Err(invalid());
    }
    let mut at = 0;
    while at < raw.len() {
        if raw.len() - at < 20 {
            return Err(invalid());
        }
        let len = u16::from_ne_bytes([raw[at + 16], raw[at + 17]]) as usize;
        if len < 24 || len & 7 != 0 || len > raw.len() - at {
            return Err(invalid());
        }
        let name = &raw[at + 19..at + len];
        let end = name.iter().position(|b| *b == 0).ok_or_else(invalid)?;
        let name = std::str::from_utf8(&name[..end]).map_err(|_| invalid())?;
        if name == "." || name == ".." {
            if raw[at + 18] != libc::DT_DIR || !dots.insert(if name == "." { "." } else { ".." }) {
                return Err(invalid());
            }
        } else {
            let fd: u32 = name.parse().map_err(|_| invalid())?;
            if name.is_empty()
                || name.len() > 10
                || fd.to_string() != name
                || raw[at + 18] != libc::DT_LNK
                || !names.insert(fd)
            {
                return Err(invalid());
            }
        }
        if names.len() + dots.len() > 8 {
            return Err(invalid());
        }
        at += len;
    }
    Ok(())
}
fn stdio_only() -> io::Result<()> {
    let directory = proc_file("/proc/thread-self/fd", true)?;
    let own = directory.as_raw_fd();
    if own < 3 {
        return Err(invalid());
    }
    let mut names = BTreeSet::new();
    let mut dots = BTreeSet::new();
    let mut eof = false;
    for _ in 0..3 {
        let mut bytes = [0u8; 512];
        // SAFETY: own directory descriptor and a sized initialized stack buffer.
        let n =
            unsafe { libc::syscall(libc::SYS_getdents64, own, bytes.as_mut_ptr(), bytes.len()) };
        if n < 0 {
            return Err(io::Error::last_os_error());
        }
        if n == 0 {
            eof = true;
            break;
        }
        if n as usize > bytes.len() {
            return Err(invalid());
        }
        records(&bytes[..n as usize], &mut names, &mut dots)?;
    }
    if !eof || names != BTreeSet::from([0, 1, 2, own as u32]) || dots != BTreeSet::from([".", ".."])
    {
        return Err(invalid());
    }
    Ok(())
}
fn install() -> io::Result<()> {
    if std::env::vars_os().next().is_some() {
        return Err(invalid());
    }
    // SAFETY: current process/thread identity queries; only the main single-thread child is supported.
    let tid = unsafe { libc::syscall(libc::SYS_gettid) };
    if tid != unsafe { libc::getpid() } as libc::c_long {
        return Err(invalid());
    }
    let before = status()?;
    if before.filters > 64 {
        return Err(invalid());
    }
    stdio_only()?;
    let limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    // SAFETY: restrict only this owned child, fixed arguments, no privilege escalation.
    if unsafe { libc::setrlimit(libc::RLIMIT_CORE, &limit) } != 0
        || unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1usize, 0usize, 0usize, 0usize) } != 0
        || unsafe { libc::prctl(libc::PR_GET_NO_NEW_PRIVS, 0usize, 0usize, 0usize, 0usize) } != 1
    {
        return Err(invalid());
    }
    let mut filters = program().map(|i| libc::sock_filter {
        code: i.code,
        jt: i.jt,
        jf: i.jf,
        k: i.k,
    });
    let header = libc::sock_fprog {
        len: 64,
        filter: filters.as_mut_ptr(),
    };
    // SAFETY: fixed complete BPF array/header remain live across the synchronous TSYNC install.
    if unsafe {
        libc::syscall(
            libc::SYS_seccomp,
            libc::SECCOMP_SET_MODE_FILTER,
            libc::SECCOMP_FILTER_FLAG_TSYNC,
            &header,
        )
    } != 0
        || unsafe { libc::prctl(libc::PR_SET_DUMPABLE, 0usize, 0usize, 0usize, 0usize) } != 0
    {
        return Err(invalid());
    }
    let after = status()?;
    stdio_only()?;
    if after.uid != before.uid
        || after.gid != before.gid
        || after.nnp != 1
        || after.mode != 2
        || after.filters != before.filters + 1
        || unsafe { libc::syscall(libc::SYS_gettid) } != tid
    {
        return Err(invalid());
    }
    Ok(())
}
fn finish(code: i32) -> ! {
    // SAFETY: terminal current probe process only; never resume after a partial install.
    unsafe { libc::_exit(code) }
}
fn native() -> bool {
    let path = c"/".as_ptr() as u64;
    for (nr, args) in [
        (272, [0, 0, 0, 0, 0, 0]),
        (308, [u64::MAX, 0, 0, 0, 0, 0]),
        (161, [0, 0, 0, 0, 0, 0]),
        (80, [path, 0, 0, 0, 0, 0]),
        (81, [u64::MAX, 0, 0, 0, 0, 0]),
        (165, [0, 0, 0, 0, 0, 0]),
        (166, [0, 0, 0, 0, 0, 0]),
        (155, [0, 0, 0, 0, 0, 0]),
        (428, [u64::MAX, 0, 0, 0, 0, 0]),
        (429, [u64::MAX, 0, u64::MAX, 0, 0, 0]),
        (430, [0, 0, 0, 0, 0, 0]),
        (431, [u64::MAX, 0, 0, 0, 0, 0]),
        (432, [u64::MAX, 0, 0, 0, 0, 0]),
        (433, [u64::MAX, 0, 0, 0, 0, 0]),
        (442, [u64::MAX, 0, 0, 0, 0, 0]),
        (59, [0, 0, 0, 0, 0, 0]),
        (322, [u64::MAX, 0, 0, 0, 0, 0]),
        (101, [u64::MAX, 0, 0, 0, 0, 0]),
        (317, [u64::MAX, 0, 0, 0, 0, 0]),
        (157, [4, 1, 0, 0, 0, 0]),
    ] {
        // SAFETY: fixed nonproduct denial probes. Bad pointers/FDs cannot exec/mount;
        // chdir('/') affects only this child if a filter defect is found.
        if unsafe {
            libc::syscall(
                nr as libc::c_long,
                args[0],
                args[1],
                args[2],
                args[3],
                args[4],
                args[5],
            )
        } != -1
            || io::Error::last_os_error().raw_os_error() != Some(libc::EPERM)
        {
            return false;
        }
    }
    true
}
pub(super) fn run(probe: FixedProbe) -> ! {
    if install().is_err() {
        finish(2);
    }
    match probe {
        FixedProbe::Native => {
            if !native() {
                finish(3);
            }
            let line = b"fixed child isolation probe passed; admission=false\n";
            // SAFETY: fixed public diagnostic bytes; no key/token/PID or authority output.
            if unsafe { libc::write(1, line.as_ptr().cast(), line.len()) } != line.len() as isize {
                finish(2);
            }
            finish(0)
        }
        FixedProbe::X32 => {
            // SAFETY: owned core-disabled child must be killed by the fixed architecture gate.
            unsafe { libc::syscall(0x4000_0027 as libc::c_long) };
            finish(3)
        }
        FixedProbe::I386 => {
            // SAFETY: fixed getpid compat probe in an owned core-disabled child; no pointer arguments.
            unsafe {
                core::arch::asm!("push rbx","xor ebx, ebx","xor ecx, ecx","xor edx, edx","mov eax, 20","int 0x80","pop rbx",lateout("eax") _,lateout("ecx") _,lateout("edx") _)
            };
            finish(3)
        }
    }
}

#[cfg(test)]
#[path = "diagnostic_signer_isolation_linux_tests.rs"]
mod tests;
