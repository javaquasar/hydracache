//! Rejected composition retained only under cfg(test) for cost and kernel probes.
//! No guardian, writer revocation, production API or start authority.
use super::*;
use crate::diagnostic_kernel_admission::{KernelError, KernelTimePoint};
use crate::diagnostic_lease::{
    DiagnosticStage, CELL_SECONDS, CONTROLLER_LOSS_SECONDS, TOTAL_SECONDS,
};
use std::io::Read;
use std::os::fd::FromRawFd;
use std::os::unix::fs::OpenOptionsExt;

const NS: u64 = 1_000_000_000;
#[derive(Debug, thiserror::Error)]
pub enum OriginalSafetyAlarmError {
    #[error("original pipe message refused: {0}")]
    Transport(#[from] OriginalPipeTransportError),
    #[error("original kernel time refused: {0}")]
    Kernel(#[from] KernelError),
    #[error("original alarm binding or kernel timer differs")]
    Binding,
    #[error("an original absolute safety deadline has expired")]
    Expired,
    #[error("original alarm owner previously refused")]
    Refused,
    #[error("kernel alarm I/O failed: {0}")]
    Io(#[from] std::io::Error),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Bounds {
    total: u64,
    cell: u64,
    progress: u64,
}
impl Bounds {
    fn first(self) -> u64 {
        self.total.min(self.cell).min(self.progress)
    }
}
fn boundaries(
    reserved: u64,
    cell: u64,
    progress: u64,
    last: u64,
    now: u64,
) -> Result<Bounds, OriginalSafetyAlarmError> {
    if reserved == 0 || progress < reserved || cell < progress || last < cell || now < last {
        return Err(OriginalSafetyAlarmError::Binding);
    }
    let add = |origin: u64, seconds: u64| {
        origin
            .checked_add(seconds * NS)
            .ok_or(OriginalSafetyAlarmError::Binding)
    };
    let bounds = Bounds {
        total: add(reserved, TOTAL_SECONDS)?,
        cell: add(cell, CELL_SECONDS)?,
        progress: add(progress, CONTROLLER_LOSS_SECONDS)?,
    };
    if now >= bounds.first() {
        return Err(OriginalSafetyAlarmError::Expired);
    }
    Ok(bounds)
}
fn timespec(ns: u64) -> libc::timespec {
    libc::timespec {
        tv_sec: (ns / NS) as libc::time_t,
        tv_nsec: (ns % NS) as libc::c_long,
    }
}
fn nanos(t: libc::timespec) -> Result<u64, OriginalSafetyAlarmError> {
    if t.tv_sec < 0 || !(0..NS as i64).contains(&t.tv_nsec) {
        return Err(OriginalSafetyAlarmError::Binding);
    }
    (t.tv_sec as u64)
        .checked_mul(NS)
        .and_then(|v| v.checked_add(t.tv_nsec as u64))
        .ok_or(OriginalSafetyAlarmError::Binding)
}
fn clock() -> Result<u64, OriginalSafetyAlarmError> {
    let mut t = unsafe { std::mem::zeroed() };
    if unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut t) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    nanos(t)
}
struct Alarm {
    file: File,
    deadline: u64,
}
impl Alarm {
    fn arm(deadline: u64) -> Result<Self, OriginalSafetyAlarmError> {
        if clock()? >= deadline {
            return Err(OriginalSafetyAlarmError::Expired);
        }
        let fd = unsafe {
            libc::timerfd_create(libc::CLOCK_BOOTTIME, libc::TFD_CLOEXEC | libc::TFD_NONBLOCK)
        };
        if fd < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        let alarm = Self {
            file: unsafe { File::from_raw_fd(fd) },
            deadline,
        };
        let spec = libc::itimerspec {
            it_interval: timespec(0),
            it_value: timespec(deadline),
        };
        if unsafe {
            libc::timerfd_settime(
                alarm.file.as_raw_fd(),
                libc::TFD_TIMER_ABSTIME,
                &spec,
                std::ptr::null_mut(),
            )
        } != 0
        {
            return Err(std::io::Error::last_os_error().into());
        }
        alarm.inspect()?;
        Ok(alarm)
    }
    fn inspect(&self) -> Result<(), OriginalSafetyAlarmError> {
        let fd = self.file.as_raw_fd();
        if unsafe { libc::fcntl(fd, libc::F_GETFD) } != libc::FD_CLOEXEC
            || unsafe { libc::fcntl(fd, libc::F_GETFL) } != libc::O_RDWR | libc::O_NONBLOCK
        {
            return Err(OriginalSafetyAlarmError::Binding);
        }
        // Bounded current-thread kernel metadata, bracketed by the original
        // receiver/origin rounds. It is never an authority or caller path.
        let info = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
            .open(format!("/proc/thread-self/fdinfo/{fd}"))?;
        let mut fs: libc::statfs = unsafe { std::mem::zeroed() };
        if unsafe { libc::fstatfs(info.as_raw_fd(), &mut fs) } != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        if fs.f_type != libc::PROC_SUPER_MAGIC as libc::c_long {
            return Err(OriginalSafetyAlarmError::Binding);
        }
        let mut raw = Vec::new();
        info.take(4097).read_to_end(&mut raw)?;
        let text = std::str::from_utf8(&raw).map_err(|_| OriginalSafetyAlarmError::Binding)?;
        let clocks: Vec<_> = text
            .lines()
            .filter_map(|l| l.strip_prefix("clockid:"))
            .collect();
        let flags: Vec<_> = text
            .lines()
            .filter_map(|l| l.strip_prefix("settime flags:"))
            .collect();
        if raw.len() > 4096
            || clocks.len() != 1
            || clocks[0].trim() != "7"
            || flags.len() != 1
            || flags[0].trim() != "01"
        {
            return Err(OriginalSafetyAlarmError::Binding);
        }
        let before = clock()?;
        let mut spec = unsafe { std::mem::zeroed() };
        if unsafe { libc::timerfd_gettime(fd, &mut spec) } != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        let after = clock()?;
        if after >= self.deadline {
            return Err(OriginalSafetyAlarmError::Expired);
        }
        let remaining = nanos(spec.it_value)?;
        if after < before
            || nanos(spec.it_interval)? != 0
            || remaining == 0
            || remaining > self.deadline.saturating_sub(before)
            || remaining < self.deadline - after
        {
            return Err(OriginalSafetyAlarmError::Binding);
        }
        Ok(())
    }
}

/// Retained one-shot alarm, not an independently running watchdog.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::pipe_transport::safety_alarm::OriginalCollectorSafetyAlarm;
/// fn start(p: OriginalCollectorSafetyAlarm<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.start_once(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::pipe_transport::safety_alarm::OriginalCollectorSafetyAlarm;
/// fn rearm(p: OriginalCollectorSafetyAlarm<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.rearm(10); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::pipe_transport::safety_alarm::OriginalCollectorSafetyAlarm;
/// fn extract(p: OriginalCollectorSafetyAlarm<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.into_fd(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::pipe_transport::safety_alarm::OriginalCollectorSafetyAlarm;
/// fn copy(p: OriginalCollectorSafetyAlarm<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.clone(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::pipe_transport::safety_alarm::OriginalCollectorSafetyAlarm;
/// fn decode<T: serde::de::DeserializeOwned>() {}
/// decode::<OriginalCollectorSafetyAlarm<'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static>>();
/// ```
pub struct OriginalCollectorSafetyAlarm<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    alarm: Option<Alarm>,
    bounds: Option<Bounds>,
    last: Option<KernelTimePoint>,
    transport: OriginalCollectorPipeTransport<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
}
impl OriginalCollectorSafetyAlarm<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn is_refused(&self) -> bool {
        self.transport.is_refused()
    }
    fn original_round(&mut self, allow_fixture: bool) -> Result<Bounds, OriginalSafetyAlarmError> {
        let material = &mut self.transport.material;
        material
            .outputs
            .material_round()
            .map_err(OriginalStartMaterialError::from)
            .map_err(OriginalPipeTransportError::from)?;
        material
            .bind(allow_fixture)
            .map_err(OriginalPipeTransportError::from)?;
        let prepared = &mut material.outputs.pipes.starting.original.original.prepared;
        let previous = self
            .last
            .as_ref()
            .or(prepared.point.as_ref())
            .ok_or(OriginalSafetyAlarmError::Binding)?;
        let progress = &mut prepared.original.original;
        let state = material
            .starting
            .as_ref()
            .ok_or(OriginalSafetyAlarmError::Binding)?;
        crate::diagnostic_lease::validate_transition(&progress.reservation.state, state)
            .map_err(|_| OriginalSafetyAlarmError::Binding)?;
        if state.stage != DiagnosticStage::Starting
            || state.completed_cells != 0
            || state.revision != 2
        {
            return Err(OriginalSafetyAlarmError::Binding);
        }
        let received = progress.packet.received_time().clone();
        let now = progress.packet.original_reservation_time(Some(previous))?;
        if received.boot_id() != state.identity.boot_id || now.boot_id() != state.identity.boot_id {
            return Err(OriginalSafetyAlarmError::Binding);
        }
        let bounds = boundaries(
            state.reserved_monotonic_ns,
            state
                .cell_started_monotonic_ns
                .ok_or(OriginalSafetyAlarmError::Binding)?,
            received.boottime_ns(),
            state.last_observed_monotonic_ns,
            now.boottime_ns(),
        )?;
        let spec = worker(&progress.reservation.state, &material.projection)?;
        let [Some(stdout), Some(stderr)] = &material.outputs.pipes.pipes else {
            return Err(OriginalSafetyAlarmError::Binding);
        };
        inspect_message(
            self.transport
                .message
                .as_ref()
                .ok_or(OriginalSafetyAlarmError::Binding)?,
            &spec,
            &stdout.writer,
            &stderr.writer,
        )?;
        self.last = Some(now);
        Ok(bounds)
    }
    fn validate(
        &mut self,
        allow_fixture: bool,
        construct: bool,
    ) -> Result<(), OriginalSafetyAlarmError> {
        if self.is_refused() || !self.transport.material.ready {
            self.transport.material.outputs.refuse();
            return Err(OriginalSafetyAlarmError::Refused);
        }
        let result = (|| {
            let bounds = self.original_round(allow_fixture)?;
            if construct {
                self.bounds = Some(bounds);
                self.alarm = Some(Alarm::arm(bounds.first())?);
            }
            if self.bounds != Some(bounds) {
                return Err(OriginalSafetyAlarmError::Binding);
            }
            let alarm = self
                .alarm
                .as_ref()
                .ok_or(OriginalSafetyAlarmError::Binding)?;
            if alarm.deadline != bounds.first() {
                return Err(OriginalSafetyAlarmError::Binding);
            }
            alarm.inspect()?;
            if self.original_round(allow_fixture)? != bounds {
                return Err(OriginalSafetyAlarmError::Binding);
            }
            self.alarm
                .as_ref()
                .ok_or(OriginalSafetyAlarmError::Binding)?
                .inspect()
        })();
        if result.is_err() {
            self.transport.material.outputs.refuse();
        }
        result
    }
    #[cfg(test)]
    pub(crate) fn revalidate_fixture_for_test(&mut self) -> Result<(), OriginalSafetyAlarmError> {
        self.validate(true, false)
    }
    #[cfg(test)]
    pub(crate) fn offsets_for_test(&self) -> [u64; 3] {
        let state = self.transport.material.starting.as_ref().unwrap();
        let bounds = self.bounds.unwrap();
        [
            bounds.total - state.reserved_monotonic_ns,
            bounds.cell - state.cell_started_monotonic_ns.unwrap(),
            bounds.progress
                - self
                    .transport
                    .material
                    .outputs
                    .pipes
                    .starting
                    .original
                    .original
                    .prepared
                    .original
                    .original
                    .packet
                    .received_time()
                    .boottime_ns(),
        ]
    }
    #[cfg(test)]
    pub(crate) fn origins_for_test(&self) -> [u64; 6] {
        let prepared = &self
            .transport
            .material
            .outputs
            .pipes
            .starting
            .original
            .original
            .prepared;
        let progress = &prepared.original.original;
        [
            progress.reservation.state.reserved_monotonic_ns,
            prepared.original.issued.boottime_ns(),
            progress.packet.received_time().boottime_ns(),
            self.transport
                .material
                .starting
                .as_ref()
                .unwrap()
                .cell_started_monotonic_ns
                .unwrap(),
            self.last.as_ref().unwrap().boottime_ns(),
            clock().unwrap(),
        ]
    }
}
impl fmt::Debug for OriginalCollectorSafetyAlarm<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OriginalCollectorSafetyAlarm")
            .field("refused", &self.is_refused())
            .finish_non_exhaustive()
    }
}
pub struct OriginalSafetyAlarmFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    error: OriginalSafetyAlarmError,
    held: Box<OriginalCollectorSafetyAlarm<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>,
}
impl OriginalSafetyAlarmFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn error(&self) -> &OriginalSafetyAlarmError {
        &self.error
    }
    pub fn is_refused(&self) -> bool {
        self.held.is_refused()
    }
}
impl fmt::Debug for OriginalSafetyAlarmFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.error, f)
    }
}
impl fmt::Display for OriginalSafetyAlarmFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.error, f)
    }
}
impl std::error::Error for OriginalSafetyAlarmFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {}
impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OriginalCollectorPipeTransport<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    #[cfg(test)]
    pub(crate) fn prepare_original_safety_alarm_for_test(
        self,
    ) -> Result<
        OriginalCollectorSafetyAlarm<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        OriginalSafetyAlarmFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        self.prepare_alarm_with(true)
    }
    fn prepare_alarm_with(
        self,
        allow_fixture: bool,
    ) -> Result<
        OriginalCollectorSafetyAlarm<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        OriginalSafetyAlarmFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        let mut held = OriginalCollectorSafetyAlarm {
            alarm: None,
            bounds: None,
            last: None,
            transport: self,
        };
        match held.validate(allow_fixture, true) {
            Ok(()) => Ok(held),
            Err(error) => Err(OriginalSafetyAlarmFailure {
                error,
                held: Box::new(held),
            }),
        }
    }
}

#[cfg(test)]
#[path = "diagnostic_original_safety_alarm_tests.rs"]
mod tests;
