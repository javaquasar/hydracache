//! All fifteen roles are assertions until original provenance is authenticated.
use super::{AssertedEntry, ObservedAssertions, SEALS};
use std::io;
fn refused() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "bootstrap complete role assertions",
    )
}
fn need(ok: bool) -> io::Result<()> {
    if ok {
        Ok(())
    } else {
        Err(refused())
    }
}
#[path = "diagnostic_bootstrap_child_role_kernel.rs"]
mod kernel;
use kernel::{id, kind, snapshot, RoleStamp};
#[derive(PartialEq, Eq)]
pub(in super::super) struct ObservedRoles {
    stamps: [RoleStamp; 15],
}
fn u64_at(raw: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(raw[at..at + 8].try_into().unwrap())
}
fn u32_at(raw: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(raw[at..at + 4].try_into().unwrap())
}
fn option(fd: i32, name: i32) -> io::Result<i32> {
    let mut value = 0i32;
    let mut size = std::mem::size_of_val(&value) as libc::socklen_t;
    need(
        unsafe {
            libc::getsockopt(
                fd,
                libc::SOL_SOCKET,
                name,
                (&mut value as *mut i32).cast(),
                &mut size,
            )
        } == 0
            && size as usize == std::mem::size_of::<i32>(),
    )?;
    Ok(value)
}
fn stream(s: &RoleStamp, identity: [u64; 2], input: bool) -> io::Result<()> {
    kind(
        s,
        libc::S_IFIFO,
        if input {
            libc::O_RDONLY
        } else {
            libc::O_WRONLY
        },
    )?;
    need(s.metadata[..2] == identity && s.flags[1] & libc::O_NONBLOCK != 0)?;
    let mut p = libc::pollfd {
        fd: s.fd,
        events: if input { libc::POLLIN } else { libc::POLLOUT },
        revents: 0,
    };
    let n = unsafe { libc::poll(&mut p, 1, 0) };
    if input {
        let mut count = 0;
        need(
            unsafe { libc::ioctl(s.fd, libc::FIONREAD, &mut count) } == 0
                && count == 0
                && n == 1
                && p.revents == libc::POLLHUP,
        )?;
    } else {
        need(n >= 0 && p.revents & !(libc::POLLOUT) == 0)?;
    }
    need(snapshot(s.fd)? == *s)
}
fn release(s: &RoleStamp, c: &AssertedEntry) -> io::Result<()> {
    kind(s, libc::S_IFSOCK, libc::O_RDWR)?;
    need(
        s.metadata[..2] == c.streams[3]
            && s.flags[1] & libc::O_NONBLOCK != 0
            && option(s.fd, libc::SO_DOMAIN)? == libc::AF_UNIX
            && option(s.fd, libc::SO_TYPE)? == libc::SOCK_SEQPACKET
            && option(s.fd, libc::SO_PASSCRED)? == 1,
    )?;
    let mut cred: libc::ucred = unsafe { std::mem::zeroed() };
    let mut size = std::mem::size_of_val(&cred) as libc::socklen_t;
    need(
        unsafe {
            libc::getsockopt(
                s.fd,
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                (&mut cred as *mut libc::ucred).cast(),
                &mut size,
            )
        } == 0
            && size as usize == std::mem::size_of::<libc::ucred>()
            && cred.pid as u64 == c.owner[0]
            && u64::from(cred.uid) == c.owner[4]
            && u64::from(cred.gid) == c.owner[5],
    )?;
    let mut byte = 0u8;
    let n = unsafe {
        libc::recv(
            s.fd,
            (&mut byte as *mut u8).cast(),
            1,
            libc::MSG_PEEK | libc::MSG_TRUNC | libc::MSG_DONTWAIT,
        )
    };
    need(
        n < 0
            && io::Error::last_os_error().raw_os_error() == Some(libc::EAGAIN)
            && snapshot(s.fd)? == *s,
    )
}
fn legacy(s: &RoleStamp, c: &AssertedEntry) -> io::Result<()> {
    kernel::regular(s, c.identities[5], c.legacy_hash, 256, false)?;
    let mut raw = [0; 256];
    kernel::bytes(s.fd, &mut raw)?;
    need(
        &raw[..8] == b"HCHL0741"
            && raw[36..40] == [0; 4]
            && raw[248..] == [0; 8]
            && raw[40..72].iter().any(|b| *b != 0)
            && u64_at(&raw, 8) == c.bounds[3]
            && u32_at(&raw, 16) as u64 == c.owner[0]
            && u32_at(&raw, 20) as u64 == c.owner[4]
            && u32_at(&raw, 24) as u64 == c.owner[5]
            && u32_at(&raw, 28) as u64 == c.guardians[0][0]
            && u32_at(&raw, 32) as u64 == c.guardians[1][0]
            && raw[72..104] == c.image_hashes[3]
            && raw[104..136] == c.image_hashes[4],
    )?;
    for (at, values) in [
        (136, c.identities[3].as_slice()),
        (160, c.identities[4].as_slice()),
        (184, c.streams[0].as_slice()),
        (200, c.streams[1].as_slice()),
        (216, c.streams[2].as_slice()),
        (232, c.streams[3].as_slice()),
    ] {
        for (i, v) in values.iter().enumerate() {
            need(u64_at(&raw, at + i * 8) == *v)?;
        }
    }
    need(snapshot(s.fd)? == *s)
}
fn capture(fds: [i32; 15], entry: &ObservedAssertions, child: u32) -> io::Result<ObservedRoles> {
    need(
        fds[14] == entry.stamp.fd
            && fds
                .iter()
                .enumerate()
                .all(|(i, fd)| *fd >= 0 && !fds[..i].contains(fd)),
    )?;
    super::revalidate(fds[14], entry)?;
    let c = AssertedEntry::decode(&entry.raw).map_err(|_| refused())?;
    need(c.owner[0] != u64::from(child) && c.guardians.iter().all(|g| g[0] != u64::from(child)))?;
    // Fixed-size result, fail before creating a partial successful observation.
    let mut values = Vec::with_capacity(15);
    for fd in fds {
        values.push(snapshot(fd)?);
    }
    let stamps: [RoleStamp; 15] = values.try_into().map_err(|_| refused())?;
    for (i, s) in stamps[..3].iter().enumerate() {
        stream(s, c.streams[i], i == 0)?;
    }
    need(
        c.streams
            .iter()
            .enumerate()
            .all(|(i, id)| !c.streams[..i].contains(id)),
    )?;
    release(&stamps[3], &c)?;
    for (slot, at, limit, elf) in [
        (4, 3, 134217728, true),
        (5, 4, 65536, false),
        (11, 0, 134217728, true),
        (12, 1, 134217728, true),
        (13, 2, 134217728, true),
    ] {
        if slot >= 11 {
            need(unsafe { libc::fcntl(fds[slot], libc::F_GET_SEALS) } == SEALS)?;
        }
        kernel::regular(
            &stamps[slot],
            c.identities[at],
            c.image_hashes[at],
            limit,
            elf,
        )?;
    }
    kernel::pidfd(&stamps[6], c.owner[0], None)?;
    for (slot, g) in [7, 8].into_iter().zip(c.guardians) {
        kernel::pidfd(&stamps[slot], g[0], Some([g[1], g[2]]))?;
    }
    need(stamps[9].flags[1] & libc::O_NONBLOCK != 0)?;
    kernel::alarm(&stamps[9], c.bounds[3])?;
    legacy(&stamps[10], &c)?;
    // Anonymous pidfd inodes may alias on older kernels. Regular material may not.
    let files = [4, 5, 10, 11, 12, 13, 14];
    for (i, slot) in files.iter().enumerate() {
        need(
            files[..i]
                .iter()
                .all(|old| id(&stamps[*old])[..2] != id(&stamps[*slot])[..2]),
        )?;
    }
    for (s, fd) in stamps.iter().zip(fds) {
        need(snapshot(fd)? == *s)?;
    }
    super::revalidate(fds[14], entry)?;
    Ok(ObservedRoles { stamps })
}
pub(in super::super) fn observe(entry: &ObservedAssertions) -> io::Result<ObservedRoles> {
    kernel::closed_inventory()?;
    let roles = capture(std::array::from_fn(|i| i as i32), entry, std::process::id())?;
    kernel::closed_inventory()?;
    Ok(roles)
}
pub(in super::super) fn revalidate(
    entry: &ObservedAssertions,
    prior: &ObservedRoles,
) -> io::Result<()> {
    need(observe(entry)? == *prior)
}

#[cfg(test)]
#[path = "diagnostic_bootstrap_child_roles_tests.rs"]
mod tests;

#[path = "diagnostic_bootstrap_installed_child.rs"]
pub(in super::super) mod installed;
