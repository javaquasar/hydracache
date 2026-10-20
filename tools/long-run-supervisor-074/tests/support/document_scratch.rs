//! Frozen old validator and one buffer-only prototype, never an admission route.
use crate::diagnostic_kernel_admission::*;
#[path = "document_scratch_impl.rs"]
mod scratch;
use scratch::{alive_read, DocumentBuffer};
impl Sender {
    pub(in crate::diagnostic_kernel_admission) fn scratch_reference(
        &self,
        origin: &Origin,
    ) -> Result<(), KernelError> {
        #[cfg(test)]
        let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
            crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Sender,
        );
        origin.revalidate()?;
        alive(&self.pidfd, self.credentials.pid)?;
        let directory = origin.child(self.credentials.pid, true)?;
        for file in [&directory, &self.directory] {
            let m = file.metadata()?;
            if (m.dev(), m.ino()) != self.directory_id {
                return Err(kernel_invalid!("sender-directory"));
            }
        }
        let named = origin::fixed_child(&self.directory, c"status", false)?;
        for file in [&named, &self.status] {
            let m = file.metadata()?;
            if (m.dev(), m.ino()) != self.status_id
                || projection(&bounded(file)?, &self.credentials)? != self.projection
            {
                return Err(kernel_invalid!("sender-status-binding"));
            }
        }
        alive(&self.pidfd, self.credentials.pid)?;
        origin.revalidate()
    }
    pub(in crate::diagnostic_kernel_admission) fn scratch_prototype(
        &self,
        origin: &Origin,
    ) -> Result<(), KernelError> {
        #[cfg(test)]
        let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
            crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Sender,
        );
        origin.revalidate()?;
        let mut scratch = DocumentBuffer::new();
        alive_read(&self.pidfd, self.credentials.pid, |file| scratch.read(file))?;
        let directory = origin.child(self.credentials.pid, true)?;
        for file in [&directory, &self.directory] {
            let m = file.metadata()?;
            if (m.dev(), m.ino()) != self.directory_id {
                return Err(kernel_invalid!("sender-directory"));
            }
        }
        let named = origin::fixed_child(&self.directory, c"status", false)?;
        for file in [&named, &self.status] {
            let m = file.metadata()?;
            if (m.dev(), m.ino()) != self.status_id
                || projection(scratch.read(file)?, &self.credentials)? != self.projection
            {
                return Err(kernel_invalid!("sender-status-binding"));
            }
        }
        alive_read(&self.pidfd, self.credentials.pid, |file| scratch.read(file))?;
        origin.revalidate()
    }
}
pub(in crate::diagnostic_kernel_admission) fn current() -> (Origin, Sender) {
    use std::os::fd::FromRawFd;
    let origin = Origin::open().unwrap();
    let credentials = libc::ucred {
        pid: unsafe { libc::getpid() },
        uid: unsafe { libc::getuid() },
        gid: unsafe { libc::getgid() },
    };
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, credentials.pid, 0) };
    assert!(fd >= 0);
    let pidfd = unsafe { OwnedFd::from_raw_fd(fd as i32) };
    let sender = Sender::pin(&origin, credentials, pidfd).unwrap();
    (origin, sender)
}
fn outcome<T>(run: impl FnOnce() -> Result<T, KernelError>) -> (String, Option<&'static str>) {
    let (value, event) = refusal_trace::capture(run);
    let value = match value {
        Ok(_) => "ok".into(),
        Err(KernelError::Io(e)) => format!("io:{:?}", e.raw_os_error()),
        Err(e) => e.to_string(),
    };
    (value, event.map(|e| e.site))
}
#[test]
fn document_scratch_sender_preserves_fresh_counts_and_original_refusals() {
    use crate::diagnostic_ipc::validation_trace::kernel_walk;
    let (origin, sender) = current();
    let (a, old) = kernel_walk::capture(|| sender.scratch_reference(&origin));
    let (b, new) = kernel_walk::capture(|| sender.scratch_prototype(&origin));
    a.unwrap();
    b.unwrap();
    assert_eq!(old.nodes.map(|s| s.calls), [0, 1, 2, 2, 2, 4, 8, 0, 0, 8]);
    assert_eq!(old.nodes.map(|s| s.calls), new.nodes.map(|s| s.calls));
    for case in 0..6 {
        let (origin, mut sender) = current();
        match case {
            0 => sender.credentials.pid += 1,
            1 => sender.credentials.uid ^= 1,
            2 => sender.directory_id.0 ^= 1,
            3 => sender.status_id.1 ^= 1,
            4 => sender.projection.groups.push(u32::MAX),
            _ => sender.pidfd = tempfile::tempfile().unwrap().into(),
        }
        let old = outcome(|| sender.scratch_reference(&origin));
        let new = outcome(|| sender.scratch_prototype(&origin));
        assert_ne!(old.0, "ok");
        assert_eq!(old, new, "case={case}");
    }
}
#[test]
fn document_scratch_actual_documents_exclude_previous_bytes_and_preserve_errors() {
    let file = tempfile::tempfile().unwrap();
    let mut scratch = DocumentBuffer::new();
    for len in [DOC_BYTES, 1, 17, 0, DOC_BYTES + 1, 2] {
        file.set_len(0).unwrap();
        file.write_at(&vec![len as u8; len], 0).unwrap();
        let old = bounded(&file);
        let new = scratch.read(&file);
        match (old, new) {
            (Ok(a), Ok(b)) => assert_eq!(a, b),
            (Err(a), Err(b)) => assert_eq!(a.to_string(), b.to_string()),
            _ => panic!("read differential"),
        }
    }
    let directory = File::open("/tmp").unwrap();
    assert_eq!(
        outcome(|| bounded(&directory)),
        outcome(|| scratch.read(&directory))
    );
}
#[test]
fn document_scratch_runtime_binding_requires_one_call_owned_buffer() {
    let source = include_str!("../../src/diagnostic_kernel_admission.rs");
    assert!(
        source.contains("let mut scratch = DocumentBuffer::new();"),
        "test-first unimplemented runtime"
    );
    assert_eq!(
        source
            .matches("alive_read(&self.pidfd, self.credentials.pid, |file| scratch.read(file))?;")
            .count(),
        2
    );
    assert_eq!(
        source
            .matches("projection(scratch.read(file)?, &self.credentials)?")
            .count(),
        1
    );
}
