//! Readiness consistency frame, never an enrollment or execution permit.
use super::*;
use crate::held_mode::transport::exchange::Challenge;
#[path = "diagnostic_bootstrap_held_inventory.rs"]
mod inventory;
#[derive(PartialEq, Eq)]
struct SocketStamp {
    metadata: [u64; 11],
    status: i32,
}
fn socket(flag: i32) -> io::Result<SocketStamp> {
    let mut m: libc::stat = unsafe { std::mem::zeroed() };
    need(unsafe { libc::fstat(17, &mut m) } == 0)?;
    let status = unsafe { libc::fcntl(17, libc::F_GETFL) };
    need(m.st_mode & libc::S_IFMT == libc::S_IFSOCK && unsafe { libc::fcntl(17, libc::F_GETFD) } == flag
        && status >= 0 && status & (libc::O_ACCMODE | libc::O_PATH | libc::O_NONBLOCK) == libc::O_RDWR | libc::O_NONBLOCK)?;
    for (key, expected) in [(libc::SO_TYPE, libc::SOCK_SEQPACKET), (libc::SO_DOMAIN, libc::AF_UNIX), (libc::SO_PASSCRED, 1), (76, 1)] {
        let mut value = 0i32;
        let mut size = std::mem::size_of_val(&value) as libc::socklen_t;
        need(unsafe { libc::getsockopt(17, libc::SOL_SOCKET, key, (&mut value as *mut i32).cast(), &mut size) } == 0
            && size as usize == std::mem::size_of::<i32>() && value == expected)?;
    }
    Ok(SocketStamp { metadata: [m.st_dev,m.st_ino,u64::from(m.st_uid),u64::from(m.st_gid),u64::from(m.st_mode),m.st_nlink,m.st_size as u64,m.st_mtime as u64,m.st_mtime_nsec as u64,m.st_ctime as u64,m.st_ctime_nsec as u64], status })
}
#[derive(PartialEq, Eq)]
pub(crate) struct ObservedHeld {
    installed: ObservedInstalled,
    readiness: SocketStamp,
}
fn observe(flag: i32) -> io::Result<ObservedHeld> {
    inventory::inspect()?;
    let readiness = socket(flag)?;
    // Same FD0..16 checks and order as the old mode, with a separate closed set.
    let original = read(14)?;
    let record = record(16)?;
    let c = AssertedInstalledEntry::decode(&record.raw).map_err(|_| refused())?;
    need(c.original.encode().map_err(|_| refused())? == original.raw)?;
    let roles = capture(std::array::from_fn(|i| i as i32), &original, std::process::id())?;
    let installed_role = kernel::snapshot(15)?;
    installed(&installed_role, &c)?;
    for slot in [4,5,10,11,12,13,14] {
        need(kernel::id(&roles.stamps[slot])[..2] != kernel::id(&installed_role)[..2]
            && kernel::id(&roles.stamps[slot])[..2] != kernel::id(&record.stamp)[..2])?;
    }
    need(kernel::id(&record.stamp)[..2] != kernel::id(&installed_role)[..2])?;
    need(readiness.metadata[..2] != roles.stamps[3].metadata[..2])?;
    let executable = executable::observe(&installed_role)?;
    need(kernel::snapshot(15)? == installed_role && self::record(16)? == record)?;
    old_revalidate(14,&original)?;
    need(socket(flag)? == readiness)?;
    inventory::inspect()?;
    Ok(ObservedHeld { installed: ObservedInstalled { original,record,roles,installed:installed_role,executable },readiness })
}
pub(crate) fn prepare() -> io::Result<ObservedHeld> {
    let inherited = observe(0)?;
    need(unsafe { libc::fcntl(17,libc::F_SETFD,libc::FD_CLOEXEC) } == 0)?;
    let active = observe(libc::FD_CLOEXEC)?;
    need(inherited == active)?;
    Ok(active)
}
impl ObservedHeld {
    pub(crate) fn revalidate(&self) -> io::Result<()> { need(observe(libc::FD_CLOEXEC)? == *self) }
    pub(crate) fn expected(&self, invocation: [u8;16]) -> io::Result<Challenge> {
        let c = AssertedInstalledEntry::decode(&self.installed.record.raw).map_err(|_| refused())?;
        let owner = c.original.owner;
        let cast = |v| u32::try_from(v).map_err(|_| refused());
        Ok(Challenge { deadline:c.original.bounds[3],creator:[cast(owner[0])?,cast(owner[4])?,cast(owner[5])?],
            child:std::process::id(),nonce:[0;32],entry:Sha256::digest(self.installed.record.raw).into(),
            context:c.original.original_context,manager:c.original.names[0],client:c.original.names[1],invocation })
    }
}
