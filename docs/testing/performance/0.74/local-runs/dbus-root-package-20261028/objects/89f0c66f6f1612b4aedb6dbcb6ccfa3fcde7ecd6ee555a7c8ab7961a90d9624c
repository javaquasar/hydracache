//! Point-in-time original pidfd exit data, never outcome or cleanup authority.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalState {
    /// Readable exit notification; some kernels retain it after owner reaping.
    Exited,
    /// A hangup notification was observed; not required on every kernel.
    Reaped,
}
/// Kernel readiness for the original retained process. No exit code, successful
/// cell outcome, original-start authentication or writer/cgroup release proof.
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_process::TerminalProcessObservation;
/// fn release(view: TerminalProcessObservation) { view.release_resources(); }
/// ```
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_process::TerminalProcessObservation;
/// fn complete(view: TerminalProcessObservation) { view.complete_cell(); }
/// ```
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_process::{TerminalProcessObservation, TerminalState};
/// fn forge() { let _ = TerminalProcessObservation { state: TerminalState::Reaped }; }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalProcessObservation {
    generation: GenerationObservation,
    state: TerminalState,
}
impl TerminalProcessObservation {
    pub fn generation(&self) -> &GenerationObservation {
        &self.generation
    }
    pub fn state(&self) -> TerminalState {
        self.state
    }
}
trait TerminalProbe {
    fn origin(&self) -> Result<(), ProcessError>;
    fn poll(&self) -> Result<(i32, i16), ProcessError>;
    fn live_generation(&self) -> Result<(), ProcessError>;
}
fn classify(ready: i32, events: i16) -> Result<TerminalState, ProcessError> {
    if ready != 1 {
        return Err(ProcessError::Invalid);
    }
    if events == libc::POLLIN {
        Ok(TerminalState::Exited)
    } else if events == libc::POLLHUP || events == libc::POLLIN | libc::POLLHUP {
        Ok(TerminalState::Reaped)
    } else {
        // NotLive also includes errors in the old live-admission reader. It is
        // deliberately never used to establish terminal readiness here.
        Err(ProcessError::Invalid)
    }
}
fn observe(probe: &impl TerminalProbe) -> Result<Option<TerminalState>, ProcessError> {
    probe.origin()?;
    let (ready, events) = probe.poll()?;
    let state = if ready == 0 && events == 0 {
        probe.live_generation()?;
        None
    } else {
        Some(classify(ready, events)?)
    };
    probe.origin()?;
    Ok(state)
}
struct OriginalProbe<'a>(&'a ProcessRead);
impl TerminalProbe for OriginalProbe<'_> {
    fn origin(&self) -> Result<(), ProcessError> {
        let root = &self.0.probe.files.root;
        require_procfs(root)?;
        let named = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open("/proc")?;
        require_procfs(&named)?;
        if FileId::of(&named.metadata()?) != FileId::of(&root.metadata()?)
            || read_boot(root)? != self.0.observation.expected.boot_id
        {
            return Err(ProcessError::Drift);
        }
        Ok(())
    }
    fn poll(&self) -> Result<(i32, i16), ProcessError> {
        let mut descriptor = libc::pollfd {
            fd: self.0.probe.pidfd.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: one valid retained process pidfd, initialized pollfd, no wait.
        let ready = unsafe { libc::poll(&mut descriptor, 1, 0) };
        if ready < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok((ready, descriptor.revents))
    }
    fn live_generation(&self) -> Result<(), ProcessError> {
        self.0.revalidate()
    }
}
impl ProcessRead {
    /// Observes only the original pidfd under procfs/boot checks. None is a
    /// point-in-time live observation; Some is exited/reaped, NOT successful
    /// execution, descendant writer cessation or resource-release authority.
    /// Does not reap, signal, refresh a PID, export a descriptor or mutate state.
    pub fn observe_terminal(&self) -> Result<Option<TerminalProcessObservation>, ProcessError> {
        Ok(
            observe(&OriginalProbe(self))?.map(|state| TerminalProcessObservation {
                generation: self.observation.clone(),
                state,
            }),
        )
    }
}
#[cfg(test)]
#[path = "diagnostic_process_terminal_tests.rs"]
mod tests;
