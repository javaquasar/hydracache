//! Bounded original-task observations, not signed enrollment or confinement.
use super::credentials::asserted_task_status;
use super::{
    child, number, parse_stat, require_procfs, AssertedWorkerCredentials, CredentialError,
    Document, FileId, ProcessError, ProcessRead, StatFields,
};
use std::ffi::CStr;
use std::fs::File;
use std::marker::PhantomData;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::MetadataExt;
use std::rc::Rc;
use thiserror::Error;

const MAX_TASKS: usize = 128;
const MAX_DIRECTORY_CALLS: usize = 16;
const MAX_DIRECTORY_BYTES: usize = 65_536;

#[derive(Debug, Error)]
pub enum TaskError {
    #[error("original task inventory or namespace descriptor is invalid")]
    Invalid,
    #[error("original task observation exceeded its bound")]
    Budget,
    #[error("original task membership, generation, credentials or namespaces drifted")]
    Drift,
    #[error("original task reader permanently refused")]
    Refused,
    #[error(transparent)]
    Process(#[from] ProcessError),
    #[error(transparent)]
    Credentials(#[from] CredentialError),
    #[error("read-only task observation failed: {0}")]
    Io(#[from] std::io::Error),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Generation,
    Inventory,
    Tasks,
}
const ORDER: [Step; 9] = [
    Step::Generation,
    Step::Inventory,
    Step::Tasks,
    Step::Generation,
    Step::Inventory,
    Step::Tasks,
    Step::Generation,
    Step::Inventory,
    Step::Generation,
];
#[derive(Default)]
struct Gate {
    refused: bool,
}
impl Gate {
    fn observe(
        &mut self,
        mut read: impl FnMut(Step) -> Result<(), TaskError>,
    ) -> Result<(), TaskError> {
        if self.refused {
            return Err(TaskError::Refused);
        }
        for step in ORDER {
            if let Err(error) = read(step) {
                self.refused = true;
                return Err(error);
            }
        }
        Ok(())
    }
}
struct Namespace {
    file: File,
    id: FileId,
    kind: libc::c_int,
}
impl Namespace {
    fn open(directory: &File, name: &CStr, kind: libc::c_int) -> Result<Self, TaskError> {
        // Deliberately follow only the fixed proc magic link for this namespace.
        // SAFETY: retained directory, constant C string, read-only open flags.
        let fd = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        // SAFETY: successful openat returned a new owned descriptor.
        let file = unsafe { File::from_raw_fd(fd) };
        let id = namespace_id(&file, kind)?;
        Ok(Self { file, id, kind })
    }
    fn check(&self, directory: &File, name: &CStr, original: FileId) -> Result<(), TaskError> {
        if self.id != original || namespace_id(&self.file, self.kind)? != self.id {
            return Err(TaskError::Drift);
        }
        if Self::open(directory, name, self.kind)?.id != self.id {
            return Err(TaskError::Drift);
        }
        Ok(())
    }
}
fn namespace_id(file: &File, kind: libc::c_int) -> Result<FileId, TaskError> {
    // SAFETY: live descriptor and initialized writable statfs.
    let mut stat: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstatfs(file.as_raw_fd(), &mut stat) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    if stat.f_type as u64 != libc::NSFS_MAGIC as u64 {
        return Err(TaskError::Invalid);
    }
    // SAFETY: NS_GET_NSTYPE has no pointer argument and is read-only.
    let actual = unsafe { libc::ioctl(file.as_raw_fd(), libc::NS_GET_NSTYPE) };
    if actual < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    if actual != kind {
        return Err(TaskError::Invalid);
    }
    Ok(FileId::of(&file.metadata()?))
}
fn current_tid() -> Result<libc::pid_t, TaskError> {
    // SAFETY: read-only scalar syscall, no pointer arguments.
    let tid = unsafe { libc::syscall(libc::SYS_gettid) };
    if tid <= 0 || tid > i32::MAX as libc::c_long {
        return Err(TaskError::Invalid);
    }
    Ok(tid as libc::pid_t)
}
fn reader_namespaces(root: &File) -> Result<File, TaskError> {
    require_procfs(root)?;
    // SAFETY: fixed proc magic link, retained root, read-only directory flags.
    let fd = unsafe {
        libc::openat(
            root.as_raw_fd(),
            c"thread-self".as_ptr(),
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_DIRECTORY,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY: successful openat returned a new owned descriptor.
    let thread = unsafe { File::from_raw_fd(fd) };
    let ns = child(&thread, "ns", true)?;
    require_procfs(&thread)?;
    require_procfs(&ns)?;
    if thread.metadata()?.dev() != root.metadata()?.dev()
        || ns.metadata()?.dev() != root.metadata()?.dev()
    {
        return Err(TaskError::Invalid);
    }
    Ok(ns)
}
struct Task {
    directory: File,
    id: FileId,
    stat: Document,
    status: Document,
    generation: StatFields,
    bounding: u64,
    ns_directory: File,
    ns_id: FileId,
    user: Namespace,
    mount: Namespace,
}
impl Task {
    fn open(
        tasks: &File,
        tid: u32,
        tgid: u32,
        policy: &AssertedWorkerCredentials,
    ) -> Result<Self, TaskError> {
        let directory = child(tasks, &tid.to_string(), true)?;
        require_procfs(&directory)?;
        let id = FileId::of(&directory.metadata()?);
        if id.0 != tasks.metadata()?.dev() {
            return Err(TaskError::Drift);
        }
        let stat = Document::open(&directory, "stat")?;
        let status = Document::open(&directory, "status")?;
        let generation = parse_stat(&stat.read(&directory)?)?;
        if generation.pid != tid {
            return Err(TaskError::Drift);
        }
        let bounding = asserted_task_status(&status.read(&directory)?, tid, tgid, policy)?;
        let ns_directory = child(&directory, "ns", true)?;
        require_procfs(&ns_directory)?;
        let ns_id = FileId::of(&ns_directory.metadata()?);
        if ns_id.0 != id.0 {
            return Err(TaskError::Drift);
        }
        let user = Namespace::open(&ns_directory, c"user", libc::CLONE_NEWUSER)?;
        let mount = Namespace::open(&ns_directory, c"mnt", libc::CLONE_NEWNS)?;
        Ok(Self {
            directory,
            id,
            stat,
            status,
            generation,
            bounding,
            ns_directory,
            ns_id,
            user,
            mount,
        })
    }
    fn check(&self, probe: &OriginalProbe<'_>) -> Result<(), TaskError> {
        let named = child(&probe.directory, &self.generation.pid.to_string(), true)?;
        if FileId::of(&named.metadata()?) != self.id
            || FileId::of(&self.directory.metadata()?) != self.id
        {
            return Err(TaskError::Drift);
        }
        if parse_stat(&self.stat.read(&self.directory)?)? != self.generation
            || self.generation.group != probe.process.observation.process_group
        {
            return Err(TaskError::Drift);
        }
        if asserted_task_status(
            &self.status.read(&self.directory)?,
            self.generation.pid,
            probe.process.observation.expected.pid,
            &probe.policy,
        )? != self.bounding
        {
            return Err(TaskError::Drift);
        }
        let named_ns = child(&self.directory, "ns", true)?;
        require_procfs(&named_ns)?;
        if FileId::of(&named_ns.metadata()?) != self.ns_id
            || FileId::of(&self.ns_directory.metadata()?) != self.ns_id
        {
            return Err(TaskError::Drift);
        }
        self.user.check(&named_ns, c"user", probe.user.id)?;
        self.mount.check(&named_ns, c"mnt", probe.mount.id)?;
        if parse_stat(&self.stat.read(&self.directory)?)? != self.generation {
            return Err(TaskError::Drift);
        }
        Ok(())
    }
}
struct OriginalProbe<'a> {
    process: &'a ProcessRead,
    policy: AssertedWorkerCredentials,
    directory: File,
    id: FileId,
    tasks: Vec<Task>,
    observer_tid: libc::pid_t,
    user: Namespace,
    mount: Namespace,
}
impl OriginalProbe<'_> {
    fn generation(&self) -> Result<(), TaskError> {
        if current_tid()? != self.observer_tid {
            return Err(TaskError::Drift);
        }
        self.process.revalidate()?;
        let ns = reader_namespaces(&self.process.probe.files.root)?;
        self.user.check(&ns, c"user", self.user.id)?;
        self.mount.check(&ns, c"mnt", self.mount.id)?;
        Ok(())
    }
    fn inventory(&self) -> Result<(), TaskError> {
        let named = child(&self.process.probe.files.directory, "task", true)?;
        require_procfs(&named)?;
        if FileId::of(&named.metadata()?) != self.id
            || FileId::of(&self.directory.metadata()?) != self.id
        {
            return Err(TaskError::Drift);
        }
        let actual = inventory(&self.directory, self.process.observation.expected.pid)?;
        if actual.len() != self.tasks.len()
            || actual
                .iter()
                .zip(&self.tasks)
                .any(|(id, task)| *id != task.generation.pid)
        {
            return Err(TaskError::Drift);
        }
        Ok(())
    }
    fn observe(&self, step: Step) -> Result<(), TaskError> {
        match step {
            Step::Generation => self.generation(),
            Step::Inventory => self.inventory(),
            Step::Tasks => self.tasks.iter().try_for_each(|task| task.check(self)),
        }
    }
}
/// Original stable observed tasks only, no continuous or signed authority.
pub struct WorkerTasksRead<'a> {
    probe: OriginalProbe<'a>,
    gate: Gate,
    _original_thread: PhantomData<Rc<()>>,
}
/// Opens only original process tasks; no selector, replacement inventory or refresh.
pub fn pin_asserted_worker_tasks(
    process: &ProcessRead,
    policy: AssertedWorkerCredentials,
) -> Result<WorkerTasksRead<'_>, TaskError> {
    process.revalidate()?;
    let observer_tid = current_tid()?;
    let ns = reader_namespaces(&process.probe.files.root)?;
    let user = Namespace::open(&ns, c"user", libc::CLONE_NEWUSER)?;
    let mount = Namespace::open(&ns, c"mnt", libc::CLONE_NEWNS)?;
    let directory = child(&process.probe.files.directory, "task", true)?;
    require_procfs(&directory)?;
    let id = FileId::of(&directory.metadata()?);
    if id.0 != process.probe.files.directory.metadata()?.dev() {
        return Err(TaskError::Drift);
    }
    let ids = inventory(&directory, process.observation.expected.pid)?;
    let tasks = ids
        .into_iter()
        .map(|tid| Task::open(&directory, tid, process.observation.expected.pid, &policy))
        .collect::<Result<Vec<_>, _>>()?;
    let mut read = WorkerTasksRead {
        probe: OriginalProbe {
            process,
            policy,
            directory,
            id,
            tasks,
            observer_tid,
            user,
            mount,
        },
        gate: Gate::default(),
        _original_thread: PhantomData,
    };
    read.revalidate()?;
    Ok(read)
}
impl WorkerTasksRead<'_> {
    pub(crate) fn refuse(&mut self) {
        self.gate.refused = true;
    }
    pub(crate) fn matches_worker_policy(
        &self,
        policy: &crate::diagnostic_worker_policy::WorkerPolicy,
    ) -> bool {
        !self.is_refused()
            && super::credentials::asserted_task_policy_matches(&self.probe.policy, policy)
            && self.probe.user.id
                == FileId(policy.user_namespace.device, policy.user_namespace.inode)
            && self.probe.mount.id
                == FileId(policy.mount_namespace.device, policy.mount_namespace.inode)
    }
    pub fn is_refused(&self) -> bool {
        self.gate.refused
    }
    pub fn revalidate(&mut self) -> Result<(), TaskError> {
        self.gate.observe(|step| self.probe.observe(step))
    }
}
fn inventory(directory: &File, leader: u32) -> Result<Vec<u32>, TaskError> {
    // A fresh open file description gives each bounded scan its own offset.
    let scan = child(directory, ".", true)?;
    require_procfs(&scan)?;
    if FileId::of(&scan.metadata()?) != FileId::of(&directory.metadata()?) {
        return Err(TaskError::Drift);
    }
    enumerate(leader, |buffer| {
        // SAFETY: original directory FD and valid bounded writable byte buffer.
        let n = unsafe {
            libc::syscall(
                libc::SYS_getdents64,
                scan.as_raw_fd(),
                buffer.as_mut_ptr(),
                buffer.len(),
            )
        };
        if n < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(n as usize)
    })
}
fn enumerate(
    leader: u32,
    mut read: impl FnMut(&mut [u8]) -> Result<usize, TaskError>,
) -> Result<Vec<u32>, TaskError> {
    let mut ids = vec![];
    let mut records = 0;
    let mut total = 0;
    let mut buffer = [0; 8192];
    for _ in 0..MAX_DIRECTORY_CALLS {
        let n = read(&mut buffer)?;
        if n > buffer.len() {
            return Err(TaskError::Budget);
        }
        if n == 0 {
            ids.sort_unstable();
            if ids.is_empty()
                || !ids.contains(&leader)
                || ids.windows(2).any(|pair| pair[0] == pair[1])
            {
                return Err(TaskError::Invalid);
            }
            return Ok(ids);
        }
        total += n;
        if total > MAX_DIRECTORY_BYTES {
            return Err(TaskError::Budget);
        }
        let mut remaining = &buffer[..n];
        while !remaining.is_empty() {
            records += 1;
            if records > MAX_TASKS + 2 {
                return Err(TaskError::Budget);
            }
            let (tid, used) = directory_record(remaining)?;
            if let Some(tid) = tid {
                if ids.len() == MAX_TASKS {
                    return Err(TaskError::Budget);
                }
                ids.push(tid);
            }
            remaining = &remaining[used..];
        }
    }
    Err(TaskError::Budget)
}
fn directory_record(bytes: &[u8]) -> Result<(Option<u32>, usize), TaskError> {
    // linux_dirent64: ino/off/reclen/type/name, parsed without unaligned loads.
    if bytes.len() < 24 {
        return Err(TaskError::Invalid);
    }
    let size = u16::from_ne_bytes([bytes[16], bytes[17]]) as usize;
    if size < 24
        || !size.is_multiple_of(8)
        || size > bytes.len()
        || bytes[18] != libc::DT_DIR
        || bytes[..8] == [0; 8]
    {
        return Err(TaskError::Invalid);
    }
    let end = bytes[19..size]
        .iter()
        .position(|byte| *byte == 0)
        .ok_or(TaskError::Invalid)?;
    let name = &bytes[19..19 + end];
    if name == b"." || name == b".." {
        return Ok((None, size));
    }
    let id = u32::try_from(number(name).map_err(|_| TaskError::Invalid)?)
        .map_err(|_| TaskError::Invalid)?;
    if id == 0 || id > i32::MAX as u32 {
        return Err(TaskError::Invalid);
    }
    Ok((Some(id), size))
}

#[cfg(test)]
#[path = "diagnostic_worker_tasks_tests.rs"]
mod tests;

#[cfg(test)]
pub(crate) use tests::Owned as OwnedTasksHelper;

#[cfg(test)]
impl WorkerTasksRead<'_> {
    pub(crate) fn mutate_signed_mapping_for_test(&mut self, case: usize) {
        match case {
            0 => self.probe.user.id.1 ^= 1,
            1 => self.probe.mount.id.1 ^= 1,
            2 => self.probe.policy = AssertedWorkerCredentials::new(12345, 12345, vec![]).unwrap(),
            3 => self.probe.tasks[0].status.file = tempfile::tempfile().unwrap(),
            _ => unreachable!(),
        }
    }
}
