//! One exact predecessor implementation shared by source views without duplicate modules.
#[path = "remaining_preparation_guard.rs"]
mod remaining;
#[macro_export]
macro_rules! hc074_document_scratch_view {
    (@items) => {
        pub(crate) const SOURCE: &str = "tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs";
        pub(crate) const HOOK: &str = "use origin::{alive_read, DocumentBuffer, Origin, TimePoint};";
        pub(crate) const ORIGIN: &str = "tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs";
        pub(crate) const BLOCK: &str = "// BEGIN INVOCATION DOCUMENT SCRATCH\npub(super) struct DocumentBuffer {\n    bytes: Vec<u8>,\n}\nimpl DocumentBuffer {\n    pub(super) fn new() -> Self {\n        Self {\n            bytes: vec![0; DOC_BYTES + 1],\n        }\n    }\n    pub(super) fn read<'a>(&'a mut self, file: &File) -> Result<&'a [u8], KernelError> {\n        self.read_with(|bytes| file.read_at(bytes, 0))\n    }\n    fn read_with(\n        &mut self,\n        read: impl FnOnce(&mut [u8]) -> std::io::Result<usize>,\n    ) -> Result<&[u8], KernelError> {\n        #[cfg(test)]\n        let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(\n            crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Document,\n        );\n        let count = read(&mut self.bytes)?;\n        if count == 0 || count > DOC_BYTES {\n            return Err(kernel_invalid!(\"proc-document-budget\"));\n        }\n        Ok(&self.bytes[..count])\n    }\n}\npub(super) fn alive_read<T: AsRef<[u8]>>(\n    pidfd: &OwnedFd,\n    pid: libc::pid_t,\n    read: impl FnOnce(&File) -> Result<T, KernelError>,\n) -> Result<(), KernelError> {\n    #[cfg(test)]\n    let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(\n        crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Alive,\n    );\n    let mut poll = libc::pollfd {\n        fd: pidfd.as_raw_fd(),\n        events: libc::POLLIN,\n        revents: 0,\n    };\n    let ready = unsafe { libc::poll(&mut poll, 1, 0) };\n    if ready < 0 {\n        return Err(std::io::Error::last_os_error().into());\n    }\n    if ready != 0 || poll.revents != 0 {\n        return Err(KernelError::NotLive);\n    }\n    let info = OpenOptions::new()\n        .read(true)\n        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)\n        .open(format!(\"/proc/thread-self/fdinfo/{}\", pidfd.as_raw_fd()))?;\n    let raw = read(&info)?;\n    origin::filesystem(&info, 0x9fa0)?;\n    let text = std::str::from_utf8(raw.as_ref()).map_err(|_| kernel_invalid!(\"pidfd-info-utf8\"))?;\n    let mut lines = text.lines().filter_map(|line| line.strip_prefix(\"Pid:\"));\n    let observed = kernel_required!(lines.next(), \"pidfd-info-missing-pid\")?\n        .trim()\n        .parse::<i32>()\n        .map_err(|_| kernel_invalid!(\"pidfd-info-pid-number\"))?;\n    if lines.next().is_some() || observed != pid || pid <= 0 {\n        return Err(KernelError::NotLive);\n    }\n    Ok(())\n}\n#[cfg(test)]\nmod document_scratch_tests {\n    use super::*;\n    #[test]\n    fn document_scratch_single_read_seeded_short_poison_error_and_budget() {\n        let mut scratch = DocumentBuffer::new();\n        let mut seed = 0x074d_5a17_u64;\n        for sample in 0..128 {\n            seed ^= seed << 13;\n            seed ^= seed >> 7;\n            seed ^= seed << 17;\n            let len = [0, 1, 2, DOC_BYTES - 1, DOC_BYTES, DOC_BYTES + 1][sample % 6];\n            let value = seed as u8;\n            scratch.bytes.fill(!value);\n            let calls = std::cell::Cell::new(0);\n            let result = scratch.read_with(|buffer| {\n                calls.set(calls.get() + 1);\n                assert_eq!(buffer.len(), DOC_BYTES + 1);\n                buffer[..len].fill(value);\n                Ok(len)\n            });\n            assert_eq!(calls.get(), 1);\n            if len == 0 || len > DOC_BYTES {\n                assert!(result.is_err());\n            } else {\n                let bytes = result.unwrap();\n                assert_eq!(bytes.len(), len);\n                assert!(bytes.iter().all(|b| *b == value));\n            }\n        }\n        for code in [libc::EINTR, libc::EIO, libc::EBADF] {\n            let calls = std::cell::Cell::new(0);\n            let result = scratch.read_with(|_| {\n                calls.set(calls.get() + 1);\n                Err(std::io::Error::from_raw_os_error(code))\n            });\n            assert_eq!(calls.get(), 1);\n            assert!(matches!(result,Err(KernelError::Io(e)) if e.raw_os_error()==Some(code)));\n        }\n    }\n}\n// END INVOCATION DOCUMENT SCRATCH\n\n";
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
            let mut raw = $crate::hc074_remaining_preparation_view!(path, source);
            if path == SOURCE && raw.contains(HOOK) {
                restore(&mut raw, HOOK, "use origin::{Origin, TimePoint};", 1);
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
            if path == ORIGIN && raw.contains("// BEGIN INVOCATION DOCUMENT SCRATCH") {
                restore(&mut raw, BLOCK, "", 1);
            }
            if path == PROTOTYPE && raw.contains("use crate::diagnostic_kernel_admission::origin::") {
                restore(&mut raw,"use crate::diagnostic_kernel_admission::origin::{alive_read, DocumentBuffer};\nuse crate::diagnostic_kernel_admission::*;\n","use crate::diagnostic_kernel_admission::*;\n#[path = \"document_scratch_impl.rs\"]\nmod scratch;\nuse scratch::{alive_read, DocumentBuffer};\n",1);
            }
            raw
        }
    };
    ($path:expr, $source:expr) => {{
        $crate::hc074_document_scratch_view!(@items);
        normalize($path, $source)
    }};
}
hc074_document_scratch_view!(@items);
