//! Initial fixed-only Reserved publication, not peer authentication or execution.
//!
//! ```compile_fail
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::{FixtureAuthorityIntentRead, reservation_publication::publish_fixed_reserved};
//! use hydracache_long_run_supervisor_074::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::OriginalEmptyReservationFence;
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::intent_projection::IntentCheck;
//! fn promote(original: &mut FixtureAuthorityIntentRead<'_,'_,'_,'_,'_,'_,'_>, fence: OriginalEmptyReservationFence<'_>, bytes: &[u8], check: &IntentCheck<'_>) {
//!     publish_fixed_reserved(original, fence, bytes, check).unwrap();
//! }
//! ```
//!
//! ```compile_fail
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::reservation_publication::FixedReservedPublicationRead;
//! fn copy(read: FixedReservedPublicationRead<'_,'_,'_,'_,'_,'_,'_,'_,'_>) { let _ = read.clone(); }
//! ```
//!
//! ```compile_fail
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::reservation_publication::FixedReservedPublicationRead;
//! fn start(read: &mut FixedReservedPublicationRead<'_,'_,'_,'_,'_,'_,'_,'_,'_>) { read.start_once(); }
//! ```
//!
//! ```compile_fail
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::{FixedAuthorityIntentRead, reservation_publication::publish_fixed_reserved};
//! use hydracache_long_run_supervisor_074::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::OriginalEmptyReservationFence;
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::intent_projection::IntentCheck;
//! fn reuse(original: &mut FixedAuthorityIntentRead<'_,'_,'_,'_,'_,'_,'_>, fence: OriginalEmptyReservationFence<'_>, bytes: &[u8], check: &IntentCheck<'_>) {
//!     let mut read = publish_fixed_reserved(original, fence, bytes, check).unwrap();
//!     original.revalidate(bytes, check).unwrap();
//!     read.revalidate(bytes, check).unwrap();
//! }
//! ```

use super::{AuthorityIntentError, FixedAuthorityIntentRead, IntentReader};
use super::empty_fence::{bind_fixed_empty_fenced_intent, EmptyFencedIntentError};
use crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::{
    OriginalEmptyReservationFence, publication_storage::PublicationStorage,
};
pub use crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::publication_storage::{
    PublicationStage, StoragePublicationError,
};
use crate::diagnostic_lease::{DiagnosticStage, validate_state};
use crate::diagnostic_worker_policy::authority_registry::{AuthorityError, RefusalReason};
use crate::diagnostic_worker_policy::intent_projection::IntentCheck;
use std::fmt;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReservationPublicationError {
    #[error("complete original initial Reserved snapshot does not match")]
    Binding,
    #[error("original empty-fence/intent preflight refused: {0}")]
    Preflight(#[from] EmptyFencedIntentError),
    #[error("original signed intent refused: {0}")]
    Intent(#[from] AuthorityIntentError),
    #[error("original state publication refused: {0}")]
    Storage(#[from] StoragePublicationError),
    #[error("original publication permanently refused")]
    Refused,
    #[error("{failure}; original authority refusal not acknowledged: {publication}")]
    RefusalUncertain {
        failure: Box<Self>,
        publication: AuthorityError,
    },
}
/// A failed attempt retains the original fence until drop; never deletes a record.
pub struct ReservationPublicationFailure<'storage> {
    error: ReservationPublicationError,
    storage: Box<PublicationStorage<'storage>>,
}
impl fmt::Debug for ReservationPublicationFailure<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(f)
    }
}
impl fmt::Display for ReservationPublicationFailure<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(f)
    }
}
impl std::error::Error for ReservationPublicationFailure<'_> {}
impl ReservationPublicationFailure<'_> {
    pub fn error(&self) -> &ReservationPublicationError {
        &self.error
    }
    pub fn is_refused(&self) -> bool {
        self.storage.is_refused()
    }
}
/// Runtime publication acknowledgement only; no state/FD export or start conversion.
pub struct FixedReservedPublicationRead<
    'join,
    'storage,
    'intent,
    'authority,
    'tasks,
    'binding,
    'guard,
    'policy,
    'process,
