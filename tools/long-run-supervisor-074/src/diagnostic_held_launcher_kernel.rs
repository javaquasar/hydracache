//! Fresh helper process only. No public library entry or original-owner permit.
use hydracache_long_run_supervisor_074::diagnostic_artifacts::{
    MAX_BINARY_BYTES, MAX_CONFIG_BYTES,
};
use hydracache_long_run_supervisor_074::diagnostic_lease::{BINARY_PATH, SOURCE_COMMIT};
use sha2::{Digest, Sha256};
use std::ffi::CString;
use std::fs::{File, Metadata, OpenOptions};
use std::io::{self, Read};
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::fs::{FileExt, MetadataExt, OpenOptionsExt};

const NS: u64 = 1_000_000_000;
const BOOT_BYTES: usize = 256;
const PACKET_BYTES: usize = 80;
const SEALS: i32 = libc::F_SEAL_SEAL | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_WRITE;
fn refused() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "held FD protocol")
}
fn need(value: bool) -> io::Result<()> {
    if value {
        Ok(())
    } else {
        Err(refused())
    }
}
fn now() -> io::Result<u64> {
    let mut t: libc::timespec = unsafe { std::mem::zeroed() };
    need(unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut t) } == 0)?;
    need(t.tv_sec >= 0 && (0..NS as i64).contains(&t.tv_nsec))?;
    (t.tv_sec as u64)
        .checked_mul(NS)
        .and_then(|v| v.checked_add(t.tv_nsec as u64))
        .ok_or_else(refused)
}
fn u64_at(b: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(b[at..at + 8].try_into().unwrap())
}
fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}
fn identity(m: &Metadata) -> [u64; 3] {
    [m.dev(), m.ino(), m.len()]
}
fn sealed(fd: RawFd) -> bool {
    (unsafe { libc::fcntl(fd, libc::F_GET_SEALS) }) == SEALS
}
fn readonly(f: &File) -> io::Result<()> {
    let flags = unsafe { libc::fcntl(f.as_raw_fd(), libc::F_GETFL) };
    need(
        flags >= 0
            && flags & (libc::O_ACCMODE | libc::O_PATH) == libc::O_RDONLY
            && unsafe { libc::fcntl(f.as_raw_fd(), libc::F_GETFD) } == libc::FD_CLOEXEC,
    )
}
fn read_exact_at(f: &File, raw: &mut [u8]) -> io::Result<()> {
    f.read_exact_at(raw, 0)
}
fn hash(f: &File, size: u64) -> io::Result<[u8; 32]> {
    let mut sha = Sha256::new();
    let mut offset = 0;
    let mut buffer = [0; 65536];
    while offset < size {
        let count = ((size - offset).min(buffer.len() as u64)) as usize;
        let n = f.read_at(&mut buffer[..count], offset)?;
        need(n > 0)?;
        sha.update(&buffer[..n]);
        offset += n as u64;
    }
    Ok(sha.finalize().into())
}
#[derive(Clone, PartialEq, Eq)]
struct Stamp {
    id: [u64; 3],
    owner: [u32; 3],
    times: [i64; 4],
    links: u64,
}
fn stamp(m: &Metadata) -> Stamp {
    Stamp {
        id: identity(m),
        owner: [m.uid(), m.gid(), m.mode()],
        times: [m.mtime(), m.mtime_nsec(), m.ctime(), m.ctime_nsec()],
        links: m.nlink(),
    }
}
struct Bootstrap {
    raw: [u8; BOOT_BYTES],
    digest: [u8; 32],
    deadline: u64,
    owner: libc::ucred,
    guardians: [i32; 2],
}
impl Bootstrap {
    fn decode(raw: [u8; BOOT_BYTES]) -> io::Result<Self> {
        need(
            &raw[..8] == b"HCHL0741"
                && raw[36..40] == [0; 4]
                && raw[248..] == [0; 8]
                && raw[40..72].iter().any(|b| *b != 0),
        )?;
        let owner = libc::ucred {
            pid: u32_at(&raw, 16).try_into().map_err(|_| refused())?,
            uid: u32_at(&raw, 20),
            gid: u32_at(&raw, 24),
        };
        let guardians =
            [u32_at(&raw, 28), u32_at(&raw, 32)].map(|v| i32::try_from(v).unwrap_or(-1));
        let pids = [owner.pid, guardians[0], guardians[1]];
        need(
            pids.iter().all(|p| *p > 1)
                && pids[0] != pids[1]
                && pids[0] != pids[2]
                && pids[1] != pids[2],
        )?;
        Ok(Self {
            digest: Sha256::digest(raw).into(),
            deadline: u64_at(&raw, 8),
            raw,
            owner,
            guardians,
        })
    }
    fn packet(&self, kind: u8) -> [u8; PACKET_BYTES] {
        let mut packet = [0; PACKET_BYTES];
        packet[..8].copy_from_slice(b"HCLR0741");
        packet[8] = kind;
        packet[16..48].copy_from_slice(&self.raw[40..72]);
        packet[48..].copy_from_slice(&self.digest);
        packet
    }
    fn matches_id(&self, m: &Metadata, at: usize) -> io::Result<()> {
        need(
            identity(m)
                == [
                    u64_at(&self.raw, at),
                    u64_at(&self.raw, at + 8),
                    u64_at(&self.raw, at + 16),
                ],
        )
    }
}
fn proc_text(path: &str) -> io::Result<String> {
    let f = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(path)?;
    let mut fs: libc::statfs = unsafe { std::mem::zeroed() };
    need(
        unsafe { libc::fstatfs(f.as_raw_fd(), &mut fs) } == 0
            && fs.f_type == libc::PROC_SUPER_MAGIC as libc::c_long,
    )?;
    let mut raw = Vec::new();
    f.take(4097).read_to_end(&mut raw)?;
    need(raw.len() <= 4096)?;
    String::from_utf8(raw).map_err(|_| refused())
}
fn fdinfo(fd: RawFd) -> io::Result<String> {
    proc_text(&format!("/proc/thread-self/fdinfo/{fd}"))
}
fn field<'a>(text: &'a str, name: &str) -> io::Result<&'a str> {
    let mut values = text.lines().filter_map(|l| l.strip_prefix(name));
    let first = values.next().ok_or_else(refused)?;
    need(values.next().is_none())?;
    Ok(first.trim())
}
fn nanos(t: libc::timespec) -> io::Result<u64> {
    need(t.tv_sec >= 0 && (0..NS as i64).contains(&t.tv_nsec))?;
    (t.tv_sec as u64)
        .checked_mul(NS)
        .and_then(|v| v.checked_add(t.tv_nsec as u64))
        .ok_or_else(refused)
}
fn guards(b: &Bootstrap) -> io::Result<()> {
    for (fd, pid) in [(6, b.owner.pid), (7, b.guardians[0]), (8, b.guardians[1])] {
        need(field(&fdinfo(fd)?, "Pid:")?.parse::<i32>().ok() == Some(pid))?;
        need(pid != unsafe { libc::getpid() })?;
    }
    let text = fdinfo(9)?;
    need(field(&text, "clockid:")? == "7" && field(&text, "settime flags:")? == "01")?;
    let before = now()?;
    let mut spec: libc::itimerspec = unsafe { std::mem::zeroed() };
    need(unsafe { libc::timerfd_gettime(9, &mut spec) } == 0)?;
    let after = now()?;
    need(after >= before && after < b.deadline && b.deadline - after <= 60 * NS)?;
    let left = nanos(spec.it_value)?;
    need(
        nanos(spec.it_interval)? == 0
            && left > 0
            && left <= b.deadline - before
            && left >= b.deadline - after,
    )?;
    let mut pins = [9, 6, 7, 8].map(|fd| libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    });
    need(unsafe { libc::poll(pins.as_mut_ptr(), pins.len() as _, 0) } == 0)
}
fn peer(b: &Bootstrap) -> io::Result<()> {
    let mut domain = 0i32;
    let mut kind = 0i32;
    for (option, out) in [(libc::SO_DOMAIN, &mut domain), (libc::SO_TYPE, &mut kind)] {
        let mut size = std::mem::size_of::<i32>() as libc::socklen_t;
        need(
            unsafe {
                libc::getsockopt(
                    3,
                    libc::SOL_SOCKET,
                    option,
                    (out as *mut i32).cast(),
                    &mut size,
                )
            } == 0
                && size == std::mem::size_of::<i32>() as libc::socklen_t,
        )?;
    }
    need(domain == libc::AF_UNIX && kind == libc::SOCK_SEQPACKET)?;
    let mut creds: libc::ucred = unsafe { std::mem::zeroed() };
    let mut size = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    need(
        unsafe {
            libc::getsockopt(
                3,
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                (&mut creds as *mut libc::ucred).cast(),
                &mut size,
            )
        } == 0
            && size == std::mem::size_of::<libc::ucred>() as libc::socklen_t
            && creds.pid == b.owner.pid
            && creds.uid == b.owner.uid
            && creds.gid == b.owner.gid,
    )
}
fn stream_ids(b: &Bootstrap) -> io::Result<()> {
    let mut ids = [[0; 2]; 3];
    for fd in 0..3 {
        let mut m: libc::stat = unsafe { std::mem::zeroed() };
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        need(
            unsafe { libc::fstat(fd, &mut m) } == 0
                && m.st_mode & libc::S_IFMT == libc::S_IFIFO
                && flags >= 0
                && flags & (libc::O_ACCMODE | libc::O_PATH)
                    == if fd == 0 {
                        libc::O_RDONLY
                    } else {
                        libc::O_WRONLY
                    },
        )?;
        ids[fd as usize] = [m.st_dev, m.st_ino];
        let at = 184 + fd as usize * 16;
        need(ids[fd as usize] == [u64_at(&b.raw, at), u64_at(&b.raw, at + 8)])?;
    }
    need(ids[0] != ids[1] && ids[0] != ids[2] && ids[1] != ids[2])
}
fn content(files: &[File; 8], b: &Bootstrap) -> io::Result<[Stamp; 2]> {
    let worker = &files[1];
    let config = &files[2];
    readonly(worker)?;
    readonly(config)?;
    let wm = worker.metadata()?;
    let cm = config.metadata()?;
    need(
        wm.is_file()
            && wm.len() > 0
            && wm.len() <= MAX_BINARY_BYTES
            && cm.is_file()
            && cm.len() > 0
            && cm.len() <= MAX_CONFIG_BYTES
            && sealed(config.as_raw_fd()),
    )?;
    need(
        sealed(worker.as_raw_fd())
            || (wm.uid() == 0 && wm.mode() & 0o022 == 0 && wm.mode() & 0o111 != 0),
    )?;
    b.matches_id(&wm, 136)?;
    b.matches_id(&cm, 160)?;
    let before = [stamp(&wm), stamp(&cm)];
    let mut header = [0; 64];
    read_exact_at(worker, &mut header)?;
    need(
        &header[..7] == b"\x7fELF\x02\x01\x01"
            && [2, 3].contains(&u16::from_le_bytes([header[16], header[17]]))
            && u16::from_le_bytes([header[18], header[19]]) == 62
            && u32_at(&header, 20) == 1
            && u16::from_le_bytes([header[52], header[53]]) == 64
            && u16::from_le_bytes([header[54], header[55]]) == 56,
    )?;
    let ph = u64_at(&header, 32);
    let count = u16::from_le_bytes([header[56], header[57]]) as u64;
    need(
        ph >= 64
            && count > 0
            && ph
                .checked_add(count * 56)
                .is_some_and(|end| end <= wm.len()),
    )?;
    need(
        hash(worker, wm.len())?.as_slice() == &b.raw[72..104]
            && hash(config, cm.len())?.as_slice() == &b.raw[104..136],
    )?;
    need(before == [stamp(&worker.metadata()?), stamp(&config.metadata()?)])?;
    Ok(before)
}
fn release_matches(b: &Bootstrap, raw: &[u8; PACKET_BYTES], len: isize, flags: i32) -> bool {
    len == PACKET_BYTES as isize && flags & !libc::MSG_CMSG_CLOEXEC == 0 && *raw == b.packet(2)
}
fn receive(b: &Bootstrap) -> io::Result<()> {
    let mut polls = [9, 6, 7, 8, 3].map(|fd| libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    });
    loop {
        let n = unsafe { libc::poll(polls.as_mut_ptr(), polls.len() as _, -1) };
        if n < 0 && io::Error::last_os_error().raw_os_error() == Some(libc::EINTR) {
            continue;
        }
        need(
            n > 0 && polls[..4].iter().all(|p| p.revents == 0) && polls[4].revents == libc::POLLIN,
        )?;
        break;
    }
    let mut raw = [0; PACKET_BYTES];
    let mut control = [0usize; 16];
    let mut iov = libc::iovec {
        iov_base: raw.as_mut_ptr().cast(),
        iov_len: raw.len(),
    };
    let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
    msg.msg_iov = &mut iov;
    msg.msg_iovlen = 1;
    msg.msg_control = control.as_mut_ptr().cast();
    msg.msg_controllen = std::mem::size_of_val(&control);
    let n = unsafe {
        libc::recvmsg(
            3,
            &mut msg,
            libc::MSG_DONTWAIT | libc::MSG_TRUNC | libc::MSG_CMSG_CLOEXEC,
        )
    };
    let mut valid = true;
    let mut credentials = 0;
    // Kernel-generated cmsg boundaries only; close EVERY delivered unexpected
    // right, including the prefix of a truncated ancillary payload.
    unsafe {
        let mut c = libc::CMSG_FIRSTHDR(&msg);
        while !c.is_null() {
            let len = (*c).cmsg_len.saturating_sub(libc::CMSG_LEN(0) as usize);
            if (*c).cmsg_level == libc::SOL_SOCKET && (*c).cmsg_type == libc::SCM_RIGHTS {
                for i in 0..len / std::mem::size_of::<RawFd>() {
                    libc::close(libc::CMSG_DATA(c).cast::<RawFd>().add(i).read_unaligned());
                }
                valid = false;
            } else if (*c).cmsg_level == libc::SOL_SOCKET
                && (*c).cmsg_type == libc::SCM_CREDENTIALS
                && len == std::mem::size_of::<libc::ucred>()
            {
                let cred = libc::CMSG_DATA(c).cast::<libc::ucred>().read_unaligned();
                credentials += 1;
                valid &=
                    cred.pid == b.owner.pid && cred.uid == b.owner.uid && cred.gid == b.owner.gid;
            } else {
                valid = false;
            }
            c = libc::CMSG_NXTHDR(&msg, c);
        }
    }
    need(valid && credentials == 1 && release_matches(b, &raw, n, msg.msg_flags))?;
    let mut extra = 0u8;
    let n = unsafe {
        libc::recv(
            3,
            (&mut extra as *mut u8).cast(),
            1,
            libc::MSG_DONTWAIT | libc::MSG_TRUNC,
        )
    };
    need(n < 0 && io::Error::last_os_error().raw_os_error() == Some(libc::EAGAIN))
}
pub(super) fn run() -> io::Result<()> {
    need(
        unsafe { libc::geteuid() } != 0
            && unsafe { libc::getuid() } == unsafe { libc::geteuid() }
            && unsafe { libc::getgid() } == unsafe { libc::getegid() },
    )?;
    need(unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) } == 0)?;
    need(std::env::vars_os().next().is_none())?;
    let status = proc_text("/proc/thread-self/status")?;
    for name in ["CapInh:", "CapPrm:", "CapEff:", "CapAmb:"] {
        need(field(&status, name)? == "0000000000000000")?;
    }
    need(field(&status, "Threads:")? == "1" && field(&status, "NoNewPrivs:")? == "1")?;
    need(unsafe { libc::syscall(libc::SYS_close_range, 11u32, u32::MAX, 0u32) } == 0)?;
    for fd in 3..=10 {
        need(unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } == 0)?;
    }
    let files: [File; 8] = std::array::from_fn(|i| unsafe { File::from_raw_fd(3 + i as i32) });
    readonly(&files[7])?;
    need(files[7].metadata()?.len() == BOOT_BYTES as u64 && sealed(10))?;
    let mut raw = [0; BOOT_BYTES];
    read_exact_at(&files[7], &mut raw)?;
    let b = Bootstrap::decode(raw)?;
    let m = files[0].metadata()?;
    need([m.dev(), m.ino()] == [u64_at(&b.raw, 232), u64_at(&b.raw, 240)])?;
    let yes = 1i32;
    need(
        unsafe {
            libc::setsockopt(
                3,
                libc::SOL_SOCKET,
                libc::SO_PASSCRED,
                (&yes as *const i32).cast(),
                4,
            )
        } == 0,
    )?;
    peer(&b)?;
    stream_ids(&b)?;
    let original = content(&files, &b)?;
    guards(&b)?;
    let packet = b.packet(1);
    need(
        unsafe {
            libc::send(
                3,
                packet.as_ptr().cast(),
                packet.len(),
                libc::MSG_NOSIGNAL | libc::MSG_DONTWAIT,
            )
        } == packet.len() as isize,
    )?;
    receive(&b)?;
    peer(&b)?;
    stream_ids(&b)?;
    need(content(&files, &b)? == original)?;
    guards(&b)?;
    let worker_hex = hex(&b.raw[72..104]);
    let args = [
        BINARY_PATH,
        "--run",
        "/proc/self/fd/5",
        worker_hex.as_str(),
        SOURCE_COMMIT,
    ]
    .map(|s| CString::new(s).unwrap());
    let argv = [
        args[0].as_ptr(),
        args[1].as_ptr(),
        args[2].as_ptr(),
        args[3].as_ptr(),
        args[4].as_ptr(),
        std::ptr::null(),
    ];
    let envp: [*const libc::c_char; 1] = [std::ptr::null()];
    // Only config and existing stdio survive. No alternate executable/path,
    // changed command or retry even if the one exec syscall returns an error.
    need(unsafe { libc::fcntl(5, libc::F_SETFD, 0) } == 0)?;
    for fd in 0..3 {
        need(unsafe { libc::fcntl(fd, libc::F_SETFD, 0) } == 0)?;
    }
    guards(&b)?;
    unsafe {
        libc::syscall(
            libc::SYS_execveat,
            4,
            c"".as_ptr(),
            argv.as_ptr(),
            envp.as_ptr(),
            libc::AT_EMPTY_PATH,
        )
    };
    Err(io::Error::last_os_error())
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn boot() -> [u8; BOOT_BYTES] {
        let mut b = [0; BOOT_BYTES];
        b[..8].copy_from_slice(b"HCHL0741");
        for (at, v) in [(16, 10u32), (28, 11), (32, 12)] {
            b[at..at + 4].copy_from_slice(&v.to_le_bytes());
        }
        b[40] = 1;
        b
    }
    #[test]
    fn bootstrap_refuses_reserved_magic_zero_token_and_bad_or_alias_pids() {
        Bootstrap::decode(boot()).unwrap();
        for at in (0..8).chain(36..40).chain(248..256) {
            let mut b = boot();
            b[at] ^= 1;
            assert!(Bootstrap::decode(b).is_err());
        }
        let mut b = boot();
        b[40] = 0;
        assert!(Bootstrap::decode(b).is_err());
        for at in [16, 28, 32] {
            for pid in [0u32, 1, u32::MAX, 10] {
                let mut b = boot();
                b[at..at + 4].copy_from_slice(&pid.to_le_bytes());
                if at == 16 && pid == 10 {
                    continue;
                }
                assert!(Bootstrap::decode(b).is_err());
            }
        }
    }
    #[test]
    fn exact_release_binds_every_packet_and_bootstrap_byte() {
        let b = Bootstrap::decode(boot()).unwrap();
        let good = b.packet(2);
        assert!(release_matches(&b, &good, 80, libc::MSG_CMSG_CLOEXEC));
        for len in [-1, 0, 79, 81] {
            assert!(!release_matches(&b, &good, len, 0));
        }
        for bit in 0..32 {
            let flags = 1i32.wrapping_shl(bit);
            if flags != libc::MSG_CMSG_CLOEXEC {
                assert!(!release_matches(&b, &good, 80, flags));
            }
        }
        assert_ne!(good, b.packet(1));
        for at in 0..PACKET_BYTES {
            let mut bad = good;
            bad[at] ^= 1;
            assert!(!release_matches(&b, &bad, 80, 0));
        }
        for at in 8..248 {
            if (16..40).contains(&at) || at == 40 {
                continue;
            }
            let mut raw = boot();
            raw[at] ^= 1;
            let changed = Bootstrap::decode(raw).unwrap();
            assert!(!release_matches(&changed, &good, 80, 0));
        }
    }
    #[test]
    fn malformed_time_and_duplicate_fdinfo_refuse() {
        assert!(nanos(libc::timespec {
            tv_sec: -1,
            tv_nsec: 0
        })
        .is_err());
        assert!(nanos(libc::timespec {
            tv_sec: 0,
            tv_nsec: NS as _
        })
        .is_err());
        assert_eq!(field("Pid:\t12\n", "Pid:").unwrap(), "12");
        assert!(field("Pid: 12\nPid: 12\n", "Pid:").is_err());
        assert!(field("", "Pid:").is_err());
    }
}
