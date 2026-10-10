//! Original observed context brackets account opening/reads, not worker/start enrollment.
use super::super::local_files::{FileSet, WorkerFilesError};
use super::super::{AssertedWorkerHost, CheckedWorkerPolicy, WorkerPolicyTrust};
use super::{inspect_worker_context, WorkerContextError, WorkerContextRead};
use std::path::Path;
use thiserror::Error;

#[path = "diagnostic_policy_credentials.rs"]
pub mod kernel_binding;

#[derive(Debug, Error)]
pub enum ContextFilesError {
    #[error("original context/account composition previously refused")]
    Refused,
    #[error(transparent)]
    Context(#[from] WorkerContextError),
    #[error(transparent)]
    Files(#[from] WorkerFilesError),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Context,
    Open,
    Files,
}
const OPENING: [Step; 5] = [
    Step::Context,
    Step::Open,
    Step::Context,
    Step::Files,
    Step::Context,
];
const LATER: [Step; 3] = [Step::Context, Step::Files, Step::Context];
#[derive(Default)]
struct Gate {
    refused: bool,
}
impl Gate {
    fn observe(
        &mut self,
        input_refused: bool,
        opening: bool,
        mut read: impl FnMut(Step) -> Result<(), ContextFilesError>,
    ) -> Result<(), ContextFilesError> {
        if self.refused || input_refused {
            self.refused = true;
            return Err(ContextFilesError::Refused);
        }
        let order: &[Step] = if opening { &OPENING } else { &LATER };
        for step in order {
            if let Err(error) = read(*step) {
                self.refused = true;
                return Err(error);
            }
        }
        Ok(())
    }
}
struct Reader<'policy> {
    context: WorkerContextRead<'policy>,
    files: FileSet,
    gate: Gate,
}
/// Fixed root-owned account names, opened here; no production start authority.
pub struct ContextFixedFilesRead<'policy> {
    reader: Reader<'policy>,
}
/// Explicit caller-owned origin, never convertible to fixed production proof.
pub struct ContextFixtureFilesRead<'policy> {
    reader: Reader<'policy>,
}

