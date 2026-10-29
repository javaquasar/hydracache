//! Frozen b0a8030cb operation references; no selected-operation observer.
use super::*;

pub(super) fn reference_filesystem(file: &File, magic: libc::c_long) -> Result<(), KernelError> {
    let mut stat: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstatfs(file.as_raw_fd(), &mut stat) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    if stat.f_type != magic {
        return Err(kernel_invalid!("filesystem-type"));
    }
    Ok(())
}

pub(super) fn reference_fixed_child(
    parent: &File,
    name: &std::ffi::CStr,
    directory: bool,
) -> Result<File, KernelError> {
    let flags = libc::O_RDONLY
        | libc::O_NOFOLLOW
        | libc::O_CLOEXEC
        | libc::O_NONBLOCK
        | if directory { libc::O_DIRECTORY } else { 0 };
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let file = unsafe { File::from_raw_fd(fd) };
    reference_filesystem(&file, 0x9fa0)?;
    Ok(file)
}

pub(super) fn reference_namespace_path(name: &str) -> std::borrow::Cow<'static, str> {
    use std::borrow::Cow;
    match name {
        "user" => Cow::Borrowed("/proc/thread-self/ns/user"),
        "mnt" => Cow::Borrowed("/proc/thread-self/ns/mnt"),
        "pid" => Cow::Borrowed("/proc/thread-self/ns/pid"),
        "time" => Cow::Borrowed("/proc/thread-self/ns/time"),
        _ => Cow::Owned(format!("/proc/thread-self/ns/{name}")),
    }
}

pub(super) fn reference_namespace(name: &str) -> Result<(File, (u64, u64)), KernelError> {
    #[cfg(test)]
    let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
        crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Namespace,
    );
    // These fixed kernel magic links must be followed, then checked as nsfs.
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC)
        .open(reference_namespace_path(name).as_ref())?;
    reference_filesystem(&file, 0x6e736673)?;
    let id = fd_identity::fresh(&file)?;
    Ok((file, id))
}

pub(super) fn reference_boot_read(
    read: impl FnOnce(&mut [u8]) -> std::io::Result<usize>,
) -> Result<([u8; 38], usize), KernelError> {
    #[cfg(test)]
    let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
        crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Document,
    );
    Ok(boot_document::read_once(read)?)
}

pub(super) fn reference_boot_view(raw: &([u8; 38], usize)) -> Result<&str, KernelError> {
    let (raw, count) = raw;
    let count = *count;
    if count == 0 {
        return Err(kernel_invalid!("proc-document-budget"));
    }
    if count != 37
        || raw[36] != b'\n'
        || raw[..36].iter().enumerate().any(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                *c != b'-'
            } else {
                !c.is_ascii_digit() && !(b'a'..=b'f').contains(c)
            }
        })
    {
        return Err(kernel_invalid!("boot-document"));
    }
    std::str::from_utf8(&raw[..36]).map_err(|_| kernel_invalid!("boot-utf8"))
}

pub(super) fn reference_boot_apply<T>(
    root: &File,
    decode: impl FnOnce(([u8; 38], usize)) -> Result<T, KernelError>,
) -> Result<T, KernelError> {
    #[cfg(test)]
    let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
        crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Boot,
    );
    let sys = reference_fixed_child(root, c"sys", true)?;
    let kernel = reference_fixed_child(&sys, c"kernel", true)?;
    let random = reference_fixed_child(&kernel, c"random", true)?;
    let file = reference_fixed_child(&random, c"boot_id", false)?;
    let raw = reference_boot_read(|bytes| file.read_at(bytes, 0))?;
    // As before, close the proc document before decoding the owned bytes.
    drop(file);
    decode(raw)
}

pub(super) fn boot_matches(root: &File, expected: &str) -> Result<bool, KernelError> {
    reference_boot_apply(root, |raw| {
        reference_boot_view(&raw).map(|value| value == expected)
    })
}
