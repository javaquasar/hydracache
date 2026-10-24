//! Transfer original acknowledged attempt into Reserved; never adopt or execute.
//!
//! ```compile_fail
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::{early_attempt::FixedEarlyAttemptRead, attempt_publication::publish_reserved_from_attempt};
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::intent_projection::IntentCheck;
//! fn duplicate(read: FixedEarlyAttemptRead<'_,'_,'_,'_,'_,'_,'_,'_,'_>, raw: &[u8], check: &IntentCheck<'_>) { let _ = publish_reserved_from_attempt(read, raw, check); let _ = publish_reserved_from_attempt(read, raw, check); }
//! ```
//!
//! ```compile_fail
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::{FixedAuthorityIntentRead, attempt_publication::publish_reserved_from_attempt};
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::intent_projection::IntentCheck;
//! fn bypass(read: FixedAuthorityIntentRead<'_,'_,'_,'_,'_,'_,'_>, raw: &[u8], check: &IntentCheck<'_>) { publish_reserved_from_attempt(read, raw, check); }
//! ```

use super::early_attempt::{self, EarlyAttemptError, FixedEarlyAttemptRead};
use super::reservation_publication::{
    self, FixedReservedPublicationRead, PublicationStage, ReservationPublicationError,
};
use crate::diagnostic_lease::{validate_state, DiagnosticStage};
use crate::diagnostic_worker_policy::intent_projection::IntentCheck;
use std::fmt;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AttemptPublicationError {
    #[error("full original initial state does not match")]
    Binding,
    #[error("original early attempt refused: {0}")]
    Attempt(#[from] EarlyAttemptError),
    #[error("original publication refused: {0}")]
    Publication(#[from] ReservationPublicationError),
}
enum Held<'join, 'storage, 'intent, 'authority, 'tasks, 'binding, 'guard, 'policy, 'process> {
    Attempt(
        Box<
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
        >,
    ),
    Publication(
        Box<
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
        >,
    ),
}
/// Failure conserves the exclusive original borrow, descriptors and lock until drop.
pub struct AttemptPublicationFailure<
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
    error: AttemptPublicationError,
    held: Held<'join, 'storage, 'intent, 'authority, 'tasks, 'binding, 'guard, 'policy, 'process>,
}
impl fmt::Debug for AttemptPublicationFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(f)
    }
}
impl fmt::Display for AttemptPublicationFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(f)
    }
}
impl std::error::Error for AttemptPublicationFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_> {}
impl AttemptPublicationFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn error(&self) -> &AttemptPublicationError {
        &self.error
    }
    pub fn is_refused(&self) -> bool {
        match &self.held {
            Held::Attempt(read) => read.is_refused(),
            Held::Publication(read) => read.is_refused(),
        }
    }
}
pub fn publish_reserved_from_attempt<
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
    read: FixedEarlyAttemptRead<
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
    AttemptPublicationFailure<
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
> {
    publish_with(read, bytes, check, |_| Ok(()))
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
    mut read: FixedEarlyAttemptRead<
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
    AttemptPublicationFailure<
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
> {
    if !read.is_refused()
        && (!read.original.pins.matches_state(check.state)
            || check.state.stage != DiagnosticStage::Reserved
            || check.state.revision != 1
            || check.state.completed_cells != 0
            || validate_state(check.state).is_err())
    {
        return Err(AttemptPublicationFailure {
            error: AttemptPublicationError::Binding,
            held: Held::Attempt(Box::new(read)),
        });
    }
    if let Err(error) = read.revalidate(bytes, check) {
        return Err(AttemptPublicationFailure {
            error: error.into(),
            held: Held::Attempt(Box::new(read)),
        });
    }
    let original = read.original;
    let storage = match read.storage.into_publication() {
        Ok(storage) => storage,
        Err(mut storage) => {
            let error = early_attempt::refuse_with(
                original,
                &mut storage,
                EarlyAttemptError::Refused,
                |journal, reason| journal.refuse(reason),
            );
            let read = FixedEarlyAttemptRead {
                original,
                storage: *storage,
            };
            return Err(AttemptPublicationFailure {
                error: error.into(),
                held: Held::Attempt(Box::new(read)),
            });
        }
    };
    let mut read = FixedReservedPublicationRead { original, storage };
    let result = read.storage.publish_with(check.state, |stage| {
        checkpoint(stage)?;
        read.original.revalidate(bytes, check)?;
        Ok(())
    });
    if let Err(failure) = result {
        let error = reservation_publication::refuse_with(
            read.original,
            &mut read.storage,
            failure,
            |journal, reason| journal.refuse(reason),
        );
        return Err(AttemptPublicationFailure {
            error: error.into(),
            held: Held::Publication(Box::new(read)),
        });
    }
    if let Err(error) = read.revalidate(bytes, check) {
        return Err(AttemptPublicationFailure {
            error: error.into(),
            held: Held::Publication(Box::new(read)),
        });
    }
    Ok(read)
}
