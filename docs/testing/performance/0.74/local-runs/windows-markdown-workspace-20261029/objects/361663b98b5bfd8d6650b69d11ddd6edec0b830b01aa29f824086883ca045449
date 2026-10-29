//! Same original sinks and fence through unchanged full validation frames.
#[allow(dead_code)]
#[path = "diagnostic_original_descriptor_plan.rs"]
mod descriptor_plan;
use super::*;
use crate::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::{self, CollectorCausalError};
use crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::attempt_storage::{
    collector_reservation_storage::starting_storage::custody_storage::OriginalCollectorFenceCustody,
    StorageAttemptError,
};

#[derive(Debug, thiserror::Error)]
pub enum OriginalCustodyError {
    #[error("original grouped validation refused: {0}")]
    Original(#[from] OriginalGroupedPrepError),
    #[error("original collector custody storage refused: {0}")]
    Storage(#[from] StorageAttemptError),
    #[error("original challenge freshness refused: {0}")]
    Challenge(#[from] CollectorCausalError),
    #[error("original collector custody is absent")]
    Binding,
    #[error("original custodied preparation already refused")]
    Refused,
}

impl From<OriginalPipeTransportError> for OriginalCustodyError {
    fn from(error: OriginalPipeTransportError) -> Self {
        Self::Original(error.into())
    }
}
impl From<KernelError> for OriginalCustodyError {
    fn from(error: KernelError) -> Self {
        Self::Original(error.into())
    }
}

/// Nonexecuting custody joined to the same original sinks and full live frames.
/// No File, message, worker, completion or recovery capability can be extracted.
/// ```
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::pipe_transport::grouped_preparation::custody::OriginalCustodiedPreparation;
/// fn inspect(mut p: OriginalCustodiedPreparation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) -> bool { let _ = p.revalidate(); p.is_refused() }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::pipe_transport::grouped_preparation::custody::OriginalCustodiedPreparation;
/// fn denied(p: OriginalCustodiedPreparation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.start_once(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::pipe_transport::grouped_preparation::custody::OriginalCustodiedPreparation;
/// fn denied(p: OriginalCustodiedPreparation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.send(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::pipe_transport::grouped_preparation::custody::OriginalCustodiedPreparation;
/// fn denied(p: OriginalCustodiedPreparation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.clone(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::pipe_transport::grouped_preparation::custody::OriginalCustodiedPreparation;
/// fn denied(p: OriginalCustodiedPreparation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.into_stdio(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::pipe_transport::grouped_preparation::custody::OriginalCustodiedPreparation;
/// fn denied(p: OriginalCustodiedPreparation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.into_message(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::OriginalCollectorSinks;
/// fn denied(mut p: OriginalCollectorSinks<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { let held = p.prepare_original_custodied(); p.revalidate(); drop(held); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::pipe_transport::grouped_preparation::custody::OriginalCustodiedPreparation;
/// fn decode<T: serde::de::DeserializeOwned>() {}
/// decode::<OriginalCustodiedPreparation<'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static>>();
/// ```
pub struct OriginalCustodiedPreparation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    // Original message/sink/pipe writers close before the last custody alias.
    original: OriginalGroupedPreparation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    custody: Option<OriginalCollectorFenceCustody>,
    ready: bool,
    #[cfg(test)]
    final_delay: bool,
}
impl OriginalCustodiedPreparation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn is_refused(&self) -> bool {
        self.original.is_refused()
    }
    fn refuse(&mut self) {
        self.original.transport.material.outputs.refuse();
    }
    fn inspect_custody(&self) -> Result<(), OriginalCustodyError> {
        self.custody
            .as_ref()
            .ok_or(OriginalCustodyError::Binding)?
            .inspect(self.original.transport.material.outputs.reservation())?;
        Ok(())
    }
    fn inspect_deadline(&self) -> Result<(), OriginalCustodyError> {
        #[cfg(test)]
        if self.final_delay {
            let bound = self
                .original
                .bounds
                .ok_or(OriginalGroupedPrepError::Binding)?
                .first();
            while clock()? < bound {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
        self.original
            .alarm
            .as_ref()
            .ok_or(OriginalGroupedPrepError::Binding)?
            .inspect()?;
        let prepared = &self
            .original
            .transport
            .material
            .outputs
            .pipes
            .starting
            .original
            .original
            .prepared;
        causal::age(prepared.original.issued.boottime_ns(), clock()?)?;
        Ok(())
    }
    fn validate(&mut self, allow_fixture: bool) -> Result<(), OriginalCustodyError> {
        if !self.ready || self.is_refused() {
            self.refuse();
            return Err(OriginalCustodyError::Refused);
        }
        let result = (|| {
            self.inspect_custody()?;
            self.original.validate(allow_fixture)?;
            self.inspect_custody()?;
            self.inspect_deadline()
        })();
        if result.is_err() {
            self.refuse();
        }
        result
    }
    pub fn revalidate(&mut self) -> Result<(), OriginalCustodyError> {
        self.validate(false)
    }
    #[cfg(test)]
    pub(crate) fn revalidate_fixture_for_test(&mut self) -> Result<(), OriginalCustodyError> {
        self.validate(true)
    }
    #[cfg(test)]
    pub(crate) fn tamper_for_test(&mut self, kind: &str) {
        match kind {
            "final-expiry" => self.final_delay = true,
            "custody-missing" => self.custody = None,
            "custody-object" | "custody-flags" => self
                .custody
                .as_mut()
                .unwrap()
                .tamper_for_test(kind == "custody-object"),
            other => self.original.tamper_fixed_grouped_for_test(other),
        }
    }
    #[cfg(test)]
    pub(crate) fn offsets_for_test(&self) -> [u64; 3] {
        self.original.offsets_for_test()
    }
    #[cfg(test)]
    pub(crate) fn origins_for_test(&self) -> [u64; 6] {
        self.original.origins_for_test()
    }
    #[cfg(test)]
    pub(crate) fn drop_eof_for_test(self) {
        use std::io::Read;
        let outputs = &self.original.transport.material.outputs;
        let mut readers = outputs
            .pipes
            .pipes
            .each_ref()
            .map(|p| p.as_ref().unwrap().reader.try_clone().unwrap());
        let sink_fds = outputs
            .sinks
            .each_ref()
            .map(|s| s.as_ref().unwrap().file.as_raw_fd());
        drop(self);
        for fd in sink_fds {
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
impl fmt::Debug for OriginalCustodiedPreparation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OriginalCustodiedPreparation")
            .field("refused", &self.is_refused())
            .finish_non_exhaustive()
    }
}
pub struct OriginalCustodyFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    error: OriginalCustodyError,
    held: Box<OriginalCustodiedPreparation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>,
}
impl OriginalCustodyFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn error(&self) -> &OriginalCustodyError {
        &self.error
    }
    pub fn is_refused(&self) -> bool {
        self.held.is_refused()
    }
    #[cfg(test)]
    pub(crate) fn retry_for_test(&mut self) -> Result<(), OriginalCustodyError> {
        self.held.revalidate_fixture_for_test()
    }
}
impl fmt::Debug for OriginalCustodyFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.error, f)
    }
}
impl fmt::Display for OriginalCustodyFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.error, f)
    }
}
impl std::error::Error for OriginalCustodyFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {}

impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OriginalCollectorSinks<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    /// Consume the original sinks inside the complete fixed preparation frame.
    pub fn prepare_original_custodied(
        self,
    ) -> Result<
        OriginalCustodiedPreparation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        OriginalCustodyFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        self.prepare_custodied_with(
            #[cfg(test)]
            None,
            #[cfg(test)]
            &mut |_, _| {},
        )
    }
    #[cfg(test)]
    pub(crate) fn prepare_custodied_fixture_for_test(
        self,
        fixture: &std::path::Path,
        mut checkpoint: impl FnMut(
            &'static str,
            &mut OriginalCustodiedPreparation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        ),
    ) -> Result<
        OriginalCustodiedPreparation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        OriginalCustodyFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        assert_ne!(unsafe { libc::geteuid() }, 0);
        self.prepare_custodied_with(Some(fixture), &mut checkpoint)
    }
    fn prepare_custodied_with(
        self,
        #[cfg(test)] fixture: Option<&std::path::Path>,
        #[cfg(test)] checkpoint: &mut dyn FnMut(
            &'static str,
            &mut OriginalCustodiedPreparation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        ),
    ) -> Result<
        OriginalCustodiedPreparation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        OriginalCustodyFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        #[cfg(test)]
        let allow_fixture = fixture.is_some();
        #[cfg(not(test))]
        let allow_fixture = false;
        let mut held = OriginalCustodiedPreparation {
            original: OriginalGroupedPreparation {
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
            },
            custody: None,
            ready: false,
            #[cfg(test)]
            final_delay: false,
        };
        let result: Result<(), OriginalCustodyError> = (|| {
            held.original
                .transport
                .material
                .outputs
                .material_round()
                .map_err(OriginalStartMaterialError::from)
                .map_err(OriginalPipeTransportError::from)?;
            #[cfg(test)]
            checkpoint("opened", &mut held);
            let material = &mut held.original.transport.material;
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
            held.original
                .transport
                .material
                .bind(allow_fixture)
                .map_err(OriginalPipeTransportError::from)?;
            let material = &held.original.transport.material;
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
                return Err(OriginalGroupedPrepError::Binding.into());
            };
            held.original.transport.message = Some(message(&spec, &stdout.writer, &stderr.writer)?);
            inspect_message(
                held.original.transport.message.as_ref().unwrap(),
                &spec,
                &stdout.writer,
                &stderr.writer,
            )?;

            // Provisional timer only. The unchanged original_round below must
            // independently confirm every binding and these original bounds.
            let material = &mut held.original.transport.material;
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
            held.original.last = Some(now);
            held.original.bounds = Some(bounds);
            held.original.alarm = Some(Alarm::arm(bounds.first())?);
            let storage = &mut held
                .original
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
                .reservation
                .storage;
            held.custody = Some(storage.retain_original_collector_custody()?);
            #[cfg(test)]
            checkpoint("retained", &mut held);
            held.inspect_custody()?;
            // No caller callback occurs after this complete closing observation.
            if held.original.original_round(allow_fixture)? != bounds {
                return Err(OriginalGroupedPrepError::Binding.into());
            }
            let alarm = held
                .original
                .alarm
                .as_ref()
                .ok_or(OriginalGroupedPrepError::Binding)?;
            if alarm.deadline != bounds.first() {
                return Err(OriginalGroupedPrepError::Binding.into());
            }
            held.inspect_custody()?;
            held.inspect_deadline()?;
            held.original.transport.material.ready = true;
            held.ready = true;
            Ok(())
        })();
        match result {
            Ok(()) => Ok(held),
            Err(error) => {
                held.refuse();
                Err(OriginalCustodyFailure {
                    error,
                    held: Box::new(held),
                })
            }
        }
    }
    #[cfg(test)]
    pub(crate) fn custody_origins_for_test(&self) -> [u64; 4] {
        let prepared = &self.pipes.starting.original.original.prepared;
        let progress = &prepared.original.original;
        [
            progress.reservation.state.reserved_monotonic_ns,
            prepared.original.issued.boottime_ns(),
            progress.packet.received_time().boottime_ns(),
            prepared
                .consent_to_sign()
                .starting_state
                .cell_started_monotonic_ns
                .unwrap(),
        ]
    }
}
