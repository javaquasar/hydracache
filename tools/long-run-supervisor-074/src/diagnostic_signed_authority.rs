//! Runtime refusal bridge only; no launch capability or crash-intent protocol.
use super::{
    FixedSignedTasksRead, FixtureSignedTasksRead, Reader as TasksReader, SignedTasksError,
};
use crate::diagnostic_worker_policy::authority_registry::fixture::{
    fixed::FixedAuthorityRegistry, FixtureAuthorityRegistry,
};
use crate::diagnostic_worker_policy::authority_registry::{
    AuthorityError, AuthorityView, RefusalReason,
};
use crate::diagnostic_worker_policy::{AssertedWorkerHost, WorkerPolicyTrust};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SignedAuthorityError {
    #[error(transparent)]
    Signed(#[from] SignedTasksError),
    #[error(transparent)]
    Authority(#[from] AuthorityError),
    #[error("original authority already durably refused")]
    Inactive,
    #[error("original authority binding permanently refused")]
    Refused,
    #[error("{failure}; refusal publication not acknowledged: {persistence}")]
    RefusalUncertain {
        failure: Box<Self>,
        persistence: AuthorityError,
    },
    #[error("operator refusal publication not acknowledged: {0}")]
    Publication(AuthorityError),
}

enum Original<'authority, 'tasks, 'binding, 'guard, 'policy, 'process> {
    Fixed(&'authority mut FixedSignedTasksRead<'tasks, 'binding, 'guard, 'policy, 'process>),
    Fixture(&'authority mut FixtureSignedTasksRead<'tasks, 'binding, 'guard, 'policy, 'process>),
}
impl<'tasks, 'binding, 'guard, 'policy, 'process>
    Original<'_, 'tasks, 'binding, 'guard, 'policy, 'process>
{
    fn reader(&self) -> &TasksReader<'tasks, 'binding, 'guard, 'policy, 'process> {
        match self {
            Self::Fixed(read) => &read.reader,
            Self::Fixture(read) => &read.reader,
        }
    }
    fn reader_mut(&mut self) -> &mut TasksReader<'tasks, 'binding, 'guard, 'policy, 'process> {
        match self {
            Self::Fixed(read) => &mut read.reader,
            Self::Fixture(read) => &mut read.reader,
        }
    }
}
enum Journal<'authority> {
    Fixed(&'authority mut FixedAuthorityRegistry),
    Fixture(&'authority mut FixtureAuthorityRegistry),
}
impl Journal<'_> {
    fn is_failed(&self) -> bool {
        match self {
            Self::Fixed(read) => read.is_failed(),
            Self::Fixture(read) => read.is_failed(),
        }
    }
    fn inspect(
        &mut self,
        trust: &WorkerPolicyTrust,
        host: &AssertedWorkerHost,
    ) -> Result<AuthorityView, AuthorityError> {
        match self {
            Self::Fixed(read) => read.inspect(trust, host),
            Self::Fixture(read) => read.inspect(trust, host),
        }
    }
    fn refuse(&mut self, reason: RefusalReason) -> Result<(), AuthorityError> {
        match self {
            Self::Fixed(read) => read.refuse(reason),
            Self::Fixture(read) => read.refuse(reason),
        }
    }
    fn latch_refusal(&mut self) {
        match self {
            Self::Fixed(read) => read.latch_refusal(),
            Self::Fixture(read) => read.latch_refusal(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Journal,
    Signed,
}
const BRACKET: [Step; 3] = [Step::Journal, Step::Signed, Step::Journal];
fn bracket(
    mut check: impl FnMut(Step) -> Result<(), SignedAuthorityError>,
) -> Result<(), SignedAuthorityError> {
    for step in BRACKET {
        check(step)?;
    }
    Ok(())
}
struct Reader<'authority, 'tasks, 'binding, 'guard, 'policy, 'process> {
    original: Original<'authority, 'tasks, 'binding, 'guard, 'policy, 'process>,
    journal: Journal<'authority>,
    original_trust: WorkerPolicyTrust,
    original_host: AssertedWorkerHost,
    refused: bool,
}
impl Reader<'_, '_, '_, '_, '_, '_> {
    fn is_refused(&self) -> bool {
        self.refused || self.original.reader().is_refused() || self.journal.is_failed()
    }
    fn latch_original(&mut self) {
        self.refused = true;
        self.original.reader_mut().refuse();
    }
    fn persist_with(
        &mut self,
        reason: RefusalReason,
        publish: impl FnOnce(&mut Journal<'_>, RefusalReason) -> Result<(), AuthorityError>,
    ) -> Result<(), AuthorityError> {
        let result = (|| {
            let view = self
                .journal
                .inspect(&self.original_trust, &self.original_host)?;
            if !view.refused {
                publish(&mut self.journal, reason)?;
            }
            Ok(())
        })();
        if result.is_err() {
            self.journal.latch_refusal();
        }
        result
    }
    fn reject_with(
        &mut self,
        failure: SignedAuthorityError,
        publish: impl FnOnce(&mut Journal<'_>, RefusalReason) -> Result<(), AuthorityError>,
    ) -> SignedAuthorityError {
        self.latch_original();
        match self.persist_with(RefusalReason::Policy, publish) {
            Ok(()) => failure,
            Err(persistence) => SignedAuthorityError::RefusalUncertain {
                failure: Box::new(failure),
                persistence,
            },
        }
    }
    fn revalidate(
        &mut self,
        bytes: &[u8],
        trust: &WorkerPolicyTrust,
        host: &AssertedWorkerHost,
    ) -> Result<(), SignedAuthorityError> {
        if self.refused {
            return Err(SignedAuthorityError::Refused);
        }
        if self.journal.is_failed() {
            self.latch_original();
            return Err(SignedAuthorityError::Authority(AuthorityError::Refused));
        }
        if self.original.reader().is_refused() {
            return Err(self.reject_with(
                SignedAuthorityError::Signed(SignedTasksError::Refused),
                |journal, reason| journal.refuse(reason),
            ));
        }
        let result = bracket(|step| match step {
            Step::Journal => {
                let view = self
                    .journal
                    .inspect(&self.original_trust, &self.original_host)?;
                if view.refused {
                    Err(SignedAuthorityError::Inactive)
                } else {
                    Ok(())
                }
            }
            Step::Signed => self
                .original
                .reader_mut()
                .revalidate(bytes, trust, host)
                .map_err(Into::into),
        });
        match result {
            Ok(()) => Ok(()),
            Err(error @ SignedAuthorityError::Signed(_)) => {
                Err(self.reject_with(error, |journal, reason| journal.refuse(reason)))
            }
            Err(error) => {
                self.latch_original();
                if !matches!(error, SignedAuthorityError::Inactive) {
                    self.journal.latch_refusal();
                }
                Err(error)
            }
        }
    }
    fn refuse(&mut self, reason: RefusalReason) -> Result<(), SignedAuthorityError> {
        if self.refused {
            return Err(SignedAuthorityError::Refused);
        }
        self.latch_original();
        self.persist_with(reason, |journal, reason| journal.refuse(reason))
            .map_err(SignedAuthorityError::Publication)
    }
}
fn construct<'authority, 'tasks, 'binding, 'guard, 'policy, 'process>(
    original: Original<'authority, 'tasks, 'binding, 'guard, 'policy, 'process>,
    journal: Journal<'authority>,
    bytes: &[u8],
    trust: &WorkerPolicyTrust,
    host: &AssertedWorkerHost,
) -> Result<Reader<'authority, 'tasks, 'binding, 'guard, 'policy, 'process>, SignedAuthorityError> {
    let checked = original.reader().original.checked_policy();
    let original_trust = checked.trust.clone();
    let original_host = checked.context.clone();
    let mut read = Reader {
        original,
        journal,
        original_trust,
        original_host,
        refused: false,
    };
    read.revalidate(bytes, trust, host)?;
    Ok(read)
}
/// Exclusive fixed-origin runtime guard, not production enrollment or restart permission.
pub struct FixedAuthorityBoundRead<'authority, 'tasks, 'binding, 'guard, 'policy, 'process> {
    reader: Reader<'authority, 'tasks, 'binding, 'guard, 'policy, 'process>,
}
/// Fixture origin cannot be promoted into the fixed public API.
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::FixtureAuthorityBoundRead;
/// fn copy(read: &FixtureAuthorityBoundRead<'_, '_, '_, '_, '_, '_>) {
///     let _ = (*read).clone();
/// }
/// ```
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::{FixtureAuthorityBoundRead, FixedAuthorityBoundRead};
/// fn promote<'a>(read: FixtureAuthorityBoundRead<'a, 'a, 'a, 'a, 'a, 'a>) -> FixedAuthorityBoundRead<'a, 'a, 'a, 'a, 'a, 'a> {
///     read.into()
/// }
/// ```
pub struct FixtureAuthorityBoundRead<'authority, 'tasks, 'binding, 'guard, 'policy, 'process> {
    reader: Reader<'authority, 'tasks, 'binding, 'guard, 'policy, 'process>,
}
pub fn bind_fixed_authority<'authority, 'tasks, 'binding, 'guard, 'policy, 'process>(
    original: &'authority mut FixedSignedTasksRead<'tasks, 'binding, 'guard, 'policy, 'process>,
    journal: &'authority mut FixedAuthorityRegistry,
    bytes: &[u8],
    trust: &WorkerPolicyTrust,
    host: &AssertedWorkerHost,
) -> Result<
    FixedAuthorityBoundRead<'authority, 'tasks, 'binding, 'guard, 'policy, 'process>,
    SignedAuthorityError,
> {
    Ok(FixedAuthorityBoundRead {
        reader: construct(
            Original::Fixed(original),
            Journal::Fixed(journal),
            bytes,
            trust,
            host,
        )?,
    })
}
pub fn bind_fixture_authority<'authority, 'tasks, 'binding, 'guard, 'policy, 'process>(
    original: &'authority mut FixtureSignedTasksRead<'tasks, 'binding, 'guard, 'policy, 'process>,
    journal: &'authority mut FixtureAuthorityRegistry,
    bytes: &[u8],
    trust: &WorkerPolicyTrust,
    host: &AssertedWorkerHost,
) -> Result<
    FixtureAuthorityBoundRead<'authority, 'tasks, 'binding, 'guard, 'policy, 'process>,
    SignedAuthorityError,
> {
    Ok(FixtureAuthorityBoundRead {
        reader: construct(
            Original::Fixture(original),
            Journal::Fixture(journal),
            bytes,
            trust,
            host,
        )?,
    })
}
macro_rules! bound_methods {
    ($kind:ident) => {
        impl $kind<'_, '_, '_, '_, '_, '_> {
            pub fn is_refused(&self) -> bool {
                self.reader.is_refused()
            }
            pub fn revalidate(
                &mut self,
                bytes: &[u8],
                trust: &WorkerPolicyTrust,
                host: &AssertedWorkerHost,
            ) -> Result<(), SignedAuthorityError> {
                self.reader.revalidate(bytes, trust, host)
            }
            pub fn refuse(&mut self, reason: RefusalReason) -> Result<(), SignedAuthorityError> {
                self.reader.refuse(reason)
            }
        }
    };
}
bound_methods!(FixedAuthorityBoundRead);
bound_methods!(FixtureAuthorityBoundRead);
#[cfg(test)]
#[path = "diagnostic_signed_authority_tests.rs"]
mod tests;
