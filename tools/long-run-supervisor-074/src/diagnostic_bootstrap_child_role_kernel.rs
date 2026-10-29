//! Query-only descriptor checks. No role acquisition, repair, traffic or authority.
use super::*;
#[path = "diagnostic_bootstrap_closed_inventory.rs"]
mod inventory;
pub(super) fn closed_inventory() -> io::Result<()> {
    inventory::inspect()
}
use std::fs::OpenOptions;
use std::os::{
    fd::AsRawFd,
    unix::fs::{FileExt, OpenOptionsExt},
};

#[derive(PartialEq, Eq)]
pub(super) struct RoleStamp {
    pub(super) fd: i32,
    pub(super) metadata: [u64; 11],
    pub(super) flags: [i32; 2],
    pub(super) offset: Option<i64>,
}
pub(super) fn snapshot(fd: i32) -> io::Result<RoleStamp> {
    let mut m: libc::stat = unsafe { std::mem::zeroed() };
    need(fd >= 0 && unsafe { libc::fstat(fd, &mut m) } == 0)?;
    let flags = unsafe {
        [
            libc::fcntl(fd, libc::F_GETFD),
            libc::fcntl(fd, libc::F_GETFL),
        ]
    };
    need(flags[0] == 0 && flags[1] >= 0 && flags[1] & libc::O_PATH == 0)?;
    let offset = if m.st_mode & libc::S_IFMT == libc::S_IFREG {
        let n = unsafe { libc::lseek(fd, 0, libc::SEEK_CUR) };
        need(n == 0)?;
        Some(n)
    } else {
        None
    };
    Ok(RoleStamp {
        fd,
        metadata: [
            m.st_dev,
            m.st_ino,
            u64::from(m.st_uid),
            u64::from(m.st_gid),
            u64::from(m.st_mode),
            m.st_nlink,
            m.st_size as u64,
            m.st_mtime as u64,
            m.st_mtime_nsec as u64,
            m.st_ctime as u64,
            m.st_ctime_nsec as u64,
        ],
        flags,
        offset,
    })
}
pub(super) fn id(s: &RoleStamp) -> [u64; 3] {
    [s.metadata[0], s.metadata[1], s.metadata[6]]
}
pub(super) fn kind(s: &RoleStamp, kind: u32, access: i32) -> io::Result<()> {
    need(s.metadata[4] as u32 & libc::S_IFMT == kind && s.flags[1] & libc::O_ACCMODE == access)
}
pub(super) fn bytes(fd: i32, raw: &mut [u8]) -> io::Result<()> {
    let mut at = 0;
    while at < raw.len() {
        let n = unsafe {
            libc::pread(
                fd,
                raw[at..].as_mut_ptr().cast(),
                raw.len() - at,
                at as libc::off_t,
            )
        };
        if n < 0 {
            return Err(io::Error::last_os_error());
        }
        need(n > 0 && n as usize <= raw.len() - at)?;
        at += n as usize;
    }
    Ok(())
}
pub(super) fn regular(
    s: &RoleStamp,
    expected: [u64; 3],
    digest: [u8; 32],
    limit: u64,
    executable: bool,
) -> io::Result<()> {
    kind(s, libc::S_IFREG, libc::O_RDONLY)?;
    need(id(s) == expected && expected[2] > 0 && expected[2] <= limit)?;
    let seals = unsafe { libc::fcntl(s.fd, libc::F_GET_SEALS) };
    // Worker may be the original root-owned executable instead of a sealed
    // image. All other material requires exact seals at its call site.
    need(
        seals == SEALS
            || (executable
                && s.metadata[2] == 0
                && s.metadata[4] & 0o022 == 0
                && s.metadata[4] & 0o111 != 0),
    )?;
    if executable {
        let mut header = [0; 64];
        need(expected[2] >= 64)?;
        bytes(s.fd, &mut header)?;
        let ph = u64::from_le_bytes(header[32..40].try_into().unwrap());
        let count = u16::from_le_bytes(header[56..58].try_into().unwrap()) as u64;
        need(
            &header[..7] == b"\x7fELF\x02\x01\x01"
                && [2, 3].contains(&u16::from_le_bytes([header[16], header[17]]))
                && u16::from_le_bytes([header[18], header[19]]) == 62
                && u32::from_le_bytes(header[20..24].try_into().unwrap()) == 1
                && u16::from_le_bytes([header[52], header[53]]) == 64
                && u16::from_le_bytes([header[54], header[55]]) == 56
                && ph >= 64
                && count > 0
                && ph
                    .checked_add(count * 56)
                    .is_some_and(|end| end <= expected[2]),
        )?;
    }
    let mut hash = ring::digest::Context::new(&ring::digest::SHA256);
    let mut buffer = [0; 65536];
    let mut at = 0;
    while at < expected[2] {
        let count = (expected[2] - at).min(buffer.len() as u64) as usize;
        let n = unsafe { libc::pread(s.fd, buffer.as_mut_ptr().cast(), count, at as libc::off_t) };
        if n < 0 {
            return Err(io::Error::last_os_error());
        }
        need(n > 0 && n as usize <= count)?;
        hash.update(&buffer[..n as usize]);
        at += n as u64;
    }
    need(hash.finish().as_ref() == digest.as_slice() && snapshot(s.fd)? == *s)
}
pub(super) fn info(fd: i32) -> io::Result<String> {
    // Fixed thread-self fdinfo for this slot, never reopen a caller path/PID.
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(format!("/proc/thread-self/fdinfo/{fd}"))?;
    let mut fs: libc::statfs = unsafe { std::mem::zeroed() };
    need(
        unsafe { libc::fstatfs(file.as_raw_fd(), &mut fs) } == 0
            && fs.f_type == libc::PROC_SUPER_MAGIC as libc::c_long,
    )?;
    let mut raw = [0; 4097];
    let n = file.read_at(&mut raw, 0)?;
    need(n > 0 && n <= 4096)?;
    Ok(std::str::from_utf8(&raw[..n])
        .map_err(|_| refused())?
        .into())
}
pub(super) fn field<'a>(text: &'a str, name: &str) -> io::Result<&'a str> {
    let mut values = text.lines().filter_map(|line| line.strip_prefix(name));
    let v = values.next().ok_or_else(refused)?;
    need(values.next().is_none())?;
    Ok(v.trim())
}
pub(super) fn live(fd: i32) -> io::Result<()> {
    let mut p = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    need(unsafe { libc::poll(&mut p, 1, 0) } == 0 && p.revents == 0)
}
pub(super) fn pidfd(s: &RoleStamp, pid: u64, identity: Option<[u64; 2]>) -> io::Result<()> {
    need(pid > 1 && pid <= i32::MAX as u64)?;
    if let Some(id) = identity {
        need(s.metadata[..2] == id)?;
    }
    live(s.fd)?;
    need(field(&info(s.fd)?, "Pid:")?.parse::<u64>().ok() == Some(pid))?;
    live(s.fd)?;
    need(snapshot(s.fd)? == *s)
}
pub(super) fn now() -> io::Result<u64> {
    let mut t: libc::timespec = unsafe { std::mem::zeroed() };
    need(
        unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut t) } == 0
            && t.tv_sec >= 0
            && (0..1_000_000_000).contains(&t.tv_nsec),
    )?;
    (t.tv_sec as u64)
        .checked_mul(1_000_000_000)
        .and_then(|v| v.checked_add(t.tv_nsec as u64))
        .ok_or_else(refused)
}
fn nanos(t: libc::timespec) -> io::Result<u64> {
    need(t.tv_sec >= 0 && (0..1_000_000_000).contains(&t.tv_nsec))?;
    (t.tv_sec as u64)
        .checked_mul(1_000_000_000)
        .and_then(|v| v.checked_add(t.tv_nsec as u64))
        .ok_or_else(refused)
}
pub(super) fn alarm(s: &RoleStamp, deadline: u64) -> io::Result<()> {
    let text = info(s.fd)?;
    need(
        field(&text, "clockid:")? == "7"
            && field(&text, "settime flags:")? == "01"
            && field(&text, "ticks:")? == "0",
    )?;
    let before = now()?;
    let mut spec: libc::itimerspec = unsafe { std::mem::zeroed() };
    need(unsafe { libc::timerfd_gettime(s.fd, &mut spec) } == 0)?;
    let after = now()?;
    need(after >= before && after < deadline && deadline - after <= 60_000_000_000)?;
    let left = nanos(spec.it_value)?;
    need(
        nanos(spec.it_interval)? == 0
            && left > 0
            && left <= deadline - before
            && left >= deadline - after,
    )?;
    live(s.fd)?;
    need(snapshot(s.fd)? == *s)
}

#[cfg(test)]
#[path = "diagnostic_bootstrap_hash_tests.rs"]
mod hash_regression;
