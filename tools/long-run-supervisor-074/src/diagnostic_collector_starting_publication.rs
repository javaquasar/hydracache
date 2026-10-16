//! Consuming first-cell intent publication, never process start or recovered acknowledgement.
use super::*;
use crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::attempt_storage::{StorageAttemptError, collector_reservation_storage::starting_storage};
use crate::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::starting_record::{encode_collector_starting_record, CollectorStartingRecordError};
pub use starting_storage::CollectorStartingStage;

#[derive(Debug, thiserror::Error)]
pub enum CollectorStartingError {
    #[error("original validation refused: {0}")]
    Original(#[from] OriginalStartConsentError),
    #[error("original Starting storage refused: {0}")]
    Storage(#[from] StorageAttemptError),
    #[error("Starting record refused: {0}")]
    Record(#[from] CollectorStartingRecordError),
    #[error("private interruption at {0:?}")]
    Interrupted(CollectorStartingStage),
}

/// Original runtime acknowledgement only. No recovered owner, start, retry or extraction API.
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::OriginalFramedConsent;
/// fn twice(p: OriginalFramedConsent<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) {
///     let first = p.publish_starting(); let second = p.publish_starting();
/// }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::OriginalCollectorStarting;
/// fn start(p: OriginalCollectorStarting<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.start_once(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::OriginalCollectorStarting;
/// fn copy(p: OriginalCollectorStarting<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.clone(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::OriginalCollectorStarting;
/// fn decode<T: serde::de::DeserializeOwned>() {}
/// decode::<OriginalCollectorStarting<'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static>>();
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::OriginalCollectorStarting;
/// fn retry(p: OriginalCollectorStarting<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.publish_starting(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::OriginalCollectorStarting;
/// fn extract(p: OriginalCollectorStarting<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.into_original(); }
/// ```
pub struct OriginalCollectorStarting<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    original: OriginalFramedConsent<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    acknowledged: bool,
}
impl fmt::Debug for OriginalCollectorStarting<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OriginalCollectorStarting")
            .field("acknowledged", &self.acknowledged)
            .field("refused", &self.is_refused())
            .finish_non_exhaustive()
    }
}
impl OriginalCollectorStarting<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn is_refused(&self) -> bool {
        self.original.is_refused()
    }
    pub fn revalidate(&mut self) -> Result<(), CollectorStartingError> {
        if !self.acknowledged {
            self.original.refuse();
            return Err(OriginalStartConsentError::Refused.into());
        }
        self.original.revalidate().map_err(Into::into)
    }
    fn publish(
        &mut self,
        mut checkpoint: impl FnMut(CollectorStartingStage) -> Result<(), CollectorStartingError>,
    ) -> Result<(), CollectorStartingError> {
        let r = &self
            .original
            .original
            .prepared
            .original
            .original
            .reservation;
        let raw = encode_collector_starting_record(
            &r.request_raw,
            &r.raw,
            self.original.original.packet.bytes(),
        )?;
        self.original
            .original
            .prepared
            .original
            .original
            .reservation
            .storage
            .begin_starting()?;
        for at in starting_storage::STAGES {
            // Two complete live rounds before and after each storage effect; no cached proof.
            self.original.revalidate()?;
            self.original
                .original
                .prepared
                .original
                .original
                .reservation
                .storage
                .starting_step(at, &raw)?;
            checkpoint(at)?;
            self.original.revalidate()?;
        }
        self.original.revalidate()?;
        self.original
            .original
            .prepared
            .original
            .original
            .reservation
            .storage
            .acknowledge_starting()?;
        self.acknowledged = true;
        Ok(())
    }
}
pub struct CollectorStartingFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    error: CollectorStartingError,
    held: Box<OriginalCollectorStarting<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>>,
}
impl CollectorStartingFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn error(&self) -> &CollectorStartingError {
        &self.error
    }
    pub fn is_refused(&self) -> bool {
        self.held.is_refused()
    }
}
impl fmt::Debug for CollectorStartingFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.error, f)
    }
}
impl fmt::Display for CollectorStartingFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.error, f)
    }
}
impl std::error::Error for CollectorStartingFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {}
impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OriginalFramedConsent<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub fn publish_starting(
        self,
    ) -> Result<
        OriginalCollectorStarting<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        CollectorStartingFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        self.publish_with(|_| Ok(()))
    }
    #[cfg(test)]
    pub(crate) fn publish_starting_with(
        self,
        checkpoint: impl FnMut(CollectorStartingStage) -> Result<(), CollectorStartingError>,
    ) -> Result<
        OriginalCollectorStarting<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        CollectorStartingFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        self.publish_with(checkpoint)
    }
    fn publish_with(
        self,
        checkpoint: impl FnMut(CollectorStartingStage) -> Result<(), CollectorStartingError>,
    ) -> Result<
        OriginalCollectorStarting<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        CollectorStartingFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        let mut held = OriginalCollectorStarting {
            original: self,
            acknowledged: false,
        };
        match held.publish(checkpoint) {
            Ok(()) => Ok(held),
            Err(error) => {
                held.original.refuse();
                Err(CollectorStartingFailure {
                    error,
                    held: Box::new(held),
                })
            }
        }
    }
}
