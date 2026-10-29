//! Original signed reader + original leader mount; no enrollment or launch authority.
use super::PolicyCredentialError;
use super::{FixedSignedCredentialRead, FixtureSignedCredentialRead, Reader as SignedReader};
use crate::diagnostic_process::{
    MountNamespaceError, NamespaceCredentialRead, SameMountNamespaceRead,
};
use crate::diagnostic_worker_policy::{AssertedWorkerHost, WorkerPolicyTrust};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SignedMountError {
    #[error(transparent)]
    Signed(#[from] PolicyCredentialError),
    #[error(transparent)]
    Mount(#[from] MountNamespaceError),
    #[error("original signed mount mapping differs")]
    Mapping,
    #[error("original signed mount reader permanently refused")]
    Refused,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Signed,
    Open,
    Mount,
}
const OPENING: [Step; 5] = [
    Step::Signed,
    Step::Open,
    Step::Signed,
    Step::Mount,
    Step::Signed,
];
const LATER: [Step; 3] = [Step::Signed, Step::Mount, Step::Signed];
#[derive(Default)]
struct Gate {
    refused: bool,
}
impl Gate {
    fn observe(
        &mut self,
        input_refused: bool,
        opening: bool,
        mut read: impl FnMut(Step) -> Result<(), SignedMountError>,
    ) -> Result<(), SignedMountError> {
        if self.refused || input_refused {
            self.refused = true;
            return Err(SignedMountError::Refused);
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
enum Original<'binding, 'guard, 'policy, 'process> {
    Fixed(&'binding mut FixedSignedCredentialRead<'guard, 'policy, 'process>),
    Fixture(&'binding mut FixtureSignedCredentialRead<'guard, 'policy, 'process>),
}
impl<'guard, 'policy, 'process> Original<'_, 'guard, 'policy, 'process> {
    fn is_refused(&self) -> bool {
        self.reader().is_refused() || self.reader().credentials.original_mount_inputs_refused()
    }
    fn reader(&self) -> &SignedReader<'guard, 'policy, 'process> {
        match self {
            Self::Fixed(r) => &r.reader,
            Self::Fixture(r) => &r.reader,
        }
    }
    fn reader_mut(&mut self) -> &mut SignedReader<'guard, 'policy, 'process> {
        match self {
            Self::Fixed(r) => &mut r.reader,
            Self::Fixture(r) => &mut r.reader,
        }
    }
    fn refuse(&mut self) {
        let r = self.reader_mut();
        r.gate.refused = true;
        r.account.refuse();
        r.credentials.refuse();
    }
}
struct Reader<'binding, 'guard, 'policy, 'process> {
    original: Original<'binding, 'guard, 'policy, 'process>,
    mount: SameMountNamespaceRead<'process>,
    gate: Gate,
}
fn check_mount(
    mount: &mut SameMountNamespaceRead<'_>,
    original: &SignedReader<'_, '_, '_>,
) -> Result<(), SignedMountError> {
    if !mount.matches_worker_policy(&original.account.reader().context.policy.original) {
        return Err(SignedMountError::Mapping);
    }
    mount.revalidate()?;
    Ok(())
}
impl Reader<'_, '_, '_, '_> {
    fn is_refused(&self) -> bool {
        self.gate.refused || self.original.is_refused() || self.mount.is_refused()
    }
    fn revalidate(
        &mut self,
        bytes: &[u8],
        trust: &WorkerPolicyTrust,
        host: &AssertedWorkerHost,
    ) -> Result<(), SignedMountError> {
        let input_refused = self.is_refused();
        let result = self.gate.observe(input_refused, false, |step| match step {
            Step::Signed => self
                .original
                .reader_mut()
                .revalidate(bytes, trust, host)
                .map_err(Into::into),
            Step::Mount => check_mount(&mut self.mount, self.original.reader()),
            Step::Open => Err(SignedMountError::Mapping),
        });
        if result.is_err() {
            self.original.refuse();
            self.mount.refuse();
        }
        result
    }
}
fn construct<'binding, 'guard, 'policy, 'process>(
    mut original: Original<'binding, 'guard, 'policy, 'process>,
    bytes: &[u8],
    trust: &WorkerPolicyTrust,
    host: &AssertedWorkerHost,
    open: impl FnOnce(
        &NamespaceCredentialRead<'process>,
    ) -> Result<SameMountNamespaceRead<'process>, MountNamespaceError>,
) -> Result<Reader<'binding, 'guard, 'policy, 'process>, SignedMountError> {
    let mut gate = Gate::default();
    let mut mount = None;
    let mut open = Some(open);
    let result = gate.observe(original.is_refused(), true, |step| match step {
        Step::Signed => original
            .reader_mut()
            .revalidate(bytes, trust, host)
            .map_err(Into::into),
        Step::Open => {
            mount = Some(open.take().ok_or(SignedMountError::Mapping)?(
                &original.reader().credentials,
            )?);
            Ok(())
        }
        Step::Mount => check_mount(
            mount.as_mut().ok_or(SignedMountError::Mapping)?,
            original.reader(),
        ),
    });
    if let Err(error) = result {
        original.refuse();
        if let Some(read) = &mut mount {
            read.refuse();
        }
        return Err(error);
    }
    let Some(mount) = mount else {
        original.refuse();
        return Err(SignedMountError::Mapping);
    };
    Ok(Reader {
        original,
        mount,
        gate,
    })
}
/// Fixed origin stays distinct; this is not production enrollment.
pub struct FixedSignedMountRead<'binding, 'guard, 'policy, 'process> {
    reader: Reader<'binding, 'guard, 'policy, 'process>,
}
/// Fixture authority cannot be converted into fixed production authority.
pub struct FixtureSignedMountRead<'binding, 'guard, 'policy, 'process> {
    reader: Reader<'binding, 'guard, 'policy, 'process>,
}
pub fn open_signed_fixed_mount<'binding, 'guard, 'policy, 'process>(
    original: &'binding mut FixedSignedCredentialRead<'guard, 'policy, 'process>,
    bytes: &[u8],
    trust: &WorkerPolicyTrust,
    host: &AssertedWorkerHost,
) -> Result<FixedSignedMountRead<'binding, 'guard, 'policy, 'process>, SignedMountError> {
    Ok(FixedSignedMountRead {
        reader: construct(
            Original::Fixed(original),
            bytes,
            trust,
            host,
            NamespaceCredentialRead::pin_original_mount,
        )?,
    })
}
pub fn open_signed_fixture_mount<'binding, 'guard, 'policy, 'process>(
    original: &'binding mut FixtureSignedCredentialRead<'guard, 'policy, 'process>,
    bytes: &[u8],
    trust: &WorkerPolicyTrust,
    host: &AssertedWorkerHost,
) -> Result<FixtureSignedMountRead<'binding, 'guard, 'policy, 'process>, SignedMountError> {
    Ok(FixtureSignedMountRead {
        reader: construct(
            Original::Fixture(original),
            bytes,
            trust,
            host,
            NamespaceCredentialRead::pin_original_mount,
        )?,
    })
}
impl FixedSignedMountRead<'_, '_, '_, '_> {
    pub fn is_refused(&self) -> bool {
        self.reader.is_refused()
    }
    pub fn revalidate(
        &mut self,
        bytes: &[u8],
        trust: &WorkerPolicyTrust,
        host: &AssertedWorkerHost,
    ) -> Result<(), SignedMountError> {
        self.reader.revalidate(bytes, trust, host)
    }
}
impl FixtureSignedMountRead<'_, '_, '_, '_> {
    pub fn is_refused(&self) -> bool {
        self.reader.is_refused()
    }
    pub fn revalidate(
        &mut self,
        bytes: &[u8],
        trust: &WorkerPolicyTrust,
        host: &AssertedWorkerHost,
    ) -> Result<(), SignedMountError> {
        self.reader.revalidate(bytes, trust, host)
    }
}
#[path = "diagnostic_signed_tasks.rs"]
pub mod task_binding;
#[cfg(test)]
#[path = "diagnostic_signed_mount_tests.rs"]
mod tests;
