//! Test-only fresh fstatfs projection; no cache and no changed syscall contract.
use super::*;
use std::mem::MaybeUninit;

pub(super) fn check(file: &File, magic: libc::c_long) -> Result<(), KernelError> {
    let _operation_fs = operation_cost::span(operation_cost::Kind::Filesystem);
    // SAFETY: libc writes f_type on success into the full aligned writable
    // statfs buffer. query_once reads only that field, never the whole object.
    unsafe { query_once(file.as_raw_fd(), magic, |fd, out| libc::fstatfs(fd, out)) }
}

/// # Safety
/// A zero result from query must initialize out.f_type. Other fields need not be
/// initialized. A nonzero result does not permit any read from the output.
pub(super) unsafe fn query_once(
    fd: std::os::fd::RawFd,
    magic: libc::c_long,
    query: impl FnOnce(std::os::fd::RawFd, *mut libc::statfs) -> libc::c_int,
) -> Result<(), KernelError> {
    let mut output = MaybeUninit::<libc::statfs>::uninit();
    if query(fd, output.as_mut_ptr()) != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY: the query contract initializes precisely this field on success;
    // addr_of! does not construct a reference to an uninitialized statfs.
    let actual = unsafe { std::ptr::addr_of!((*output.as_ptr()).f_type).read() };
    if actual != magic {
        return Err(kernel_invalid!("filesystem-type"));
    }
    Ok(())
}
