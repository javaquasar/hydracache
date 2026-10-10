//! Original read-only execution observations, never execution or stop authority.
use crate::diagnostic_artifacts::{linux::PinnedStartMaterial, ArtifactError};
use crate::diagnostic_lease::DiagnosticState;
use crate::diagnostic_live_identity::{IdentityError, LiveIdentityRead};
use crate::diagnostic_manager::ManagerClient;
use crate::diagnostic_named_output::{NamedOutputError, ProductionOutputRead};
use crate::diagnostic_process::{ProcessError, ProcessIoRead};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExecutionError {
    #[error("original execution inputs have incompatible state or origin")]
    Invalid,
    #[error("original execution observation previously refused")]
    Refused,
    #[error(transparent)]
    Identity(#[from] IdentityError),
    #[error(transparent)]
    Outputs(#[from] NamedOutputError),
    #[error(transparent)]
    Material(#[from] ArtifactError),
    #[error(transparent)]
    Io(#[from] ProcessError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Identity,
    Outputs,
    Material,
    Io,
}

#[derive(Default)]
struct ExecutionGuard {
    refused: bool,
}
impl ExecutionGuard {
    fn revalidate_with(
        &mut self,
        input_refused: bool,
        mut observe: impl FnMut(Step) -> Result<(), ExecutionError>,
    ) -> Result<(), ExecutionError> {
        if self.refused || input_refused {
            self.refused = true;
            return Err(ExecutionError::Refused);
        }
        for step in [
            Step::Identity,
            Step::Outputs,
            Step::Material,
            Step::Io,
            Step::Material,
            Step::Outputs,
            Step::Identity,
        ] {
            if let Err(error) = observe(step) {
                self.refused = true;
                return Err(error);
            }
        }
        Ok(())
    }
}

/// Borrows all original guards; cannot export descriptors or replace inputs.
/// Success is sequential consistency, not authenticated original start.
pub struct LiveExecutionRead<'guard, 'process> {
    identity: &'guard mut LiveIdentityRead<'process>,
    outputs: &'guard mut ProductionOutputRead,
    material: &'guard mut PinnedStartMaterial,
    io: ProcessIoRead<'process>,
    state: DiagnosticState,
    guard: ExecutionGuard,
}

fn refuse_inputs(
    identity: &mut LiveIdentityRead<'_>,
    outputs: &mut ProductionOutputRead,
    material: &mut PinnedStartMaterial,
) {
    identity.refuse();
    outputs.refuse();
    material.refuse();
}

/// Only the identity reader's original process can be bound. No caller process,
/// stream FD, pathname, refresh or execution operation is accepted here.
pub fn pin_live_execution<'guard, 'process>(
    identity: &'guard mut LiveIdentityRead<'process>,
    outputs: &'guard mut ProductionOutputRead,
    material: &'guard mut PinnedStartMaterial,
    manager: &mut ManagerClient,
) -> Result<LiveExecutionRead<'guard, 'process>, ExecutionError> {
    let state = identity.state().clone();
    let process = identity.original_process();
    let result = (|| {
        if identity.is_refused() || outputs.is_refused() || material.is_refused() {
            return Err(ExecutionError::Refused);
        }
        if !outputs.matches_production_state(&state) || !material.matches_production_state(&state) {
            return Err(ExecutionError::Invalid);
        }
        identity.revalidate(manager)?;
        Ok(outputs.bind_original_process_io(&state, material, process)?)
    })();
    let io = match result {
        Ok(io) => io,
        Err(error) => {
            refuse_inputs(identity, outputs, material);
            return Err(error);
        }
    };
    let mut read = LiveExecutionRead {
        identity,
        outputs,
        material,
        io,
        state,
        guard: ExecutionGuard::default(),
    };
    read.revalidate(manager)?;
    Ok(read)
}

impl LiveExecutionRead<'_, '_> {
    pub fn is_refused(&self) -> bool {
        self.guard.refused
            || self.identity.is_refused()
            || self.outputs.is_refused()
            || self.material.is_refused()
            || self.io.is_refused()
    }
    /// Four bounded manager observations and retained reader checks. No atomic
    /// snapshot, continuity proof or whole-operation deadline is promised.
    pub fn revalidate(&mut self, manager: &mut ManagerClient) -> Result<(), ExecutionError> {
        let refused = self.is_refused();
        let result = self.guard.revalidate_with(refused, |step| match step {
            Step::Identity => self.identity.revalidate(manager).map_err(Into::into),
            Step::Outputs => self.outputs.revalidate().map_err(Into::into),
            Step::Material => self
                .material
                .revalidate_for(&self.state)
                .map_err(Into::into),
            Step::Io => self.io.revalidate().map_err(Into::into),
        });
        self.finish_observation(result)
    }
    fn finish_observation(
        &mut self,
        result: Result<(), ExecutionError>,
    ) -> Result<(), ExecutionError> {
        if result.is_err() {
            self.guard.refused = true;
            refuse_inputs(self.identity, self.outputs, self.material);
        }
        result
    }
}

#[cfg(test)]
pub(crate) fn finish_fixture_execution_refusal<'a>(
    identity: &mut LiveIdentityRead<'a>,
    outputs: &mut ProductionOutputRead,
    material: &mut PinnedStartMaterial,
    io: ProcessIoRead<'a>,
    error: ExecutionError,
) -> ExecutionError {
    // Private negative-only fixture holder; never calls a successful observation
    // or the public production constructor, never certifies these fake origins.
    assert!(material.is_fixture());
    let state = identity.state().clone();
    assert!(!outputs.matches_production_state(&state));
    let mut read = LiveExecutionRead {
        identity,
        outputs,
        material,
        io,
        state,
        guard: ExecutionGuard::default(),
    };
    let error = read.finish_observation(Err(error)).unwrap_err();
    assert!(read.is_refused());
    assert!(matches!(
        read.revalidate(&mut ManagerClient::default()),
        Err(ExecutionError::Refused)
    ));
    error // Dropping the wrapper cannot restore the three borrowed guards.
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic_live_identity::IdentityError;
    use crate::diagnostic_manager::{WorkerFailure, WorkerFailureKind};

    fn failure() -> ExecutionError {
        ExecutionError::Identity(IdentityError::Manager(WorkerFailure {
            kind: WorkerFailureKind::Deadline,
            stdout: vec![1, 2],
            stderr: vec![3],
            cleanup_confirmed: false,
        }))
    }
    #[test]
    fn synthetic_execution_order_brackets_io_with_all_original_inputs() {
        let mut guard = ExecutionGuard::default();
        let mut order = vec![];
        guard
            .revalidate_with(false, |step| {
                order.push(step);
                Ok(())
            })
            .unwrap();
        assert_eq!(
            order,
            [
                Step::Identity,
                Step::Outputs,
                Step::Material,
                Step::Io,
                Step::Material,
                Step::Outputs,
                Step::Identity
            ]
        );
        assert!(!guard.refused);
    }
    #[test]
    fn synthetic_every_execution_failure_preserves_first_error_and_stops_reads() {
        for fail_at in 0..7 {
            let mut guard = ExecutionGuard::default();
            let mut calls = 0;
            let error = guard
                .revalidate_with(false, |_| {
                    let position = calls;
                    calls += 1;
                    if position == fail_at {
                        Err(failure())
                    } else {
                        Ok(())
                    }
                })
                .unwrap_err();
            let ExecutionError::Identity(IdentityError::Manager(error)) = error else {
                panic!("lost first error")
            };
            assert_eq!(error.kind, WorkerFailureKind::Deadline);
            assert_eq!(error.stdout, [1, 2]);
            assert_eq!(error.stderr, [3]);
            assert!(!error.cleanup_confirmed);
            assert_eq!(calls, fail_at + 1);
            assert!(guard.refused);
            assert!(matches!(
                guard.revalidate_with(false, |_| panic!("read after refusal")),
                Err(ExecutionError::Refused)
            ));
        }
    }
    #[test]
    fn synthetic_preexisting_execution_refusal_never_reads_or_repairs() {
        let mut guard = ExecutionGuard::default();
        assert!(matches!(
            guard.revalidate_with(true, |_| panic!("read refused input")),
            Err(ExecutionError::Refused)
        ));
        assert!(guard.refused);
        assert!(guard
            .revalidate_with(false, |_| panic!("restoration repaired refusal"))
            .is_err());
    }
    #[test]
    fn synthetic_execution_guards_are_independent_under_concurrency() {
        std::thread::scope(|scope| {
            for fail in [false, true, false, true] {
                scope.spawn(move || {
                    let mut guard = ExecutionGuard::default();
                    for _ in 0..32 {
                        let result = guard.revalidate_with(false, |_| {
                            if fail {
                                Err(failure())
                            } else {
                                Ok(())
                            }
                        });
                        assert_eq!(result.is_err(), fail);
                    }
                    assert_eq!(guard.refused, fail);
                });
            }
        });
    }
}
