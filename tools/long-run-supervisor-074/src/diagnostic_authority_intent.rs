//! Original authority and signed expected intent only, not publication or start.
#[path = "diagnostic_early_attempt.rs"]
pub mod early_attempt;
#[path = "diagnostic_empty_fenced_intent.rs"]
pub mod empty_fence;
#[path = "diagnostic_fenced_intent.rs"]
pub mod fenced_state;
#[path = "diagnostic_initial_consumption.rs"]
pub mod initial_consumption;
#[path = "diagnostic_reservation_publication.rs"]
pub mod reservation_publication;
use super::{FixedAuthorityBoundRead, FixtureAuthorityBoundRead, Reader, SignedAuthorityError};
use crate::diagnostic_worker_policy::authority_registry::{AuthorityError, RefusalReason};
use crate::diagnostic_worker_policy::intent_projection::{
    capture_bound_intent, BoundIntentPins, IntentCheck, IntentProjectionError,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthorityIntentError {
    #[error("original authority refused: {0}")]
    Authority(#[from] SignedAuthorityError),
    #[error("signed intent projection refused: {0}")]
    Projection(#[from] IntentProjectionError),
    #[error("original intent/authority binding permanently refused")]
    Refused,
    #[error("{failure}; original authority refusal not acknowledged: {publication}")]
    RefusalUncertain {
        failure: Box<Self>,
        publication: AuthorityError,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Authority,
    Intent,
}
fn bracket(
    mut check: impl FnMut(Step) -> Result<(), AuthorityIntentError>,
) -> Result<(), AuthorityIntentError> {
    for step in [Step::Authority, Step::Intent, Step::Authority] {
        check(step)?;
    }
    Ok(())
}

struct IntentReader<'intent, 'authority, 'tasks, 'binding, 'guard, 'policy, 'process> {
    original: &'intent mut Reader<'authority, 'tasks, 'binding, 'guard, 'policy, 'process>,
    pins: BoundIntentPins,
}
impl IntentReader<'_, '_, '_, '_, '_, '_, '_> {
    fn is_refused(&self) -> bool {
        self.original.is_refused()
    }
    fn reject_projection_with(
        &mut self,
        failure: IntentProjectionError,
        publish: impl FnOnce(&mut super::Journal<'_>, RefusalReason) -> Result<(), AuthorityError>,
    ) -> AuthorityIntentError {
        self.original.latch_original();
        let failure = AuthorityIntentError::Projection(failure);
        match self.original.persist_with(RefusalReason::Policy, publish) {
            Ok(()) => failure,
            Err(publication) => AuthorityIntentError::RefusalUncertain {
                failure: Box::new(failure),
                publication,
            },
        }
    }
    fn revalidate(
        &mut self,
        bytes: &[u8],
        check: &IntentCheck<'_>,
    ) -> Result<(), AuthorityIntentError> {
        if self.is_refused() {
            return Err(AuthorityIntentError::Refused);
        }
        bracket(|step| match step {
            Step::Authority => self
                .original
                .revalidate(check.worker_bytes, check.worker_trust, check.worker_host)
                .map_err(Into::into),
            Step::Intent => {
                let observed = capture_bound_intent(
                    self.original.original.reader().original.checked_policy(),
                    bytes,
                    check,
                );
                let result = observed.and_then(|pins| {
                    if pins == self.pins {
                        Ok(())
                    } else {
                        Err(IntentProjectionError::Drift)
                    }
                });
                result.map_err(|error| {
                    self.reject_projection_with(error, |journal, reason| journal.refuse(reason))
                })
            }
        })
    }
}
fn construct<'intent, 'authority, 'tasks, 'binding, 'guard, 'policy, 'process>(
    original: &'intent mut Reader<'authority, 'tasks, 'binding, 'guard, 'policy, 'process>,
    bytes: &[u8],
    check: &IntentCheck<'_>,
) -> Result<
    IntentReader<'intent, 'authority, 'tasks, 'binding, 'guard, 'policy, 'process>,
    AuthorityIntentError,
> {
    if original.is_refused() {
        return Err(AuthorityIntentError::Refused);
    }
    // Unbound caller bytes are not authority to revoke a healthy original worker.
    let pins = capture_bound_intent(
        original.original.reader().original.checked_policy(),
        bytes,
        check,
    )?;
    let mut reader = IntentReader { original, pins };
    reader.revalidate(bytes, check)?;
    Ok(reader)
}

/// Exclusive original fixed authority borrow, with no publication/start conversion.
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::{FixedAuthorityBoundRead, intent_binding::bind_fixed_intent};
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::intent_projection::IntentCheck;
/// fn reuse(original: &mut FixedAuthorityBoundRead<'_, '_, '_, '_, '_, '_>, bytes: &[u8], check: &IntentCheck<'_>) {
///     let mut bound = bind_fixed_intent(original, bytes, check).unwrap();
///     original.revalidate(check.worker_bytes, check.worker_trust, check.worker_host).unwrap();
///     bound.revalidate(bytes, check).unwrap();
/// }
/// ```
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::FixedAuthorityIntentRead;
/// fn duplicate(read: FixedAuthorityIntentRead<'_, '_, '_, '_, '_, '_, '_>) { let _ = read.clone(); }
/// ```
pub struct FixedAuthorityIntentRead<
    'intent,
    'authority,
    'tasks,
    'binding,
    'guard,
    'policy,
    'process,
> {
    reader: IntentReader<'intent, 'authority, 'tasks, 'binding, 'guard, 'policy, 'process>,
}
/// Distinct fixture-origin guard; neither cloneable nor promotable.
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::{FixtureAuthorityIntentRead, FixedAuthorityIntentRead};
/// fn promote<'a>(read: FixtureAuthorityIntentRead<'a, 'a, 'a, 'a, 'a, 'a, 'a>) -> FixedAuthorityIntentRead<'a, 'a, 'a, 'a, 'a, 'a, 'a> { read.into() }
/// ```
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::FixtureAuthorityIntentRead;
/// fn start(read: &mut FixtureAuthorityIntentRead<'_, '_, '_, '_, '_, '_, '_>) { read.start_once(); }
/// ```
pub struct FixtureAuthorityIntentRead<
    'intent,
    'authority,
    'tasks,
    'binding,
    'guard,
    'policy,
    'process,
> {
    reader: IntentReader<'intent, 'authority, 'tasks, 'binding, 'guard, 'policy, 'process>,
}
pub fn bind_fixed_intent<'intent, 'authority, 'tasks, 'binding, 'guard, 'policy, 'process>(
    original: &'intent mut FixedAuthorityBoundRead<
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
    FixedAuthorityIntentRead<'intent, 'authority, 'tasks, 'binding, 'guard, 'policy, 'process>,
    AuthorityIntentError,
> {
    Ok(FixedAuthorityIntentRead {
        reader: construct(&mut original.reader, bytes, check)?,
    })
}
pub fn bind_fixture_intent<'intent, 'authority, 'tasks, 'binding, 'guard, 'policy, 'process>(
    original: &'intent mut FixtureAuthorityBoundRead<
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
    FixtureAuthorityIntentRead<'intent, 'authority, 'tasks, 'binding, 'guard, 'policy, 'process>,
    AuthorityIntentError,
> {
    Ok(FixtureAuthorityIntentRead {
        reader: construct(&mut original.reader, bytes, check)?,
    })
}
macro_rules! methods {
    ($kind:ident) => {
        impl $kind<'_, '_, '_, '_, '_, '_, '_> {
            pub fn is_refused(&self) -> bool {
                self.reader.is_refused()
            }
            pub fn revalidate(
                &mut self,
                bytes: &[u8],
                check: &IntentCheck<'_>,
            ) -> Result<(), AuthorityIntentError> {
                self.reader.revalidate(bytes, check)
            }
        }
    };
}
methods!(FixedAuthorityIntentRead);
methods!(FixtureAuthorityIntentRead);

#[cfg(test)]
#[path = "diagnostic_authority_intent_tests.rs"]
mod tests;
