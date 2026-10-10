//! Signed observer context brackets original status opening; no start authority.
#[cfg(test)]
use super::ContextFilesError;
use super::{
    Account, AssertedWorkerHost, ContextFixedFilesRead, ContextFixtureFilesRead,
    PolicyCredentialError, WorkerPolicyTrust,
};
use crate::diagnostic_process::{
    open_namespace_checked_credentials_unobserved, AssertedWorkerCredentials,
    NamespaceCredentialError, NamespaceCredentialRead, ProcessRead,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Account,
    Open,
    Credentials,
}
const OPENING: [Step; 5] = [
    Step::Account,
    Step::Open,
    Step::Account,
    Step::Credentials,
    Step::Account,
];
const LATER: [Step; 3] = [Step::Account, Step::Credentials, Step::Account];
#[derive(Default)]
struct Gate {
    refused: bool,
}
impl Gate {
    fn observe(
        &mut self,
        input_refused: bool,
        opening: bool,
        mut read: impl FnMut(Step) -> Result<(), PolicyCredentialError>,
    ) -> Result<(), PolicyCredentialError> {
        if self.refused || input_refused {
            self.refused = true;
            return Err(PolicyCredentialError::Refused);
        }
        let steps: &[Step] = if opening { &OPENING } else { &LATER };
        for step in steps {
            if let Err(error) = read(*step) {
                self.refused = true;
                return Err(error);
            }
        }
        Ok(())
    }
}
struct Reader<'guard, 'policy, 'process> {
    account: Account<'guard, 'policy>,
    credentials: NamespaceCredentialRead<'process>,
    gate: Gate,
}
impl Reader<'_, '_, '_> {
    fn is_refused(&self) -> bool {
        self.gate.refused || self.account.reader().is_refused() || self.credentials.is_refused()
    }
    fn revalidate(
        &mut self,
        bytes: &[u8],
        trust: &WorkerPolicyTrust,
        host: &AssertedWorkerHost,
    ) -> Result<(), PolicyCredentialError> {
        let input_refused = self.is_refused();
        let result = self.gate.observe(input_refused, false, |step| match step {
            Step::Account => self
                .account
                .reader_mut()
                .revalidate(bytes, trust, host)
                .map_err(Into::into),
            Step::Credentials => {
                self.credentials.revalidate()?;
                if !self
                    .credentials
                    .matches_worker_policy(&self.account.reader().context.policy.original)
                {
                    return Err(PolicyCredentialError::Mapping);
                }
                Ok(())
            }
            Step::Open => Err(PolicyCredentialError::Mapping),
        });
        if result.is_err() {
            self.account.refuse();
            self.credentials.refuse();
        }
        result
    }
}
fn construct<'guard, 'policy, 'process>(
    mut account: Account<'guard, 'policy>,
    process: &'process ProcessRead,
    bytes: &[u8],
    trust: &WorkerPolicyTrust,
    host: &AssertedWorkerHost,
    open: impl FnOnce(
        &'process ProcessRead,
        AssertedWorkerCredentials,
    ) -> Result<NamespaceCredentialRead<'process>, NamespaceCredentialError>,
) -> Result<Reader<'guard, 'policy, 'process>, PolicyCredentialError> {
    let mut gate = Gate::default();
    let mut credentials: Option<NamespaceCredentialRead<'process>> = None;
    let mut open = Some(open);
    let result = gate.observe(account.reader().is_refused(), true, |step| match step {
        Step::Account => account
            .reader_mut()
            .revalidate(bytes, trust, host)
            .map_err(Into::into),
        Step::Open => {
            let policy = &account.reader().context.policy.original;
            // Only this original checked signed mapping supplies the numbers.
            // Its supplementary list is bounded to 32; no primary-GID union.
            let asserted = AssertedWorkerCredentials::new(
                policy.uid,
                policy.gid,
                policy.supplementary_gids.clone(),
            )
            .map_err(NamespaceCredentialError::from)?;
            credentials = Some(open.take().ok_or(PolicyCredentialError::Mapping)?(
                process, asserted,
            )?);
            Ok(())
        }
        Step::Credentials => {
            let read = credentials.as_mut().ok_or(PolicyCredentialError::Mapping)?;
            read.revalidate()?;
            if !read.matches_worker_policy(&account.reader().context.policy.original) {
                return Err(PolicyCredentialError::Mapping);
            }
            Ok(())
        }
    });
    if let Err(error) = result {
        account.refuse();
        if let Some(read) = &mut credentials {
            read.refuse();
        }
        return Err(error);
    }
    let Some(credentials) = credentials else {
        account.refuse();
        return Err(PolicyCredentialError::Mapping);
    };
    Ok(Reader {
        account,
        credentials,
        gate,
    })
}
/// Fixed file origin, with newly owned credentials; no production enrollment.
pub struct FixedSignedCredentialRead<'guard, 'policy, 'process> {
    reader: Reader<'guard, 'policy, 'process>,
}
/// Explicit fixture origin, never convertible to fixed production proof.
pub struct FixtureSignedCredentialRead<'guard, 'policy, 'process> {
    reader: Reader<'guard, 'policy, 'process>,
}
pub fn open_signed_fixed_credentials<'guard, 'policy, 'process>(
    account: &'guard mut ContextFixedFilesRead<'policy>,
    process: &'process ProcessRead,
    bytes: &[u8],
    trust: &WorkerPolicyTrust,
    host: &AssertedWorkerHost,
) -> Result<FixedSignedCredentialRead<'guard, 'policy, 'process>, PolicyCredentialError> {
    let reader = construct(
        Account::Fixed(account),
        process,
        bytes,
        trust,
        host,
        open_namespace_checked_credentials_unobserved,
    )?;
    Ok(FixedSignedCredentialRead { reader })
}
pub fn open_signed_fixture_credentials<'guard, 'policy, 'process>(
    account: &'guard mut ContextFixtureFilesRead<'policy>,
    process: &'process ProcessRead,
    bytes: &[u8],
    trust: &WorkerPolicyTrust,
    host: &AssertedWorkerHost,
) -> Result<FixtureSignedCredentialRead<'guard, 'policy, 'process>, PolicyCredentialError> {
    let reader = construct(
        Account::Fixture(account),
        process,
        bytes,
        trust,
        host,
        open_namespace_checked_credentials_unobserved,
    )?;
    Ok(FixtureSignedCredentialRead { reader })
}
impl FixedSignedCredentialRead<'_, '_, '_> {
    pub fn is_refused(&self) -> bool {
        self.reader.is_refused()
    }
    pub fn revalidate(
        &mut self,
        bytes: &[u8],
        trust: &WorkerPolicyTrust,
        host: &AssertedWorkerHost,
    ) -> Result<(), PolicyCredentialError> {
        self.reader.revalidate(bytes, trust, host)
    }
}
impl FixtureSignedCredentialRead<'_, '_, '_> {
    pub fn is_refused(&self) -> bool {
        self.reader.is_refused()
    }
    pub fn revalidate(
        &mut self,
        bytes: &[u8],
        trust: &WorkerPolicyTrust,
        host: &AssertedWorkerHost,
    ) -> Result<(), PolicyCredentialError> {
        self.reader.revalidate(bytes, trust, host)
    }
}
#[cfg(test)]
#[path = "diagnostic_signed_opening_tests.rs"]
mod tests;
