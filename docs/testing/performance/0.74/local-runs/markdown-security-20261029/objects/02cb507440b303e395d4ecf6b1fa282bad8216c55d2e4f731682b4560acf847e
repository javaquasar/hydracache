//! Read-only original intent/empty-fence composition, never publisher or execution.
//!
//! ```compile_fail
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::{FixedAuthorityIntentRead, empty_fence::bind_fixed_empty_fenced_intent};
//! use hydracache_long_run_supervisor_074::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::OriginalEmptyReservationFence;
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::intent_projection::IntentCheck;
//! fn reuse(original: &mut FixedAuthorityIntentRead<'_,'_,'_,'_,'_,'_,'_>, fence: &mut OriginalEmptyReservationFence<'_>, bytes: &[u8], check: &IntentCheck<'_>) {
//!     let mut read = bind_fixed_empty_fenced_intent(original, fence, bytes, check).unwrap();
//!     original.revalidate(bytes, check).unwrap();
//!     read.revalidate(bytes, check).unwrap();
//! }
//! ```
//!
//! ```compile_fail
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::empty_fence::{FixtureEmptyFencedIntentRead, FixedEmptyFencedIntentRead};
//! fn promote<'a>(read: FixtureEmptyFencedIntentRead<'a,'a,'a,'a,'a,'a,'a,'a,'a>) -> FixedEmptyFencedIntentRead<'a,'a,'a,'a,'a,'a,'a,'a,'a> { read.into() }
//! ```
//!
//! ```compile_fail
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::empty_fence::FixedEmptyFencedIntentRead;
//! fn duplicate(read: FixedEmptyFencedIntentRead<'_,'_,'_,'_,'_,'_,'_,'_,'_>) { let _ = read.clone(); }
//! ```
//!
//! ```compile_fail
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::empty_fence::FixtureEmptyFencedIntentRead;
//! fn start(read: &mut FixtureEmptyFencedIntentRead<'_,'_,'_,'_,'_,'_,'_,'_,'_>) { read.start_once(); }
//! ```

