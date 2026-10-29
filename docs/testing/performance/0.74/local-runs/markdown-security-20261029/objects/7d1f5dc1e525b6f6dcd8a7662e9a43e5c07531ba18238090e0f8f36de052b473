use std::io;
use std::mem::{offset_of, zeroed};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

const READY: &[u8] = b"READY=1\nSTATUS=HydraCache 0.74 supervisor accepting requests";

pub fn ready() -> io::Result<bool> {
    let Some(socket) = std::env::var_os("NOTIFY_SOCKET") else {
        return Ok(false);
    };
    notify(socket.as_os_str().as_bytes(), READY)?;
    Ok(true)
}

pub fn notify_path(path: &Path, message: &[u8]) -> io::Result<()> {
    notify(path.as_os_str().as_bytes(), message)
}

fn notify(name: &[u8], message: &[u8]) -> io::Result<()> {
    if name.is_empty() || message.is_empty() || message.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid sd_notify address or message",
        ));
    }
    // SAFETY: socket has no pointer arguments and returns a new descriptor on success.
    let raw = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_DGRAM | libc::SOCK_CLOEXEC, 0) };
    if raw < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: socket returned a new owned descriptor.
    let fd = unsafe { OwnedFd::from_raw_fd(raw) };
    // SAFETY: zeroed sockaddr_un is a valid starting representation.
    let mut address: libc::sockaddr_un = unsafe { zeroed() };
    address.sun_family = libc::AF_UNIX as libc::sa_family_t;
    let abstract_name = name.first() == Some(&b'@');
    let encoded = if abstract_name { &name[1..] } else { name };
    let prefix = usize::from(abstract_name);
    if encoded.is_empty() || encoded.len() + prefix >= address.sun_path.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "sd_notify address is too long",
        ));
    }
    for (target, source) in address.sun_path[prefix..].iter_mut().zip(encoded) {
        *target = *source as libc::c_char;
    }
    let terminator = usize::from(!abstract_name);
    let length = offset_of!(libc::sockaddr_un, sun_path) + prefix + encoded.len() + terminator;
    // SAFETY: address and message remain valid for the blocking sendto call.
    let sent = unsafe {
        libc::sendto(
            fd.as_raw_fd(),
            message.as_ptr().cast(),
            message.len(),
            libc::MSG_NOSIGNAL,
            (&address as *const libc::sockaddr_un).cast(),
            length as libc::socklen_t,
        )
    };
    if sent < 0 {
        return Err(io::Error::last_os_error());
    }
    if sent as usize != message.len() {
        return Err(io::Error::new(
            io::ErrorKind::WriteZero,
            "partial sd_notify datagram",
        ));
    }
    Ok(())
}
