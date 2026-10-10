//! Observed local context under original external policy pins, never host/start authority.
use super::{
    AssertedWorkerHost, CheckedWorkerPolicy, NamespaceIdentity, WorkerPolicyError,
    WorkerPolicyTrust,
};
use std::ffi::CStr;
use std::fs::{File, Metadata, OpenOptions};
use std::marker::PhantomData;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{FileExt, MetadataExt, OpenOptionsExt};
use std::rc::Rc;
use thiserror::Error;

pub const MACHINE_DOCUMENT_BYTES: usize = 33;
pub const BOOT_DOCUMENT_BYTES: usize = 37;

#[derive(Debug, Error)]
pub enum WorkerContextError {
    #[error("original worker context or policy previously refused")]
    Refused,
    #[error(transparent)]
    Policy(#[from] WorkerPolicyError),
    #[error("read-only worker context observation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("worker context ancestry, document or namespace type is unsafe")]
    Security,
    #[error("original host, boot, thread, namespace or named object changed")]
    Drift,
}

#[derive(PartialEq, Eq)]
struct DirectoryStamp {
    device: u64,
    inode: u64,
    uid: u32,
    gid: u32,
    mode: u32,
}
impl From<Metadata> for DirectoryStamp {
    fn from(m: Metadata) -> Self {
        Self {
            device: m.dev(),
            inode: m.ino(),
            uid: m.uid(),
            gid: m.gid(),
            mode: m.mode(),
        }
    }
}
#[derive(PartialEq, Eq)]
struct FileStamp {
    directory: DirectoryStamp,
    links: u64,
    length: u64,
    mtime: i64,
    mtime_ns: i64,
    ctime: i64,
    ctime_ns: i64,
}
impl From<Metadata> for FileStamp {
    fn from(m: Metadata) -> Self {
        Self {
            links: m.nlink(),
            length: m.len(),
            mtime: m.mtime(),
            mtime_ns: m.mtime_nsec(),
            ctime: m.ctime(),
            ctime_ns: m.ctime_nsec(),
            directory: m.into(),
        }
    }
}
struct PinnedDirectory {
    file: File,
    stamp: DirectoryStamp,
}
impl PinnedDirectory {
    fn new(file: File, procfs: bool) -> Result<Self, WorkerContextError> {
        let m = file.metadata()?;
        if !m.is_dir()
            || m.uid() != 0
            || m.gid() != 0
            || m.mode() & 0o7022 != 0
            || m.mode() & 0o100 == 0
        {
            return Err(WorkerContextError::Security);
        }
        if procfs {
            filesystem(&file, libc::PROC_SUPER_MAGIC as u64)?;
        }
        Ok(Self {
            stamp: m.into(),
            file,
        })
    }
    fn same(&self, named: &Self) -> Result<(), WorkerContextError> {
        if DirectoryStamp::from(self.file.metadata()?) != self.stamp || named.stamp != self.stamp {
            return Err(WorkerContextError::Drift);
        }
        Ok(())
    }
}
struct Document {
    file: File,
    stamp: FileStamp,
}
impl Document {
    fn new(file: File, procfs: bool) -> Result<Self, WorkerContextError> {
        let m = file.metadata()?;
        if !m.is_file() || m.uid() != 0 || m.gid() != 0 || m.nlink() != 1 || m.mode() & 0o7133 != 0
        {
            return Err(WorkerContextError::Security);
        }
        if procfs {
            filesystem(&file, libc::PROC_SUPER_MAGIC as u64)?;
        } else if m.len() != MACHINE_DOCUMENT_BYTES as u64 {
            return Err(WorkerContextError::Security);
        }
        Ok(Self {
            stamp: m.into(),
            file,
        })
    }
    fn same(&self, named: &Self) -> Result<(), WorkerContextError> {
        if FileStamp::from(self.file.metadata()?) != self.stamp || named.stamp != self.stamp {
            return Err(WorkerContextError::Drift);
        }
        Ok(())
    }
    fn read(&self, expected: &str, length: usize) -> Result<(), WorkerContextError> {
        self.read_with(expected, length, |buffer| self.file.read_at(buffer, 0))
    }
    fn read_with(
        &self,
        expected: &str,
        length: usize,
        read: impl FnOnce(&mut [u8]) -> std::io::Result<usize>,
    ) -> Result<(), WorkerContextError> {
        if FileStamp::from(self.file.metadata()?) != self.stamp {
            return Err(WorkerContextError::Drift);
        }
        // One bounded positional read. Short/EINTR reads refuse; no retry or IO deadline claim.
        let mut buffer = vec![0; length + 1];
        let count = read(&mut buffer)?;
        if FileStamp::from(self.file.metadata()?) != self.stamp {
            return Err(WorkerContextError::Drift);
        }
        if count != length
            || expected.len() + 1 != length
            || buffer[count - 1] != b'\n'
            || &buffer[..count - 1] != expected.as_bytes()
        {
            return Err(WorkerContextError::Drift);
        }
        Ok(())
    }
}
struct Namespaces {
    user: File,
    mount: File,
}
impl Namespaces {
    fn open(proc: &File) -> Result<Self, WorkerContextError> {
        // Only these literal proc magic links are intentionally followed.
        let thread = child(proc, c"thread-self", true, true)?;
        filesystem(&thread, libc::PROC_SUPER_MAGIC as u64)?;
        let ns = child(&thread, c"ns", true, false)?;
        filesystem(&ns, libc::PROC_SUPER_MAGIC as u64)?;
        Ok(Self {
            user: child(&ns, c"user", false, true)?,
            mount: child(&ns, c"mnt", false, true)?,
        })
    }
    fn require(&self, host: &AssertedWorkerHost) -> Result<(), WorkerContextError> {
        if namespace(&self.user, libc::CLONE_NEWUSER)? != host.user_namespace
            || namespace(&self.mount, libc::CLONE_NEWNS)? != host.mount_namespace
        {
            return Err(WorkerContextError::Drift);
        }
        Ok(())
    }
}
struct FixedPaths {
    root: PinnedDirectory,
    etc: PinnedDirectory,
    proc: PinnedDirectory,
    boot_dirs: Vec<PinnedDirectory>,
    machine: Document,
    boot: Document,
}
impl FixedPaths {
    fn open(
        expected: &AssertedWorkerHost,
        original_ns: &Namespaces,
    ) -> Result<Self, WorkerContextError> {
        let root = root()?;
        let proc = PinnedDirectory::new(child(&root.file, c"proc", true, false)?, true)?;
        original_ns.require(expected)?;
        Namespaces::open(&proc.file)?.require(expected)?;
        let etc = PinnedDirectory::new(child(&root.file, c"etc", true, false)?, false)?;
        let machine = Document::new(child(&etc.file, c"machine-id", false, false)?, false)?;
        let mut boot_dirs = Vec::with_capacity(3);
        for name in [c"sys", c"kernel", c"random"] {
            let parent = boot_dirs
                .last()
                .map_or(&proc.file, |d: &PinnedDirectory| &d.file);
            boot_dirs.push(PinnedDirectory::new(
                child(parent, name, true, false)?,
                true,
            )?);
        }
        let boot = Document::new(child(&boot_dirs[2].file, c"boot_id", false, false)?, true)?;
        Namespaces::open(&proc.file)?.require(expected)?;
        original_ns.require(expected)?;
        Ok(Self {
            root,
            etc,
            proc,
            boot_dirs,
            machine,
            boot,
        })
    }
    fn named(&self) -> Result<(), WorkerContextError> {
        let r = root()?;
        self.root.same(&r)?;
        let etc = PinnedDirectory::new(child(&r.file, c"etc", true, false)?, false)?;
        self.etc.same(&etc)?;
        self.machine.same(&Document::new(
            child(&etc.file, c"machine-id", false, false)?,
            false,
        )?)?;
        let proc = PinnedDirectory::new(child(&r.file, c"proc", true, false)?, true)?;
        self.proc.same(&proc)?;
        let mut current = child(&proc.file, c"sys", true, false)?;
        for (index, name) in [c"sys", c"kernel", c"random"].iter().enumerate() {
            if index != 0 {
                current = child(&current, name, true, false)?;
            }
            let d = PinnedDirectory::new(current.try_clone()?, true)?;
            self.boot_dirs[index].same(&d)?;
        }
        self.boot.same(&Document::new(
            child(&current, c"boot_id", false, false)?,
            true,
        )?)?;
        Ok(())
    }
}
struct Probe {
    tid: libc::pid_t,
    namespaces: Namespaces,
    paths: FixedPaths,
}
impl Probe {
    fn open(expected: &AssertedWorkerHost) -> Result<Self, WorkerContextError> {
        let tid = thread_id();
        let r = root()?;
        let proc = PinnedDirectory::new(child(&r.file, c"proc", true, false)?, true)?;
        let namespaces = Namespaces::open(&proc.file)?;
        namespaces.require(expected)?;
        let paths = FixedPaths::open(expected, &namespaces)?;
        // Refuse a changed proc root between initial namespace and document opening.
        proc.same(&paths.proc)?;
        r.same(&paths.root)?;
        let probe = Self {
            tid,
            namespaces,
            paths,
        };
        probe.observe(expected)?;
        Ok(probe)
    }
    fn namespace_bracket(&self, expected: &AssertedWorkerHost) -> Result<(), WorkerContextError> {
        if thread_id() != self.tid {
            return Err(WorkerContextError::Drift);
        }
        self.paths.named()?;
        self.namespaces.require(expected)?;
        Namespaces::open(&self.paths.proc.file)?.require(expected)?;
        if thread_id() != self.tid {
            return Err(WorkerContextError::Drift);
        }
        Ok(())
    }
    fn observe(&self, expected: &AssertedWorkerHost) -> Result<(), WorkerContextError> {
        for _ in 0..2 {
            self.namespace_bracket(expected)?;
            self.paths
                .machine
                .read(&expected.machine_id, MACHINE_DOCUMENT_BYTES)?;
            self.paths
                .boot
                .read(&expected.boot_id, BOOT_DOCUMENT_BYTES)?;
            self.namespace_bracket(expected)?;
        }
        Ok(())
    }
}

/// Current-thread observations only. Cannot be sent/shared, extracted or converted to start authority.
pub struct WorkerContextRead<'policy> {
    policy: &'policy mut CheckedWorkerPolicy,
    probe: Probe,
    thread_only: PhantomData<Rc<()>>,
}
pub fn inspect_worker_context<'policy>(
    policy: &'policy mut CheckedWorkerPolicy,
    bytes: &[u8],
    trust: &WorkerPolicyTrust,
    host: &AssertedWorkerHost,
) -> Result<WorkerContextRead<'policy>, WorkerContextError> {
    if policy.is_refused() {
        return Err(WorkerContextError::Refused);
    }
    let result = (|| {
        policy.revalidate(bytes, trust, host)?;
        Probe::open(&policy.context)
    })();
    match result {
        Ok(probe) => Ok(WorkerContextRead {
            policy,
            probe,
            thread_only: PhantomData,
        }),
        Err(error) => {
            policy.refused = true;
            Err(error)
        }
    }
}
impl WorkerContextRead<'_> {
    pub fn is_refused(&self) -> bool {
        self.policy.is_refused()
    }
    pub fn revalidate(
        &mut self,
        bytes: &[u8],
        trust: &WorkerPolicyTrust,
        host: &AssertedWorkerHost,
    ) -> Result<(), WorkerContextError> {
        if self.is_refused() {
            return Err(WorkerContextError::Refused);
        }
        let result = (|| {
            self.policy.revalidate(bytes, trust, host)?;
            self.probe.observe(&self.policy.context)
        })();
        if result.is_err() {
            self.policy.refused = true;
        }
        result
    }
}
fn root() -> Result<PinnedDirectory, WorkerContextError> {
    PinnedDirectory::new(
        OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
            .open("/")?,
        false,
    )
}
fn child(
    directory: &File,
    name: &CStr,
    dir: bool,
    follow: bool,
) -> Result<File, WorkerContextError> {
    let flags = libc::O_RDONLY
        | libc::O_CLOEXEC
        | libc::O_NONBLOCK
        | if dir { libc::O_DIRECTORY } else { 0 }
        | if follow { 0 } else { libc::O_NOFOLLOW };
    // SAFETY: retained directory, literal NUL-terminated component, read-only flags.
    let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY: successful openat returned one new owned descriptor.
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn filesystem(file: &File, expected: u64) -> Result<(), WorkerContextError> {
    // SAFETY: live descriptor and initialized writable statfs.
    let mut stat: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstatfs(file.as_raw_fd(), &mut stat) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    if stat.f_type as u64 != expected {
        return Err(WorkerContextError::Security);
    }
    Ok(())
}
fn namespace(file: &File, expected: libc::c_int) -> Result<NamespaceIdentity, WorkerContextError> {
    filesystem(file, libc::NSFS_MAGIC as u64)?;
    // SAFETY: NS_GET_NSTYPE takes no pointer argument and never joins a namespace.
    let kind = unsafe { libc::ioctl(file.as_raw_fd(), libc::NS_GET_NSTYPE) };
    if kind < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    if kind != expected {
        return Err(WorkerContextError::Security);
    }
    let m = file.metadata()?;
    Ok(NamespaceIdentity {
        device: m.dev(),
        inode: m.ino(),
    })
}
fn thread_id() -> libc::pid_t {
    // SAFETY: gettid has no arguments and returns the calling thread's ID.
    unsafe { libc::gettid() }
}

#[cfg(test)]
mod tests {
    use super::super::{
        verify_worker_policy, SignedWorkerPolicy, WorkerPolicy, SIGNED_WORKER_POLICY_SCHEMA,
        WORKER_POLICY_DOMAIN, WORKER_POLICY_SCHEMA,
    };
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use std::fs;
    use std::os::unix::fs::{symlink, PermissionsExt};

    fn material() -> (Vec<u8>, WorkerPolicyTrust, AssertedWorkerHost) {
        let host = AssertedWorkerHost {
            machine_id: fs::read_to_string("/etc/machine-id")
                .unwrap()
                .trim_end_matches('\n')
                .into(),
            boot_id: fs::read_to_string("/proc/sys/kernel/random/boot_id")
                .unwrap()
                .trim_end_matches('\n')
                .into(),
            user_namespace: namespace(
                &File::open("/proc/thread-self/ns/user").unwrap(),
                libc::CLONE_NEWUSER,
            )
            .unwrap(),
            mount_namespace: namespace(
                &File::open("/proc/thread-self/ns/mnt").unwrap(),
                libc::CLONE_NEWNS,
            )
            .unwrap(),
        };
        let p = WorkerPolicy {
            schema_version: WORKER_POLICY_SCHEMA.into(),
            repository_id: 1_217_101_761,
            purpose: "diagnostic-worker-only".into(),
            account_source: "local-files".into(),
            account: "hydracache-perf".into(),
            group: "hydracache-perf".into(),
            policy_epoch: 7,
            uid: 986,
            gid: 986,
            supplementary_gids: vec![],
            machine_id: host.machine_id.clone(),
            boot_id: host.boot_id.clone(),
            user_namespace: host.user_namespace,
            mount_namespace: host.mount_namespace,
            passwd_sha256: "b".repeat(64),
            group_sha256: "c".repeat(64),
        };
        let key = SigningKey::from_bytes(&[74; 32]);
        let body = super::super::canonical_line(&p).unwrap();
        let trust =
            WorkerPolicyTrust::new(key.verifying_key(), &crate::sha256_hex(&body), 7).unwrap();
        let mut message = WORKER_POLICY_DOMAIN.to_vec();
        message.push(0);
        message.extend(body);
        let signed = SignedWorkerPolicy {
            schema_version: SIGNED_WORKER_POLICY_SCHEMA.into(),
            policy: p,
            signature_hex: key
                .sign(&message)
                .to_bytes()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
        };
        (super::super::canonical_line(&signed).unwrap(), trust, host)
    }
    #[test]
    fn worker_context_guard_cannot_be_sent_or_shared() {
        trait AmbiguousSend<A> {
            fn marker() {}
        }
        impl<T: ?Sized> AmbiguousSend<()> for T {}
        impl<T: ?Sized + Send> AmbiguousSend<u8> for T {}
        let _ = <WorkerContextRead<'static> as AmbiguousSend<_>>::marker;
        trait AmbiguousSync<A> {
            fn marker() {}
        }
        impl<T: ?Sized> AmbiguousSync<()> for T {}
        impl<T: ?Sized + Sync> AmbiguousSync<u8> for T {}
        let _ = <WorkerContextRead<'static> as AmbiguousSync<_>>::marker;
    }
    #[test]
    fn worker_context_namespace_descriptors_are_typed_readonly_cloexec() {
        let proc = File::open("/proc").unwrap();
        let ns = Namespaces::open(&proc).unwrap();
        for file in [&ns.user, &ns.mount] {
            assert_eq!(
                unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) } & libc::O_ACCMODE,
                libc::O_RDONLY
            );
            assert_ne!(
                unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC,
                0
            );
        }
        assert!(matches!(
            namespace(&ns.mount, libc::CLONE_NEWUSER),
            Err(WorkerContextError::Security)
        ));
        assert!(matches!(
            namespace(&ns.user, libc::CLONE_NEWNS),
            Err(WorkerContextError::Security)
        ));
        assert!(matches!(
            namespace(&tempfile::tempfile().unwrap(), libc::CLONE_NEWUSER),
            Err(WorkerContextError::Security)
        ));
        assert!(matches!(
            filesystem(
                &tempfile::tempfile().unwrap(),
                libc::PROC_SUPER_MAGIC as u64
            ),
            Err(WorkerContextError::Security)
        ));
    }
    #[test]
    fn worker_context_original_descriptor_and_thread_restoration_stay_refused() {
        let (bytes, trust, host) = material();
        for slot in 0..5 {
            let mut policy = verify_worker_policy(&bytes, &trust, &host).unwrap();
            {
                let mut read = inspect_worker_context(&mut policy, &bytes, &trust, &host).unwrap();
                let original_tid = read.probe.tid;
                let target = match slot {
                    0 => &mut read.probe.namespaces.user,
                    1 => &mut read.probe.namespaces.mount,
                    2 => &mut read.probe.paths.machine.file,
                    _ => &mut read.probe.paths.boot.file,
                };
                let original = target.try_clone().unwrap();
                if slot == 4 {
                    read.probe.tid += 1;
                } else {
                    *target = File::open("/dev/null").unwrap();
                }
                assert!(read.revalidate(&bytes, &trust, &host).is_err());
                let target = match slot {
                    0 => &mut read.probe.namespaces.user,
                    1 => &mut read.probe.namespaces.mount,
                    2 => &mut read.probe.paths.machine.file,
                    _ => &mut read.probe.paths.boot.file,
                };
                *target = original;
                read.probe.tid = original_tid;
                assert!(matches!(
                    read.revalidate(&bytes, &trust, &host),
                    Err(WorkerContextError::Refused)
                ));
            }
            assert!(policy.is_refused());
        }
    }
    #[test]
    fn worker_context_policy_failure_precedes_broken_probe_io() {
        let (bytes, trust, host) = material();
        let mut policy = verify_worker_policy(&bytes, &trust, &host).unwrap();
        let mut read = inspect_worker_context(&mut policy, &bytes, &trust, &host).unwrap();
        read.probe.paths.proc.file = File::open("/dev/null").unwrap();
        assert!(matches!(
            read.revalidate(b"bad", &trust, &host),
            Err(WorkerContextError::Policy(WorkerPolicyError::Drift))
        ));
        assert!(matches!(
            read.revalidate(&bytes, &trust, &host),
            Err(WorkerContextError::Refused)
        ));
        drop(read);
        assert!(matches!(
            inspect_worker_context(&mut policy, &bytes, &trust, &host),
            Err(WorkerContextError::Refused)
        ));
        let mut policy = verify_worker_policy(&bytes, &trust, &host).unwrap();
        assert!(matches!(
            inspect_worker_context(&mut policy, b"bad", &trust, &host),
            Err(WorkerContextError::Policy(WorkerPolicyError::Drift))
        ));
        assert!(policy.is_refused());
    }
    #[test]
    fn worker_context_single_read_refuses_short_overrun_error_and_midread_growth() {
        let temporary = tempfile::NamedTempFile::new().unwrap();
        fs::write(temporary.path(), b"abc\n").unwrap();
        let file = File::open(temporary.path()).unwrap();
        // Caller-owned internal fixture tests the read, not production root ownership.
        let doc = Document {
            stamp: file.metadata().unwrap().into(),
            file,
        };
        for count in [0, 3, 5] {
            assert!(matches!(
                doc.read_with("abc", 4, |_| Ok(count)),
                Err(WorkerContextError::Drift)
            ));
        }
        assert!(matches!(
            doc.read_with("abc", 4, |_| Err(std::io::Error::from_raw_os_error(
                libc::EINTR
            ))),
            Err(WorkerContextError::Io(_))
        ));
        doc.read("abc", 4).unwrap();
        assert!(matches!(doc.read("abd", 4), Err(WorkerContextError::Drift)));
        assert!(matches!(
            doc.read_with("abc", 4, |buffer| {
                fs::write(temporary.path(), b"abcd\n")?;
                buffer[..4].copy_from_slice(b"abc\n");
                Ok(4)
            }),
            Err(WorkerContextError::Drift)
        ));
        assert!(matches!(
            doc.read_with("abc", 4, |_| panic!(
                "changed original must refuse before read"
            )),
            Err(WorkerContextError::Drift)
        ));
    }
    #[test]
    fn worker_context_fixed_security_and_nofollow_refuse_owned_fixtures() {
        let directory = tempfile::tempdir().unwrap();
        let regular = directory.path().join("machine-id");
        fs::write(&regular, b"a".repeat(MACHINE_DOCUMENT_BYTES)).unwrap();
        assert!(matches!(
            Document::new(File::open(&regular).unwrap(), false),
            Err(WorkerContextError::Security)
        ));
        assert!(matches!(
            PinnedDirectory::new(File::open(directory.path()).unwrap(), false),
            Err(WorkerContextError::Security)
        ));
        let root = File::open(directory.path()).unwrap();
        symlink(&regular, directory.path().join("link")).unwrap();
        assert!(matches!(
            child(&root, c"link", false, false),
            Err(WorkerContextError::Io(_))
        ));
        fs::hard_link(&regular, directory.path().join("alias")).unwrap();
        assert!(matches!(
            Document::new(File::open(&regular).unwrap(), false),
            Err(WorkerContextError::Security)
        ));
        fs::set_permissions(&regular, fs::Permissions::from_mode(0o777)).unwrap();
        assert!(matches!(
            Document::new(File::open(&regular).unwrap(), false),
            Err(WorkerContextError::Security)
        ));
        assert!(matches!(
            Document::new(File::open(directory.path()).unwrap(), false),
            Err(WorkerContextError::Security)
        ));
        assert!(matches!(
            child(&root, c"machine-id", true, false),
            Err(WorkerContextError::Io(_))
        ));
    }
    #[test]
    fn worker_context_original_directory_and_named_document_objects_are_not_refreshed() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("a");
        fs::write(&path, b"abc\n").unwrap();
        let file = File::open(&path).unwrap();
        let original = Document {
            stamp: file.metadata().unwrap().into(),
            file,
        };
        fs::rename(&path, directory.path().join("old")).unwrap();
        fs::write(&path, b"abc\n").unwrap();
        let file = File::open(&path).unwrap();
        let replacement = Document {
            stamp: file.metadata().unwrap().into(),
            file,
        };
        assert!(matches!(
            original.same(&replacement),
            Err(WorkerContextError::Drift)
        ));
        let d = File::open(directory.path()).unwrap();
        let pinned = PinnedDirectory {
            stamp: d.metadata().unwrap().into(),
            file: d,
        };
        let other = tempfile::tempdir().unwrap();
        let d = File::open(other.path()).unwrap();
        let changed = PinnedDirectory {
            stamp: d.metadata().unwrap().into(),
            file: d,
        };
        assert!(matches!(
            pinned.same(&changed),
            Err(WorkerContextError::Drift)
        ));
        // Unrelated directory maintenance timestamps do not change the pin.
        fs::write(directory.path().join("unrelated"), b"x").unwrap();
        pinned.same(&pinned).unwrap();
    }
}
