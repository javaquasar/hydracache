//! Original signed mount plus bounded original tasks; no host/start authority.
use super::{
    FixedSignedMountRead, FixtureSignedMountRead, Reader as MountReader, SignedMountError,
};
use crate::diagnostic_process::{NamespaceCredentialRead, TaskError, WorkerTasksRead};
use crate::diagnostic_worker_policy::{AssertedWorkerHost, WorkerPolicy, WorkerPolicyTrust};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SignedTasksError {
    #[error(transparent)]
    Signed(#[from] SignedMountError),
    #[error(transparent)]
    Tasks(#[from] TaskError),
    #[error("original signed task mapping differs")]
    Mapping,
    #[error("original signed task reader permanently refused")]
    Refused,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Signed,
    Open,
    Tasks,
}
const OPENING: [Step; 5] = [
    Step::Signed,
    Step::Open,
    Step::Signed,
    Step::Tasks,
    Step::Signed,
];
const LATER: [Step; 3] = [Step::Signed, Step::Tasks, Step::Signed];
#[derive(Default)]
struct Gate {
    refused: bool,
}
impl Gate {
    fn observe(
        &mut self,
        input_refused: bool,
        opening: bool,
        mut read: impl FnMut(Step) -> Result<(), SignedTasksError>,
    ) -> Result<(), SignedTasksError> {
        if self.refused || input_refused {
            self.refused = true;
            return Err(SignedTasksError::Refused);
        }
        for step in if opening { &OPENING[..] } else { &LATER[..] } {
            if let Err(error) = read(*step) {
                self.refused = true;
                return Err(error);
            }
        }
        Ok(())
    }
}
enum Original<'tasks, 'binding, 'guard, 'policy, 'process> {
    Fixed(&'tasks mut FixedSignedMountRead<'binding, 'guard, 'policy, 'process>),
    Fixture(&'tasks mut FixtureSignedMountRead<'binding, 'guard, 'policy, 'process>),
}
impl<'binding, 'guard, 'policy, 'process> Original<'_, 'binding, 'guard, 'policy, 'process> {
    fn reader(&self) -> &MountReader<'binding, 'guard, 'policy, 'process> {
        match self {
            Self::Fixed(read) => &read.reader,
            Self::Fixture(read) => &read.reader,
        }
    }
    fn reader_mut(&mut self) -> &mut MountReader<'binding, 'guard, 'policy, 'process> {
        match self {
            Self::Fixed(read) => &mut read.reader,
            Self::Fixture(read) => &mut read.reader,
        }
    }
    fn refuse(&mut self) {
        let read = self.reader_mut();
        read.gate.refused = true;
        read.mount.refuse();
        read.original.refuse();
    }
    fn policy(&self) -> &WorkerPolicy {
        &self
            .reader()
            .original
            .reader()
            .account
            .reader()
            .context
            .policy
            .original
    }
}
struct Reader<'tasks, 'binding, 'guard, 'policy, 'process> {
    original: Original<'tasks, 'binding, 'guard, 'policy, 'process>,
    tasks: WorkerTasksRead<'process>,
    gate: Gate,
}
fn check_tasks(
    tasks: &mut WorkerTasksRead<'_>,
    policy: &WorkerPolicy,
) -> Result<(), SignedTasksError> {
    if !tasks.matches_worker_policy(policy) {
        return Err(SignedTasksError::Mapping);
    }
    tasks.revalidate()?;
    Ok(())
}
impl Reader<'_, '_, '_, '_, '_> {
    fn is_refused(&self) -> bool {
        self.gate.refused || self.original.reader().is_refused() || self.tasks.is_refused()
    }
    fn revalidate(
        &mut self,
        bytes: &[u8],
        trust: &WorkerPolicyTrust,
        host: &AssertedWorkerHost,
    ) -> Result<(), SignedTasksError> {
        let input_refused = self.is_refused();
        let result = self.gate.observe(input_refused, false, |step| match step {
            Step::Signed => self
                .original
                .reader_mut()
                .revalidate(bytes, trust, host)
                .map_err(Into::into),
            Step::Tasks => check_tasks(&mut self.tasks, self.original.policy()),
            Step::Open => Err(SignedTasksError::Mapping),
        });
        if result.is_err() {
            self.original.refuse();
            self.tasks.refuse();
        }
        result
    }
}
fn construct<'tasks, 'binding, 'guard, 'policy, 'process>(
    mut original: Original<'tasks, 'binding, 'guard, 'policy, 'process>,
    bytes: &[u8],
    trust: &WorkerPolicyTrust,
    host: &AssertedWorkerHost,
    open: impl FnOnce(
        &NamespaceCredentialRead<'process>,
        &WorkerPolicy,
    ) -> Result<WorkerTasksRead<'process>, TaskError>,
) -> Result<Reader<'tasks, 'binding, 'guard, 'policy, 'process>, SignedTasksError> {
    let mut gate = Gate::default();
    let mut tasks = None;
    let mut open = Some(open);
    let result = gate.observe(original.reader().is_refused(), true, |step| match step {
        Step::Signed => original
            .reader_mut()
            .revalidate(bytes, trust, host)
            .map_err(Into::into),
        Step::Open => {
            tasks = Some(open.take().ok_or(SignedTasksError::Mapping)?(
                &original.reader().original.reader().credentials,
                original.policy(),
            )?);
            Ok(())
        }
        Step::Tasks => check_tasks(
            tasks.as_mut().ok_or(SignedTasksError::Mapping)?,
            original.policy(),
        ),
    });
    if let Err(error) = result {
        original.refuse();
        if let Some(tasks) = &mut tasks {
            tasks.refuse();
        }
        return Err(error);
    }
    let Some(tasks) = tasks else {
        original.refuse();
        return Err(SignedTasksError::Mapping);
    };
    Ok(Reader {
        original,
        tasks,
        gate,
    })
}
/// Exclusive original fixed mount borrow, not controlled-host enrollment.
pub struct FixedSignedTasksRead<'tasks, 'binding, 'guard, 'policy, 'process> {
    reader: Reader<'tasks, 'binding, 'guard, 'policy, 'process>,
}
/// Fixture origin remains distinct and cannot become production authority.
pub struct FixtureSignedTasksRead<'tasks, 'binding, 'guard, 'policy, 'process> {
    reader: Reader<'tasks, 'binding, 'guard, 'policy, 'process>,
}
pub fn open_signed_fixed_tasks<'tasks, 'binding, 'guard, 'policy, 'process>(
    original: &'tasks mut FixedSignedMountRead<'binding, 'guard, 'policy, 'process>,
    bytes: &[u8],
    trust: &WorkerPolicyTrust,
    host: &AssertedWorkerHost,
) -> Result<FixedSignedTasksRead<'tasks, 'binding, 'guard, 'policy, 'process>, SignedTasksError> {
    Ok(FixedSignedTasksRead {
        reader: construct(
            Original::Fixed(original),
            bytes,
            trust,
            host,
            NamespaceCredentialRead::pin_original_tasks,
        )?,
    })
}
pub fn open_signed_fixture_tasks<'tasks, 'binding, 'guard, 'policy, 'process>(
    original: &'tasks mut FixtureSignedMountRead<'binding, 'guard, 'policy, 'process>,
    bytes: &[u8],
    trust: &WorkerPolicyTrust,
    host: &AssertedWorkerHost,
) -> Result<FixtureSignedTasksRead<'tasks, 'binding, 'guard, 'policy, 'process>, SignedTasksError> {
    Ok(FixtureSignedTasksRead {
        reader: construct(
            Original::Fixture(original),
            bytes,
            trust,
            host,
            NamespaceCredentialRead::pin_original_tasks,
        )?,
    })
}
impl FixedSignedTasksRead<'_, '_, '_, '_, '_> {
    pub fn is_refused(&self) -> bool {
        self.reader.is_refused()
    }
    pub fn revalidate(
        &mut self,
        bytes: &[u8],
        trust: &WorkerPolicyTrust,
        host: &AssertedWorkerHost,
    ) -> Result<(), SignedTasksError> {
        self.reader.revalidate(bytes, trust, host)
    }
}
impl FixtureSignedTasksRead<'_, '_, '_, '_, '_> {
    pub fn is_refused(&self) -> bool {
        self.reader.is_refused()
    }
    pub fn revalidate(
        &mut self,
        bytes: &[u8],
        trust: &WorkerPolicyTrust,
        host: &AssertedWorkerHost,
    ) -> Result<(), SignedTasksError> {
        self.reader.revalidate(bytes, trust, host)
    }
}
#[cfg(test)]
#[path = "diagnostic_signed_tasks_tests.rs"]
mod tests;
