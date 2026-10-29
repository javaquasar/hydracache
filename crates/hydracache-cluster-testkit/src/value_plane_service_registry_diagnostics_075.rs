//! Paginated metadata observations over actual queued jobs, attempts and service leases.
use super::*;
use sha2::{Digest, Sha256};

const HEADER_BYTES: usize = 320;
const ROW_BYTES: usize = 96;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvocationClass {
    Read,
    Mutation,
    Maintenance,
    TypedHelper,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvocationWaitReason {
    Queued,
    ExecutorOwned,
    RootReservation,
    Dispatch,
    CancellationRequested,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseBlockReason {
    PendingOperation,
    Listener,
    StreamOrHelper,
    SnapshotOrTransfer,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvocationDiagnosticRow {
    pub ticket: u64,
    pub class: InvocationClass,
    pub phase: ClusterInvocationPhase,
    pub reason: InvocationWaitReason,
    pub attempts: u8,
    pub cancelled: bool,
    pub deadline_remaining: u64,
    pub request_bytes: usize,
    pub growth_bytes: usize,
    pub replica_bytes: usize,
}
#[derive(Clone, PartialEq, Eq)]
pub struct InvocationDiagnosticCursor {
    registry: u64,
    scope: [u8; 32],
    observation: [u8; 32],
    after: u64,
}
impl fmt::Debug for InvocationDiagnosticCursor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("InvocationDiagnosticCursor([opaque])")
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvocationDiagnosticPage {
    pub namespace_generation: u64,
    pub logical_time: u64,
    pub admission_watermark: u64,
    pub observation: [u8; 32],
    pub rows: Vec<InvocationDiagnosticRow>,
    pub total_invocations: usize,
    pub owners: NamespaceOwners,
    pub resources: ServiceResourceUsage,
    pub blocked_by: Vec<LeaseBlockReason>,
    pub next: Option<InvocationDiagnosticCursor>,
    /// Fixed bounded local diagnostic representation; this is not a production wire format.
    pub encoded_bytes: usize,
}
struct HashWriter(Sha256);
impl fmt::Write for HashWriter {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        self.0.update(value.as_bytes());
        Ok(())
    }
}

fn control_fields(
    control: Option<&Weak<Mutex<invocation::Control>>>,
) -> Option<(bool, ClusterInvocationPhase, u8, bool)> {
    let control = control?.upgrade()?;
    let control = control.lock().unwrap();
    Some((
        control.mutating,
        control.phase,
        control.attempts,
        control.cancelled,
    ))
}
struct RowCredit {
    deadline: u64,
    bytes: usize,
    growth: usize,
    replica: usize,
}
fn make_row(
    ticket: u64,
    command: Option<&ServiceCommand>,
    control: Option<&Weak<Mutex<invocation::Control>>>,
    now: u64,
    credit: RowCredit,
) -> InvocationDiagnosticRow {
    let observed = control_fields(control);
    let mutating = observed.map_or_else(
        || {
            command.is_some_and(|command| {
                crate::value_plane_service_075::is_mutating_operation(&command.operation)
            })
        },
        |value| value.0,
    );
    let phase = observed.map_or(ClusterInvocationPhase::Queued, |value| value.1);
    let cancelled = observed.is_some_and(|value| value.3);
    let reason = if cancelled {
        InvocationWaitReason::CancellationRequested
    } else {
        match phase {
            ClusterInvocationPhase::Queued => InvocationWaitReason::Queued,
            ClusterInvocationPhase::Claimed => InvocationWaitReason::ExecutorOwned,
            ClusterInvocationPhase::Reserving | ClusterInvocationPhase::Reserved => {
                InvocationWaitReason::RootReservation
            }
            _ => InvocationWaitReason::Dispatch,
        }
    };
    InvocationDiagnosticRow {
        ticket,
        class: if command.is_none() && observed.is_none() {
            InvocationClass::Maintenance
        } else if mutating {
            InvocationClass::Mutation
        } else {
            InvocationClass::Read
        },
        phase,
        reason,
        attempts: observed.map_or(0, |value| value.2),
        cancelled,
        deadline_remaining: credit.deadline.saturating_sub(now),
        request_bytes: credit.bytes,
        growth_bytes: credit.growth,
        replica_bytes: credit.replica,
    }
}
fn visit_rows(
    state: &RegistryState,
    address: &NamespaceAddress,
    mut visit: impl FnMut(InvocationDiagnosticRow),
) {
    let mut queued = state.jobs.iter().peekable();
    let mut active = state.active_invocations.iter().peekable();
    while queued.peek().is_some() || active.peek().is_some() {
        let take_queued = match (queued.peek(), active.peek()) {
            (Some((left, _)), Some((right, _))) => left < right,
            (Some(_), None) => true,
            _ => false,
        };
        if take_queued {
            let (ticket, job) = queued.next().expect("queued diagnostic row");
            if &job.handle.address != address {
                continue;
            }
            let command = match &job.kind {
                JobKind::Execute { command, .. } | JobKind::Reserve(command) => Some(command),
                _ => None,
            };
            let deadline = match &job.kind {
                JobKind::Helper { request, .. } => request.deadline.min(job.authenticated.deadline),
                _ => command.map_or(job.authenticated.deadline, |command| {
                    command.deadline.min(job.authenticated.deadline)
                }),
            };
            let replica_bytes = job
                .admission
                .and_then(|ticket| state.cluster_admission.as_ref()?.credit_usage(ticket))
                .map_or(0, |(_, replica)| replica);
            let mut row = make_row(
                *ticket,
                command,
                job.invocation.as_ref(),
                state.now,
                RowCredit {
                    deadline,
                    bytes: job.bytes,
                    growth: job.growth,
                    replica: replica_bytes,
                },
            );
            if matches!(job.kind, JobKind::Helper { .. }) {
                row.class = InvocationClass::TypedHelper;
            }
            visit(row);
        } else {
            let (ticket, owner) = active.next().expect("active diagnostic row");
            if &owner.address != address {
                continue;
            }
            let replica_bytes = state
                .cluster_admission
                .as_ref()
                .and_then(|scheduler| scheduler.credit_usage(owner.admission))
                .map_or(0, |(_, replica)| replica);
            visit(make_row(
                *ticket,
                None,
                Some(&owner.control),
                state.now,
                RowCredit {
                    deadline: owner.deadline,
                    bytes: owner.bytes,
                    growth: owner.growth,
                    replica: replica_bytes,
                },
            ));
        }
    }
}
impl UnifiedServiceRegistry {
    pub fn invocation_diagnostics(
        &self,
        handle: &RegistryHandle,
        authenticated: &AuthenticatedNamespace,
        maximum_items: usize,
        maximum_bytes: usize,
        cursor: Option<&InvocationDiagnosticCursor>,
    ) -> Result<InvocationDiagnosticPage, RegistryError> {
        if maximum_items == 0
            || maximum_items > 256
            || !(HEADER_BYTES..=64 * 1024).contains(&maximum_bytes)
        {
            return Err(RegistryError::InvalidBound("diagnostic_bounds"));
        }
        let state = self.state.lock().unwrap();
        validate_handle(&state, handle, authenticated, true)?;
        authenticated.authorize(GrantOperation::Admin, &[], true)?;
        let owner = &state.maps[&address_key(&handle.address)];
        let owners = owner.service.namespace_owners();
        let resources = owner.service.resource_usage();
        let mut scope = HashWriter(Sha256::new());
        fmt::Write::write_fmt(
            &mut scope,
            format_args!(
                "{}:{:?}:{:?}:{:?}",
                state.id,
                handle.address,
                effective_policy_epoch(&state, &handle.address),
                authenticated.grant_digest()
            ),
        )
        .expect("scope hash");
        let scope: [u8; 32] = scope.0.finalize().into();
        let mut hash = HashWriter(Sha256::new());
        let mut total = 0usize;
        visit_rows(&state, &handle.address, |row| {
            total += 1;
            fmt::Write::write_fmt(&mut hash, format_args!("{row:?}")).expect("observation hash");
        });
        fmt::Write::write_fmt(
            &mut hash,
            format_args!(
                "{owners:?}:{resources:?}:{}:{}",
                state.now, owner.last_admission
            ),
        )
        .expect("owner hash");
        let observation: [u8; 32] = hash.0.finalize().into();
        if let Some(cursor) = cursor {
            if cursor.registry != state.id || cursor.scope != scope {
                return Err(RegistryError::InvalidBound("diagnostic_cursor_mismatch"));
            }
            if cursor.observation != observation {
                return Err(RegistryError::InvalidBound("diagnostic_cursor_stale"));
            }
        }
        let after = cursor.map_or(0, |cursor| cursor.after);
        let mut rows = Vec::new();
        let mut more = false;
        let mut checked = HashWriter(Sha256::new());
        visit_rows(&state, &handle.address, |row| {
            fmt::Write::write_fmt(&mut checked, format_args!("{row:?}"))
                .expect("stable observation hash");
            if row.ticket <= after {
                return;
            }
            if rows.len() == maximum_items
                || HEADER_BYTES + (rows.len() + 1) * ROW_BYTES > maximum_bytes
            {
                more = true;
            } else if !more {
                rows.push(row);
            }
        });
        fmt::Write::write_fmt(
            &mut checked,
            format_args!(
                "{:?}:{:?}:{}:{}",
                owner.service.namespace_owners(),
                owner.service.resource_usage(),
                state.now,
                owner.last_admission
            ),
        )
        .expect("stable owner hash");
        let checked: [u8; 32] = checked.0.finalize().into();
        if checked != observation {
            return Err(RegistryError::InvalidBound(
                "diagnostic_observation_changed",
            ));
        }
        if more && rows.is_empty() {
            return Err(RegistryError::InvalidBound("diagnostic_response_bytes"));
        }
        let next = more.then(|| InvocationDiagnosticCursor {
            registry: state.id,
            scope,
            observation,
            after: rows.last().expect("nonempty diagnostic page").ticket,
        });
        let mut blocked_by = Vec::new();
        if owners.pending != 0 {
            blocked_by.push(LeaseBlockReason::PendingOperation);
        }
        if owners.listeners != 0 {
            blocked_by.push(LeaseBlockReason::Listener);
        }
        if owners.streams != 0 {
            blocked_by.push(LeaseBlockReason::StreamOrHelper);
        }
        if owners.resync_leases != 0 {
            blocked_by.push(LeaseBlockReason::SnapshotOrTransfer);
        }
        let encoded_bytes = HEADER_BYTES + rows.len() * ROW_BYTES;
        Ok(InvocationDiagnosticPage {
            namespace_generation: handle.address.generation,
            logical_time: state.now,
            admission_watermark: owner.last_admission,
            observation,
            rows,
            total_invocations: total,
            owners,
            resources,
            blocked_by,
            next,
            encoded_bytes,
        })
    }
}
