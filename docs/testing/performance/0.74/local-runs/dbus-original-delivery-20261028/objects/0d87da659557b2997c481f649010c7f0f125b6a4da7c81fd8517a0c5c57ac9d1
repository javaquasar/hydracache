//! Fixed-literal open experiment. The original path projection/fallback remains.
use super::*;
use std::ffi::CStr;
use std::io;

pub(super) fn retry_interrupted<T>(mut call: impl FnMut() -> io::Result<T>) -> io::Result<T> {
    loop {
        let outcome = call();
        if outcome.as_ref().err().map(io::Error::kind) != Some(io::ErrorKind::Interrupted) {
            return outcome;
        }
    }
}
pub(super) fn open_literal(path: &CStr) -> io::Result<File> {
    let fd = retry_interrupted(|| {
        // SAFETY: borrowed terminated path, read-only open with the same GNU
        // libc entry point, atomic CLOEXEC and unused mode as the pinned std.
        let fd = unsafe {
            libc::open64(
                path.as_ptr(),
                libc::O_RDONLY | libc::O_CLOEXEC,
                0o666 as libc::c_int,
            )
        };
        if fd == -1 {
            Err(io::Error::last_os_error())
        } else {
            Ok(fd)
        }
    })?;
    // SAFETY: one successful open returned one new owned descriptor.
    Ok(unsafe { File::from_raw_fd(fd) })
}
pub(super) fn read(name: &str) -> io::Result<File> {
    // Keep this original projection once, including its diagnostic Path hook.
    // The fallback retains OpenOptions' input/error handling byte-for-byte.
    let original = namespace_path(name);
    match name {
        "user" => open_literal(c"/proc/thread-self/ns/user"),
        "mnt" => open_literal(c"/proc/thread-self/ns/mnt"),
        "pid" => open_literal(c"/proc/thread-self/ns/pid"),
        "time" => open_literal(c"/proc/thread-self/ns/time"),
        _ => OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_CLOEXEC)
            .open(original.as_ref()),
    }
}
