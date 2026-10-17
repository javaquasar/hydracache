//! Exclusive empty sinks under the original ancestry, not execution or drain authority.
use super::*;
use std::ffi::CString;
#[path = "diagnostic_original_output_join.rs"]
mod joined;
pub use joined::OutputJoinStage;
#[path = "diagnostic_original_start_material.rs"]
pub mod start_material;

const PREFIX: &str = "diagnostic-collector-output-074-";
const DIRECTORY: &str = "collector-attempt-v1";
const NAMES: [&str; 2] = ["collector-stdout.bin", "collector-stderr.bin"];
// Linux x86_64 kernel F_GETFL ABI. libc::O_LARGEFILE is zero on this target,
// but the kernel forces the actual large-file status bit even with SYS_openat.
const KERNEL_LARGEFILE: i32 = 0o100000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SinkPreparationStage {
    BeforeCreate,
    RootCreated,
    RootSynced,
    DirectoryCreated,
    DirectorySynced,
    StdoutCreated,
    StdoutSynced,
    StderrCreated,
    StderrSynced,
    Acknowledging,
}
const STAGES: [SinkPreparationStage; 10] = [
    SinkPreparationStage::BeforeCreate,
    SinkPreparationStage::RootCreated,
    SinkPreparationStage::RootSynced,
    SinkPreparationStage::DirectoryCreated,
    SinkPreparationStage::DirectorySynced,
    SinkPreparationStage::StdoutCreated,
    SinkPreparationStage::StdoutSynced,
    SinkPreparationStage::StderrCreated,
    SinkPreparationStage::StderrSynced,
    SinkPreparationStage::Acknowledging,
];
#[derive(Debug, thiserror::Error)]
pub enum OriginalSinkError {
    #[error("original pipe/Starting refused: {0}")]
    Original(#[from] OriginalPipeError),
    #[error("original storage parent refused: {0}")]
    Storage(#[from] crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::attempt_storage::StorageAttemptError),
    #[error("root-only collector preparation refused")]
    Privilege,
    #[error("collector sink identity, flags, bytes or inventory refused")]
    Invalid,
    #[error("collector sink context exists; no adoption or repair")]
    Conflict,
    #[error("original collector sink preparation already refused")]
    Refused,
    #[error("collector sink I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("private interruption at {0:?}")]
    Interrupted(SinkPreparationStage),
    #[error("private joined interruption at {0:?}")]
    JoinedInterrupted(OutputJoinStage),
}
fn output_name(reserved: &[u8]) -> String {
    format!("{PREFIX}{}", crate::sha256_hex(reserved))
}
fn open(parent: &File, name: &str, flags: i32) -> Result<File, OriginalSinkError> {
    let name = CString::new(name).map_err(|_| OriginalSinkError::Invalid)?;
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            flags | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            0o600,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn mkdir(parent: &File, name: &str) -> Result<(), OriginalSinkError> {
    let name = CString::new(name).map_err(|_| OriginalSinkError::Invalid)?;
    if unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } != 0 {
        let error = std::io::Error::last_os_error();
        return Err(if error.kind() == std::io::ErrorKind::AlreadyExists {
            OriginalSinkError::Conflict
        } else {
            error.into()
        });
    }
    Ok(())
}
fn descriptor_flags(file: &File, command: i32) -> Result<i32, OriginalSinkError> {
    let value = unsafe { libc::fcntl(file.as_raw_fd(), command) };
    if value < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(value)
}
#[derive(PartialEq, Eq)]
struct Stamp([u64; 11]);
impl Stamp {
    fn of(m: &std::fs::Metadata, directory: bool) -> Self {
        Self([
            m.dev(),
            m.ino(),
            u64::from(m.uid()),
            u64::from(m.gid()),
            u64::from(m.mode()),
            if directory { 0 } else { m.nlink() },
            if directory { 0 } else { m.len() },
            if directory { 0 } else { m.mtime() as u64 },
            if directory { 0 } else { m.mtime_nsec() as u64 },
            if directory { 0 } else { m.ctime() as u64 },
            if directory { 0 } else { m.ctime_nsec() as u64 },
        ])
    }
}
struct Object {
    file: File,
    stamp: Option<Stamp>,
    directory: bool,
}
impl Object {
    fn retain(file: File, directory: bool) -> Self {
        Self {
            file,
            stamp: None,
            directory,
        }
    }
    fn inspect_file(&self, file: &File, uid: u32, gid: u32) -> Result<(), OriginalSinkError> {
        let m = file.metadata()?;
        if m.uid() != uid
            || m.gid() != gid
            || m.mode() & 0o7777 != if self.directory { 0o700 } else { 0o600 }
            || if self.directory {
                !m.is_dir()
            } else {
                !m.is_file() || m.nlink() != 1 || m.len() != 0
            }
            || self.stamp.as_ref() != Some(&Stamp::of(&m, self.directory))
            || descriptor_flags(file, libc::F_GETFD)? != libc::FD_CLOEXEC
            || descriptor_flags(file, libc::F_GETFL)?
                != (KERNEL_LARGEFILE
                    | libc::O_NOFOLLOW
                    | libc::O_NONBLOCK
                    | if self.directory {
                        libc::O_RDONLY | libc::O_DIRECTORY
                    } else {
                        libc::O_RDWR
                    })
        {
            return Err(OriginalSinkError::Invalid);
        }
        Ok(())
    }
    fn inspect(
        &self,
        parent: &File,
        name: &str,
        uid: u32,
        gid: u32,
    ) -> Result<(), OriginalSinkError> {
        if self.file.metadata()?.dev() != parent.metadata()?.dev() {
            return Err(OriginalSinkError::Invalid);
        }
        self.inspect_file(&self.file, uid, gid)?;
        let named = open(
            parent,
            name,
            if self.directory {
                libc::O_RDONLY | libc::O_DIRECTORY
            } else {
                libc::O_RDWR
            },
        )
        .map_err(|_| OriginalSinkError::Invalid)?;
        self.inspect_file(&named, uid, gid)?;
        self.inspect_file(&self.file, uid, gid)
    }
    fn initialize(&mut self, uid: u32, gid: u32) -> Result<(), OriginalSinkError> {
        self.stamp = Some(Stamp::of(&self.file.metadata()?, self.directory));
        self.inspect_file(&self.file, uid, gid)
    }
}
fn inventory(directory: &File, expected: &[&str]) -> Result<(), OriginalSinkError> {
    // Stop at the first unknown/duplicate entry; no unbounded attacker-selected collection.
    let mut seen = vec![false; expected.len()];
    let fresh = open(directory, ".", libc::O_RDONLY | libc::O_DIRECTORY)?;
    for entry in std::fs::read_dir(format!("/proc/self/fd/{}", fresh.as_raw_fd()))? {
        let name = entry?.file_name();
        let index = expected
            .iter()
            .position(|v| name == *v)
            .ok_or(OriginalSinkError::Invalid)?;
        if seen[index] {
            return Err(OriginalSinkError::Invalid);
        }
        seen[index] = true;
    }
    if seen.iter().any(|s| !s) {
        return Err(OriginalSinkError::Invalid);
    }
    Ok(())
}

/// Private empty original sinks. No descriptors, pump, start or recovery are exported.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::OriginalCollectorSinks;
/// fn denied(p: OriginalCollectorSinks<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.clone(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::OriginalCollectorSinks;
/// fn denied(p: OriginalCollectorSinks<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.into_stdio(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::OriginalCollectorSinks;
/// fn denied(p: OriginalCollectorSinks<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { use std::os::fd::AsRawFd; p.as_raw_fd(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::OriginalCollectorSinks;
/// fn denied(p: OriginalCollectorSinks<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.start_once(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::OriginalCollectorSinks;
/// fn denied(p: OriginalCollectorSinks<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.finish(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::OriginalCollectorSinks;
/// fn denied(p: OriginalCollectorSinks<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.prepare_original_sinks(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::OriginalCollectorSinks;
/// fn denied(p: OriginalCollectorSinks<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.into_pipes(); }
/// ```
pub struct OriginalCollectorSinks<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    // Declaration order closes evidence writers before pipes and original fence.
    sinks: [Option<Object>; 2],
    directory: Option<Object>,
    root: Option<Object>,
    pipes: OriginalCollectorPipes<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    name: String,
    uid: u32,
    gid: u32,
    acknowledged: bool,
    refused: bool,
}
impl fmt::Debug for OriginalCollectorSinks<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OriginalCollectorSinks")
            .field("refused", &self.is_refused())
            .finish_non_exhaustive()
    }
}
impl OriginalCollectorSinks<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn is_refused(&self) -> bool {
        self.refused || self.pipes.is_refused()
    }
    fn refuse(&mut self) {
        self.refused = true;
        self.pipes.refuse();
    }
    fn reservation(&self) -> &crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::attempt_storage::collector_reservation_storage::CollectorReservationStorage<'_>{
        &self
            .pipes
            .starting
            .original
            .original
            .prepared
            .original
            .original
            .reservation
            .storage
    }
    fn parent(&self) -> Result<&File, OriginalSinkError> {
        Ok(self.reservation().collector_output_parent()?)
    }
    fn inspect(&self, complete: bool) -> Result<(), OriginalSinkError> {
        if let Some(root) = &self.root {
            root.inspect(self.parent()?, &self.name, self.uid, self.gid)?;
            inventory(
                &root.file,
                if self.directory.is_some() {
                    &[DIRECTORY]
                } else {
                    &[]
                },
            )?;
        }
        if let Some(directory) = &self.directory {
            directory.inspect(
                &self.root.as_ref().ok_or(OriginalSinkError::Invalid)?.file,
                DIRECTORY,
                self.uid,
                self.gid,
            )?;
            let expected = NAMES
                .iter()
                .zip(&self.sinks)
                .filter_map(|(n, s)| s.as_ref().map(|_| *n))
                .collect::<Vec<_>>();
            inventory(&directory.file, &expected)?;
            for (name, sink) in NAMES.iter().zip(&self.sinks) {
                if let Some(sink) = sink {
                    sink.inspect(&directory.file, name, self.uid, self.gid)?;
                }
            }
        }
        if complete
            && (self.root.is_none()
                || self.directory.is_none()
                || self.sinks.iter().any(Option::is_none))
        {
            return Err(OriginalSinkError::Invalid);
        }
        Ok(())
    }
    fn prepare(
        &mut self,
        mut checkpoint: impl FnMut(SinkPreparationStage) -> Result<(), OriginalSinkError>,
    ) -> Result<(), OriginalSinkError> {
        self.pipes.join_revalidate()?;
        if unsafe { libc::geteuid() } != self.uid || unsafe { libc::getegid() } != self.gid {
            return Err(OriginalSinkError::Privilege);
        }
        let m = self.parent()?.metadata()?;
        if m.uid() != self.uid || !m.is_dir() || m.mode() & 0o7022 != 0 {
            return Err(OriginalSinkError::Invalid);
        }
        let r = &self
            .pipes
            .starting
            .original
            .original
            .prepared
            .original
            .original
            .reservation;
        self.name = output_name(&r.raw);
        for stage in STAGES {
            self.inspect(false)?;
            self.pipes.join_revalidate()?;
            self.inspect(false)?;
            self.effect(stage)?;
            checkpoint(stage)?;
            self.inspect(false)?;
            self.pipes.join_revalidate()?;
            self.inspect(false)?;
        }
        self.inspect(true)?;
        self.acknowledged = true;
        Ok(())
    }
    fn effect(&mut self, at: SinkPreparationStage) -> Result<(), OriginalSinkError> {
        match at {
            SinkPreparationStage::BeforeCreate | SinkPreparationStage::Acknowledging => (),
            SinkPreparationStage::RootCreated => {
                mkdir(self.parent()?, &self.name)?;
                self.root = Some(Object::retain(
                    open(
                        self.parent()?,
                        &self.name,
                        libc::O_RDONLY | libc::O_DIRECTORY,
                    )?,
                    true,
                ));
                self.root.as_mut().unwrap().initialize(self.uid, self.gid)?;
            }
            SinkPreparationStage::RootSynced => {
                self.root.as_ref().unwrap().file.sync_all()?;
                self.parent()?.sync_all()?;
            }
            SinkPreparationStage::DirectoryCreated => {
                let root = &self.root.as_ref().unwrap().file;
                mkdir(root, DIRECTORY)?;
                self.directory = Some(Object::retain(
                    open(root, DIRECTORY, libc::O_RDONLY | libc::O_DIRECTORY)?,
                    true,
                ));
                self.directory
                    .as_mut()
                    .unwrap()
                    .initialize(self.uid, self.gid)?;
            }
            SinkPreparationStage::DirectorySynced => {
                self.directory.as_ref().unwrap().file.sync_all()?;
                self.root.as_ref().unwrap().file.sync_all()?;
            }
            SinkPreparationStage::StdoutCreated | SinkPreparationStage::StderrCreated => {
                let index = usize::from(at == SinkPreparationStage::StderrCreated);
                self.sinks[index] = Some(Object::retain(
                    open(
                        &self.directory.as_ref().unwrap().file,
                        NAMES[index],
                        libc::O_RDWR | libc::O_CREAT | libc::O_EXCL,
                    )?,
                    false,
                ));
                self.sinks[index]
                    .as_mut()
                    .unwrap()
                    .initialize(self.uid, self.gid)?;
            }
            SinkPreparationStage::StdoutSynced | SinkPreparationStage::StderrSynced => {
                let index = usize::from(at == SinkPreparationStage::StderrSynced);
                self.sinks[index].as_ref().unwrap().file.sync_all()?;
                self.directory.as_ref().unwrap().file.sync_all()?;
            }
        }
        Ok(())
    }
    pub fn revalidate(&mut self) -> Result<(), OriginalSinkError> {
        if self.is_refused() || !self.acknowledged {
            self.refuse();
            return Err(OriginalSinkError::Refused);
        }
        let result = (|| {
            self.inspect(true)?;
            self.pipes.revalidate()?;
            self.inspect(true)
        })();
        if result.is_err() {
            self.refuse();
        }
        result
    }
    #[cfg(test)]
    pub(crate) fn corrupt_descriptor_for_test(&mut self, kind: usize) {
        match kind {
            0 => set_flags(&self.sinks[0].as_ref().unwrap().file, libc::F_SETFD, 0).unwrap(),
            1 => set_flags(
                &self.sinks[0].as_ref().unwrap().file,
                libc::F_SETFL,
                libc::O_APPEND | libc::O_NONBLOCK,
            )
            .unwrap(),
            2 => set_flags(&self.directory.as_ref().unwrap().file, libc::F_SETFD, 0).unwrap(),
            3 => self.pipes.corrupt_for_test(1),
            _ => panic!("unknown sink descriptor drift"),
        }
    }
    #[cfg(test)]
    pub(crate) fn drop_eof_for_test(self) {
        use std::io::Read;
        let mut readers = self
            .pipes
            .pipes
            .each_ref()
            .map(|p| p.as_ref().unwrap().reader.try_clone().unwrap());
        let sinks = self
            .sinks
            .each_ref()
            .map(|p| p.as_ref().unwrap().file.as_raw_fd());
        drop(self);
        for fd in sinks {
            assert_eq!(unsafe { libc::fcntl(fd, libc::F_GETFD) }, -1);
            assert_eq!(
                std::io::Error::last_os_error().raw_os_error(),
                Some(libc::EBADF)
            );
        }
        for reader in &mut readers {
            assert_eq!(reader.read(&mut [0; 1]).unwrap(), 0);
        }
    }
}
pub struct OriginalSinkFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    error: OriginalSinkError,
    held: Box<OriginalCollectorSinks<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>,
}
impl OriginalSinkFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn error(&self) -> &OriginalSinkError {
        &self.error
    }
    pub fn is_refused(&self) -> bool {
        self.held.is_refused()
    }
}
impl fmt::Debug for OriginalSinkFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.error, f)
    }
}
impl fmt::Display for OriginalSinkFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.error, f)
    }
}
impl std::error::Error for OriginalSinkFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {}

#[cfg(test)]
#[path = "diagnostic_original_collector_sink_kernel_tests.rs"]
mod tests;
