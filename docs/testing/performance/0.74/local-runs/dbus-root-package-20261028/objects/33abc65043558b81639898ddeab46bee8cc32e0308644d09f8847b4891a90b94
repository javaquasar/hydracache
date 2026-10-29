//! One invocation's pathname bytes, never an observation or descriptor cache.
use super::PidName;
use std::os::unix::ffi::OsStrExt;

const PREFIX: &[u8] = b"/proc/thread-self/fdinfo/";
pub(super) struct InfoPath {
    bytes: [u8; PREFIX.len() + 11],
    length: usize,
}
impl InfoPath {
    pub(super) fn new(fd: i32) -> Self {
        let digits = PidName::new(fd);
        let digits = digits.as_c_str().to_bytes();
        let length = PREFIX.len() + digits.len();
        let mut bytes = [0; PREFIX.len() + 11];
        bytes[..PREFIX.len()].copy_from_slice(PREFIX);
        bytes[PREFIX.len()..length].copy_from_slice(digits);
        Self { bytes, length }
    }
    pub(super) fn as_path(&self) -> &std::path::Path {
        std::path::Path::new(std::ffi::OsStr::from_bytes(&self.bytes[..self.length]))
    }
}

#[cfg(test)]
#[path = "../tests/support/pidfd_info_path.rs"]
mod tests;
