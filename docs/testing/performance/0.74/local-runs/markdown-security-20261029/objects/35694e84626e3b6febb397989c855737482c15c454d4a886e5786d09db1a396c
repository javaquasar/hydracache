//! Read-only original intent/state composition, never publisher or execution.
//!
//! ```compile_fail
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::{FixedAuthorityIntentRead, fenced_state::bind_fixed_fenced_intent};
//! use hydracache_long_run_supervisor_074::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::FencedReservedStateRead;
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::intent_projection::IntentCheck;
//! fn reuse(original: &mut FixedAuthorityIntentRead<'_,'_,'_,'_,'_,'_,'_>, fence: &mut FencedReservedStateRead<'_,'_>, bytes: &[u8], check: &IntentCheck<'_>) {
//!     let mut read = bind_fixed_fenced_intent(original, fence, bytes, check).unwrap();
//!     original.revalidate(bytes, check).unwrap();
//!     read.revalidate(bytes, check).unwrap();
//! }
//! ```
//!
//! ```compile_fail
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::fenced_state::{FixtureFencedIntentRead, FixedFencedIntentRead};
//! fn promote<'a>(read: FixtureFencedIntentRead<'a,'a,'a,'a,'a,'a,'a,'a,'a,'a>) -> FixedFencedIntentRead<'a,'a,'a,'a,'a,'a,'a,'a,'a,'a> { read.into() }
//! ```
//!
//! ```compile_fail
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::fenced_state::FixedFencedIntentRead;
//! fn duplicate(read: FixedFencedIntentRead<'_,'_,'_,'_,'_,'_,'_,'_,'_,'_>) { let _ = read.clone(); }
//! ```
//!
//! ```compile_fail
//! use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::fenced_state::FixtureFencedIntentRead;
//! fn start(read: &mut FixtureFencedIntentRead<'_,'_,'_,'_,'_,'_,'_,'_,'_,'_>) { read.start_once(); }
//! ```

use super::{
    AuthorityIntentError, FixedAuthorityIntentRead, FixtureAuthorityIntentRead, IntentReader,
};
use crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::{
    FencedReservedStateRead, OriginalHostFenceError,
};
use crate::diagnostic_worker_policy::authority_registry::{AuthorityError, RefusalReason};
use crate::diagnostic_worker_policy::intent_projection::IntentCheck;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FencedIntentError {
    #[error("original authority/intent refused: {0}")]
    Intent(#[from] AuthorityIntentError),
    #[error("original state/fence refused: {0}")]
    Fence(#[from] OriginalHostFenceError),
    #[error("full expected Reserved snapshots do not match")]
    Binding,
    #[error("original intent/state composition permanently refused")]
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
    mut check: impl FnMut(Step) -> Result<(), FencedIntentError>,
) -> Result<(), FencedIntentError> {
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
    'read,
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
    fence: &'join mut FencedReservedStateRead<'read, 'storage>,
}
impl Reader<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn is_refused(&self) -> bool {
        self.original.is_refused() || self.fence.is_refused()
    }
    fn latch_both(&mut self) {
        self.original.original.latch_original();
        self.fence.latch_refusal();
    }
    pub(super) fn reject_fence_with(
        &mut self,
        failure: FencedIntentError,
        publish: impl FnOnce(
            &mut super::super::Journal<'_>,
            RefusalReason,
        ) -> Result<(), AuthorityError>,
    ) -> FencedIntentError {
        self.latch_both();
        match self
            .original
            .original
            .persist_with(RefusalReason::Policy, publish)
        {
            Ok(()) => failure,
            Err(publication) => FencedIntentError::RefusalUncertain {
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
    ) -> Result<(), FencedIntentError> {
        if self.is_refused() {
            self.latch_both();
            return Err(FencedIntentError::Refused);
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
            Err(failure @ FencedIntentError::Intent(_)) => {
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
fn construct<
    'join,
    'read,
    'storage,
    'intent,
    'authority,
    'tasks,
    'binding,
    'guard,
    'policy,
    'process,
>(
    original: &'join mut IntentReader<
        'intent,
        'authority,
        'tasks,
        'binding,
        'guard,
        'policy,
        'process,
    >,
    fence: &'join mut FencedReservedStateRead<'read, 'storage>,
    bytes: &[u8],
    check: &IntentCheck<'_>,
) -> Result<
    Reader<
        'join,
        'read,
        'storage,
        'intent,
        'authority,
        'tasks,
        'binding,
        'guard,
        'policy,
        'process,
    >,
    FencedIntentError,
> {
    let mut read = Reader { original, fence };
    if read.is_refused() {
        read.latch_both();
        return Err(FencedIntentError::Refused);
    }
    // Caller assertions cannot revoke healthy inputs before composition.
    if !read.original.pins.matches_state(check.state) || !read.fence.matches_expected(check.state) {
        return Err(FencedIntentError::Binding);
    }
    read.observe_with(bytes, check, |_| {})?;
    Ok(read)
}
macro_rules! define {
    ($kind:ident, $origin:ident, $bind:ident) => {
        /// Exclusive original intent and state/fence borrows; no start conversion.
        pub struct $kind<
            'join,
            'read,
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
                'read,
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
            'read,
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
            fence: &'join mut FencedReservedStateRead<'read, 'storage>,
            bytes: &[u8],
            check: &IntentCheck<'_>,
        ) -> Result<
            $kind<
                'join,
                'read,
                'storage,
                'intent,
                'authority,
                'tasks,
                'binding,
                'guard,
                'policy,
                'process,
            >,
            FencedIntentError,
        > {
            Ok($kind {
                reader: construct(&mut original.reader, fence, bytes, check)?,
            })
        }
        impl $kind<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
            pub fn is_refused(&self) -> bool {
                self.reader.is_refused()
            }
            pub fn revalidate(
                &mut self,
                bytes: &[u8],
                check: &IntentCheck<'_>,
            ) -> Result<(), FencedIntentError> {
                self.reader.observe_with(bytes, check, |_| {})
            }
        }
    };
}
define!(
    FixedFencedIntentRead,
    FixedAuthorityIntentRead,
    bind_fixed_fenced_intent
);
define!(
    FixtureFencedIntentRead,
    FixtureAuthorityIntentRead,
    bind_fixture_fenced_intent
);
