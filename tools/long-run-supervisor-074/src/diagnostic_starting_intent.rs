//! Owned first departure only. No live-state replacement or execution capability.
//!
//! ```compile_fail
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::starting_intent::FixedStartingIntentRead;
//! fn clone(read: &FixedStartingIntentRead<'_,'_,'_,'_,'_,'_,'_,'_,'_>) { let _: FixedStartingIntentRead<'_,'_,'_,'_,'_,'_,'_,'_,'_> = read.clone(); }
//! ```
//!
//! ```compile_fail
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::starting_intent::FixedStartingIntentRead;
//! fn start(read: &mut FixedStartingIntentRead<'_,'_,'_,'_,'_,'_,'_,'_,'_>) { read.start_once(); }
//! ```
//!
//! ```compile_fail
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::{starting_intent::retain_fixed_starting_intent, reservation_publication::FixedReservedPublicationRead};
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::intent_projection::IntentCheck;
//! use hydracache_long_run_supervisor_074::diagnostic_lease::DiagnosticState;
//! fn skip(read: FixedReservedPublicationRead<'_,'_,'_,'_,'_,'_,'_,'_,'_>, bytes: &[u8], check: &IntentCheck<'_>, next: &DiagnosticState) { retain_fixed_starting_intent(read, bytes, check, next); }
//! ```
//!
//! ```compile_fail
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::{starting_intent::retain_fixed_starting_intent, initial_consumption::FixedConsumedInitialIntentRead};
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::intent_projection::IntentCheck;
//! use hydracache_long_run_supervisor_074::diagnostic_lease::DiagnosticState;
//! fn repeat(read: FixedConsumedInitialIntentRead<'_,'_,'_,'_,'_,'_,'_,'_,'_>, bytes: &[u8], check: &IntentCheck<'_>, next: &DiagnosticState) { let _ = retain_fixed_starting_intent(read, bytes, check, next); let _ = retain_fixed_starting_intent(read, bytes, check, next); }
//! ```
use super::initial_consumption::FixedConsumedInitialIntentRead;
use super::reservation_publication::{self, ReservationPublicationError, StoragePublicationError};
use crate::diagnostic_lease::{encode, validate_transition, DiagnosticStage, DiagnosticState};
use crate::diagnostic_worker_policy::intent_projection::IntentCheck;
use crate::{canonical_json, sha256_hex};
use crate::diagnostic_worker_policy::authority_registry::{AuthorityError, RefusalReason};
use serde::{Deserialize, Serialize};
use std::fmt;
pub use crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::publication_storage::StartingStage;

pub const STARTING_SCHEMA: &str = "diagnostic-starting-intent-074-v1";
const MAX_BYTES: usize = 4096;

/// Refusal acknowledgement only, never cell completion or cleanup permission.
#[derive(Debug)]
pub enum QuarantineDisposition {
    Acknowledged,
    Uncertain(AuthorityError),
    /// No new I/O; prior refusal does not establish durable acknowledgement.
    PreviouslyRefused,
}

/// Retains the original chain and fence, exposing neither retry nor execution.
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::starting_intent::FixedStartingQuarantine;
/// fn clone(q: &FixedStartingQuarantine<'_,'_,'_,'_,'_,'_,'_,'_,'_>) { let _: FixedStartingQuarantine<'_,'_,'_,'_,'_,'_,'_,'_,'_> = q.clone(); }
/// ```
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::starting_intent::FixedStartingQuarantine;
/// fn start(q: &mut FixedStartingQuarantine<'_,'_,'_,'_,'_,'_,'_,'_,'_>) { q.start_once(); }
/// ```
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::starting_intent::FixedStartingQuarantine;
/// fn release(q: &mut FixedStartingQuarantine<'_,'_,'_,'_,'_,'_,'_,'_,'_>) { q.release_resources(); }
/// ```
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::starting_intent::FixedStartingIntentRead;
/// fn reuse(read: FixedStartingIntentRead<'_,'_,'_,'_,'_,'_,'_,'_,'_>) { let _ = read.quarantine(); let _ = read.quarantine(); }
/// ```
pub struct FixedStartingQuarantine<
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
    _read: FixedStartingIntentRead<
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
    disposition: QuarantineDisposition,
}
impl FixedStartingQuarantine<'_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn disposition(&self) -> &QuarantineDisposition {
        &self.disposition
    }
}
impl fmt::Debug for FixedStartingQuarantine<'_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FixedStartingQuarantine")
            .field("disposition", &self.disposition)
            .finish_non_exhaustive()
    }
}

