use super::*;
use std::mem::{size_of, zeroed};
use std::os::fd::FromRawFd;

// Linux UAPI include/linux/socket.h. libc exposes SO_PASSPIDFD but not this tag.
const SCM_PIDFD: i32 = 4;
const CONTROL_BYTES: usize = 1088;
#[repr(C, align(16))]
struct Control([u8; CONTROL_BYTES]);
fn option(fd: &OwnedFd, name: i32) -> Result<i32, KernelError> {
    let mut value = 0i32;
    let mut len = size_of::<i32>() as libc::socklen_t;
    if unsafe {
        libc::getsockopt(
            fd.as_raw_fd(),
            libc::SOL_SOCKET,
            name,
            (&mut value as *mut i32).cast(),
            &mut len,
        )
    } != 0
    {
        return Err(std::io::Error::last_os_error().into());
    }
    if len as usize != size_of::<i32>() {
        return Err(KernelError::Invalid);
    }
    Ok(value)
}
pub(super) fn configure(fd: &OwnedFd) -> Result<(), KernelError> {
    if option(fd, libc::SO_TYPE)? != libc::SOCK_SEQPACKET
        || option(fd, libc::SO_DOMAIN)? != libc::AF_UNIX
    {
        return Err(KernelError::Invalid);
    }
    let value = 1i32;
    for name in [libc::SO_PASSCRED, libc::SO_PASSPIDFD] {
        if unsafe {
            libc::setsockopt(
                fd.as_raw_fd(),
                libc::SOL_SOCKET,
                name,
                (&value as *const i32).cast(),
                size_of::<i32>() as libc::socklen_t,
            )
        } != 0
        {
            return Err(std::io::Error::last_os_error().into());
        }
    }
    revalidate_socket(fd)
}
pub(super) fn revalidate_socket(fd: &OwnedFd) -> Result<(), KernelError> {
    if option(fd, libc::SO_TYPE)? != libc::SOCK_SEQPACKET
        || option(fd, libc::SO_DOMAIN)? != libc::AF_UNIX
        || option(fd, libc::SO_PASSCRED)? != 1
        || option(fd, libc::SO_PASSPIDFD)? != 1
        || unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC == 0
    {
        return Err(KernelError::Invalid);
    }
    Ok(())
}
pub(super) fn receive(
    fd: &OwnedFd,
) -> Result<Option<(Vec<u8>, libc::ucred, OwnedFd)>, KernelError> {
    let mut bytes = vec![0; MAX_PACKET_BYTES + 1];
    let mut control = Control([0; CONTROL_BYTES]);
    let mut iov = libc::iovec {
        iov_base: bytes.as_mut_ptr().cast(),
        iov_len: bytes.len(),
    };
    let mut msg: libc::msghdr = unsafe { zeroed() };
    msg.msg_iov = &mut iov;
    msg.msg_iovlen = 1;
    msg.msg_control = control.0.as_mut_ptr().cast();
    msg.msg_controllen = CONTROL_BYTES;
    let count = unsafe {
        libc::recvmsg(
            fd.as_raw_fd(),
            &mut msg,
            libc::MSG_CMSG_CLOEXEC | libc::MSG_DONTWAIT | libc::MSG_TRUNC,
        )
    };
    if count < 0 {
        let error = std::io::Error::last_os_error();
        if error.kind() == std::io::ErrorKind::WouldBlock {
            return Ok(None);
        }
        return Err(error.into());
    }
    // Scan all kernel-delivered control records before refusing: received FDs
    // must close even when payload size, flags or another control record fail.
    let (credentials, pidfd) = parse(&mut msg)?;
    if count == 0 || count as usize > MAX_PACKET_BYTES {
        return Err(KernelError::Invalid);
    }
    bytes.truncate(count as usize);
    Ok(Some((bytes, credentials, pidfd)))
}
fn parse(msg: &mut libc::msghdr) -> Result<(libc::ucred, OwnedFd), KernelError> {
    if msg.msg_controllen > CONTROL_BYTES {
        return Err(KernelError::Invalid);
    }
    let mut invalid = msg.msg_flags & !(libc::MSG_CMSG_CLOEXEC | libc::MSG_EOR) != 0;
    let mut credentials = None;
    let mut pidfds = Vec::new();
    let mut rights = Vec::new();
    let mut c = unsafe { libc::CMSG_FIRSTHDR(msg) };
    while !c.is_null() {
        let header = unsafe { &*c };
        let base = unsafe { libc::CMSG_LEN(0) } as usize;
        let offset = c as usize - msg.msg_control as usize;
        if header.cmsg_len < base
            || offset
                .checked_add(header.cmsg_len)
                .is_none_or(|end| end > msg.msg_controllen)
        {
            return Err(KernelError::Invalid);
        }
        let length = header.cmsg_len - base;
        let data = unsafe { libc::CMSG_DATA(c) };
        if header.cmsg_level != libc::SOL_SOCKET {
            invalid = true;
        } else if header.cmsg_type == libc::SCM_CREDENTIALS && length == size_of::<libc::ucred>() {
            let value = unsafe { std::ptr::read_unaligned(data.cast::<libc::ucred>()) };
            if credentials.replace(value).is_some() {
                invalid = true;
            }
        } else if header.cmsg_type == SCM_PIDFD || header.cmsg_type == libc::SCM_RIGHTS {
            if !length.is_multiple_of(size_of::<i32>()) {
                invalid = true;
            }
            for index in 0..length / size_of::<i32>() {
                let descriptor = unsafe {
                    std::ptr::read_unaligned(data.add(index * size_of::<i32>()).cast::<i32>())
                };
                if descriptor < 0 {
                    invalid = true;
                    continue;
                }
                let fd = unsafe { OwnedFd::from_raw_fd(descriptor) };
                if header.cmsg_type == SCM_PIDFD {
                    pidfds.push(fd);
                } else {
                    rights.push(fd);
                    invalid = true;
                }
            }
        } else {
            invalid = true;
        }
        c = unsafe { libc::CMSG_NXTHDR(msg, c) };
    }
    if invalid || pidfds.len() != 1 || !rights.is_empty() {
        return Err(KernelError::Invalid);
    }
    let credentials = credentials.ok_or(KernelError::Invalid)?;
    if credentials.pid <= 0 || credentials.uid == u32::MAX || credentials.gid == u32::MAX {
        return Err(KernelError::Invalid);
    }
    let pidfd = pidfds.pop().unwrap();
    if unsafe { libc::fcntl(pidfd.as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC == 0 {
        return Err(KernelError::Invalid);
    }
    Ok((credentials, pidfd))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::os::fd::IntoRawFd;
    #[test]
    fn kernel_ancillary_model_rejects_unknown_flags_and_duplicate_tags_without_fd_leaks() {
        for case in 0..8 {
            let mut fds = [-1; 2];
            assert_eq!(
                unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_NONBLOCK | libc::O_CLOEXEC) },
                0
            );
            let mut reader = unsafe { File::from_raw_fd(fds[0]) };
            let writer = unsafe { OwnedFd::from_raw_fd(fds[1]) };
            let raw = writer.into_raw_fd(); // Private synthetic ownership transfer, not sender authentication.
            let mut control = Control([0; CONTROL_BYTES]);
            let mut msg: libc::msghdr = unsafe { zeroed() };
            msg.msg_control = control.0.as_mut_ptr().cast();
            let credentials = libc::ucred {
                pid: 123,
                uid: 1000,
                gid: 1000,
            };
            let raw_credentials = unsafe {
                std::slice::from_raw_parts(
                    (&credentials as *const libc::ucred).cast::<u8>(),
                    size_of::<libc::ucred>(),
                )
            };
            let mut records = vec![(SCM_PIDFD, raw.to_ne_bytes().to_vec())];
            if case != 0 {
                records.push((libc::SCM_CREDENTIALS, raw_credentials.to_vec()));
            }
            if case == 2 {
                records.push((libc::SCM_CREDENTIALS, raw_credentials.to_vec()));
            }
            if case == 3 {
                records.push((999, vec![0; 4]));
            }
            if case == 4 {
                records[0].0 = libc::SCM_RIGHTS;
            }
            if case == 5 {
                records[1].1 = vec![0; 4];
            }
            if case == 6 {
                let duplicate = unsafe { libc::fcntl(raw, libc::F_DUPFD_CLOEXEC, 0) };
                assert!(duplicate >= 0);
                records.push((SCM_PIDFD, duplicate.to_ne_bytes().to_vec()));
            }
            let mut offset = 0usize;
            for (kind, data) in records {
                let c = unsafe { control.0.as_mut_ptr().add(offset).cast::<libc::cmsghdr>() };
                unsafe {
                    (*c).cmsg_level = libc::SOL_SOCKET;
                    (*c).cmsg_type = kind;
                    (*c).cmsg_len = libc::CMSG_LEN(data.len() as u32) as usize;
                    std::ptr::copy_nonoverlapping(data.as_ptr(), libc::CMSG_DATA(c), data.len());
                }
                offset += unsafe { libc::CMSG_SPACE(data.len() as u32) } as usize;
            }
            msg.msg_controllen = offset;
            if case == 1 {
                msg.msg_flags = libc::MSG_CTRUNC;
            }
            let result = parse(&mut msg);
            if case == 7 {
                assert!(result.is_ok());
                drop(result);
            } else {
                assert!(result.is_err(), "case {case}");
            }
            assert_eq!(
                reader.read(&mut [0]).unwrap(),
                0,
                "synthetic received descriptor leaked"
            );
        }
    }
    #[test]
    fn kernel_ancillary_bounded_malformed_headers_refuse_without_panic() {
        for length in [
            0,
            1,
            size_of::<libc::cmsghdr>() - 1,
            CONTROL_BYTES + 1,
            usize::MAX,
        ] {
            let mut control = Control([0; CONTROL_BYTES]);
            let mut msg: libc::msghdr = unsafe { zeroed() };
            msg.msg_control = control.0.as_mut_ptr().cast();
            msg.msg_controllen = CONTROL_BYTES;
            let c = msg.msg_control.cast::<libc::cmsghdr>();
            unsafe {
                (*c).cmsg_len = length;
            }
            assert!(parse(&mut msg).is_err());
        }
    }
}
