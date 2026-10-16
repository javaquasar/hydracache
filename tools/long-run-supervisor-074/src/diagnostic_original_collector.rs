//! Original-chain signature preflight only; supplied state is not durable lifecycle authority.
use super::{latch, ControllerAttemptError, ControllerRequestAttempt};
use crate::diagnostic_lease::{DiagnosticStage, DiagnosticState};
use crate::diagnostic_worker_policy::trusted_input::TrustInputError;
use std::fmt;

#[path = "diagnostic_observed_reservation.rs"]
pub mod observed_reservation;

#[derive(Debug, thiserror::Error)]
pub enum OriginalCollectorError {
    #[error("original collector preflight permanently refused")]
    Refused,
    #[error("original acknowledged request refused: {0}")]
    Attempt(#[from] ControllerAttemptError),
    #[error("original collector input refused: {0}")]
    Input(#[from] TrustInputError),
    #[error("collector differs from original request or initial state")]
    Binding,
}

fn capture(
    attempt: &mut ControllerRequestAttempt<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    bytes: &[u8],
    state: &DiagnosticState,
) -> Result<(), OriginalCollectorError> {
    attempt.revalidate()?;
    if state.stage != DiagnosticStage::Reserved || state.revision != 1 || state.completed_cells != 0
    {
        return Err(OriginalCollectorError::Binding);
    }
    let controller = &mut attempt.original.original.original;
    let body = controller.worker.verify_original_collector_requirements(
        bytes,
        state,
        &controller.controller.source.policy,
    )?;
    let request = &controller.request;
    if state.identity != request.identity
        || body.request_id != request.request_id
        || body.nonce_sha256 != request.nonce_sha256
        || body.controller != request.controller
        || body.issued_at_unix_seconds != request.issued_at_unix_seconds
        || body.expires_at_unix_seconds != request.expires_at_unix_seconds
    {
        return Err(OriginalCollectorError::Binding);
    }
    attempt.revalidate()?;
    Ok(())
}

/// Exclusive borrow of the acknowledged original, with no replacement inputs.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::OriginalCollectorPreflight;
/// fn clone(g: OriginalCollectorPreflight<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { g.clone(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::OriginalCollectorPreflight;
/// fn start(mut g: OriginalCollectorPreflight<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { g.start_once(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::OriginalCollectorPreflight;
/// fn replace(mut g: OriginalCollectorPreflight<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { g.revalidate(b"new"); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::OriginalCollectorPreflight;
/// fn release(mut g: OriginalCollectorPreflight<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { g.prepare_root_outputs(); }
/// ```
pub struct OriginalCollectorPreflight<'bind, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    attempt: &'bind mut ControllerRequestAttempt<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    bytes: Vec<u8>,
    state: DiagnosticState,
}
impl fmt::Debug for OriginalCollectorPreflight<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OriginalCollectorPreflight")
            .field("refused", &self.is_refused())
            .finish_non_exhaustive()
    }
}
impl OriginalCollectorPreflight<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn is_refused(&self) -> bool {
        self.attempt.is_refused()
    }
    pub fn revalidate(&mut self) -> Result<(), OriginalCollectorError> {
        if self.is_refused() {
            latch(self.attempt.original, &mut self.attempt.storage);
            return Err(OriginalCollectorError::Refused);
        }
        let result = capture(self.attempt, &self.bytes, &self.state);
        if result.is_err() {
            latch(self.attempt.original, &mut self.attempt.storage);
        }
        result
    }
}
impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    ControllerRequestAttempt<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub fn bind_collector_requirements<'bind>(
        &'bind mut self,
        bytes: &[u8],
        state: &DiagnosticState,
    ) -> Result<
        OriginalCollectorPreflight<'bind, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        OriginalCollectorError,
    > {
        if self.is_refused() {
            latch(self.original, &mut self.storage);
            return Err(OriginalCollectorError::Refused);
        }
        if let Err(error) = capture(self, bytes, state) {
            latch(self.original, &mut self.storage);
            return Err(error);
        }
        Ok(OriginalCollectorPreflight {
            attempt: self,
            bytes: bytes.to_vec(),
            state: state.clone(),
        })
    }
}