use super::{
    AuthorityIntentError, FixedAuthorityIntentRead, FixtureAuthorityIntentRead, IntentReader,
};
use crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::{
    OriginalEmptyReservationFence, EmptyReservationFenceError,
};
use crate::diagnostic_worker_policy::authority_registry::{AuthorityError, RefusalReason};
use crate::diagnostic_worker_policy::intent_projection::IntentCheck;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EmptyFencedIntentError {
    #[error("original authority/intent refused: {0}")]
    Intent(#[from] AuthorityIntentError),
    #[error("original empty storage/fence refused: {0}")]
    Fence(#[from] EmptyReservationFenceError),
    #[error("asserted Reserved snapshot does not match original intent")]
    Binding,
    #[error("original intent/empty-fence composition permanently refused")]
    Refused,
    #[error("{failure}; original authority refusal not acknowledged: {publication}")]
    RefusalUncertain {
        failure: Box<Self>,
        publication: AuthorityError,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Step {
    Fence,
    Intent,
}
pub(super) fn bracket(
    mut check: impl FnMut(Step) -> Result<(), EmptyFencedIntentError>,
) -> Result<(), EmptyFencedIntentError> {
    for step in [
        Step::Fence,
        Step::Intent,
        Step::Fence,
        Step::Intent,
        Step::Fence,
    ] {
        check(step)?;
    }
    Ok(())
}

pub(super) struct Reader<
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
    fence: &'join mut OriginalEmptyReservationFence<'storage>,
}
impl Reader<'_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn is_refused(&self) -> bool {
        self.original.is_refused() || self.fence.is_refused()
    }
    fn latch_both(&mut self) {
        self.original.original.latch_original();
        self.fence.latch_refusal();
    }
    pub(super) fn reject_fence_with(
        &mut self,
        failure: EmptyFencedIntentError,
        publish: impl FnOnce(
            &mut super::super::Journal<'_>,
            RefusalReason,
        ) -> Result<(), AuthorityError>,
    ) -> EmptyFencedIntentError {
        self.latch_both();
        match self
            .original
            .original
            .persist_with(RefusalReason::Policy, publish)
        {
            Ok(()) => failure,
            Err(publication) => EmptyFencedIntentError::RefusalUncertain {
                failure: Box::new(failure),
                publication,
            },
        }
    }
    pub(super) fn observe_with(
        &mut self,
        bytes: &[u8],
        check: &IntentCheck<'_>,
        mut before: impl FnMut(Step),
    ) -> Result<(), EmptyFencedIntentError> {
        if self.is_refused() {
            self.latch_both();
            return Err(EmptyFencedIntentError::Refused);
        }
        let result = bracket(|step| {
            before(step);
            match step {
                Step::Fence => self.fence.revalidate().map_err(Into::into),
                Step::Intent => self.original.revalidate(bytes, check).map_err(Into::into),
            }
        });
        match result {
            Ok(()) => Ok(()),
            Err(failure @ EmptyFencedIntentError::Intent(_)) => {
                // Intent's original journal bridge already handled refusal.
                self.latch_both();
                Err(failure)
            }
            Err(failure) => {
                Err(self.reject_fence_with(failure, |journal, reason| journal.refuse(reason)))
            }
        }
    }
}
fn construct<'join, 'storage, 'intent, 'authority, 'tasks, 'binding, 'guard, 'policy, 'process>(
    original: &'join mut IntentReader<
        'intent,
        'authority,
        'tasks,
        'binding,
        'guard,
        'policy,
        'process,
    >,
    fence: &'join mut OriginalEmptyReservationFence<'storage>,
    bytes: &[u8],
    check: &IntentCheck<'_>,
) -> Result<
    Reader<'join, 'storage, 'intent, 'authority, 'tasks, 'binding, 'guard, 'policy, 'process>,
    EmptyFencedIntentError,
> {
    let mut read = Reader { original, fence };
    if read.is_refused() {
        read.latch_both();
        return Err(EmptyFencedIntentError::Refused);
    }
    // Caller assertions cannot revoke healthy inputs before composition.
    // The empty leaf has no persisted snapshot; matching intent pins is not publication.
    if !read.original.pins.matches_state(check.state) {
        return Err(EmptyFencedIntentError::Binding);
    }
    read.observe_with(bytes, check, |_| {})?;
    Ok(read)
}
macro_rules! define {
    ($kind:ident, $origin:ident, $bind:ident) => {
        /// Exclusive original intent and empty-fence borrows; no start conversion.
        pub struct $kind<
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
            pub(super) reader: Reader<
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
        pub fn $bind<
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
            original: &'join mut $origin<
                'intent,
                'authority,
                'tasks,
                'binding,
                'guard,
                'policy,
                'process,
            >,
            fence: &'join mut OriginalEmptyReservationFence<'storage>,
            bytes: &[u8],
            check: &IntentCheck<'_>,
        ) -> Result<
            $kind<
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
            EmptyFencedIntentError,
        > {
            Ok($kind {
                reader: construct(&mut original.reader, fence, bytes, check)?,
            })
        }
        impl $kind<'_, '_, '_, '_, '_, '_, '_, '_, '_> {
            pub fn is_refused(&self) -> bool {
                self.reader.is_refused()
            }
            pub fn revalidate(
                &mut self,
                bytes: &[u8],
                check: &IntentCheck<'_>,
            ) -> Result<(), EmptyFencedIntentError> {
                self.reader.observe_with(bytes, check, |_| {})
            }
        }
    };
}
define!(
    FixedEmptyFencedIntentRead,
    FixedAuthorityIntentRead,
    bind_fixed_empty_fenced_intent
);
define!(
    FixtureEmptyFencedIntentRead,
    FixtureAuthorityIntentRead,
    bind_fixture_empty_fenced_intent
);
