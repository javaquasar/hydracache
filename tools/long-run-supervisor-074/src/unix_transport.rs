use crate::protocol::MAX_PACKET_BYTES;
use std::fs;
use std::io;
use std::mem::{offset_of, size_of, zeroed};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use thiserror::Error;

const LISTEN_BACKLOG: libc::c_int = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerCredentials {
    pub pid: u32,
    pub uid: u32,
    pub gid: u32,
    pub supplemental_gids: Vec<u32>,
}

impl PeerCredentials {
    pub fn belongs_to_group(&self, gid: u32) -> bool {
        self.gid == gid || self.supplemental_gids.contains(&gid)
    }
}

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("Unix socket path is invalid, too long, or already exists")]
    Path,
    #[error("packet is empty or exceeds the frozen 65536-byte limit")]
    PacketSize,
    #[error("peer credentials or supplemental groups are unavailable")]
    PeerCredentials,
    #[error("Unix socket I/O failed: {0}")]
    Io(#[from] io::Error),
}

#[derive(Debug)]
pub struct SeqpacketListener {
    fd: OwnedFd,
    path: PathBuf,
    device: u64,
    inode: u64,
}

impl SeqpacketListener {
    pub fn bind(path: &Path, mode: u32, group_gid: u32) -> Result<Self, TransportError> {
        if mode & !0o777 != 0 || path.exists() {
            return Err(TransportError::Path);
        }
        let address = socket_address(path)?;
        let fd = new_socket()?;
        // SAFETY: address points to an initialized sockaddr_un with the returned byte length.
        if unsafe {
            libc::bind(
                fd.as_raw_fd(),
                (&address.value as *const libc::sockaddr_un).cast(),
                address.length,
            )
        } != 0
        {
            return Err(io::Error::last_os_error().into());
        }
        // SAFETY: path is nul-terminated for the duration of chown; uid -1 preserves the owner.
        let path_bytes = std::ffi::CString::new(path.as_os_str().as_bytes())
            .map_err(|_| TransportError::Path)?;
        if unsafe { libc::chown(path_bytes.as_ptr(), u32::MAX, group_gid) } != 0 {
            let error = io::Error::last_os_error();
            let _ = fs::remove_file(path);
            return Err(error.into());
        }
        fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
        // SAFETY: fd is a valid bound SOCK_SEQPACKET socket.
        if unsafe { libc::listen(fd.as_raw_fd(), LISTEN_BACKLOG) } != 0 {
            let error = io::Error::last_os_error();
            let _ = fs::remove_file(path);
            return Err(error.into());
        }
        let metadata = fs::symlink_metadata(path)?;
        if !metadata.file_type().is_socket() {
            return Err(TransportError::Path);
        }
        Ok(Self {
            fd,
            path: path.to_owned(),
            device: metadata.dev(),
            inode: metadata.ino(),
        })
    }