pub fn inspect_context_fixed_files<'policy>(
    policy: &'policy mut CheckedWorkerPolicy,
    bytes: &[u8],
    trust: &WorkerPolicyTrust,
    host: &AssertedWorkerHost,
) -> Result<ContextFixedFilesRead<'policy>, ContextFilesError> {
    construct(policy, bytes, trust, host, FileSet::open_fixed)
        .map(|reader| ContextFixedFilesRead { reader })
}
pub fn inspect_context_fixture_files<'policy>(
    path: &Path,
    uid: u32,
    gid: u32,
    policy: &'policy mut CheckedWorkerPolicy,
    bytes: &[u8],
    trust: &WorkerPolicyTrust,
    host: &AssertedWorkerHost,
) -> Result<ContextFixtureFilesRead<'policy>, ContextFilesError> {
    construct(policy, bytes, trust, host, || {
        FileSet::open_fixture(path, uid, gid)
    })
    .map(|reader| ContextFixtureFilesRead { reader })
}
fn construct<'policy>(
    policy: &'policy mut CheckedWorkerPolicy,
    bytes: &[u8],
    trust: &WorkerPolicyTrust,
    host: &AssertedWorkerHost,
    open: impl FnOnce() -> Result<FileSet, WorkerFilesError>,
) -> Result<Reader<'policy>, ContextFilesError> {
    if policy.is_refused() {
        return Err(ContextFilesError::Refused);
    }
    // No preopened account/context reader can enter. Context construction gates all IO.
    let mut context = inspect_worker_context(policy, bytes, trust, host)?;
    let mut files = None;
    let mut open = Some(open);
    let mut gate = Gate::default();
    let result = gate.observe(context.is_refused(), true, |step| match step {
        Step::Context => context.revalidate(bytes, trust, host).map_err(Into::into),
        Step::Open => {
            let opener = open.take().ok_or(WorkerFilesError::Security)?;
            files = Some(opener()?);
            Ok(())
        }
        Step::Files => files
            .as_ref()
            .ok_or(WorkerFilesError::Security)?
            .observe(&context.policy.original)
            .map_err(Into::into),
    });
    if let Err(error) = result {
        context.policy.refused = true;
        return Err(error);
    }
    let Some(files) = files else {
        context.policy.refused = true;
        return Err(WorkerFilesError::Security.into());
    };
    Ok(Reader {
        context,
        files,
        gate,
    })
}
impl Reader<'_> {
    fn is_refused(&self) -> bool {
        self.gate.refused || self.context.is_refused()
    }
    fn revalidate(
        &mut self,
        bytes: &[u8],
        trust: &WorkerPolicyTrust,
        host: &AssertedWorkerHost,
    ) -> Result<(), ContextFilesError> {
        let refused = self.is_refused();
        let result = self.gate.observe(refused, false, |step| match step {
            Step::Context => self
                .context
                .revalidate(bytes, trust, host)
                .map_err(Into::into),
            Step::Files => self
                .files
                .observe(&self.context.policy.original)
                .map_err(Into::into),
            Step::Open => Err(WorkerFilesError::Security.into()),
        });
        if result.is_err() {
            self.context.policy.refused = true;
        }
        result
    }
}
impl ContextFixedFilesRead<'_> {
    pub fn is_refused(&self) -> bool {
        self.reader.is_refused()
    }
    pub fn revalidate(
        &mut self,
        bytes: &[u8],
        trust: &WorkerPolicyTrust,
        host: &AssertedWorkerHost,
    ) -> Result<(), ContextFilesError> {
        self.reader.revalidate(bytes, trust, host)
    }
}
impl ContextFixtureFilesRead<'_> {
    pub fn is_refused(&self) -> bool {
        self.reader.is_refused()
    }
    pub fn revalidate(
        &mut self,
        bytes: &[u8],
        trust: &WorkerPolicyTrust,
        host: &AssertedWorkerHost,
    ) -> Result<(), ContextFilesError> {
        self.reader.revalidate(bytes, trust, host)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn context_files_opening_and_later_orders_are_exact() {
        let mut gate = Gate::default();
        let mut order = Vec::new();
        gate.observe(false, true, |step| {
            order.push(step);
            Ok(())
        })
        .unwrap();
        assert_eq!(order, OPENING);
        order.clear();
        gate.observe(false, false, |step| {
            order.push(step);
            Ok(())
        })
        .unwrap();
        assert_eq!(order, LATER);
        assert!(!order.contains(&Step::Open));
        assert!(!gate.refused);
    }
    #[test]
    fn context_files_every_failure_stops_and_preserves_original_error() {
        for opening in [false, true] {
            let length = if opening { 5 } else { 3 };
            for failed in 0..length {
                let mut gate = Gate::default();
                let mut calls = 0;
                let error = gate
                    .observe(false, opening, |step| {
                        let current = calls;
                        calls += 1;
                        if current != failed {
                            return Ok(());
                        }
                        match step {
                            Step::Context => Err(WorkerContextError::Drift.into()),
                            Step::Open | Step::Files => Err(WorkerFilesError::Io(
                                std::io::Error::from_raw_os_error(libc::EACCES),
                            )
                            .into()),
                        }
                    })
                    .unwrap_err();
                assert_eq!(calls, failed + 1);
                assert!(gate.refused);
                match if opening {
                    OPENING[failed]
                } else {
                    LATER[failed]
                } {
                    Step::Context => assert!(matches!(
                        error,
                        ContextFilesError::Context(WorkerContextError::Drift)
                    )),
                    _ => assert!(
                        matches!(error,ContextFilesError::Files(WorkerFilesError::Io(e)) if e.raw_os_error()==Some(libc::EACCES))
                    ),
                }
                assert!(matches!(
                    gate.observe(false, opening, |_| panic!("no observation after refusal")),
                    Err(ContextFilesError::Refused)
                ));
            }
        }
    }
    #[test]
    fn context_files_prior_refusal_never_opens_or_reads() {
        for opening in [false, true] {
            let mut gate = Gate::default();
            assert!(matches!(
                gate.observe(true, opening, |_| panic!("no IO")),
                Err(ContextFilesError::Refused)
            ));
            assert!(matches!(
                gate.observe(false, opening, |_| panic!("no refresh")),
                Err(ContextFilesError::Refused)
            ));
        }
    }
    #[test]
    fn context_files_both_origin_guards_cannot_be_sent_or_shared() {
        trait AmbiguousSend<A> {
            fn marker() {}
        }
        impl<T: ?Sized> AmbiguousSend<()> for T {}
        impl<T: ?Sized + Send> AmbiguousSend<u8> for T {}
        let _ = <ContextFixedFilesRead<'static> as AmbiguousSend<_>>::marker;
        let _ = <ContextFixtureFilesRead<'static> as AmbiguousSend<_>>::marker;
        trait AmbiguousSync<A> {
            fn marker() {}
        }
        impl<T: ?Sized> AmbiguousSync<()> for T {}
        impl<T: ?Sized + Sync> AmbiguousSync<u8> for T {}
        let _ = <ContextFixedFilesRead<'static> as AmbiguousSync<_>>::marker;
        let _ = <ContextFixtureFilesRead<'static> as AmbiguousSync<_>>::marker;
    }
}
