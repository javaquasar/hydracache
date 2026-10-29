//! Initial-cell retention only: consumption is not permission to execute.
//!
//! ```compile_fail
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::initial_consumption::FixedConsumedInitialIntentRead;
//! fn copy(read: FixedConsumedInitialIntentRead<'_, '_, '_, '_, '_, '_, '_, '_, '_>) { let _ = read.clone(); }
//! ```
//!
//! ```compile_fail
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::initial_consumption::FixedConsumedInitialIntentRead;
//! fn start(read: &mut FixedConsumedInitialIntentRead<'_, '_, '_, '_, '_, '_, '_, '_, '_>) { read.start_once(); }
//! ```
//!
//! ```compile_fail
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::{FixedAuthorityIntentRead, initial_consumption::consume_fixed_initial_intent};
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::intent_projection::IntentCheck;
//! fn adopt(read: FixedAuthorityIntentRead<'_,'_,'_,'_,'_,'_,'_>, raw: &[u8], check: &IntentCheck<'_>) { consume_fixed_initial_intent(read, raw, check); }
//! ```
//!
//! ```compile_fail
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::{initial_consumption::consume_fixed_initial_intent, reservation_publication::FixedReservedPublicationRead};
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::intent_projection::IntentCheck;
//! fn reuse(read: FixedReservedPublicationRead<'_, '_, '_, '_, '_, '_, '_, '_, '_>, raw: &[u8], check: &IntentCheck<'_>) { let _ = consume_fixed_initial_intent(read, raw, check); let _ = consume_fixed_initial_intent(read, raw, check); }
//! ```

use super::reservation_publication::{self, FixedReservedPublicationRead};
pub use super::reservation_publication::{ReservationPublicationError, StoragePublicationError};
pub use crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::publication_storage::ConsumptionStage;
use crate::diagnostic_output_preparation::intent_journal::{record_bytes, Record, RECORD_SCHEMA};
use crate::diagnostic_worker_policy::intent_projection::IntentCheck;
use std::fmt;

/// Retains only the original consumed objects, intent borrow and common fence.
pub struct FixedConsumedInitialIntentRead<
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
    pub(super) read: FixedReservedPublicationRead<
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
}
/// First error and original publisher/lock are retained until this value drops.
pub struct InitialConsumptionFailure<
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
    error: ReservationPublicationError,
    read: Box<
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
}
impl fmt::Debug for InitialConsumptionFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(f)
    }
}
impl fmt::Display for InitialConsumptionFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(f)
    }
}
impl std::error::Error for InitialConsumptionFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_> {}
impl InitialConsumptionFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn error(&self) -> &ReservationPublicationError {
        &self.error
    }
    pub fn is_refused(&self) -> bool {
        self.read.is_refused()
    }
}
impl fmt::Debug for FixedConsumedInitialIntentRead<'_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FixedConsumedInitialIntentRead")
            .field("refused", &self.is_refused())
            .finish_non_exhaustive()
    }
}
impl FixedConsumedInitialIntentRead<'_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn is_refused(&self) -> bool {
        self.read.is_refused()
    }
    pub fn revalidate(
        &mut self,
        bytes: &[u8],
        check: &IntentCheck<'_>,
    ) -> Result<(), ReservationPublicationError> {
        if !self.read.is_refused() {
            if let Err(error) = self.read.storage.revalidate_initial_consumption() {
                return Err(reservation_publication::refuse_with(
                    self.read.original,
                    &mut self.read.storage,
                    error.into(),
                    |journal, reason| journal.refuse(reason),
                ));
            }
        }
        self.read.revalidate(bytes, check)
    }
}
pub fn consume_fixed_initial_intent<
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
    read: FixedReservedPublicationRead<
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
    FixedConsumedInitialIntentRead<
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
    InitialConsumptionFailure<
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
    consume_with(read, bytes, check, |_| Ok(()))
}
pub(super) fn consume_with<
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
    mut read: FixedReservedPublicationRead<
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
    mut checkpoint: impl FnMut(ConsumptionStage) -> Result<(), ReservationPublicationError>,
) -> Result<
    FixedConsumedInitialIntentRead<
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
    InitialConsumptionFailure<
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
    if !read.is_refused() && !read.original.pins.matches_state(check.state) {
        return Err(InitialConsumptionFailure {
            error: ReservationPublicationError::Binding,
            read: Box::new(read),
        });
    }
    if let Err(error) = read.revalidate(bytes, check) {
        return Err(InitialConsumptionFailure {
            error,
            read: Box::new(read),
        });
    }
    let result = (|| {
        let record = Record {
            schema_version: RECORD_SCHEMA.into(),
            state: check.state.clone(),
            signed_intent: std::str::from_utf8(bytes)
                .map_err(|_| StoragePublicationError::ConsumptionDocument)?
                .into(),
            peer_uid: check.peer.uid,
            peer_gids: check.peer.gids.clone(),
            previous_record_sha256: None,
        };
        let raw = record_bytes(&record).map_err(StoragePublicationError::from)?;
        read.storage.consume_initial_with(&raw, |at| {
            checkpoint(at)?;
            read.original.revalidate(bytes, check)?;
            Ok(())
        })
    })();
    if let Err(failure) = result {
        let error = reservation_publication::refuse_with(
            read.original,
            &mut read.storage,
            failure,
            |journal, reason| journal.refuse(reason),
        );
        return Err(InitialConsumptionFailure {
            error,
            read: Box::new(read),
        });
    }
    let mut consumed = FixedConsumedInitialIntentRead { read };
    if let Err(error) = consumed.revalidate(bytes, check) {
        return Err(InitialConsumptionFailure {
            error,
            read: Box::new(consumed.read),
        });
    }
    Ok(consumed)
}
#[cfg(test)]
pub(super) const ALL_CONSUMPTION_STAGES: [ConsumptionStage; 14] = [
    ConsumptionStage::BeforeRecord,
    ConsumptionStage::RecordCreated,
    ConsumptionStage::RecordPrefixWritten,
    ConsumptionStage::RecordBytesWritten,
    ConsumptionStage::RecordSealed,
    ConsumptionStage::RecordSynced,
    ConsumptionStage::BeforeCommit,
    ConsumptionStage::CommitCreated,
    ConsumptionStage::CommitPrefixWritten,
    ConsumptionStage::CommitBytesWritten,
    ConsumptionStage::CommitSealed,
    ConsumptionStage::CommitSynced,
    ConsumptionStage::DirectorySynced,
    ConsumptionStage::Acknowledging,
];
