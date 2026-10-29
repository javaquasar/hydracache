//! Observed in-memory Reserved state, not durable acknowledgement or start authority.
use super::{capture, latch, ControllerRequestAttempt, OriginalCollectorError};
use crate::diagnostic_kernel_admission::KernelTimePoint;
use crate::diagnostic_lease::{DiagnosticStage, DiagnosticState};
use std::fmt;

#[path = "diagnostic_collector_reserved.rs"]
pub mod publication;

/// Consumes the original request owner; state exported for signing is data only.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::ControllerRequestAttempt;
/// fn twice(g: ControllerRequestAttempt<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) {
///     let first = g.into_observed_reservation();
///     let second = g.into_observed_reservation();
/// }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::OriginalObservedReservation;
/// fn clone(g: OriginalObservedReservation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { g.clone(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::OriginalObservedReservation;
/// fn replace(mut g: OriginalObservedReservation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>, state: &hydracache_long_run_supervisor_074::diagnostic_lease::DiagnosticState) { g.bind_collector_requirements(b"", state); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::OriginalObservedReservation;
/// fn start(mut g: OriginalObservedReservation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { g.prepare_root_outputs(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::OriginalObservedReservation;
/// fn decode<T: serde::de::DeserializeOwned>() {}
/// decode::<OriginalObservedReservation<'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static>>();
/// ```
pub struct OriginalObservedReservation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    attempt: ControllerRequestAttempt<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    state: Option<DiagnosticState>,
    last: Option<KernelTimePoint>,
}
impl fmt::Debug for OriginalObservedReservation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OriginalObservedReservation")
            .field("refused", &self.is_refused())
            .finish_non_exhaustive()
    }
}
impl OriginalObservedReservation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn is_refused(&self) -> bool {
        self.attempt.is_refused()
    }

    pub fn revalidate(&mut self) -> Result<(), OriginalCollectorError> {
        if self.is_refused() {
            latch(self.attempt.original, &mut self.attempt.storage);
            return Err(OriginalCollectorError::Refused);
        }
        let result = (|| {
            self.attempt.revalidate()?;
            let controller = &mut self.attempt.original.original.original;
            let now = controller
                .worker
                .original_reservation_time(self.last.as_ref())?;
            if now.boot_id() != controller.request.identity.boot_id || now.boottime_ns() == 0 {
                return Err(OriginalCollectorError::Binding);
            }
            if self.state.is_none() {
                // One initial origin, never refreshed by observation or state export.
                let ns = now.boottime_ns();
                self.state = Some(DiagnosticState {
                    identity: controller.request.identity.clone(),
                    revision: 1,
                    stage: DiagnosticStage::Reserved,
                    completed_cells: 0,
                    reserved_monotonic_ns: ns,
                    last_observed_monotonic_ns: ns,
                    controller_monotonic_ns: ns,
                    cell_started_monotonic_ns: None,
                    cgroup_inode: None,
                    reason: None,
                    cleanup_confirmed: true,
                    promotable: false,
                    admission_allowed: false,
                });
            }
            self.attempt.revalidate()?;
            self.last = Some(now);
            Ok(())
        })();
        if result.is_err() {
            latch(self.attempt.original, &mut self.attempt.storage);
        }
        result
    }

    /// Data for the existing collector signer; not a state replacement or permit.
    pub fn state_to_sign(&mut self) -> Result<DiagnosticState, OriginalCollectorError> {
        self.revalidate()?;
        Ok(self
            .state
            .as_ref()
            .expect("observed original state")
            .clone())
    }

    fn check_collector(&mut self, bytes: &[u8]) -> Result<(), OriginalCollectorError> {
        self.revalidate()?;
        let result = (|| {
            capture(
                &mut self.attempt,
                bytes,
                self.state.as_ref().expect("observed original state"),
            )?;
            self.revalidate()
        })();
        if result.is_err() {
            latch(self.attempt.original, &mut self.attempt.storage);
        }
        result
    }
}
impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    ControllerRequestAttempt<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    /// Ownership transfer only. The first observation validates before exporting data.
    pub fn into_observed_reservation(
        self,
    ) -> OriginalObservedReservation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
        OriginalObservedReservation {
            attempt: self,
            state: None,
            last: None,
        }
    }
}

/// Distinct from the supplied-state preflight; neither guard can create outputs.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::{OriginalCollectorPreflight, observed_reservation::ObservedCollectorPreflight};
/// fn promote(g: OriginalCollectorPreflight<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { let _: ObservedCollectorPreflight<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> = g.into(); }
/// ```
pub struct ObservedCollectorPreflight<'bind, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    owner: &'bind mut OriginalObservedReservation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    bytes: Vec<u8>,
}
impl fmt::Debug for ObservedCollectorPreflight<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ObservedCollectorPreflight")
            .field("refused", &self.is_refused())
            .finish_non_exhaustive()
    }
}
impl ObservedCollectorPreflight<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn is_refused(&self) -> bool {
        self.owner.is_refused()
    }
    pub fn revalidate(&mut self) -> Result<(), OriginalCollectorError> {
        self.owner.check_collector(&self.bytes)
    }
}
impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OriginalObservedReservation<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub fn bind_collector_requirements<'bind>(
        &'bind mut self,
        bytes: &[u8],
    ) -> Result<
        ObservedCollectorPreflight<'bind, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        OriginalCollectorError,
    > {
        self.check_collector(bytes)?;
        Ok(ObservedCollectorPreflight {
            owner: self,
            bytes: bytes.to_vec(),
        })
    }
}
