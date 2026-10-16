//! Original anonymous pipes retained under acknowledged Starting; never execution authority.
use super::*;
use std::fs::File;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{FileTypeExt, MetadataExt};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PipePreparationStage {
    BeforeCreate,
    StdoutCreated,
    StdoutReady,
    StderrCreated,
    StderrReady,
    Acknowledging,
}
#[derive(Debug, thiserror::Error)]
pub enum OriginalPipeError {
    #[error("original Starting refused: {0}")]
    Original(#[from] CollectorStartingError),
    #[error("original pipe identity, flags or ownership refused")]
    Invalid,
    #[error("original pipe preparation already refused")]
    Refused,
    #[error("original pipe I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("private interruption at {0:?}")]
    Interrupted(PipePreparationStage),
}
#[derive(PartialEq, Eq)]
struct Identity {
    dev: u64,
    ino: u64,
    uid: u32,
    gid: u32,
    mode: u32,
    links: u64,
}
impl Identity {
    fn of(m: &std::fs::Metadata) -> Self {
        Self {
            dev: m.dev(),
            ino: m.ino(),
            uid: m.uid(),
            gid: m.gid(),
            mode: m.mode(),
            links: m.nlink(),
        }
    }
}
struct Pair {
    reader: File,
    writer: File,
    identity: Option<Identity>,
}
fn flags(file: &File, command: i32) -> Result<i32, OriginalPipeError> {
    let result = unsafe { libc::fcntl(file.as_raw_fd(), command) };
    if result < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(result)
}
fn set_flags(file: &File, command: i32, value: i32) -> Result<(), OriginalPipeError> {
    if unsafe { libc::fcntl(file.as_raw_fd(), command, value) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}
fn status_valid(status: i32, read: bool) -> bool {
    status
        == if read {
            libc::O_RDONLY | libc::O_NONBLOCK
        } else {
            libc::O_WRONLY
        }
}
impl Pair {
    fn open() -> Result<Self, OriginalPipeError> {
        let mut fds = [-1; 2];
        if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) } != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        // Both descriptors become owned immediately, before any fallible inspection.
        Ok(Self {
            reader: unsafe { File::from_raw_fd(fds[0]) },
            writer: unsafe { File::from_raw_fd(fds[1]) },
            identity: None,
        })
    }
    fn initialize(&mut self) -> Result<(), OriginalPipeError> {
        set_flags(
            &self.reader,
            libc::F_SETFL,
            flags(&self.reader, libc::F_GETFL)? | libc::O_NONBLOCK,
        )?;
        self.identity = Some(Identity::of(&self.reader.metadata()?));
        self.inspect()
    }
    fn inspect(&self) -> Result<(), OriginalPipeError> {
        let expected = self.identity.as_ref().ok_or(OriginalPipeError::Invalid)?;
        for (file, read) in [(&self.reader, true), (&self.writer, false)] {
            let m = file.metadata()?;
            if !m.file_type().is_fifo()
                || m.mode() & 0o7777 != 0o600
                || m.nlink() != 1
                || m.uid() != unsafe { libc::geteuid() }
                || m.gid() != unsafe { libc::getegid() }
                || Identity::of(&m) != *expected
                || flags(file, libc::F_GETFD)? != libc::FD_CLOEXEC
                || !status_valid(flags(file, libc::F_GETFL)?, read)
            {
                return Err(OriginalPipeError::Invalid);
            }
        }
        Ok(())
    }
}
/// Private original pipe objects, not root sink, writer handoff or start authority.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::OriginalCollectorPipes;
/// fn copy(p: OriginalCollectorPipes<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.clone(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::OriginalCollectorPipes;
/// fn fd(p: OriginalCollectorPipes<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { use std::os::fd::AsRawFd; p.as_raw_fd(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::OriginalCollectorPipes;
/// fn stdio(p: OriginalCollectorPipes<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.into_stdio(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::OriginalCollectorPipes;
/// fn start(p: OriginalCollectorPipes<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.start_once(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::OriginalCollectorPipes;
/// fn retry(p: OriginalCollectorPipes<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.prepare_original_pipes(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::OriginalCollectorPipes;
/// fn extract(p: OriginalCollectorPipes<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.into_starting(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::OriginalCollectorPipes;
/// fn decode<T: serde::de::DeserializeOwned>() {}
/// decode::<OriginalCollectorPipes<'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static>>();
/// ```
pub struct OriginalCollectorPipes<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    // Rust drops fields in declaration order: private pipes before the fence owner.
    pipes: [Option<Pair>; 2],
    starting: OriginalCollectorStarting<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    acknowledged: bool,
    refused: bool,
}
impl fmt::Debug for OriginalCollectorPipes<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OriginalCollectorPipes")
            .field("refused", &self.is_refused())
            .finish_non_exhaustive()
    }
}
impl OriginalCollectorPipes<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn is_refused(&self) -> bool {
        self.refused || self.starting.is_refused()
    }
    fn refuse(&mut self) {
        self.refused = true;
        self.starting.original.refuse();
    }
    fn inspect(&self, complete: bool) -> Result<(), OriginalPipeError> {
        for pair in self.pipes.iter().flatten() {
            pair.inspect()?;
        }
        if complete && self.pipes.iter().any(Option::is_none) {
            return Err(OriginalPipeError::Invalid);
        }
        if let [Some(a), Some(b)] = &self.pipes {
            if a.identity == b.identity {
                return Err(OriginalPipeError::Invalid);
            }
        }
        Ok(())
    }
    fn prepare(
        &mut self,
        mut checkpoint: impl FnMut(PipePreparationStage) -> Result<(), OriginalPipeError>,
    ) -> Result<(), OriginalPipeError> {
        self.starting.revalidate()?;
        checkpoint(PipePreparationStage::BeforeCreate)?;
        for index in 0..2 {
            self.pipes[index] = Some(Pair::open()?);
            checkpoint(if index == 0 {
                PipePreparationStage::StdoutCreated
            } else {
                PipePreparationStage::StderrCreated
            })?;
            self.pipes[index].as_mut().unwrap().initialize()?;
            self.inspect(false)?;
            self.starting.revalidate()?;
            self.inspect(false)?;
            checkpoint(if index == 0 {
                PipePreparationStage::StdoutReady
            } else {
                PipePreparationStage::StderrReady
            })?;
        }
        self.inspect(true)?;
        checkpoint(PipePreparationStage::Acknowledging)?;
        self.starting.revalidate()?;
        self.inspect(true)?;
        self.acknowledged = true;
        Ok(())
    }
    fn validate_with(&mut self, before_original: impl FnOnce()) -> Result<(), OriginalPipeError> {
        if self.is_refused() || !self.acknowledged {
            self.refuse();
            return Err(OriginalPipeError::Refused);
        }
        let result = (|| {
            self.inspect(true)?;
            before_original();
            self.starting.revalidate()?;
            self.inspect(true)
        })();
        if result.is_err() {
            self.refuse();
        }
        result
    }
    pub fn revalidate(&mut self) -> Result<(), OriginalPipeError> {
        self.validate_with(|| {})
    }
    #[cfg(test)]
    pub(crate) fn corrupt_for_test(&mut self, kind: usize) {
        match kind {
            0 => set_flags(
                &self.pipes[0].as_ref().unwrap().reader,
                libc::F_SETFL,
                libc::O_RDONLY,
            )
            .unwrap(),
            1 => set_flags(
                &self.pipes[0].as_ref().unwrap().writer,
                libc::F_SETFL,
                libc::O_NONBLOCK,
            )
            .unwrap(),
            2 => set_flags(&self.pipes[0].as_ref().unwrap().reader, libc::F_SETFD, 0).unwrap(),
            3 => set_flags(&self.pipes[0].as_ref().unwrap().writer, libc::F_SETFD, 0).unwrap(),
            4 => self.pipes[0].as_mut().unwrap().reader = File::open("/dev/null").unwrap(),
            5 => self.pipes[0].as_mut().unwrap().writer = Pair::open().unwrap().writer,
            6 => {
                self.pipes[1].as_mut().unwrap().reader =
                    self.pipes[0].as_ref().unwrap().reader.try_clone().unwrap()
            }
            7 => {
                use std::os::unix::fs::PermissionsExt;
                self.pipes[0]
                    .as_ref()
                    .unwrap()
                    .reader
                    .set_permissions(std::fs::Permissions::from_mode(0o400))
                    .unwrap();
            }
            _ => panic!("unknown private pipe fault"),
        }
    }
    #[cfg(test)]
    pub(crate) fn drop_eof_for_test(self) {
        use std::io::Read;
        let mut readers = self
            .pipes
            .each_ref()
            .map(|p| p.as_ref().unwrap().reader.try_clone().unwrap());
        drop(self);
        for reader in &mut readers {
            assert_eq!(reader.read(&mut [0_u8; 1]).unwrap(), 0);
        }
    }
    #[cfg(test)]
    pub(crate) fn concurrent_drift_for_test(&mut self) -> Result<(), OriginalPipeError> {
        let writer = self.pipes[0].as_ref().unwrap().writer.try_clone().unwrap();
        self.validate_with(|| {
            std::thread::spawn(move || set_flags(&writer, libc::F_SETFL, libc::O_NONBLOCK).unwrap())
                .join()
                .unwrap()
        })
    }
}
pub struct OriginalPipeFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    error: OriginalPipeError,
    held: Box<OriginalCollectorPipes<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>,
}
impl OriginalPipeFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn error(&self) -> &OriginalPipeError {
        &self.error
    }
    pub fn is_refused(&self) -> bool {
        self.held.is_refused()
    }
    #[cfg(test)]
    pub(crate) fn held_pipe_count_for_test(&self) -> usize {
        self.held.pipes.iter().flatten().count()
    }
}
impl fmt::Debug for OriginalPipeFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.error, f)
    }
}
impl fmt::Display for OriginalPipeFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.error, f)
    }
}
impl std::error::Error for OriginalPipeFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {}
impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OriginalCollectorStarting<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub fn prepare_original_pipes(
        self,
    ) -> Result<
        OriginalCollectorPipes<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        OriginalPipeFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        self.prepare_pipes_with(|_| Ok(()))
    }
    #[cfg(test)]
    pub(crate) fn prepare_original_pipes_with(
        self,
        checkpoint: impl FnMut(PipePreparationStage) -> Result<(), OriginalPipeError>,
    ) -> Result<
        OriginalCollectorPipes<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        OriginalPipeFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        self.prepare_pipes_with(checkpoint)
    }
    fn prepare_pipes_with(
        self,
        checkpoint: impl FnMut(PipePreparationStage) -> Result<(), OriginalPipeError>,
    ) -> Result<
        OriginalCollectorPipes<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        OriginalPipeFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        let mut held = OriginalCollectorPipes {
            starting: self,
            pipes: [None, None],
            acknowledged: false,
            refused: false,
        };
        match held.prepare(checkpoint) {
            Ok(()) => Ok(held),
            Err(error) => {
                held.refuse();
                Err(OriginalPipeFailure {
                    error,
                    held: Box::new(held),
                })
            }
        }
    }
}
#[cfg(test)]
#[path = "diagnostic_original_collector_pipe_kernel_tests.rs"]
mod tests;
