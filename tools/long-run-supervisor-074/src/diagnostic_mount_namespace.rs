//! Original worker leader/reading-thread mount consistency, not signed enrollment.
use super::{child, require_procfs, FileId, ProcessError, ProcessRead};
use std::ffi::CStr;
use std::fs::File;
use std::marker::PhantomData;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::MetadataExt;
use std::rc::Rc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MountNamespaceError {
    #[error("descriptor is not a mount namespace in nsfs")]
    Invalid,
    #[error("original leader mount namespace, directory or observer thread drifted")]
    Drift,
    #[error("original mount namespace reader previously refused")]
    Refused,
    #[error(transparent)]
    Process(#[from] ProcessError),
    #[error("read-only mount namespace observation failed: {0}")]
    Io(#[from] std::io::Error),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Generation,
    Namespace,
}
struct Gate {
    original: FileId,
    refused: bool,
}
impl Gate {
    fn new(original: FileId) -> Self {
        Self {
            original,
            refused: false,
        }
    }
    fn observe(
        &mut self,
        original_thread: bool,
        mut read: impl FnMut(Step) -> Result<FileId, MountNamespaceError>,
    ) -> Result<(), MountNamespaceError> {
        if self.refused {
            return Err(MountNamespaceError::Refused);
        }
        let result = (|| {
            if !original_thread {
                return Err(MountNamespaceError::Drift);
            }
            for step in [
                Step::Generation,
                Step::Namespace,
                Step::Generation,
                Step::Namespace,
                Step::Generation,
            ] {
                let actual = read(step)?;
                if step == Step::Namespace && actual != self.original {
                    return Err(MountNamespaceError::Drift);
                }
            }
            Ok(())
        })();
        if result.is_err() {
            self.refused = true;
        }
        result
    }
}
struct OriginalProbe<'a> {
    process: &'a ProcessRead,
    directory: File,
    directory_id: FileId,
    worker: File,
    observer: File,
    observer_tid: libc::pid_t,
    original: FileId,
}
impl OriginalProbe<'_> {
    fn inspect(&self) -> Result<FileId, MountNamespaceError> {
        if current_tid()? != self.observer_tid {
            return Err(MountNamespaceError::Drift);
        }
        let root = &self.process.probe.files.root;
        if namespace_id(&current_reader(root)?)? != self.original
            || namespace_id(&self.worker)? != self.original
            || namespace_id(&self.observer)? != self.original
        {
            return Err(MountNamespaceError::Drift);
        }
        let named = child(&self.process.probe.files.directory, "ns", true)?;
        require_procfs(&named)?;
        require_procfs(&self.directory)?;
        let named_meta = named.metadata()?;
        let retained_meta = self.directory.metadata()?;
        let process_device = self.process.probe.files.directory.metadata()?.dev();
        if named_meta.dev() != process_device
            || retained_meta.dev() != process_device
            || FileId::of(&named_meta) != self.directory_id
            || FileId::of(&retained_meta) != self.directory_id
        {
            return Err(MountNamespaceError::Drift);
        }
        let mount = follow_fixed(&named, c"mnt", false)?;
        if namespace_id(&mount)? != self.original
            || namespace_id(&current_reader(root)?)? != self.original
            || current_tid()? != self.observer_tid
        {
            return Err(MountNamespaceError::Drift);
        }
        Ok(self.original)
    }
}
/// Original leader and original reader only; no signed, initial-host or all-thread authority.
pub struct SameMountNamespaceRead<'a> {
    probe: OriginalProbe<'a>,
    guard: Gate,
    _original_thread: PhantomData<Rc<()>>,
}
/// No PID/path/namespace selector or replacement process is accepted.
pub fn pin_same_mount_namespace(
    process: &ProcessRead,
) -> Result<SameMountNamespaceRead<'_>, MountNamespaceError> {
    process.revalidate()?;
    let observer_tid = current_tid()?;
    let directory = child(&process.probe.files.directory, "ns", true)?;
    require_procfs(&directory)?;
    let metadata = directory.metadata()?;
    if metadata.dev() != process.probe.files.directory.metadata()?.dev() {
        return Err(MountNamespaceError::Drift);
    }
    let worker = follow_fixed(&directory, c"mnt", false)?;
    let observer = current_reader(&process.probe.files.root)?;
    let original = namespace_id(&observer)?;
    if namespace_id(&worker)? != original {
        return Err(MountNamespaceError::Drift);
    }
    let mut read = SameMountNamespaceRead {
        probe: OriginalProbe {
            process,
            directory,
            directory_id: FileId::of(&metadata),
            worker,
            observer,
            observer_tid,
            original,
        },
        guard: Gate::new(original),
        _original_thread: PhantomData,
    };
    read.revalidate()?;
    Ok(read)
}
impl SameMountNamespaceRead<'_> {
    pub fn is_refused(&self) -> bool {
        self.guard.refused
    }
    /// Sequential generation brackets; namespace content and all tasks are not pinned.
    pub fn revalidate(&mut self) -> Result<(), MountNamespaceError> {
        if self.guard.refused {
            return Err(MountNamespaceError::Refused);
        }
        let original_thread = match current_tid() {
            Ok(tid) => tid == self.probe.observer_tid,
            Err(error) => {
                self.guard.refused = true;
                return Err(error);
            }
        };
        self.guard.observe(original_thread, |step| match step {
            Step::Generation => {
                self.probe.process.revalidate()?;
                Ok(self.probe.original)
            }
            Step::Namespace => self.probe.inspect(),
        })
    }
}
fn current_tid() -> Result<libc::pid_t, MountNamespaceError> {
    // SAFETY: scalar read-only kernel query with no pointer arguments.
    let tid = unsafe { libc::syscall(libc::SYS_gettid) };
    if tid <= 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(tid as libc::pid_t)
}
fn namespace_id(file: &File) -> Result<FileId, MountNamespaceError> {
    // SAFETY: initialized statfs output and live descriptor.
    let mut stat: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstatfs(file.as_raw_fd(), &mut stat) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    if stat.f_type as u64 != libc::NSFS_MAGIC as u64 {
        return Err(MountNamespaceError::Invalid);
    }
    // SAFETY: read-only NS_GET_NSTYPE accepts no pointer argument.
    let kind = unsafe { libc::ioctl(file.as_raw_fd(), libc::NS_GET_NSTYPE) };
    if kind < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    if kind != libc::CLONE_NEWNS {
        return Err(MountNamespaceError::Invalid);
    }
    Ok(FileId::of(&file.metadata()?))
}
fn current_reader(root: &File) -> Result<File, MountNamespaceError> {
    require_procfs(root)?;
    let thread = follow_fixed(root, c"thread-self", true)?;
    require_procfs(&thread)?;
    let device = root.metadata()?.dev();
    if thread.metadata()?.dev() != device {
        return Err(MountNamespaceError::Invalid);
    }
    let ns = child(&thread, "ns", true)?;
    require_procfs(&ns)?;
    if ns.metadata()?.dev() != device {
        return Err(MountNamespaceError::Invalid);
    }
    follow_fixed(&ns, c"mnt", false)
}
fn follow_fixed(directory: &File, name: &CStr, is_dir: bool) -> Result<File, MountNamespaceError> {
    // Only literal proc magic links passed by the private callers are followed.
    let flags = libc::O_RDONLY | libc::O_CLOEXEC | if is_dir { libc::O_DIRECTORY } else { 0 };
    // SAFETY: retained directory, fixed NUL-terminated name and read-only flags.
    let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY: successful openat returns one new owned descriptor.
    Ok(unsafe { File::from_raw_fd(fd) })
}
#[cfg(test)]
#[path = "diagnostic_mount_namespace_tests.rs"]
mod tests;