    pub fn accept(&self) -> Result<SeqpacketConnection, TransportError> {
        // SAFETY: fd is a listening socket; null address arguments intentionally discard it.
        let accepted = unsafe {
            libc::accept4(
                self.fd.as_raw_fd(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                libc::SOCK_CLOEXEC,
            )
        };
        if accepted < 0 {
            return Err(io::Error::last_os_error().into());
        }
        // SAFETY: accept4 returned a new owned descriptor.
        let fd = unsafe { OwnedFd::from_raw_fd(accepted) };
        Ok(SeqpacketConnection { fd })
    }
}

impl Drop for SeqpacketListener {
    fn drop(&mut self) {
        let Ok(metadata) = fs::symlink_metadata(&self.path) else {
            return;
        };
        if metadata.file_type().is_socket()
            && metadata.dev() == self.device
            && metadata.ino() == self.inode
        {
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[derive(Debug)]
pub struct SeqpacketConnection {
    fd: OwnedFd,
}

impl SeqpacketConnection {
    pub fn connect(path: &Path) -> Result<Self, TransportError> {
        let address = socket_address(path)?;
        let fd = new_socket()?;
        // SAFETY: address points to an initialized sockaddr_un with the returned byte length.
        if unsafe {
            libc::connect(
                fd.as_raw_fd(),
                (&address.value as *const libc::sockaddr_un).cast(),
                address.length,
            )
        } != 0
        {
            return Err(io::Error::last_os_error().into());
        }
        Ok(Self { fd })
    }

    pub fn send_packet(&self, packet: &[u8]) -> Result<(), TransportError> {
        if packet.is_empty() || packet.len() > MAX_PACKET_BYTES {
            return Err(TransportError::PacketSize);
        }
        // SAFETY: packet remains valid for the blocking send call and fd is connected.
        let sent = unsafe {
            libc::send(
                self.fd.as_raw_fd(),
                packet.as_ptr().cast(),
                packet.len(),
                libc::MSG_NOSIGNAL,
            )
        };
        if sent < 0 {
            return Err(io::Error::last_os_error().into());
        }
        if sent as usize != packet.len() {
            return Err(io::Error::new(io::ErrorKind::WriteZero, "partial seqpacket send").into());
        }
        Ok(())
    }

    pub fn receive_packet(&self) -> Result<Vec<u8>, TransportError> {
        let mut packet = vec![0_u8; MAX_PACKET_BYTES + 1];
        // MSG_TRUNC makes Linux return the original packet length when the buffer is too small.
        // SAFETY: packet is writable for its full capacity and fd is connected.
        let received = unsafe {
            libc::recv(
                self.fd.as_raw_fd(),
                packet.as_mut_ptr().cast(),
                packet.len(),
                libc::MSG_TRUNC,
            )
        };
        if received < 0 {
            return Err(io::Error::last_os_error().into());
        }
        if received == 0 || received as usize > MAX_PACKET_BYTES {
            return Err(TransportError::PacketSize);
        }
        packet.truncate(received as usize);
        Ok(packet)
    }

    pub fn peer_credentials(&self) -> Result<PeerCredentials, TransportError> {
        // SAFETY: ucred and its length are initialized and writable for getsockopt.
        let mut credentials: libc::ucred = unsafe { zeroed() };
        let mut length = size_of::<libc::ucred>() as libc::socklen_t;
        if unsafe {
            libc::getsockopt(
                self.fd.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                (&mut credentials as *mut libc::ucred).cast(),
                &mut length,
            )
        } != 0
            || length as usize != size_of::<libc::ucred>()
            || credentials.pid <= 0
        {
            return Err(TransportError::PeerCredentials);
        }
        let pid = credentials.pid as u32;
        Ok(PeerCredentials {
            pid,
            uid: credentials.uid,
            gid: credentials.gid,
            supplemental_gids: read_supplemental_groups(pid)?,
        })
    }
}

struct SocketAddress {
    value: libc::sockaddr_un,
    length: libc::socklen_t,
}

fn socket_address(path: &Path) -> Result<SocketAddress, TransportError> {
    let bytes = path.as_os_str().as_bytes();
    // Linux pathname sockets need a trailing nul and do not permit embedded nuls.
    if bytes.is_empty()
        || bytes.contains(&0)
        || bytes.len() >= unsafe { zeroed::<libc::sockaddr_un>() }.sun_path.len()
    {
        return Err(TransportError::Path);
    }
    // SAFETY: zeroed sockaddr_un is a valid starting representation.
    let mut value: libc::sockaddr_un = unsafe { zeroed() };
    value.sun_family = libc::AF_UNIX as libc::sa_family_t;
    for (target, source) in value.sun_path.iter_mut().zip(bytes) {
        *target = *source as libc::c_char;
    }
    let length = offset_of!(libc::sockaddr_un, sun_path) + bytes.len() + 1;
    Ok(SocketAddress {
        value,
        length: length.try_into().map_err(|_| TransportError::Path)?,
    })
}

fn new_socket() -> Result<OwnedFd, TransportError> {
    // SAFETY: socket has no pointer arguments and returns a new descriptor on success.
    let fd = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC, 0) };
    if fd < 0 {
        return Err(io::Error::last_os_error().into());
    }
    // SAFETY: socket returned a new owned descriptor.
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

fn read_supplemental_groups(pid: u32) -> Result<Vec<u32>, TransportError> {
    let status = fs::read_to_string(format!("/proc/{pid}/status"))?;
    let groups = status
        .lines()
        .find_map(|line| line.strip_prefix("Groups:"))
        .ok_or(TransportError::PeerCredentials)?;
    let mut result = groups
        .split_ascii_whitespace()
        .map(|value| {
            value
                .parse::<u32>()
                .map_err(|_| TransportError::PeerCredentials)
        })
        .collect::<Result<Vec<_>, _>>()?;
    result.sort_unstable();
    result.dedup();
    Ok(result)
}
