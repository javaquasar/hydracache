//! Fixed-origin grouped preparation, never a send/start or guardian capability.
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
pub enum OriginalGroupedPrepError {
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
) -> Result<Bounds, OriginalGroupedPrepError> {
    if reserved == 0 || progress < reserved || cell < progress || last < cell || now < last {
        return Err(OriginalGroupedPrepError::Binding);
    }
    let add = |origin: u64, seconds: u64| {
        origin
            .checked_add(seconds * NS)
            .ok_or(OriginalGroupedPrepError::Binding)
    };
    let bounds = Bounds {
        total: add(reserved, TOTAL_SECONDS)?,
        cell: add(cell, CELL_SECONDS)?,
        progress: add(progress, CONTROLLER_LOSS_SECONDS)?,
    };
    if now >= bounds.first() {
        return Err(OriginalGroupedPrepError::Expired);
    }
    Ok(bounds)
}
fn timespec(ns: u64) -> libc::timespec {
    libc::timespec {
        tv_sec: (ns / NS) as libc::time_t,
        tv_nsec: (ns % NS) as libc::c_long,
    }
}
fn nanos(t: libc::timespec) -> Result<u64, OriginalGroupedPrepError> {
    if t.tv_sec < 0 || !(0..NS as i64).contains(&t.tv_nsec) {
        return Err(OriginalGroupedPrepError::Binding);
    }
    (t.tv_sec as u64)
        .checked_mul(NS)
        .and_then(|v| v.checked_add(t.tv_nsec as u64))
        .ok_or(OriginalGroupedPrepError::Binding)
}
fn clock() -> Result<u64, OriginalGroupedPrepError> {
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
    fn arm(deadline: u64) -> Result<Self, OriginalGroupedPrepError> {
        if clock()? >= deadline {
            return Err(OriginalGroupedPrepError::Expired);
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
    fn inspect(&self) -> Result<(), OriginalGroupedPrepError> {
        let fd = self.file.as_raw_fd();
        if unsafe { libc::fcntl(fd, libc::F_GETFD) } != libc::FD_CLOEXEC
            || unsafe { libc::fcntl(fd, libc::F_GETFL) } != libc::O_RDWR | libc::O_NONBLOCK
        {
            return Err(OriginalGroupedPrepError::Binding);
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
            return Err(OriginalGroupedPrepError::Binding);
        }
        let mut raw = Vec::new();
        info.take(4097).read_to_end(&mut raw)?;
        let text = std::str::from_utf8(&raw).map_err(|_| OriginalGroupedPrepError::Binding)?;
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
            return Err(OriginalGroupedPrepError::Binding);
        }
        let before = clock()?;
        let mut spec = unsafe { std::mem::zeroed() };
        if unsafe { libc::timerfd_gettime(fd, &mut spec) } != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        let after = clock()?;
        if after >= self.deadline {
            return Err(OriginalGroupedPrepError::Expired);
        }
        let remaining = nanos(spec.it_value)?;
        if after < before
            || nanos(spec.it_interval)? != 0
            || remaining == 0
            || remaining > self.deadline.saturating_sub(before)
            || remaining < self.deadline - after
        {
            return Err(OriginalGroupedPrepError::Binding);
        }
        Ok(())
    }
}

// No production path, key, callback or fallback input. The fixture branch
// is absent from non-test builds, and is never entered after a fixed refusal.
fn load_bundle(
    policy: &CheckedBuilderPolicy,
    identity: &DiagnosticIdentity,
    #[cfg(test)] fixture: Option<&std::path::Path>,
) -> Result<BundleSnapshot, ArtifactError> {
    #[cfg(test)]
    if let Some(path) = fixture {
        let policy: crate::diagnostic_builder::BuilderPolicy =
            serde_json::from_slice(include_bytes!(
                "../../../docs/testing/performance/0.74/diagnostic-builder-public-policy.json"
            ))
            .unwrap();
        let trust = crate::diagnostic_artifacts::BuildTrust {
            key: crate::diagnostic_builder::decode_key(&policy.builder_key_hex).unwrap(),
            repository_id: policy.repository_id,
            builder_id: policy.builder_id,
        };
        return crate::diagnostic_artifacts::linux::inspect_fixture(
            path,
            unsafe { libc::geteuid() },
            unsafe { libc::getegid() },
            identity,
            &trust,
        );
    }
    inspect_fixed_install(identity, policy)
}

/// Nonexecuting grouped owner. The timer is not an independent watchdog.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::pipe_transport::grouped_preparation::OriginalGroupedPreparation;
/// fn denied(p: OriginalGroupedPreparation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.start_once(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::pipe_transport::grouped_preparation::OriginalGroupedPreparation;
/// fn denied(p: OriginalGroupedPreparation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.send(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::pipe_transport::grouped_preparation::OriginalGroupedPreparation;
/// fn denied(p: OriginalGroupedPreparation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.rearm(10); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::pipe_transport::grouped_preparation::OriginalGroupedPreparation;
/// fn denied(p: OriginalGroupedPreparation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.into_message(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::pipe_transport::grouped_preparation::OriginalGroupedPreparation;
/// fn denied(p: OriginalGroupedPreparation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.clone(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::pipe_transport::grouped_preparation::OriginalGroupedPreparation;
/// fn denied(mut p: OriginalGroupedPreparation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.revalidate_fixture_for_test(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::pipe_transport::grouped_preparation::OriginalGroupedPreparation;
/// fn decode<T: serde::de::DeserializeOwned>() {}
/// decode::<OriginalGroupedPreparation<'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static>>();
/// ```
pub struct OriginalGroupedPreparation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    alarm: Option<Alarm>,
    bounds: Option<Bounds>,
    last: Option<KernelTimePoint>,
    transport: OriginalCollectorPipeTransport<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
}
impl OriginalGroupedPreparation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn is_refused(&self) -> bool {
        self.transport.is_refused()
    }
    fn original_round(&mut self, allow_fixture: bool) -> Result<Bounds, OriginalGroupedPrepError> {
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
            .ok_or(OriginalGroupedPrepError::Binding)?;
        let progress = &mut prepared.original.original;
        let state = material
            .starting
            .as_ref()
            .ok_or(OriginalGroupedPrepError::Binding)?;
        crate::diagnostic_lease::validate_transition(&progress.reservation.state, state)
            .map_err(|_| OriginalGroupedPrepError::Binding)?;
        if state.stage != DiagnosticStage::Starting
            || state.completed_cells != 0
            || state.revision != 2
        {
            return Err(OriginalGroupedPrepError::Binding);
        }
        let received = progress.packet.received_time().clone();
        let now = progress.packet.original_reservation_time(Some(previous))?;
        if received.boot_id() != state.identity.boot_id || now.boot_id() != state.identity.boot_id {
            return Err(OriginalGroupedPrepError::Binding);
        }
        let bounds = boundaries(
            state.reserved_monotonic_ns,
            state
                .cell_started_monotonic_ns
                .ok_or(OriginalGroupedPrepError::Binding)?,
            received.boottime_ns(),
            state.last_observed_monotonic_ns,
            now.boottime_ns(),
        )?;
        let spec = worker(&progress.reservation.state, &material.projection)?;
        let [Some(stdout), Some(stderr)] = &material.outputs.pipes.pipes else {
            return Err(OriginalGroupedPrepError::Binding);
        };
        inspect_message(
            self.transport
                .message
                .as_ref()
                .ok_or(OriginalGroupedPrepError::Binding)?,
            &spec,
            &stdout.writer,
            &stderr.writer,
        )?;
        self.last = Some(now);
        Ok(bounds)
    }
    fn validate(&mut self, allow_fixture: bool) -> Result<(), OriginalGroupedPrepError> {
        if self.is_refused() || !self.transport.material.ready {
            self.transport.material.outputs.refuse();
            return Err(OriginalGroupedPrepError::Refused);
        }
        let result = (|| {
            let bounds = self.original_round(allow_fixture)?;
            if self.bounds != Some(bounds) {
                return Err(OriginalGroupedPrepError::Binding);
            }
            let alarm = self
                .alarm
                .as_ref()
                .ok_or(OriginalGroupedPrepError::Binding)?;
            if alarm.deadline != bounds.first() {
                return Err(OriginalGroupedPrepError::Binding);
            }
            alarm.inspect()?;
            if self.original_round(allow_fixture)? != bounds {
                return Err(OriginalGroupedPrepError::Binding);
            }
            self.alarm
                .as_ref()
                .ok_or(OriginalGroupedPrepError::Binding)?
                .inspect()
        })();
        if result.is_err() {
            self.transport.material.outputs.refuse();
        }
        result
    }
    /// Reobserve the fixed installed origin; never sends or renews a deadline.
    pub fn revalidate(&mut self) -> Result<(), OriginalGroupedPrepError> {
        self.validate(false)
    }
    #[cfg(test)]
    pub(crate) fn revalidate_fixture_for_test(&mut self) -> Result<(), OriginalGroupedPrepError> {
        self.validate(true)
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
impl fmt::Debug for OriginalGroupedPreparation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OriginalGroupedPreparation")
            .field("refused", &self.is_refused())
            .finish_non_exhaustive()
    }
}
pub struct OriginalGroupedPreparationFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    error: OriginalGroupedPrepError,
    held: Box<OriginalGroupedPreparation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>,
}
impl OriginalGroupedPreparationFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn error(&self) -> &OriginalGroupedPrepError {
        &self.error
    }
    pub fn is_refused(&self) -> bool {
        self.held.is_refused()
    }
}
impl fmt::Debug for OriginalGroupedPreparationFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.error, f)
    }
}
impl fmt::Display for OriginalGroupedPreparationFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.error, f)
    }
}
impl std::error::Error
    for OriginalGroupedPreparationFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>
{
}
impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OriginalCollectorSinks<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    /// Consume acknowledged original sinks using only the fixed trusted install.
    /// This prepares private material, message and timer; it never starts a worker.
    /// ```compile_fail
    /// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::OriginalCollectorSinks;
    /// fn denied(p: OriginalCollectorSinks<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.prepare_fixed_grouped_fixture_for_test(std::path::Path::new("/tmp/fixture"), |_, _| {}); }
    /// ```
    pub fn prepare_original_grouped(
        self,
    ) -> Result<
        OriginalGroupedPreparation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        OriginalGroupedPreparationFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        self.prepare_grouped_with(
            #[cfg(test)]
            None,
            #[cfg(test)]
            &mut |_, _| {},
        )
    }
    #[cfg(test)]
    pub(crate) fn prepare_fixed_grouped_fixture_for_test(
        self,
        path: &std::path::Path,
        mut checkpoint: impl FnMut(
            &'static str,
            &mut OriginalGroupedPreparation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        ),
    ) -> Result<
        OriginalGroupedPreparation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        OriginalGroupedPreparationFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        assert_ne!(unsafe { libc::geteuid() }, 0);
        assert_ne!(unsafe { libc::getegid() }, 0);
        self.prepare_grouped_with(Some(path), &mut checkpoint)
    }
    fn prepare_grouped_with(
        self,
        #[cfg(test)] fixture: Option<&std::path::Path>,
        #[cfg(test)] checkpoint: &mut dyn FnMut(
            &'static str,
            &mut OriginalGroupedPreparation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        ),
    ) -> Result<
        OriginalGroupedPreparation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        OriginalGroupedPreparationFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        #[cfg(test)]
        let allow_fixture = fixture.is_some();
        #[cfg(not(test))]
        let allow_fixture = false;
        let mut held = OriginalGroupedPreparation {
            alarm: None,
            bounds: None,
            last: None,
            transport: OriginalCollectorPipeTransport {
                message: None,
                material: OriginalCollectorStartMaterial {
                    bundle: None,
                    projection: Vec::new(),
                    starting: None,
                    ready: false,
                    outputs: self,
                },
            },
        };
        let result = (|| {
            held.transport
                .material
                .outputs
                .material_round()
                .map_err(OriginalStartMaterialError::from)
                .map_err(OriginalPipeTransportError::from)?;
            #[cfg(test)]
            checkpoint("opened", &mut held);
            let material = &mut held.transport.material;
            let prepared = &material.outputs.pipes.starting.original.original.prepared;
            let r = &prepared.original.original.reservation;
            let body = prepared.consent_to_sign();
            material.projection = requirements(
                &r.state,
                &body.collector_unit_sha256,
                &body.reserved_state_sha256,
            )
            .map_err(OriginalPipeTransportError::from)?;
            material.starting = Some(body.starting_state.clone());
            material.bundle = Some(
                load_bundle(
                    &r.original.original.policy,
                    r.original.original.build.identity(),
                    #[cfg(test)]
                    fixture,
                )
                .map_err(OriginalStartMaterialError::from)
                .map_err(OriginalPipeTransportError::from)?,
            );
            #[cfg(test)]
            checkpoint("loaded", &mut held);
            held.transport
                .material
                .bind(allow_fixture)
                .map_err(OriginalPipeTransportError::from)?;
            #[cfg(test)]
            checkpoint("bound", &mut held);
            let material = &held.transport.material;
            let r = &material
                .outputs
                .pipes
                .starting
                .original
                .original
                .prepared
                .original
                .original
                .reservation;
            let spec = worker(&r.state, &material.projection)?;
            let [Some(stdout), Some(stderr)] = &material.outputs.pipes.pipes else {
                return Err(OriginalGroupedPrepError::Binding);
            };
            held.transport.message = Some(message(&spec, &stdout.writer, &stderr.writer)?);
            inspect_message(
                held.transport.message.as_ref().unwrap(),
                &spec,
                &stdout.writer,
                &stderr.writer,
            )?;
            #[cfg(test)]
            checkpoint("message", &mut held);

            // Provisional timer only. The unchanged original_round below must
            // independently confirm every binding and these original bounds.
            let material = &mut held.transport.material;
            let prepared = &mut material.outputs.pipes.starting.original.original.prepared;
            let previous = prepared
                .point
                .as_ref()
                .ok_or(OriginalGroupedPrepError::Binding)?;
            let progress = &mut prepared.original.original;
            let state = material
                .starting
                .as_ref()
                .ok_or(OriginalGroupedPrepError::Binding)?;
            let now = progress.packet.original_reservation_time(Some(previous))?;
            let bounds = boundaries(
                state.reserved_monotonic_ns,
                state
                    .cell_started_monotonic_ns
                    .ok_or(OriginalGroupedPrepError::Binding)?,
                progress.packet.received_time().boottime_ns(),
                state.last_observed_monotonic_ns,
                now.boottime_ns(),
            )?;
            held.last = Some(now);
            held.bounds = Some(bounds);
            held.alarm = Some(Alarm::arm(bounds.first())?);
            #[cfg(test)]
            checkpoint("armed", &mut held);
            // No caller callback occurs after this complete closing observation.
            if held.original_round(allow_fixture)? != bounds {
                return Err(OriginalGroupedPrepError::Binding);
            }
            let alarm = held
                .alarm
                .as_ref()
                .ok_or(OriginalGroupedPrepError::Binding)?;
            if alarm.deadline != bounds.first() {
                return Err(OriginalGroupedPrepError::Binding);
            }
            alarm.inspect()?;
            held.transport.material.ready = true;
            Ok(())
        })();
        match result {
            Ok(()) => Ok(held),
            Err(error) => {
                held.transport.material.outputs.refuse();
                Err(OriginalGroupedPreparationFailure {
                    error,
                    held: Box::new(held),
                })
            }
        }
    }
}

#[cfg(test)]
#[path = "diagnostic_fixed_grouped_preparation_tests.rs"]
mod tests;

#[cfg(test)]
impl OriginalGroupedPreparation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn tamper_fixed_grouped_for_test(&mut self, kind: &str) {
        match kind {
            "message" => self.transport.message = None,
            "projection" => self.transport.material.projection.push(b' '),
            "state" => self.transport.material.starting.as_mut().unwrap().revision += 1,
            "writer" => self
                .transport
                .material
                .outputs
                .corrupt_descriptor_for_test(3),
            "alarm" => tests::set(self.alarm.as_ref().unwrap(), 0, 0, libc::TFD_TIMER_ABSTIME),
            _ => panic!("unknown grouped fault"),
        }
    }
}
#[cfg(test)]
impl OriginalGroupedPreparationFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn retry_fixed_grouped_for_test(&mut self) -> Result<(), OriginalGroupedPrepError> {
        self.held.revalidate_fixture_for_test()
    }
}
