//! Reverse only the measured scratch mechanism, retaining every preceding check.
pub(crate) const SOURCE: &str = "tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs";
pub(crate) const HOOK: &str = "\n#[path = \"diagnostic_document_scratch.rs\"]\nmod document_scratch;\nuse document_scratch::{alive_read, DocumentBuffer};\n";
pub(crate) const OLD_ALIVE: &str = "fn alive(pidfd: &OwnedFd, pid: libc::pid_t) -> Result<(), KernelError> {\n    #[cfg(test)]\n    let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(\n        crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Alive,\n    );\n    let mut poll = libc::pollfd {\n        fd: pidfd.as_raw_fd(),\n        events: libc::POLLIN,\n        revents: 0,\n    };\n    let ready = unsafe { libc::poll(&mut poll, 1, 0) };\n    if ready < 0 {\n        return Err(std::io::Error::last_os_error().into());\n    }\n    if ready != 0 || poll.revents != 0 {\n        return Err(KernelError::NotLive);\n    }\n    let info = OpenOptions::new()\n        .read(true)\n        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)\n        .open(format!(\"/proc/thread-self/fdinfo/{}\", pidfd.as_raw_fd()))?;\n    let raw = bounded(&info)?;\n    origin::filesystem(&info, 0x9fa0)?;\n    let text = std::str::from_utf8(&raw).map_err(|_| kernel_invalid!(\"pidfd-info-utf8\"))?;\n    let mut lines = text.lines().filter_map(|line| line.strip_prefix(\"Pid:\"));\n    let observed = kernel_required!(lines.next(), \"pidfd-info-missing-pid\")?\n        .trim()\n        .parse::<i32>()\n        .map_err(|_| kernel_invalid!(\"pidfd-info-pid-number\"))?;\n    if lines.next().is_some() || observed != pid || pid <= 0 {\n        return Err(KernelError::NotLive);\n    }\n    Ok(())\n}";
pub(crate) const WRAPPER: &str = "fn alive(pidfd: &OwnedFd, pid: libc::pid_t) -> Result<(), KernelError> {\n    alive_read(pidfd, pid, bounded)\n}";
pub(crate) const PROTOTYPE: &str =
    "tools/long-run-supervisor-074/tests/support/document_scratch.rs";
fn restore(raw: &mut String, new: &str, old: &str, count: usize) {
    assert_eq!(
        raw.matches(new).count(),
        count,
        "exact document scratch substitution"
    );
    *raw = raw.replace(new, old);
}
pub(crate) fn normalize(path: &str, source: &str) -> String {
    let mut raw = source.to_owned();
    if path == SOURCE && raw.contains(HOOK) {
        restore(&mut raw, HOOK, "", 1);
        restore(
            &mut raw,
            "        let mut scratch = DocumentBuffer::new();\n",
            "",
            1,
        );
        restore(
            &mut raw,
            "alive_read(&self.pidfd, self.credentials.pid, |file| scratch.read(file))?;",
            "alive(&self.pidfd, self.credentials.pid)?;",
            2,
        );
        restore(
            &mut raw,
            "projection(scratch.read(file)?, &self.credentials)?",
            "projection(&bounded(file)?, &self.credentials)?",
            1,
        );
        restore(&mut raw, WRAPPER, OLD_ALIVE, 1);
    }
    if path == PROTOTYPE
        && raw.contains("use crate::diagnostic_kernel_admission::document_scratch::")
    {
        restore(&mut raw,"use crate::diagnostic_kernel_admission::document_scratch::{alive_read, DocumentBuffer};\nuse crate::diagnostic_kernel_admission::*;\n","use crate::diagnostic_kernel_admission::*;\n#[path = \"document_scratch_impl.rs\"]\nmod scratch;\nuse scratch::{alive_read, DocumentBuffer};\n",1);
    }
    raw
}
