//! Test receiver adapter only; atomically receive rights without an exec leak window.
use std::io;
use std::mem::{size_of, size_of_val, zeroed};
use std::os::fd::{FromRawFd, OwnedFd, RawFd};

pub(super) fn receive(fd: RawFd, bytes: &mut [u8]) -> io::Result<(usize, Vec<OwnedFd>)> {
    receive_at(fd, bytes, &mut 0)
}
fn receive_at(fd: RawFd, bytes: &mut [u8], flags: &mut i32) -> io::Result<(usize, Vec<OwnedFd>)> {
    let mut control = [0usize; 128];
    let mut iov = libc::iovec {
        iov_base: bytes.as_mut_ptr().cast(),
        iov_len: bytes.len(),
    };
    let mut message: libc::msghdr = unsafe { zeroed() };
    message.msg_iov = &mut iov;
    message.msg_iovlen = 1;
    message.msg_control = control.as_mut_ptr().cast();
    message.msg_controllen = size_of_val(&control);
    let count = unsafe {
        libc::recvmsg(
            fd,
            &mut message,
            libc::MSG_DONTWAIT | libc::MSG_CMSG_CLOEXEC,
        )
    };
    if count < 0 {
        return Err(io::Error::last_os_error());
    }
    *flags = message.msg_flags;
    let mut fds = Vec::new();
    let mut unexpected = false;
    let mut header = unsafe { libc::CMSG_FIRSTHDR(&message) };
    // Kernel-produced ancillary headers; take ownership of ALL installed rights
    // before any validation error, so truncation or excess cannot leak descriptors.
    while !header.is_null() {
        unsafe {
            if (*header).cmsg_level == libc::SOL_SOCKET && (*header).cmsg_type == libc::SCM_RIGHTS {
                let length = (*header).cmsg_len - libc::CMSG_LEN(0) as usize;
                for index in 0..length / size_of::<RawFd>() {
                    fds.push(OwnedFd::from_raw_fd(
                        *libc::CMSG_DATA(header).cast::<RawFd>().add(index),
                    ));
                }
                unexpected |= length % size_of::<RawFd>() != 0;
            } else {
                unexpected = true;
            }
            header = libc::CMSG_NXTHDR(&message, header);
        }
    }
    if count == 0 {
        return Err(io::ErrorKind::UnexpectedEof.into());
    }
    if unexpected || message.msg_flags & (libc::MSG_CTRUNC | libc::MSG_TRUNC) != 0 || fds.len() > 2
    {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok((count as usize, fds))
}

#[test]
fn negotiated_atomic_receiver_closes_excess_rights_and_preserves_original_flags() {
    use std::io::Read;
    use std::os::fd::AsRawFd;
    use std::os::unix::net::UnixDatagram;
    for count in [0usize, 2, 3, 253] {
        let (sender, receiver) = UnixDatagram::pair().unwrap();
        let mut pairs: Vec<_> = (0..count).map(|_| super::pipe()).collect();
        let fds: Vec<_> = pairs.iter().map(|pair| pair[1].as_raw_fd()).collect();
        let mut data = [b'x'];
        let mut iov = libc::iovec {
            iov_base: data.as_mut_ptr().cast(),
            iov_len: 1,
        };
        let mut control = [0usize; 140];
        let mut message: libc::msghdr = unsafe { zeroed() };
        message.msg_iov = &mut iov;
        message.msg_iovlen = 1;
        if count != 0 {
            message.msg_control = control.as_mut_ptr().cast();
            message.msg_controllen =
                unsafe { libc::CMSG_SPACE((count * size_of::<RawFd>()) as u32) } as usize;
            unsafe {
                let header = libc::CMSG_FIRSTHDR(&message);
                (*header).cmsg_level = libc::SOL_SOCKET;
                (*header).cmsg_type = libc::SCM_RIGHTS;
                (*header).cmsg_len = libc::CMSG_LEN((count * size_of::<RawFd>()) as u32) as usize;
                std::ptr::copy_nonoverlapping(
                    fds.as_ptr(),
                    libc::CMSG_DATA(header).cast::<RawFd>(),
                    count,
                );
            }
        }
        assert_eq!(
            unsafe { libc::sendmsg(sender.as_raw_fd(), &message, libc::MSG_NOSIGNAL) },
            1
        );
        let mut flags = 0;
        let received = receive_at(receiver.as_raw_fd(), &mut [0; 1], &mut flags);
        assert_eq!(flags & libc::MSG_CTRUNC != 0, count == 253);
        if count <= 2 {
            let (bytes, rights) = received.unwrap();
            assert_eq!(bytes, 1);
            assert_eq!(rights.len(), count);
            for (original, received) in pairs.iter().zip(&rights) {
                assert_eq!(
                    super::writer_shape(original[1].as_raw_fd()).unwrap(),
                    super::writer_shape(received.as_raw_fd()).unwrap()
                );
            }
        } else {
            assert_eq!(received.unwrap_err().kind(), io::ErrorKind::InvalidData);
        }
        // All received aliases must have closed, even after actual MSG_CTRUNC.
        for [mut reader, writer] in pairs.drain(..) {
            drop(writer);
            assert_eq!(reader.read(&mut [0; 1]).unwrap(), 0);
        }
    }
}

#[test]
fn negotiated_atomic_receiver_preserves_wouldblock_and_disconnect() {
    use std::os::fd::AsRawFd;
    use std::os::unix::net::UnixStream;
    let (sender, receiver) = UnixStream::pair().unwrap();
    assert_eq!(
        receive(receiver.as_raw_fd(), &mut [0; 1])
            .unwrap_err()
            .kind(),
        io::ErrorKind::WouldBlock
    );
    drop(sender);
    assert_eq!(
        receive(receiver.as_raw_fd(), &mut [0; 1])
            .unwrap_err()
            .kind(),
        io::ErrorKind::UnexpectedEof
    );
}

#[test]
fn negotiated_atomic_receiver_refuses_unexpected_kernel_credentials() {
    use std::os::fd::AsRawFd;
    use std::os::unix::net::UnixDatagram;
    let (sender, receiver) = UnixDatagram::pair().unwrap();
    let enabled = 1i32;
    assert_eq!(
        unsafe {
            libc::setsockopt(
                receiver.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PASSCRED,
                (&enabled as *const i32).cast(),
                size_of::<i32>() as _,
            )
        },
        0
    );
    assert_eq!(sender.send(b"x").unwrap(), 1);
    assert_eq!(
        receive(receiver.as_raw_fd(), &mut [0; 1])
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidData
    );
}