> {
    pub(super) original:
        &'join mut IntentReader<'intent, 'authority, 'tasks, 'binding, 'guard, 'policy, 'process>,
    pub(super) storage: PublicationStorage<'storage>,
}
fn refuse(
    original: &mut IntentReader<'_, '_, '_, '_, '_, '_, '_>,
    storage: &mut PublicationStorage<'_>,
    failure: ReservationPublicationError,
) -> ReservationPublicationError {
    refuse_with(original, storage, failure, |journal, reason| {
        journal.refuse(reason)
    })
}
pub(super) fn refuse_with(
    original: &mut IntentReader<'_, '_, '_, '_, '_, '_, '_>,
    storage: &mut PublicationStorage<'_>,
    failure: ReservationPublicationError,
    publish: impl FnOnce(&mut super::super::Journal<'_>, RefusalReason) -> Result<(), AuthorityError>,
) -> ReservationPublicationError {
    storage.latch_refusal();
    original.original.latch_original();
    if matches!(
        failure,
        ReservationPublicationError::Intent(_)
            | ReservationPublicationError::Preflight(_)
            | ReservationPublicationError::Refused
    ) {
        // Those original bridges already handled refusal, or require no I/O.
        return failure;
    }
    match original
        .original
        .persist_with(RefusalReason::Policy, publish)
    {
        Ok(()) => failure,
        Err(publication) => ReservationPublicationError::RefusalUncertain {
            failure: Box::new(failure),
            publication,
        },
    }
}
impl FixedReservedPublicationRead<'_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn is_refused(&self) -> bool {
        self.original.is_refused() || self.storage.is_refused()
    }
    pub fn revalidate(
        &mut self,
        bytes: &[u8],
        check: &IntentCheck<'_>,
    ) -> Result<(), ReservationPublicationError> {
        if self.is_refused() {
            self.storage.latch_refusal();
            self.original.original.latch_original();
            return Err(ReservationPublicationError::Refused);
        }
        let result = (|| {
            self.storage.revalidate()?;
            self.original.revalidate(bytes, check)?;
            self.storage.revalidate()?;
            self.original.revalidate(bytes, check)?;
            self.storage.revalidate()?;
            Ok(())
        })();
        result.map_err(|failure| refuse(self.original, &mut self.storage, failure))
    }
}
impl fmt::Debug for FixedReservedPublicationRead<'_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FixedReservedPublicationRead")
            .field("refused", &self.is_refused())
            .finish_non_exhaustive()
    }
}
pub fn publish_fixed_reserved<
    'join,
    'storage,
    'intent,
    'authority,
    'tasks,
    'binding,
    'guard,
    'policy,
    'process,
>(
    original: &'join mut FixedAuthorityIntentRead<
        'intent,
        'authority,
        'tasks,
        'binding,
        'guard,
        'policy,
        'process,
    >,
    fence: OriginalEmptyReservationFence<'storage>,
    bytes: &[u8],
    check: &IntentCheck<'_>,
) -> Result<
    FixedReservedPublicationRead<
        'join,
        'storage,
        'intent,
        'authority,
        'tasks,
        'binding,
        'guard,
        'policy,
        'process,
    >,
    ReservationPublicationFailure<'storage>,
> {
    publish_with(original, fence, bytes, check, |_| Ok(()))
}
pub(super) fn publish_with<
    'join,
    'storage,
    'intent,
    'authority,
    'tasks,
    'binding,
    'guard,
    'policy,
    'process,
>(
    original: &'join mut FixedAuthorityIntentRead<
        'intent,
        'authority,
        'tasks,
        'binding,
        'guard,
        'policy,
        'process,
    >,
    fence: OriginalEmptyReservationFence<'storage>,
    bytes: &[u8],
    check: &IntentCheck<'_>,
    mut checkpoint: impl FnMut(PublicationStage) -> Result<(), ReservationPublicationError>,
) -> Result<
    FixedReservedPublicationRead<
        'join,
        'storage,
        'intent,
        'authority,
        'tasks,
        'binding,
        'guard,
        'policy,
        'process,
    >,
    ReservationPublicationFailure<'storage>,
> {
    let mut storage = PublicationStorage::new(fence);
    if original.is_refused() || storage.is_refused() {
        let error = refuse(
            &mut original.reader,
            &mut storage,
            ReservationPublicationError::Refused,
        );
        return Err(ReservationPublicationFailure {
            error,
            storage: Box::new(storage),
        });
    }
    if !original.reader.pins.matches_state(check.state)
        || check.state.stage != DiagnosticStage::Reserved
        || check.state.revision != 1
        || check.state.completed_cells != 0
        || validate_state(check.state).is_err()
    {
        return Err(ReservationPublicationFailure {
            error: ReservationPublicationError::Binding,
            storage: Box::new(storage),
        });
    }
    let preflight =
        bind_fixed_empty_fenced_intent(original, &mut storage.empty, bytes, check).map(|_| ());
    if let Err(failure) = preflight {
        let error = refuse(&mut original.reader, &mut storage, failure.into());
        return Err(ReservationPublicationFailure {
            error,
            storage: Box::new(storage),
        });
    }
    let result = storage.publish_with(check.state, |stage| {
        checkpoint(stage)?;
        original.reader.revalidate(bytes, check)?;
        Ok(())
    });
    if let Err(failure) = result {
        let error = refuse(&mut original.reader, &mut storage, failure);
        return Err(ReservationPublicationFailure {
            error,
            storage: Box::new(storage),
        });
    }
    let mut read = FixedReservedPublicationRead {
        original: &mut original.reader,
        storage,
    };
    if let Err(error) = read.revalidate(bytes, check) {
        return Err(ReservationPublicationFailure {
            error,
            storage: Box::new(read.storage),
        });
    }
    Ok(read)
}
#[cfg(test)]
pub(super) const ALL_STAGES: [PublicationStage; 8] = [
    PublicationStage::BeforeCreate,
    PublicationStage::StateCreated,
    PublicationStage::PrefixWritten,
    PublicationStage::BytesWritten,
    PublicationStage::FileSynced,
    PublicationStage::DirectorySynced,
    PublicationStage::DocumentBound,
    PublicationStage::Acknowledging,
];
