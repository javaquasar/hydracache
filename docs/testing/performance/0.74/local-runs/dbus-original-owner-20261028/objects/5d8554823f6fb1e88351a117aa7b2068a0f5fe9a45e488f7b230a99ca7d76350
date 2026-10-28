//! Test-only report IPC for the independent original-tracer-loss observer.
use super::*;
use std::mem::{size_of, zeroed};

pub(super) fn sockets() -> io::Result<(File, File)> {
    let (channel, child) = pair(Case::Good)?;
    let parent = channel.socket;
    let yes = 1i32;
    // SAFETY: only the new private observer socket; fixed initialized option input.
    if unsafe {
        libc::setsockopt(
            parent.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PASSCRED,
            (&yes as *const i32).cast(),
            size_of::<i32>() as libc::socklen_t,
        )
    } != 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok((parent, child))
}
pub(super) fn send(fd: &File, bytes: &[u8], fds: &[i32]) -> io::Result<()> {
    if bytes.len() > 32 || fds.len() > 5 {
        return Err(invalid());
    }
    let mut control = [0usize; 16];
    let mut iov = libc::iovec {
        iov_base: bytes.as_ptr() as *mut _,
        iov_len: bytes.len(),
    };
    // SAFETY: initialized libc integral/pointer structure.
    let mut msg: libc::msghdr = unsafe { zeroed() };
    msg.msg_iov = &mut iov;
    msg.msg_iovlen = 1;
    msg.msg_control = control.as_mut_ptr().cast();
    // SAFETY: fixed bounded CMSG sizes and aligned writable stack storage.
    unsafe {
        msg.msg_controllen = libc::CMSG_SPACE(std::mem::size_of_val(fds) as u32) as usize;
        let c = libc::CMSG_FIRSTHDR(&msg);
        if c.is_null() || msg.msg_controllen > 128 {
            return Err(invalid());
        }
        (*c).cmsg_level = libc::SOL_SOCKET;
        (*c).cmsg_type = libc::SCM_RIGHTS;
        (*c).cmsg_len = libc::CMSG_LEN(std::mem::size_of_val(fds) as u32) as usize;
        std::ptr::copy_nonoverlapping(
            fds.as_ptr().cast::<u8>(),
            libc::CMSG_DATA(c),
            std::mem::size_of_val(fds),
        );
        if libc::sendmsg(
            fd.as_raw_fd(),
            &msg,
            libc::MSG_NOSIGNAL | libc::MSG_DONTWAIT,
        ) != bytes.len() as isize
        {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}
pub(super) struct Report {
    pub(super) bytes: [u8; 32],
    pub(super) fds: Vec<File>,
}
pub(super) fn packet(fd: &File, original_tracer: u32) -> io::Result<(Vec<u8>, Vec<File>)> {
    readable(fd, 5000)?;
    let mut bytes = [0u8; 32];
    let mut control = [0usize; 16];
    let mut iov = libc::iovec {
        iov_base: bytes.as_mut_ptr().cast(),
        iov_len: bytes.len(),
    };
    // SAFETY: initialized kernel output; pointers below cover all owned stack buffers.
    let mut msg: libc::msghdr = unsafe { zeroed() };
    msg.msg_iov = &mut iov;
    msg.msg_iovlen = 1;
    msg.msg_control = control.as_mut_ptr().cast();
    msg.msg_controllen = 128;
    // SAFETY: exactly bounded data/control outputs, nonblocking owned socket.
    let count = unsafe {
        libc::recvmsg(
            fd.as_raw_fd(),
            &mut msg,
            libc::MSG_CMSG_CLOEXEC | libc::MSG_DONTWAIT | libc::MSG_TRUNC,
        )
    };
    if count < 0 {
        return Err(io::Error::last_os_error());
    }
    // Own every delivered FD before checking payload/credentials: refusal must close it.
    let mut fds = Vec::new();
    let mut creds = None;
    let mut rights = 0;
    let mut bad =
        msg.msg_controllen > 128 || msg.msg_flags & !(libc::MSG_CMSG_CLOEXEC | libc::MSG_EOR) != 0;
    // SAFETY: kernel-created ancillary records in the aligned bounded recvmsg output.
    unsafe {
        let mut c = libc::CMSG_FIRSTHDR(&msg);
        while !c.is_null() {
            let head = &*c;
            let base = libc::CMSG_LEN(0) as usize;
            let offset = c as usize - msg.msg_control as usize;
            if head.cmsg_len < base
                || offset
                    .checked_add(head.cmsg_len)
                    .is_none_or(|end| end > msg.msg_controllen)
            {
                return Err(invalid());
            }
            let n = head.cmsg_len - base;
            let data = libc::CMSG_DATA(c);
            if head.cmsg_level == libc::SOL_SOCKET && head.cmsg_type == libc::SCM_RIGHTS {
                rights += 1;
                bad |= !n.is_multiple_of(size_of::<i32>());
                for index in 0..n / size_of::<i32>() {
                    let raw = std::ptr::read_unaligned(data.add(index * 4).cast::<i32>());
                    if raw < 0 {
                        bad = true;
                    } else {
                        fds.push(File::from_raw_fd(raw));
                    }
                }
            } else if head.cmsg_level == libc::SOL_SOCKET
                && head.cmsg_type == libc::SCM_CREDENTIALS
                && n == size_of::<libc::ucred>()
            {
                bad |= creds
                    .replace(std::ptr::read_unaligned(data.cast::<libc::ucred>()))
                    .is_some();
            } else {
                bad = true;
            }
            c = libc::CMSG_NXTHDR(&msg, c);
        }
    }
    let creds = creds.ok_or_else(invalid)?;
    // SAFETY: observer's own unchanged nonroot namespace identity.
    let ids = unsafe { (libc::geteuid(), libc::getegid()) };
    if bad
        || count == 0
        || count > 32
        || rights != usize::from(!fds.is_empty())
        || fds.len() > 5
        || creds.pid as u32 != original_tracer
        || (creds.uid, creds.gid) != ids
    {
        return Err(invalid());
    }
    for file in &fds {
        // SAFETY: fixed flags query on a freshly owned kernel-delivered FD.
        if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC == 0 {
            return Err(invalid());
        }
    }
    Ok((bytes[..count as usize].to_vec(), fds))
}
pub(super) fn receive(fd: &File, original_tracer: u32, stage: Stage) -> io::Result<Report> {
    for ordinal in 0..=stage as u8 + 1 {
        let (bytes, fds) = packet(fd, original_tracer)?;
        if bytes != [ordinal] || !fds.is_empty() {
            return Err(invalid());
        }
    }
    let (bytes, fds) = packet(fd, original_tracer)?;
    if bytes.len() != 32 || fds.len() != 5 {
        return Err(invalid());
    }
    Ok(Report {
        bytes: bytes.try_into().map_err(|_| invalid())?,
        fds,
    })
}
pub(super) fn response(fd: &File, value: u8) -> io::Result<()> {
    // SAFETY: one bounded private control byte, never a credential.
    if unsafe {
        libc::send(
            fd.as_raw_fd(),
            (&value as *const u8).cast(),
            1,
            libc::MSG_NOSIGNAL | libc::MSG_DONTWAIT,
        )
    } != 1
    {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
pub(super) fn command(fd: &File) -> io::Result<u8> {
    readable(fd, 5000)?;
    let mut byte = 255u8;
    // SAFETY: one-byte initialized stack output; oversized packet length detected.
    let n = unsafe {
        libc::recv(
            fd.as_raw_fd(),
            (&mut byte as *mut u8).cast(),
            1,
            libc::MSG_DONTWAIT | libc::MSG_TRUNC,
        )
    };
    if n != 1 || byte > 1 {
        return Err(invalid());
    }
    Ok(byte)
}
