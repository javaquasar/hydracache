//! Early retained attempt only; not cell consumption, state publication or start.
//!
//! ```compile_fail
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::early_attempt::FixedEarlyAttemptRead;
//! fn copy(read: FixedEarlyAttemptRead<'_,'_,'_,'_,'_,'_,'_,'_,'_>) { let _ = read.clone(); }
//! ```
//!
//! ```compile_fail
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::early_attempt::FixedEarlyAttemptRead;
//! fn start(read: &mut FixedEarlyAttemptRead<'_,'_,'_,'_,'_,'_,'_,'_,'_>) { read.start_once(); }
//! ```
//!
//! ```compile_fail
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::{FixtureAuthorityIntentRead, early_attempt::retain_fixed_attempt};
//! use hydracache_long_run_supervisor_074::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::OriginalEmptyReservationFence;
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::intent_projection::IntentCheck;
//! fn promote(original: &mut FixtureAuthorityIntentRead<'_,'_,'_,'_,'_,'_,'_>, fence: OriginalEmptyReservationFence<'_>, raw: &[u8], check: &IntentCheck<'_>) { retain_fixed_attempt(original, fence, raw, check); }
//! ```

use super::{AuthorityIntentError, FixedAuthorityIntentRead, IntentReader};
use super::empty_fence::{bind_fixed_empty_fenced_intent, EmptyFencedIntentError};
use crate::diagnostic_lease::{validate_state, DiagnosticStage};
use crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::{
    OriginalEmptyReservationFence, attempt_storage::AttemptStorage,
};
pub use crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::attempt_storage::{AttemptStage, StorageAttemptError};
use crate::diagnostic_output_preparation::intent_journal::{record_bytes, Record, RECORD_SCHEMA};
use crate::diagnostic_worker_policy::authority_registry::{AuthorityError, RefusalReason};
use crate::diagnostic_worker_policy::intent_projection::IntentCheck;
use std::fmt;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EarlyAttemptError {
    #[error("original initial state does not match")]
    Binding,
    #[error("original empty-fence/intent preflight refused: {0}")]
    Preflight(#[from] EmptyFencedIntentError),
    #[error("original signed intent refused: {0}")]
    Intent(#[from] AuthorityIntentError),
    #[error("original attempt storage refused: {0}")]
    Storage(#[from] StorageAttemptError),
    #[error("original attempt permanently refused")]
    Refused,
    #[error("{failure}; original authority refusal not acknowledged: {publication}")]
    RefusalUncertain {
        failure: Box<Self>,
        publication: AuthorityError,
    },
}
pub struct EarlyAttemptFailure<'storage> {
    error: EarlyAttemptError,
    storage: Box<AttemptStorage<'storage>>,
}
impl fmt::Debug for EarlyAttemptFailure<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(f)
    }
}
impl fmt::Display for EarlyAttemptFailure<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(f)
    }
}
impl std::error::Error for EarlyAttemptFailure<'_> {}
impl EarlyAttemptFailure<'_> {
    pub fn error(&self) -> &EarlyAttemptError {
        &self.error
    }
    pub fn is_refused(&self) -> bool {
        self.storage.is_refused()
    }
}
/// Original runtime acknowledgement only. No recovered-file constructor.
pub struct FixedEarlyAttemptRead<
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
    pub(super) storage: AttemptStorage<'storage>,
}
impl fmt::Debug for FixedEarlyAttemptRead<'_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FixedEarlyAttemptRead")
            .field("refused", &self.is_refused())
            .finish_non_exhaustive()
    }
}
pub(super) fn refuse_with(
    original: &mut IntentReader<'_, '_, '_, '_, '_, '_, '_>,
    storage: &mut AttemptStorage<'_>,
    failure: EarlyAttemptError,
    publish: impl FnOnce(&mut super::super::Journal<'_>, RefusalReason) -> Result<(), AuthorityError>,
) -> EarlyAttemptError {
    storage.latch_refusal();
    original.original.latch_original();
    if matches!(
        failure,
        EarlyAttemptError::Intent(_) | EarlyAttemptError::Preflight(_) | EarlyAttemptError::Refused
    ) {
        return failure;
    }
    match original
        .original
        .persist_with(RefusalReason::Policy, publish)
    {
        Ok(()) => failure,
        Err(publication) => EarlyAttemptError::RefusalUncertain {
            failure: Box::new(failure),
            publication,
        },
    }
}
fn refuse(
    original: &mut IntentReader<'_, '_, '_, '_, '_, '_, '_>,
    storage: &mut AttemptStorage<'_>,
    failure: EarlyAttemptError,
) -> EarlyAttemptError {
    refuse_with(original, storage, failure, |journal, reason| {
        journal.refuse(reason)
    })
}
impl FixedEarlyAttemptRead<'_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn is_refused(&self) -> bool {
        self.original.is_refused() || self.storage.is_refused()
    }
    pub fn revalidate(
        &mut self,
        bytes: &[u8],
        check: &IntentCheck<'_>,
    ) -> Result<(), EarlyAttemptError> {
        if self.is_refused() {
            return Err(refuse(
                self.original,
                &mut self.storage,
                EarlyAttemptError::Refused,
            ));
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
pub fn retain_fixed_attempt<
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
    FixedEarlyAttemptRead<
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
    EarlyAttemptFailure<'storage>,
> {
    retain_with(original, fence, bytes, check, |_| Ok(()))
}
pub(super) fn retain_with<
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
    mut checkpoint: impl FnMut(AttemptStage) -> Result<(), EarlyAttemptError>,
) -> Result<
    FixedEarlyAttemptRead<
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
    EarlyAttemptFailure<'storage>,
> {
    let mut storage = AttemptStorage::new(fence);
    if original.is_refused() || storage.is_refused() {
        let error = refuse(
            &mut original.reader,
            &mut storage,
            EarlyAttemptError::Refused,
        );
        return Err(EarlyAttemptFailure {
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
        return Err(EarlyAttemptFailure {
            error: EarlyAttemptError::Binding,
            storage: Box::new(storage),
        });
    }
    let result = (|| {
        bind_fixed_empty_fenced_intent(original, &mut storage.empty, bytes, check)?;
        let record = Record {
            schema_version: RECORD_SCHEMA.into(),
            state: check.state.clone(),
            signed_intent: std::str::from_utf8(bytes)
                .map_err(|_| StorageAttemptError::Document)?
                .into(),
            peer_uid: check.peer.uid,
            peer_gids: check.peer.gids.clone(),
            previous_record_sha256: None,
        };
        let raw = record_bytes(&record).map_err(StorageAttemptError::from)?;
        storage.retain_with(&raw, |at| {
            checkpoint(at)?;
            original.reader.revalidate(bytes, check)?;
            Ok(())
        })
    })();
    if let Err(failure) = result {
        let error = refuse(&mut original.reader, &mut storage, failure);
        return Err(EarlyAttemptFailure {
            error,
            storage: Box::new(storage),
        });
    }
    let mut read = FixedEarlyAttemptRead {
        original: &mut original.reader,
        storage,
    };
    if let Err(error) = read.revalidate(bytes, check) {
        return Err(EarlyAttemptFailure {
            error,
            storage: Box::new(read.storage),
        });
    }
    Ok(read)
}
#[cfg(test)]
pub(super) const ALL_ATTEMPT_STAGES: [AttemptStage; 8] = [
    AttemptStage::BeforeCreate,
    AttemptStage::Created,
    AttemptStage::PrefixWritten,
    AttemptStage::BytesWritten,
    AttemptStage::Sealed,
    AttemptStage::FileSynced,
    AttemptStage::DirectorySynced,
    AttemptStage::Acknowledging,
];
