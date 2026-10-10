//! Original signed context/account and namespace-checked numeric consistency, not start authority.
use super::{
    AssertedWorkerHost, ContextFilesError, ContextFixedFilesRead, ContextFixtureFilesRead,
    WorkerPolicyTrust,
};
use crate::diagnostic_process::{NamespaceCredentialError, NamespaceCredentialRead};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PolicyCredentialError {
    #[error("original signed mapping and original credential assertions differ")]
    Mapping,
    #[error("original policy/credential binding previously refused")]
    Refused,
    #[error(transparent)]
    Account(#[from] ContextFilesError),
    #[error(transparent)]
    Credentials(#[from] NamespaceCredentialError),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Account,
    Credentials,
}
#[derive(Default)]
struct Gate {
    refused: bool,
}
impl Gate {
    fn observe(
        &mut self,
        input_refused: bool,
        mapping_matches: bool,
        mut read: impl FnMut(Step) -> Result<(), PolicyCredentialError>,
    ) -> Result<(), PolicyCredentialError> {
        if self.refused || input_refused {
            self.refused = true;
            return Err(PolicyCredentialError::Refused);
        }
        if !mapping_matches {
            self.refused = true;
            return Err(PolicyCredentialError::Mapping);
        }
        for step in [Step::Account, Step::Credentials, Step::Account] {
            if let Err(error) = read(step) {
                self.refused = true;
                return Err(error);
            }
        }
        Ok(())
    }
}
enum Account<'guard, 'policy> {
    Fixed(&'guard mut ContextFixedFilesRead<'policy>),
    Fixture(&'guard mut ContextFixtureFilesRead<'policy>),
}
impl<'policy> Account<'_, 'policy> {
    fn reader(&self) -> &super::Reader<'policy> {
        match self {
            Self::Fixed(a) => &a.reader,
            Self::Fixture(a) => &a.reader,
        }
    }
    fn reader_mut(&mut self) -> &mut super::Reader<'policy> {
        match self {
            Self::Fixed(a) => &mut a.reader,
            Self::Fixture(a) => &mut a.reader,
        }
    }
    fn refuse(&mut self) {
        let reader = self.reader_mut();
        reader.gate.refused = true;
        reader.context.policy.refused = true;
    }
}
struct Reader<'guard, 'policy, 'process> {
    account: Account<'guard, 'policy>,
    credentials: &'guard mut NamespaceCredentialRead<'process>,
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
        let mapping_matches = self
            .credentials
            .matches_worker_policy(&self.account.reader().context.policy.original);
        let result = self
            .gate
            .observe(input_refused, mapping_matches, |step| match step {
                Step::Account => self
                    .account
                    .reader_mut()
                    .revalidate(bytes, trust, host)
                    .map_err(Into::into),
                Step::Credentials => self.credentials.revalidate().map_err(Into::into),
            });
        if result.is_err() {
            self.account.refuse();
            self.credentials.refuse();
        }
        result
    }
}
/// Fixed account-file origin only; no production preparation or start capability.
pub struct FixedPolicyCredentialRead<'guard, 'policy, 'process> {
    reader: Reader<'guard, 'policy, 'process>,
}
/// Explicit owned-fixture origin; never convertible to fixed production proof.
pub struct FixturePolicyCredentialRead<'guard, 'policy, 'process> {
    reader: Reader<'guard, 'policy, 'process>,
}
pub fn bind_context_fixed_credentials<'guard, 'policy, 'process>(
    account: &'guard mut ContextFixedFilesRead<'policy>,
    credentials: &'guard mut NamespaceCredentialRead<'process>,
    bytes: &[u8],
    trust: &WorkerPolicyTrust,
    host: &AssertedWorkerHost,
) -> Result<FixedPolicyCredentialRead<'guard, 'policy, 'process>, PolicyCredentialError> {
    let mut reader = Reader {
        account: Account::Fixed(account),
        credentials,
        gate: Gate::default(),
    };
    reader.revalidate(bytes, trust, host)?;
    Ok(FixedPolicyCredentialRead { reader })
}
pub fn bind_context_fixture_credentials<'guard, 'policy, 'process>(
    account: &'guard mut ContextFixtureFilesRead<'policy>,
    credentials: &'guard mut NamespaceCredentialRead<'process>,
    bytes: &[u8],
    trust: &WorkerPolicyTrust,
    host: &AssertedWorkerHost,
) -> Result<FixturePolicyCredentialRead<'guard, 'policy, 'process>, PolicyCredentialError> {
    let mut reader = Reader {
        account: Account::Fixture(account),
        credentials,
        gate: Gate::default(),
    };
    reader.revalidate(bytes, trust, host)?;
    Ok(FixturePolicyCredentialRead { reader })
}
impl FixedPolicyCredentialRead<'_, '_, '_> {
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
impl FixturePolicyCredentialRead<'_, '_, '_> {
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
#[path = "diagnostic_policy_credentials_tests.rs"]
mod tests;
