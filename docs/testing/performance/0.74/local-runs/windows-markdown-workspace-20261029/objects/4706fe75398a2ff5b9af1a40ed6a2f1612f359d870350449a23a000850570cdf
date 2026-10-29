//! One invocation's temporary bytes; never a retained observation or FD.
use super::*;
pub(super) struct DocumentBuffer {
    bytes: Vec<u8>,
}
impl DocumentBuffer {
    pub(super) fn new() -> Self {
        Self {
            bytes: vec![0; DOC_BYTES + 1],
        }
    }
    pub(super) fn read<'a>(&'a mut self, file: &File) -> Result<&'a [u8], KernelError> {
        self.read_with(|bytes| file.read_at(bytes, 0))
    }
    fn read_with(
        &mut self,
        read: impl FnOnce(&mut [u8]) -> std::io::Result<usize>,
    ) -> Result<&[u8], KernelError> {
        #[cfg(test)]
        let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
            crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Document,
        );
        let count = read(&mut self.bytes)?;
        if count == 0 || count > DOC_BYTES {
            return Err(kernel_invalid!("proc-document-budget"));
        }
        Ok(&self.bytes[..count])
    }
}
pub(super) fn alive_read<T: AsRef<[u8]>>(
    pidfd: &OwnedFd,
    pid: libc::pid_t,
    read: impl FnOnce(&File) -> Result<T, KernelError>,
) -> Result<(), KernelError> {
    #[cfg(test)]
    let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
        crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Alive,
    );
    let mut poll = libc::pollfd {
        fd: pidfd.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    let ready = unsafe { libc::poll(&mut poll, 1, 0) };
    if ready < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    if ready != 0 || poll.revents != 0 {
        return Err(KernelError::NotLive);
    }
    let info = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(format!("/proc/thread-self/fdinfo/{}", pidfd.as_raw_fd()))?;
    let raw = read(&info)?;
    origin::filesystem(&info, 0x9fa0)?;
    let text = std::str::from_utf8(raw.as_ref()).map_err(|_| kernel_invalid!("pidfd-info-utf8"))?;
    let mut lines = text.lines().filter_map(|line| line.strip_prefix("Pid:"));
    let observed = kernel_required!(lines.next(), "pidfd-info-missing-pid")?
        .trim()
        .parse::<i32>()
        .map_err(|_| kernel_invalid!("pidfd-info-pid-number"))?;
    if lines.next().is_some() || observed != pid || pid <= 0 {
        return Err(KernelError::NotLive);
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn document_scratch_single_read_seeded_short_poison_error_and_budget() {
        let mut scratch = DocumentBuffer::new();
        let mut seed = 0x074d_5a17_u64;
        for sample in 0..128 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let len = [0, 1, 2, DOC_BYTES - 1, DOC_BYTES, DOC_BYTES + 1][sample % 6];
            let value = seed as u8;
            scratch.bytes.fill(!value);
            let calls = std::cell::Cell::new(0);
            let result = scratch.read_with(|buffer| {
                calls.set(calls.get() + 1);
                assert_eq!(buffer.len(), DOC_BYTES + 1);
                buffer[..len].fill(value);
                Ok(len)
            });
            assert_eq!(calls.get(), 1);
            if len == 0 || len > DOC_BYTES {
                assert!(result.is_err());
            } else {
                let bytes = result.unwrap();
                assert_eq!(bytes.len(), len);
                assert!(bytes.iter().all(|b| *b == value));
            }
        }
        for code in [libc::EINTR, libc::EIO, libc::EBADF] {
            let calls = std::cell::Cell::new(0);
            let result = scratch.read_with(|_| {
                calls.set(calls.get() + 1);
                Err(std::io::Error::from_raw_os_error(code))
            });
            assert_eq!(calls.get(), 1);
            assert!(matches!(result,Err(KernelError::Io(e)) if e.raw_os_error()==Some(code)));
        }
    }
}
