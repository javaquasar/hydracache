//! Adjacent state checks only. No consumption, outcome or execution authority.
use super::{
    validate_state, DiagnosticError, DiagnosticStage as Stage, DiagnosticState,
    DiagnosticStopReason as Reason, CONTROLLER_LOSS_SECONDS, NS, TOTAL_SECONDS,
};

pub(crate) fn validate_transition(
    previous: &DiagnosticState,
    next: &DiagnosticState,
) -> Result<(), DiagnosticError> {
    validate_state(previous)?;
    validate_state(next)?;
    if previous.stage == Stage::Terminal
        || previous.identity != next.identity
        || previous.reserved_monotonic_ns != next.reserved_monotonic_ns
        || previous.revision.checked_add(1) != Some(next.revision)
        || next.last_observed_monotonic_ns < previous.last_observed_monotonic_ns
        || next.controller_monotonic_ns < previous.controller_monotonic_ns
        || (next.controller_monotonic_ns != previous.controller_monotonic_ns
            && (next.controller_monotonic_ns != next.last_observed_monotonic_ns
                || next.last_observed_monotonic_ns - previous.controller_monotonic_ns
                    >= CONTROLLER_LOSS_SECONDS * NS))
        || (previous.reason.is_some() && previous.reason != next.reason)
    {
        return Err(DiagnosticError::Invalid);
    }

    let completed =
        previous.stage == Stage::Running && matches!(next.stage, Stage::Reserved | Stage::Terminal);
    if completed {
        if previous.reason.is_some()
            || next.completed_cells != previous.completed_cells + 1
            || !next.cleanup_confirmed
            || (next.stage == Stage::Terminal
                && (next.completed_cells != 4 || next.reason != Some(Reason::Completed)))
        {
            return Err(DiagnosticError::Invalid);
        }
    } else if next.completed_cells != previous.completed_cells {
        return Err(DiagnosticError::Invalid);
    }

    let cleared_cell = completed && next.stage == Stage::Reserved;
    if !cleared_cell {
        if previous.stage != Stage::Reserved
            && previous.cell_started_monotonic_ns != next.cell_started_monotonic_ns
        {
            return Err(DiagnosticError::Invalid);
        }
        if previous.cgroup_inode.is_some() && previous.cgroup_inode != next.cgroup_inode {
            return Err(DiagnosticError::Invalid);
        }
    }

    let allowed = match (previous.stage, next.stage) {
        (Stage::Reserved, Stage::Reserved) => true,
        (Stage::Reserved, Stage::Starting) => {
            next.cell_started_monotonic_ns == Some(next.last_observed_monotonic_ns)
                && next.cgroup_inode.is_none()
                && next.reason.is_none()
                && next.last_observed_monotonic_ns - next.controller_monotonic_ns
                    < CONTROLLER_LOSS_SECONDS * NS
                && next.last_observed_monotonic_ns - next.reserved_monotonic_ns < TOTAL_SECONDS * NS
        }
        (Stage::Reserved, Stage::Terminal) => {
            next.cell_started_monotonic_ns.is_none()
                && next.cgroup_inode.is_none()
                && matches!(
                    next.reason,
                    Some(
                        Reason::OperatorCancelled | Reason::ControllerLost | Reason::TotalDeadline
                    )
                )
        }
        (Stage::Starting, Stage::Starting | Stage::Running) | (Stage::Running, Stage::Running) => {
            previous.reason.is_none() && next.reason.is_none()
        }
        (Stage::Starting | Stage::Running, Stage::Stopping) => {
            next.reason.is_some() && next.reason != Some(Reason::Completed)
        }
        (Stage::Running, Stage::Reserved | Stage::Terminal) => completed,
        (Stage::Stopping, Stage::Stopping | Stage::Terminal) => previous.reason == next.reason,
        _ => false,
    };
    if allowed {
        Ok(())
    } else {
        Err(DiagnosticError::Invalid)
    }
}