impl<'join, 'storage, 'intent, 'authority, 'tasks, 'binding, 'guard, 'policy, 'process>
    FixedStartingIntentRead<
        'join,
        'storage,
        'intent,
        'authority,
        'tasks,
        'binding,
        'guard,
        'policy,
        'process,
    >
{
    /// Permanently decline this original attempt without inferring absent writers.
    /// This is revocation, not a renewal of expired launch consent.
    pub fn quarantine(
        self,
    ) -> FixedStartingQuarantine<
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
        self.quarantine_with(|journal, reason| journal.refuse(reason))
    }

    pub(super) fn quarantine_with(
        mut self,
        publish: impl FnOnce(
            &mut super::super::Journal<'_>,
            RefusalReason,
        ) -> Result<(), AuthorityError>,
    ) -> FixedStartingQuarantine<
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
        let disposition = if self.is_refused() {
            // Never retry an uncertain or previously refused authority append.
            self.read.read.storage.latch_refusal();
            self.read.read.original.original.latch_original();
            QuarantineDisposition::PreviouslyRefused
        } else {
            // Latch before persistence; failure cannot return a healthy capability.
            self.read.read.storage.latch_refusal();
            let original = &mut self.read.read.original.original;
            original.latch_original();
            match original.persist_with(RefusalReason::Policy, publish) {
                Ok(()) => QuarantineDisposition::Acknowledged,
                Err(error) => QuarantineDisposition::Uncertain(error),
            }
        };
        FixedStartingQuarantine {
            _read: self,
            disposition,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StartingRecord {
    schema_version: String,
    previous_state_sha256: String,
    initial_intent_record_sha256: String,
    original_signed_intent_sha256: String,
    next_state: DiagnosticState,
}
pub struct FixedStartingIntentRead<
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
    pub(super) read: FixedConsumedInitialIntentRead<
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
pub struct StartingIntentFailure<
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
    >,
}
impl fmt::Debug for StartingIntentFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(f)
    }
}
impl fmt::Display for StartingIntentFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(f)
    }
}
impl std::error::Error for StartingIntentFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_> {}
impl StartingIntentFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn error(&self) -> &ReservationPublicationError {
        &self.error
    }
    pub fn is_refused(&self) -> bool {
        self.read.is_refused()
    }
}
impl fmt::Debug for FixedStartingIntentRead<'_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FixedStartingIntentRead")
            .field("refused", &self.is_refused())
            .finish_non_exhaustive()
    }
}
impl FixedStartingIntentRead<'_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn is_refused(&self) -> bool {
        self.read.is_refused()
    }
    pub fn revalidate(
        &mut self,
        bytes: &[u8],
        check: &IntentCheck<'_>,
    ) -> Result<(), ReservationPublicationError> {
        if self.is_refused() {
            return Err(refuse(&mut self.read, ReservationPublicationError::Refused));
        }
        if let Err(failure) = self.read.read.storage.revalidate_starting() {
            return Err(refuse(&mut self.read, failure.into()));
        }
        self.read.revalidate(bytes, check)?;
        self.read
            .read
            .storage
            .revalidate_starting()
            .map_err(|failure| refuse(&mut self.read, failure.into()))
    }
}
fn refuse(
    read: &mut FixedConsumedInitialIntentRead<'_, '_, '_, '_, '_, '_, '_, '_, '_>,
    failure: ReservationPublicationError,
) -> ReservationPublicationError {
    reservation_publication::refuse_with(
        read.read.original,
        &mut read.read.storage,
        failure,
        |journal, reason| journal.refuse(reason),
    )
}
pub(super) fn valid_departure(previous: &DiagnosticState, next: &DiagnosticState) -> bool {
    previous.stage == DiagnosticStage::Reserved
        && previous.revision == 1
        && previous.completed_cells == 0
        && next.stage == DiagnosticStage::Starting
        && next.revision == 2
        && next.completed_cells == 0
        && validate_transition(previous, next).is_ok()
}
pub fn retain_fixed_starting_intent<
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
    read: FixedConsumedInitialIntentRead<
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
    next: &DiagnosticState,
) -> Result<
    FixedStartingIntentRead<
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
    StartingIntentFailure<
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
    retain_with(read, bytes, check, next, |_| Ok(()))
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
    mut read: FixedConsumedInitialIntentRead<
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
    next: &DiagnosticState,
    mut checkpoint: impl FnMut(StartingStage) -> Result<(), ReservationPublicationError>,
) -> Result<
    FixedStartingIntentRead<
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
    StartingIntentFailure<
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
        && (!read.read.original.pins.matches_state(check.state)
            || !read.read.storage.has_owned_attempt()
            || !valid_departure(check.state, next))
    {
        return Err(StartingIntentFailure {
            error: ReservationPublicationError::Binding,
            read: Box::new(read),
        });
    }
    if let Err(error) = read.revalidate(bytes, check) {
        return Err(StartingIntentFailure {
            error,
            read: Box::new(read),
        });
    }
    let result = (|| {
        let previous =
            encode(check.state).map_err(|_| StoragePublicationError::ConsumptionDocument)?;
        let record = StartingRecord {
            schema_version: STARTING_SCHEMA.into(),
            previous_state_sha256: sha256_hex(&previous),
            initial_intent_record_sha256: read.read.storage.initial_record_sha256()?,
            original_signed_intent_sha256: sha256_hex(bytes),
            next_state: next.clone(),
        };
        let mut raw =
            canonical_json(&record).map_err(|_| StoragePublicationError::ConsumptionDocument)?;
        raw.push(b'\n');
        if raw.len() > MAX_BYTES {
            return Err(StoragePublicationError::ConsumptionDocument.into());
        }
        read.read.storage.publish_starting_with(&raw, |at| {
            checkpoint(at)?;
            read.read.original.revalidate(bytes, check)?;
            Ok(())
        })
    })();
    if let Err(failure) = result {
        let error = refuse(&mut read, failure);
        return Err(StartingIntentFailure {
            error,
            read: Box::new(read),
        });
    }
    let mut starting = FixedStartingIntentRead { read };
    if let Err(error) = starting.revalidate(bytes, check) {
        return Err(StartingIntentFailure {
            error,
            read: Box::new(starting.read),
        });
    }
    Ok(starting)
}
#[cfg(test)]
pub(super) const ALL_STARTING_STAGES: [StartingStage; 8] = [
    StartingStage::BeforeCreate,
    StartingStage::Created,
    StartingStage::PrefixWritten,
    StartingStage::BytesWritten,
    StartingStage::Sealed,
    StartingStage::FileSynced,
    StartingStage::DirectorySynced,
    StartingStage::Acknowledging,
];
