//! Seventeen-role consistency only. Original authority remains closed.
use super::{capture, kernel, ObservedRoles};
use super::super::{read, revalidate as old_revalidate, ObservedAssertions, SEALS};
use hydracache_long_run_supervisor_074::diagnostic_builder::bootstrap_build::installed_entry_format::{AssertedInstalledEntry, BYTES};
use sha2::{Digest, Sha256};
use std::{fs::OpenOptions, io, os::{fd::AsRawFd, unix::fs::OpenOptionsExt}};
fn refused() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "installed bootstrap consistency assertions",
    )
}
fn need(value: bool) -> io::Result<()> {
    if value {
        Ok(())
    } else {
        Err(refused())
    }
}
#[path = "diagnostic_bootstrap_installed_executable.rs"]
mod executable;
#[path = "diagnostic_bootstrap_installed_inventory.rs"]
mod inventory;
#[derive(PartialEq, Eq)]
struct Record {
    stamp: kernel::RoleStamp,
    raw: [u8; BYTES],
}
fn record(fd: i32) -> io::Result<Record> {
    need(fd >= 3)?;
    let stamp = kernel::snapshot(fd)?;
    kernel::kind(&stamp, libc::S_IFREG, libc::O_RDONLY)?;
    need(
        stamp.metadata[6] == BYTES as u64 && unsafe { libc::fcntl(fd, libc::F_GET_SEALS) } == SEALS,
    )?;
    let mut raw = [0; BYTES];
    kernel::bytes(fd, &mut raw)?;
    AssertedInstalledEntry::decode(&raw).map_err(|_| refused())?;
    need(kernel::snapshot(fd)? == stamp)?;
    Ok(Record { stamp, raw })
}
fn installed(s: &kernel::RoleStamp, c: &AssertedInstalledEntry) -> io::Result<()> {
    need(c.installed_identity[2] > 0 && c.installed_identity[2] <= 134217728)?;
    kernel::kind(s, libc::S_IFREG, libc::O_RDONLY)?;
    need(
        kernel::id(s) == c.installed_identity
            && s.metadata[5] == 1
            && matches!(s.metadata[4] & 0o7777, 0o555 | 0o755),
    )?;
    // The unchanged old capture verifies the sealed reference's ELF shape/hash.
    // This separate bounded full hash binds the installed bytes to that reference.
    let mut hash = Sha256::new();
    let mut buffer = [0; 65536];
    let mut at = 0;
    while at < c.installed_identity[2] {
        let count = (c.installed_identity[2] - at).min(buffer.len() as u64) as usize;
        let n = unsafe { libc::pread(s.fd, buffer.as_mut_ptr().cast(), count, at as libc::off_t) };
        if n < 0 {
            return Err(io::Error::last_os_error());
        }
        need(n > 0 && n as usize <= count)?;
        hash.update(&buffer[..n as usize]);
        at += n as u64;
    }
    need(<[u8; 32]>::from(hash.finalize()) == c.installed_sha256 && kernel::snapshot(s.fd)? == *s)
}
#[derive(PartialEq, Eq)]
pub(crate) struct ObservedInstalled {
    original: ObservedAssertions,
    record: Record,
    roles: ObservedRoles,
    installed: kernel::RoleStamp,
    executable: [u64; 11],
}
pub(crate) fn observe() -> io::Result<ObservedInstalled> {
    inventory::inspect()?;
    let original = read(14)?;
    let record = record(16)?;
    let c = AssertedInstalledEntry::decode(&record.raw).map_err(|_| refused())?;
    need(c.original.encode().map_err(|_| refused())? == original.raw)?;
    let roles = capture(
        std::array::from_fn(|i| i as i32),
        &original,
        std::process::id(),
    )?;
    let installed_role = kernel::snapshot(15)?;
    installed(&installed_role, &c)?;
    let regular = [4, 5, 10, 11, 12, 13, 14];
    for slot in regular {
        need(
            kernel::id(&roles.stamps[slot])[..2] != kernel::id(&installed_role)[..2]
                && kernel::id(&roles.stamps[slot])[..2] != kernel::id(&record.stamp)[..2],
        )?;
    }
    need(kernel::id(&record.stamp)[..2] != kernel::id(&installed_role)[..2])?;
    let executable = executable::observe(&installed_role)?;
    need(kernel::snapshot(15)? == installed_role && self::record(16)? == record)?;
    old_revalidate(14, &original)?;
    inventory::inspect()?;
    Ok(ObservedInstalled {
        original,
        record,
        roles,
        installed: installed_role,
        executable,
    })
}
pub(crate) fn revalidate(prior: &ObservedInstalled) -> io::Result<()> {
    need(observe()? == *prior)
}
#[cfg(test)]
#[path = "diagnostic_bootstrap_installed_child_tests.rs"]
mod tests;

#[path = "diagnostic_bootstrap_held_child.rs"]
pub(crate) mod held;
