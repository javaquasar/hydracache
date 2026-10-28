//! Original process and current reader namespace consistency, not host enrollment.

use super::{child, require_procfs, FileId, ProcessError, ProcessRead};
use std::ffi::CStr;
use std::fs::File;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::MetadataExt;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NamespaceError {
    #[error("descriptor is not a user namespace in nsfs")]
    Invalid,
    #[error("original process or reading thread user namespace drifted")]
    Drift,
    #[error("original user namespace observation previously refused")]
    Refused,
    #[error(transparent)]
    Process(#[from] ProcessError),
    #[error("read-only namespace observation failed: {0}")]
    Io(#[from] std::io::Error),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        mut read: impl FnMut(Step) -> Result<FileId, NamespaceError>,
    ) -> Result<(), NamespaceError> {
        if self.refused {
            return Err(NamespaceError::Refused);
        }
        let result = (|| {
            for step in [
                Step::Generation,
                Step::Namespace,
                Step::Generation,
                Step::Namespace,
                Step::Generation,
            ] {
                let id = read(step)?;
                if step == Step::Namespace {
                    require_same(id, self.original)?;
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
    original: FileId,
}
impl OriginalProbe<'_> {
    fn inspect(&self) -> Result<FileId, NamespaceError> {
        let root = &self.process.probe.files.root;
        if namespace_id(&current_reader(root)?)? != self.original
            || namespace_id(&self.worker)? != self.original
            || namespace_id(&self.observer)? != self.original
        {
            return Err(NamespaceError::Drift);
        }
        let named = child(&self.process.probe.files.directory, "ns", true)?;
        require_procfs(&named)?;
        if FileId::of(&named.metadata()?) != self.directory_id
            || FileId::of(&self.directory.metadata()?) != self.directory_id
        {
            return Err(NamespaceError::Drift);
        }
        let user = follow_fixed(&named, c"user", false)?;
        if namespace_id(&user)? != self.original
            || namespace_id(&current_reader(root)?)? != self.original
        {
            return Err(NamespaceError::Drift);
        }
        Ok(self.original)
    }
}
/// Retains a shared user namespace only, never trusted host/initial namespace authority.
pub struct SameUserNamespaceRead<'a> {
    probe: OriginalProbe<'a>,
    guard: Gate,
}
/// Uses only the already retained original process and the current reading thread.
pub fn pin_same_user_namespace(
    process: &ProcessRead,
) -> Result<SameUserNamespaceRead<'_>, NamespaceError> {
    process.revalidate()?;
    let observer = current_reader(&process.probe.files.root)?;
    let original = namespace_id(&observer)?;
    let directory = child(&process.probe.files.directory, "ns", true)?;
    require_procfs(&directory)?;
    let directory_id = FileId::of(&directory.metadata()?);
    let worker = follow_fixed(&directory, c"user", false)?;
    require_same(namespace_id(&worker)?, original)?;
    let mut read = SameUserNamespaceRead {
        probe: OriginalProbe {
            process,
            directory,
            directory_id,
            worker,
            observer,
            original,
        },
        guard: Gate::new(original),
    };
    read.revalidate()?;
    Ok(read)
}
impl SameUserNamespaceRead<'_> {
    pub(crate) fn refuse(&mut self) {
        self.guard.refused = true;
    }
    pub fn is_refused(&self) -> bool {
        self.guard.refused
    }
    /// Sequential only; no all-thread, continuous or credential-opener proof.
    pub fn revalidate(&mut self) -> Result<(), NamespaceError> {
        self.guard.observe(|step| match step {
            Step::Generation => {
                self.probe.process.revalidate()?;
                Ok(self.probe.original)
            }
            Step::Namespace => self.probe.inspect(),
        })
    }
}
fn namespace_id(file: &File) -> Result<FileId, NamespaceError> {
    // SAFETY: a live descriptor and initialized writable statfs.
    let mut stat: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstatfs(file.as_raw_fd(), &mut stat) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    if stat.f_type as u64 != libc::NSFS_MAGIC as u64 {
        return Err(NamespaceError::Invalid);
    }
    // SAFETY: NS_GET_NSTYPE takes no pointer argument and does not mutate the namespace.
    let kind = unsafe { libc::ioctl(file.as_raw_fd(), libc::NS_GET_NSTYPE) };
    if kind < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    if kind != libc::CLONE_NEWUSER {
        return Err(NamespaceError::Invalid);
    }
    Ok(FileId::of(&file.metadata()?))
}
fn require_same(actual: FileId, original: FileId) -> Result<(), NamespaceError> {
    if actual != original {
        Err(NamespaceError::Drift)
    } else {
        Ok(())
    }
}
fn current_reader(root: &File) -> Result<File, NamespaceError> {
    require_procfs(root)?;
    let thread = follow_fixed(root, c"thread-self", true)?;
    require_procfs(&thread)?;
    if thread.metadata()?.dev() != root.metadata()?.dev() {
        return Err(NamespaceError::Invalid);
    }
    let ns = child(&thread, "ns", true)?;
    require_procfs(&ns)?;
    if ns.metadata()?.dev() != root.metadata()?.dev() {
        return Err(NamespaceError::Invalid);
    }
    follow_fixed(&ns, c"user", false)
}
fn follow_fixed(directory: &File, name: &CStr, is_dir: bool) -> Result<File, NamespaceError> {
    // These fixed proc magic links must be followed to retain their kernel objects.
    // All other directory lookups keep the existing no-follow policy.
    let flags = libc::O_RDONLY | libc::O_CLOEXEC | if is_dir { libc::O_DIRECTORY } else { 0 };
    // SAFETY: retained directory, fixed NUL-terminated name and read-only flags.
    let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY: successful openat returned one new owned descriptor.
    Ok(unsafe { File::from_raw_fd(fd) })
}

#[cfg(test)]
impl SameUserNamespaceRead<'_> {
    pub(super) fn replace_worker_for_test(&mut self, replacement: File) -> File {
        std::mem::replace(&mut self.probe.worker, replacement)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic_process::pin_owned_test_helper;
    use std::cell::RefCell;
    use std::process::{Child, Command, Stdio};

    fn identity() -> super::super::FileId {
        super::super::FileId(1, 2)
    }
    #[test]
    fn namespace_bracket_is_generation_namespace_generation_namespace_generation() {
        let mut gate = Gate::new(identity());
        let order = RefCell::new(vec![]);
        gate.observe(|step| {
            order.borrow_mut().push(step);
            Ok(identity())
        })
        .unwrap();
        assert_eq!(
            *order.borrow(),
            [
                Step::Generation,
                Step::Namespace,
                Step::Generation,
                Step::Namespace,
                Step::Generation
            ]
        );
        assert!(!gate.refused);
    }
    #[test]
    fn namespace_every_failure_preserves_first_error_and_stops_reads() {
        for failed in 0..5 {
            let mut gate = Gate::new(identity());
            let mut calls = 0;
            let error = gate
                .observe(|_| {
                    let index = calls;
                    calls += 1;
                    if index == failed {
                        Err(NamespaceError::Io(std::io::Error::from_raw_os_error(
                            libc::EACCES,
                        )))
                    } else {
                        Ok(identity())
                    }
                })
                .unwrap_err();
            assert_eq!(calls, failed + 1);
            assert!(
                matches!(error, NamespaceError::Io(error) if error.raw_os_error() == Some(libc::EACCES))
            );
            assert!(gate.refused);
            assert!(matches!(
                gate.observe(|_| panic!("must not read")),
                Err(NamespaceError::Refused)
            ));
        }
    }
    #[test]
    fn namespace_mismatch_at_each_namespace_read_cannot_refresh() {
        for failed in [1, 3] {
            let mut gate = Gate::new(identity());
            let mut calls = 0;
            assert!(matches!(
                gate.observe(|_| {
                    let index = calls;
                    calls += 1;
                    Ok(if index == failed {
                        super::super::FileId(1, 3)
                    } else {
                        identity()
                    })
                }),
                Err(NamespaceError::Drift)
            ));
            assert_eq!(calls, failed + 1);
            assert!(matches!(
                gate.observe(|_| panic!("no restoration read")),
                Err(NamespaceError::Refused)
            ));
        }
    }
    #[test]
    fn namespace_seeded_valid_inode_mutations_are_not_adopted() {
        let seed = 0x074e_2026_u64;
        eprintln!("namespace mutation seed={seed:#x}");
        let mut rng = seed;
        for _ in 0..256 {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let changed = if rng & 1 == 0 {
                super::super::FileId(3, 2)
            } else {
                super::super::FileId(1, 3)
            };
            let mut gate = Gate::new(identity());
            assert!(matches!(
                gate.observe(|_| Ok(changed)),
                Err(NamespaceError::Drift)
            ));
            assert!(gate.refused);
        }
    }
    struct Owned(Child);
    impl Owned {
        fn new() -> Self {
            Self(
                Command::new("/bin/cat")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .unwrap(),
            )
        }
        fn finish(&mut self) {
            self.0.stdin.take();
            self.0.wait().unwrap();
        }
    }
    impl Drop for Owned {
        fn drop(&mut self) {
            self.finish();
        }
    }
    #[test]
    fn namespace_owned_original_and_reader_match_repeatedly() {
        let child = Owned::new();
        let process = pin_owned_test_helper(child.0.id());
        let mut read = pin_same_user_namespace(&process).unwrap();
        for _ in 0..8 {
            read.revalidate().unwrap();
        }
        assert!(!read.is_refused());
        for file in [&read.probe.worker, &read.probe.observer] {
            assert_eq!(
                unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) } & libc::O_ACCMODE,
                libc::O_RDONLY
            );
            assert_ne!(
                unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC,
                0
            );
        }
    }
    #[test]
    fn namespace_owned_exit_refuses_even_though_namespace_fd_survives() {
        let mut child = Owned::new();
        let process = pin_owned_test_helper(child.0.id());
        let mut read = pin_same_user_namespace(&process).unwrap();
        child.finish();
        assert!(namespace_id(&read.probe.worker).is_ok());
        assert!(matches!(read.revalidate(), Err(NamespaceError::Process(_))));
        assert!(read.is_refused());
        assert!(matches!(read.revalidate(), Err(NamespaceError::Refused)));
    }
    #[test]
    fn namespace_constructor_refuses_original_process_after_exit() {
        let mut child = Owned::new();
        let process = pin_owned_test_helper(child.0.id());
        child.finish();
        assert!(matches!(
            pin_same_user_namespace(&process),
            Err(NamespaceError::Process(_))
        ));
    }
    #[test]
    fn namespace_owned_descriptor_substitution_and_restoration_stays_refused() {
        let child = Owned::new();
        let process = pin_owned_test_helper(child.0.id());
        for observer in [false, true] {
            let mut read = pin_same_user_namespace(&process).unwrap();
            let slot = if observer {
                &mut read.probe.observer
            } else {
                &mut read.probe.worker
            };
            let original = std::mem::replace(slot, File::open("/dev/null").unwrap());
            assert!(matches!(read.revalidate(), Err(NamespaceError::Invalid)));
            if observer {
                read.probe.observer = original;
            } else {
                read.probe.worker = original;
            }
            assert!(matches!(read.revalidate(), Err(NamespaceError::Refused)));
        }
    }
    #[test]
    fn namespace_real_wrong_type_and_ordinary_files_refuse() {
        let mount = File::open("/proc/thread-self/ns/mnt").unwrap();
        assert!(matches!(namespace_id(&mount), Err(NamespaceError::Invalid)));
        let regular = tempfile::tempfile().unwrap();
        assert!(matches!(
            namespace_id(&regular),
            Err(NamespaceError::Invalid)
        ));
    }
    #[test]
    fn namespace_identity_comparison_refuses_actual_distinct_objects() {
        let user = File::open("/proc/thread-self/ns/user").unwrap();
        let mount = File::open("/proc/thread-self/ns/mnt").unwrap();
        let original = namespace_id(&user).unwrap();
        assert!(require_same(original, original).is_ok());
        let foreign = FileId::of(&mount.metadata().unwrap());
        assert_ne!(foreign, original);
        assert!(matches!(
            require_same(foreign, original),
            Err(NamespaceError::Drift)
        ));
    }
    #[test]
    fn namespace_original_directory_substitution_and_restoration_stays_refused() {
        let child = Owned::new();
        let process = pin_owned_test_helper(child.0.id());
        let mut read = pin_same_user_namespace(&process).unwrap();
        let original = std::mem::replace(&mut read.probe.directory, File::open("/proc").unwrap());
        assert!(matches!(read.revalidate(), Err(NamespaceError::Drift)));
        read.probe.directory = original;
        assert!(matches!(read.revalidate(), Err(NamespaceError::Refused)));
    }
    #[test]
    fn namespace_concurrent_original_guards_do_not_share_refusal() {
        let child = Owned::new();
        let process = pin_owned_test_helper(child.0.id());
        std::thread::scope(|scope| {
            for index in 0..4 {
                let process = &process;
                scope.spawn(move || {
                    let mut read = pin_same_user_namespace(process).unwrap();
                    if index == 0 {
                        read.probe.worker = File::open("/dev/null").unwrap();
                        assert!(read.revalidate().is_err());
                    } else {
                        for _ in 0..8 {
                            read.revalidate().unwrap();
                        }
                    }
                    assert_eq!(read.is_refused(), index == 0);
                });
            }
        });
    }
}
