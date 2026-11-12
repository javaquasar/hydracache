//! Cooperative actual-cluster invocations. The registry retains all payload and receipt owners.
use super::*;
use crate::value_plane_async_075::{AsyncOutcome, CancellationDisposition, InvocationRejection};
use crate::value_plane_cluster_runtime_075::{
    ClusterHelperRecovery, ClusterHelperReservation, ClusterHelperResult,
};
use crate::value_plane_service_075::{is_mutating_operation, ServiceRecovery};
use std::future::Future;
use std::pin::Pin;
use std::task::Waker;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClusterInvocationPhase {
    Queued,
    Claimed,
    Reserving,
    Reserved,
    Dispatching,
    Finished,
}
pub(super) struct Control {
    pub(super) mutating: bool,
    pub(super) attempts: u8,
    pub(super) phase: ClusterInvocationPhase,
    pub(super) cancelled: bool,
    pub(super) waiting_for_capacity: bool,
    reservation_attempted: bool,
    waker: Option<Waker>,
    error: Option<RegistryError>,
}
#[derive(Clone)]
pub struct ClusterInvocationHandle {
    control: Arc<Mutex<Control>>,
    ticket: RequestTicket,
}
impl ClusterInvocationHandle {
    pub fn ticket(&self) -> RequestTicket {
        self.ticket
    }
    pub fn phase(&self) -> ClusterInvocationPhase {
        self.control.lock().unwrap().phase
    }
    pub fn last_error(&self) -> Option<RegistryError> {
        self.control.lock().unwrap().error.clone()
    }
    pub fn attempts(&self) -> u8 {
        self.control.lock().unwrap().attempts
    }
    pub fn cancel(&self) -> CancellationDisposition {
        let mut control = self.control.lock().unwrap();
        if control.phase == ClusterInvocationPhase::Finished {
            return CancellationDisposition::AlreadyFinished;
        }
        control.cancelled = true;
        let outcome = if !control.mutating
            || (!control.reservation_attempted
                && matches!(
                    control.phase,
                    ClusterInvocationPhase::Queued | ClusterInvocationPhase::Claimed
                )) {
            CancellationDisposition::NotApplied
        } else {
            CancellationDisposition::OutcomeUnknown
        };
        let wake = control.waker.take();
        drop(control);
        if let Some(wake) = wake {
            wake.wake();
        }
        outcome
    }
}
/// Ordinary command future; reads never allocate an idempotency journal.
pub struct RegistryClusterInvocation {
    inner: Invocation,
}
/// Typed helper future over the actual root/child ledgers used by synchronous dispatch.
pub struct RegistryClusterHelperInvocation {
    inner: Invocation,
}
struct Invocation {
    mutating: bool,
    retry: ClusterRetryPolicy,
    retry_wait: Option<u64>,
    registry: UnifiedServiceRegistry,
    handle: RegistryHandle,
    authenticated: AuthenticatedNamespace,
    ticket: RequestTicket,
    control: Arc<Mutex<Control>>,
    job: Option<QueuedJob>,
    permit: Option<ClusterPermit>,
    reserved: bool,
    finished: bool,
}
// One bounded attempt keeps its DTO inline instead of adding an uncharged heap allocation.
#[allow(clippy::large_enum_variant)]
enum InvocationWork {
    Command(ServiceCommand, Option<RuntimeCallOptions>),
    Helper(ClusterHelperRequest, RuntimeCallOptions),
}
enum InvocationExecution {
    Command(ServiceClusterExecution),
    Helper(ClusterHelperExecution),
}
// The completed result already owns bounded payload; an inline tag needs no extra heap owner.
#[allow(clippy::large_enum_variant)]
enum InvocationStep {
    Reserved,
    NotApplied,
    Executed(InvocationExecution),
}
impl InvocationExecution {
    fn failure(&self) -> Option<&ServiceClusterError> {
        match self {
            Self::Command(value) => value.failure.as_ref(),
            Self::Helper(value) => value.failure.as_ref(),
        }
    }
    fn uncertain(&self) -> bool {
        match self {
            Self::Command(value) => {
                value.result.kind == "outcome_unknown"
                    || !value.backup_proved
                    || value.failure.is_some()
            }
            Self::Helper(value) => {
                !value.backup_proved
                    || value.failure.is_some()
                    || matches!(value.result, ClusterHelperResult::OutcomeUnknown)
                    || matches!(&value.result, ClusterHelperResult::Versions(result) if result.items.iter().any(|item| item.kind == "outcome_unknown"))
            }
        }
    }
}
impl InvocationWork {
    fn options(&self) -> Option<&RuntimeCallOptions> {
        match self {
            Self::Command(_, options) => options.as_ref(),
            Self::Helper(_, options) => Some(options),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClusterRetryPolicy {
    maximum_attempts: u8,
}
impl ClusterRetryPolicy {
    pub const DISABLED: Self = Self {
        maximum_attempts: 1,
    };
    pub fn new(maximum_attempts: u8) -> Result<Self, RegistryError> {
        if !(1..=16).contains(&maximum_attempts) {
            return Err(RegistryError::InvalidBound("retry_attempts"));
        }
        Ok(Self { maximum_attempts })
    }
}
fn is_read_invocation(command: &ServiceCommand) -> bool {
    matches!(
        command.operation.as_str(),
        "get"
            | "get_all"
            | "contains_key"
            | "contains_entry"
            | "remaining_ttl"
            | "get_or_default"
            | "get_entry_view"
            | "query_operation_outcome"
            | "local_statistics"
            | "capability_descriptor"
    )
}
impl UnifiedServiceRegistry {
    pub fn cluster_invocation(
        &self,
        handle: &RegistryHandle,
        authenticated: &AuthenticatedNamespace,
        surface: SurfaceId,
        command: ServiceCommand,
        options: RuntimeCallOptions,
    ) -> Result<(RegistryClusterInvocation, ClusterInvocationHandle), RegistryError> {
        self.cluster_invocation_with_retry(
            handle,
            authenticated,
            surface,
            command,
            options,
            ClusterRetryPolicy::DISABLED,
        )
    }
    pub fn cluster_read_invocation(
        &self,
        handle: &RegistryHandle,
        authenticated: &AuthenticatedNamespace,
        surface: SurfaceId,
        command: ServiceCommand,
        options: RuntimeCallOptions,
    ) -> Result<(RegistryClusterInvocation, ClusterInvocationHandle), RegistryError> {
        self.cluster_read_invocation_with_retry(
            handle,
            authenticated,
            surface,
            command,
            options,
            ClusterRetryPolicy::DISABLED,
        )
    }
    pub fn cluster_read_invocation_with_retry(
        &self,
        handle: &RegistryHandle,
        authenticated: &AuthenticatedNamespace,
        surface: SurfaceId,
        command: ServiceCommand,
        options: RuntimeCallOptions,
        retry: ClusterRetryPolicy,
    ) -> Result<(RegistryClusterInvocation, ClusterInvocationHandle), RegistryError> {
        if !is_read_invocation(&command) {
            return Err(MapServiceError::InvalidArgument.into());
        }
        self.start_command_invocation(handle, authenticated, surface, command, options, retry)
    }
    pub fn cluster_invocation_with_retry(
        &self,
        handle: &RegistryHandle,
        authenticated: &AuthenticatedNamespace,
        surface: SurfaceId,
        command: ServiceCommand,
        options: RuntimeCallOptions,
        retry: ClusterRetryPolicy,
    ) -> Result<(RegistryClusterInvocation, ClusterInvocationHandle), RegistryError> {
        if !is_mutating_operation(&command.operation) || command.identity.is_none() {
            return Err(MapServiceError::InvalidArgument.into());
        }
        self.start_command_invocation(handle, authenticated, surface, command, options, retry)
    }
    fn start_command_invocation(
        &self,
        handle: &RegistryHandle,
        authenticated: &AuthenticatedNamespace,
        surface: SurfaceId,
        command: ServiceCommand,
        options: RuntimeCallOptions,
        retry: ClusterRetryPolicy,
    ) -> Result<(RegistryClusterInvocation, ClusterInvocationHandle), RegistryError> {
        let mutating = is_mutating_operation(&command.operation);
        if command.operation == "destroy" {
            return Err(RegistryError::SafetyLaneRequired);
        }
        let (inner, handle) = self.start_invocation(
            handle,
            authenticated,
            JobKind::Execute {
                cooperative: true,
                surface,
                command,
                options: Some(options),
            },
            mutating,
            retry,
        )?;
        Ok((RegistryClusterInvocation { inner }, handle))
    }
    pub fn cluster_helper_invocation(
        &self,
        handle: &RegistryHandle,
        authenticated: &AuthenticatedNamespace,
        surface: SurfaceId,
        request: ClusterHelperRequest,
        options: RuntimeCallOptions,
    ) -> Result<(RegistryClusterHelperInvocation, ClusterInvocationHandle), RegistryError> {
        self.cluster_helper_invocation_with_retry(
            handle,
            authenticated,
            surface,
            request,
            options,
            ClusterRetryPolicy::DISABLED,
        )
    }
    pub fn cluster_helper_invocation_with_retry(
        &self,
        handle: &RegistryHandle,
        authenticated: &AuthenticatedNamespace,
        surface: SurfaceId,
        request: ClusterHelperRequest,
        options: RuntimeCallOptions,
        retry: ClusterRetryPolicy,
    ) -> Result<(RegistryClusterHelperInvocation, ClusterInvocationHandle), RegistryError> {
        let mutating = request.is_mutating();
        let (inner, handle) = self.start_invocation(
            handle,
            authenticated,
            JobKind::Helper {
                cooperative: true,
                surface,
                request,
                options,
            },
            mutating,
            retry,
        )?;
        Ok((RegistryClusterHelperInvocation { inner }, handle))
    }
    fn start_invocation(
        &self,
        handle: &RegistryHandle,
        authenticated: &AuthenticatedNamespace,
        kind: JobKind,
        mutating: bool,
        retry: ClusterRetryPolicy,
    ) -> Result<(Invocation, ClusterInvocationHandle), RegistryError> {
        {
            let state = self.state.lock().unwrap();
            validate_handle(&state, handle, authenticated, false)?;
            if !matches!(
                state.maps[&address_key(&handle.address)].service,
                NamespaceBackend::Cluster(_)
            ) {
                return Err(ServiceClusterError::UnsupportedScope.into());
            }
        }
        let control = Arc::new(Mutex::new(Control {
            mutating,
            attempts: 1,
            phase: ClusterInvocationPhase::Queued,
            cancelled: false,
            waiting_for_capacity: false,
            reservation_attempted: false,
            waker: None,
            error: None,
        }));
        let ticket =
            self.enqueue_with_control(handle, authenticated, kind, Some(Arc::downgrade(&control)))?;
        Ok((
            Invocation {
                mutating,
                retry,
                retry_wait: None,
                registry: self.clone(),
                handle: handle.clone(),
                authenticated: authenticated.clone(),
                ticket,
                control: Arc::clone(&control),
                job: None,
                permit: None,
                reserved: false,
                finished: false,
            },
            ClusterInvocationHandle { control, ticket },
        ))
    }
}
impl Invocation {
    fn runtime(&self) -> Result<ClusterNamespaceRuntime, RegistryError> {
        let state = self.registry.state.lock().unwrap();
        validate_handle(&state, &self.handle, &self.authenticated, false)?;
        match &state.maps[&address_key(&self.handle.address)].service {
            NamespaceBackend::Cluster(runtime) => Ok(runtime.clone()),
            _ => Err(ServiceClusterError::UnsupportedScope.into()),
        }
    }
    fn inspect_work<T>(
        &self,
        inspect: impl FnOnce(&JobKind, u64) -> Result<T, RegistryError>,
    ) -> Result<T, RegistryError> {
        let state = self.registry.state.lock().unwrap();
        validate_handle(&state, &self.handle, &self.authenticated, false)?;
        let job = self
            .job
            .as_ref()
            .or_else(|| state.jobs.get(&self.ticket.0))
            .ok_or(RegistryError::UnknownNamespace)?;
        authorize_job(&self.authenticated, &job.kind)?;
        inspect(&job.kind, state.now)
    }
    fn work(&self) -> Result<InvocationWork, RegistryError> {
        self.inspect_work(|kind, now| match kind {
            JobKind::Execute {
                command, options, ..
            } => {
                let mut command = command.clone();
                command.now = now;
                command.deadline = command.deadline.min(self.authenticated.deadline);
                Ok(InvocationWork::Command(command, options.clone()))
            }
            JobKind::Helper {
                request, options, ..
            } => {
                let mut request = request.clone();
                request.now = now;
                request.deadline = request.deadline.min(self.authenticated.deadline);
                Ok(InvocationWork::Helper(request, options.clone()))
            }
            _ => Err(MapServiceError::InvalidArgument.into()),
        })
    }
    fn clock_and_deadline(&self) -> Result<(u64, u64), RegistryError> {
        self.inspect_work(|kind, now| match kind {
            JobKind::Execute { command, .. } => {
                Ok((now, command.deadline.min(self.authenticated.deadline)))
            }
            JobKind::Helper { request, .. } => {
                Ok((now, request.deadline.min(self.authenticated.deadline)))
            }
            _ => Err(MapServiceError::InvalidArgument.into()),
        })
    }
    fn claim(&mut self, context: &Context<'_>) -> Result<bool, RegistryError> {
        let _dispatch = self
            .registry
            .dispatching
            .try_lock()
            .map_err(|_| RegistryError::DispatcherBusy)?;
        let mut state = self.registry.state.lock().unwrap();
        validate_handle(&state, &self.handle, &self.authenticated, false)?;
        let job = state
            .jobs
            .get(&self.ticket.0)
            .ok_or(RegistryError::UnknownNamespace)?;
        authorize_job(&self.authenticated, &job.kind)?;
        let admission = job.admission.ok_or(ServiceClusterError::UnsupportedScope)?;
        let scheduler = state
            .cluster_admission
            .as_ref()
            .ok_or(ServiceClusterError::UnsupportedScope)?;
        scheduler.register_waiter(admission, context.waker());
        let Some(permit) = scheduler.pop_for(admission)? else {
            return Ok(false);
        };
        let job = state
            .jobs
            .remove(&self.ticket.0)
            .expect("claimed registry job");
        let deadline = self.authenticated.deadline.min(match &job.kind {
            JobKind::Execute { command, .. } => command.deadline,
            JobKind::Helper { request, .. } => request.deadline,
            _ => self.authenticated.deadline,
        });
        state.active_invocations.insert(
            self.ticket.0,
            ActiveInvocation {
                helper: matches!(job.kind, JobKind::Helper { .. }),
                address: job.handle.address.clone(),
                bytes: job.bytes,
                growth: job.growth,
                deadline,
                admission,
                control: Arc::downgrade(&self.control),
            },
        );
        self.job = Some(job);
        self.permit = Some(permit);
        Ok(true)
    }
    fn preflight(&self, work: &InvocationWork) -> Result<(), RegistryError> {
        let mut state = self.registry.state.lock().unwrap();
        validate_handle(&state, &self.handle, &self.authenticated, false)?;
        let service = &state.maps[&address_key(&self.handle.address)].service;
        let growth = match work {
            InvocationWork::Command(command, _) => {
                self.authenticated.authorize_command(command)?;
                service.preflight_registry_growth(command)?
            }
            InvocationWork::Helper(request, _) => {
                service
                    .helper_preflight(&self.authenticated, request)?
                    .growth
            }
        };
        let owner = state
            .active_invocations
            .get(&self.ticket.0)
            .ok_or(RegistryError::UnknownNamespace)?;
        let reserved = owner.growth;
        let overhead = owner
            .bytes
            .checked_add(self.handle.address.tenant.len())
            .ok_or(RegistryError::InvalidBound("invocation_growth_credit"))?;
        let credit = self
            .permit
            .as_ref()
            .ok_or(ServiceClusterError::Authorization("admission_binding"))?
            .work()
            .bytes
            .checked_sub(overhead)
            .ok_or(RegistryError::InvalidBound("invocation_growth_credit"))?;
        // The original signed lane peak is immutable. Consume only that held slack,
        // and publish any larger global reservation before a backend can mutate.
        quota("invocation_growth_credit", credit, growth)?;
        let peak = reserved.max(growth);
        check_owned_growth(
            &state,
            &self.handle.address.tenant,
            peak,
            Some(self.ticket.0),
        )?;
        let next = state
            .reserved_growth
            .checked_add(peak - reserved)
            .ok_or(RegistryError::InvalidBound("invocation_growth_credit"))?;
        state
            .active_invocations
            .get_mut(&self.ticket.0)
            .expect("active invocation")
            .growth = peak;
        state.reserved_growth = next;
        Ok(())
    }
    fn scheduler(&self) -> Option<ClusterAdmission> {
        self.registry
            .state
            .lock()
            .unwrap()
            .cluster_admission
            .clone()
    }
    fn register_waiter_since(&self, context: &Context<'_>, observed: Option<u64>) {
        let state = self.registry.state.lock().unwrap();
        let mut wake = false;
        if let Some(scheduler) = &state.cluster_admission {
            if let Some(ticket) = self
                .permit
                .as_ref()
                .map(ClusterPermit::ticket)
                .or_else(|| state.jobs.get(&self.ticket.0).and_then(|job| job.admission))
            {
                wake = observed
                    .is_none_or(|cut| !scheduler.wait_for_change(ticket, cut, context.waker()));
            }
        }
        drop(state);
        if wake {
            context.waker().wake_by_ref();
        }
    }
    fn cleanup(&mut self) {
        if self.finished {
            return;
        }
        if self.reserved {
            let runtime = {
                let state = self.registry.state.lock().unwrap();
                state
                    .maps
                    .get(&address_key(&self.handle.address))
                    .and_then(|owner| match &owner.service {
                        NamespaceBackend::Cluster(runtime) => Some(runtime.clone()),
                        _ => None,
                    })
            };
            if let (Some(runtime), Some(job)) = (runtime, self.job.as_ref()) {
                match &job.kind {
                    JobKind::Execute { command, .. } => {
                        let _ = runtime.cancel_pending(command);
                    }
                    JobKind::Helper { request, .. } => {
                        let _ = runtime.cancel_helper_invocation(request);
                    }
                    _ => {}
                }
            }
        }
        let mut state = self.registry.state.lock().unwrap();
        if let Some(job) = state.jobs.remove(&self.ticket.0) {
            if let (Some(scheduler), Some(ticket)) = (&state.cluster_admission, job.admission) {
                let _ = scheduler.cancel(ticket);
            }
            state.queued_bytes -= job.bytes;
            state.reserved_growth -= job.growth;
        }
        if let Some(owner) = state.active_invocations.remove(&self.ticket.0) {
            state.queued_bytes -= owner.bytes;
            state.reserved_growth -= owner.growth;
        }
        let scheduler = state.cluster_admission.clone();
        drop(state);
        let permit = self.permit.take();
        let released = permit.is_some();
        drop(permit);
        self.job = None;
        self.finished = true;
        let mut control = self.control.lock().unwrap();
        control.phase = ClusterInvocationPhase::Finished;
        control.waiting_for_capacity = false;
        control.waker = None;
        drop(control);
        if !released {
            if let Some(scheduler) = scheduler {
                scheduler.wake_waiters();
            }
        }
    }
    fn complete(
        &mut self,
        outcome: AsyncOutcome<InvocationExecution, RegistryError>,
    ) -> Poll<AsyncOutcome<InvocationExecution, RegistryError>> {
        self.cleanup();
        Poll::Ready(outcome)
    }
    fn interrupted(
        &mut self,
        reason: InvocationRejection,
    ) -> AsyncOutcome<InvocationExecution, RegistryError> {
        if !self.mutating {
            return AsyncOutcome::NotApplied(reason);
        }
        let recovery = self.work().and_then(|work| {
            let runtime = self.runtime()?;
            match work {
                InvocationWork::Command(command, _) => match runtime
                    .recover_invocation(&self.authenticated, &command)?
                {
                    ServiceRecovery::Completed(result) => Ok(Some(InvocationExecution::Command(
                        runtime.recovered_execution(&command, *result),
                    ))),
                    ServiceRecovery::Unknown if !self.reserved => Ok(None),
                    _ => Err(MapServiceError::ActiveOwners.into()),
                },
                InvocationWork::Helper(request, _) => {
                    match runtime.recover_helper_invocation(&self.authenticated, &request)? {
                        ClusterHelperRecovery::Completed(result) => {
                            Ok(Some(InvocationExecution::Helper(result)))
                        }
                        ClusterHelperRecovery::Unknown if !self.reserved => Ok(None),
                        _ => Err(MapServiceError::ActiveOwners.into()),
                    }
                }
            }
        });
        match recovery {
            Ok(Some(execution)) if !execution.uncertain() => AsyncOutcome::Completed(Ok(execution)),
            Ok(None) => AsyncOutcome::NotApplied(reason),
            Err(error) if !self.reserved => AsyncOutcome::Completed(Err(error)),
            _ => AsyncOutcome::OutcomeUnknown,
        }
    }
    fn retry_failure(
        &mut self,
        error: &ServiceClusterError,
        options: Option<&RuntimeCallOptions>,
        reserving: bool,
        context: &Context<'_>,
    ) -> bool {
        let allowed = matches!(
            error,
            ServiceClusterError::NodeUnavailable
                | ServiceClusterError::ReplicaNotProved
                | ServiceClusterError::TransportTimeout
                | ServiceClusterError::TransferBackpressure
        ) || (matches!(
            error,
            ServiceClusterError::StaleEpoch | ServiceClusterError::StaleOwner
        ) && options
            .is_none_or(|options| options.call.expected_epoch.is_none()));
        if !allowed {
            return false;
        }
        self.park_retry(reserving, context)
    }
    fn park_retry(&mut self, reserving: bool, context: &Context<'_>) -> bool {
        let mut control = self.control.lock().unwrap();
        if control.cancelled || control.attempts >= self.retry.maximum_attempts {
            return false;
        }
        control.attempts += 1;
        control.waiting_for_capacity = true;
        control.phase = if reserving || !self.mutating {
            ClusterInvocationPhase::Claimed
        } else {
            ClusterInvocationPhase::Reserved
        };
        drop(control);
        if let (Some(scheduler), Some(permit)) = (self.scheduler(), self.permit.as_ref()) {
            let Ok(observed) = permit.park() else {
                return false;
            };
            self.retry_wait = Some(observed);
            if !scheduler.wait_for_change(permit.ticket(), observed, context.waker()) {
                context.waker().wake_by_ref();
            }
        }
        true
    }
    fn retry_ready(&mut self, context: &Context<'_>) -> Result<bool, RegistryError> {
        let Some(observed) = self.retry_wait else {
            return Ok(true);
        };
        if let (Some(scheduler), Some(permit)) = (self.scheduler(), self.permit.as_ref()) {
            if scheduler.wait_for_change(permit.ticket(), observed, context.waker()) {
                return Ok(false);
            }
            let cut = scheduler.notification_epoch();
            if !permit.try_resume()? {
                self.retry_wait = Some(cut);
                if !scheduler.wait_for_change(permit.ticket(), cut, context.waker()) {
                    context.waker().wake_by_ref();
                }
                return Ok(false);
            }
        }
        self.retry_wait = None;
        self.control.lock().unwrap().waiting_for_capacity = false;
        Ok(true)
    }
    fn poll(
        &mut self,
        context: &mut Context<'_>,
    ) -> Poll<AsyncOutcome<InvocationExecution, RegistryError>> {
        if self.finished {
            return Poll::Ready(AsyncOutcome::Completed(Err(RegistryError::InvalidBound(
                "invocation_finished",
            ))));
        }
        let phase = {
            let mut control = self.control.lock().unwrap();
            control.waker = Some(context.waker().clone());
            if control.cancelled {
                drop(control);
                let outcome = self.interrupted(InvocationRejection::Cancelled);
                return self.complete(outcome);
            }
            control.phase
        };
        let (now, deadline) = match self.clock_and_deadline() {
            Ok(clock) => clock,
            Err(error) => {
                return self.complete(if self.reserved {
                    AsyncOutcome::OutcomeUnknown
                } else {
                    AsyncOutcome::Completed(Err(error))
                })
            }
        };
        if now >= deadline {
            let outcome = self.interrupted(InvocationRejection::Deadline);
            return self.complete(outcome);
        }
        match self.retry_ready(context) {
            Ok(false) => return Poll::Pending,
            Ok(true) => {}
            Err(error) => {
                return self.complete(if self.mutating {
                    AsyncOutcome::OutcomeUnknown
                } else {
                    AsyncOutcome::Completed(Err(error))
                })
            }
        }
        if phase == ClusterInvocationPhase::Queued {
            let cut = self
                .scheduler()
                .map(|scheduler| scheduler.notification_epoch());
            match self.claim(context) {
                Ok(false) => return Poll::Pending,
                Err(RegistryError::DispatcherBusy) => {
                    self.register_waiter_since(context, cut);
                    return Poll::Pending;
                }
                Err(error) => return self.complete(AsyncOutcome::Completed(Err(error))),
                Ok(true) => {}
            }
            self.control.lock().unwrap().phase = ClusterInvocationPhase::Claimed;
            context.waker().wake_by_ref();
            return Poll::Pending;
        }
        let dispatching = Arc::clone(&self.registry.dispatching);
        let cut = self
            .scheduler()
            .map(|scheduler| scheduler.notification_epoch());
        let Ok(dispatch) = dispatching.try_lock() else {
            self.register_waiter_since(context, cut);
            return Poll::Pending;
        };
        let work = match self.work() {
            Ok(work) => work,
            Err(error) => {
                drop(dispatch);
                return self.complete(if self.reserved {
                    AsyncOutcome::OutcomeUnknown
                } else {
                    AsyncOutcome::Completed(Err(error))
                });
            }
        };
        if let Err(error) = self.preflight(&work) {
            drop(dispatch);
            if matches!(
                &error,
                RegistryError::Quota {
                    dimension: "invocation_growth_credit",
                    ..
                }
            ) {
                self.control.lock().unwrap().error = Some(error.clone());
                if self.park_retry(
                    self.mutating && phase == ClusterInvocationPhase::Claimed,
                    context,
                ) {
                    return Poll::Pending;
                }
            }
            if let RegistryError::Cluster(cluster_error) = &error {
                self.control.lock().unwrap().error = Some(error.clone());
                if self.retry_failure(
                    cluster_error,
                    work.options(),
                    self.mutating && phase == ClusterInvocationPhase::Claimed,
                    context,
                ) {
                    return Poll::Pending;
                }
            }
            let outcome = if self.reserved {
                AsyncOutcome::OutcomeUnknown
            } else {
                AsyncOutcome::Completed(Err(error))
            };
            return self.complete(outcome);
        }
        let runtime = match self.runtime() {
            Ok(runtime) => runtime,
            Err(error) => {
                drop(dispatch);
                return self.complete(AsyncOutcome::Completed(Err(error)));
            }
        };
        let reserving = self.mutating && phase == ClusterInvocationPhase::Claimed;
        let options = work.options().cloned();
        {
            // Cancellation and crossing the admission boundary share one control cut.
            // A caller observing NotApplied in Claimed cannot race a later root admission.
            let mut control = self.control.lock().unwrap();
            if control.cancelled {
                drop(control);
                drop(dispatch);
                let outcome = self.interrupted(InvocationRejection::Cancelled);
                return self.complete(outcome);
            }
            control.phase = if reserving {
                ClusterInvocationPhase::Reserving
            } else {
                ClusterInvocationPhase::Dispatching
            };
            control.reservation_attempted |= reserving;
        }
        let result = if reserving {
            self.reserved = true;
            match &work {
                InvocationWork::Command(command, options) => runtime
                    .reserve_invocation(
                        &self.authenticated,
                        self.permit.as_ref().expect("claimed permit"),
                        command,
                        options.as_ref(),
                    )
                    .map(|_| InvocationStep::Reserved),
                InvocationWork::Helper(request, options) => runtime
                    .reserve_helper_invocation(
                        &self.authenticated,
                        self.permit.as_ref().expect("claimed permit"),
                        request,
                        Some(options),
                    )
                    .map(|reservation| match reservation {
                        ClusterHelperReservation::Recovered(result) => {
                            InvocationStep::Executed(InvocationExecution::Helper(result))
                        }
                        ClusterHelperReservation::NotApplied => InvocationStep::NotApplied,
                        ClusterHelperReservation::Reserved => InvocationStep::Reserved,
                    }),
            }
        } else {
            match work {
                InvocationWork::Command(command, options) => runtime
                    .execute(
                        &self.authenticated,
                        self.permit.as_ref().expect("claimed permit"),
                        command,
                        options.as_ref(),
                    )
                    .map(|value| InvocationStep::Executed(InvocationExecution::Command(value))),
                InvocationWork::Helper(request, options) => runtime
                    .execute_helper(
                        &self.authenticated,
                        self.permit.as_ref().expect("claimed permit"),
                        request,
                        Some(&options),
                    )
                    .map(|value| InvocationStep::Executed(InvocationExecution::Helper(value))),
            }
        };
        drop(dispatch);
        if let Some(scheduler) = self.scheduler() {
            scheduler.wake_waiters()
        }
        match result {
            Err(error) => {
                self.control.lock().unwrap().error = Some(error.clone().into());
                if self.retry_failure(&error, options.as_ref(), reserving, context) {
                    Poll::Pending
                } else if self.mutating {
                    self.complete(AsyncOutcome::OutcomeUnknown)
                } else {
                    self.complete(AsyncOutcome::Completed(Err(error.into())))
                }
            }
            Ok(InvocationStep::NotApplied) => {
                self.reserved = false;
                self.complete(AsyncOutcome::NotApplied(
                    InvocationRejection::BackendDidNotAdmit,
                ))
            }
            Ok(InvocationStep::Reserved) => {
                self.control.lock().unwrap().phase = ClusterInvocationPhase::Reserved;
                context.waker().wake_by_ref();
                Poll::Pending
            }
            Ok(InvocationStep::Executed(execution)) if self.mutating && execution.uncertain() => {
                let error = execution
                    .failure()
                    .cloned()
                    .unwrap_or(ServiceClusterError::ReplicaNotProved);
                self.control.lock().unwrap().error = Some(error.clone().into());
                if self.retry_failure(&error, options.as_ref(), false, context) {
                    Poll::Pending
                } else {
                    self.complete(AsyncOutcome::OutcomeUnknown)
                }
            }
            Ok(InvocationStep::Executed(execution)) => {
                self.complete(AsyncOutcome::Completed(Ok(execution)))
            }
        }
    }
}
impl Future for RegistryClusterInvocation {
    type Output = AsyncOutcome<ServiceClusterExecution, RegistryError>;
    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        self.get_mut()
            .inner
            .poll(context)
            .map(|outcome| match outcome {
                AsyncOutcome::Completed(Ok(InvocationExecution::Command(value))) => {
                    AsyncOutcome::Completed(Ok(value))
                }
                AsyncOutcome::Completed(Ok(_)) => {
                    AsyncOutcome::Completed(Err(MapServiceError::InvalidArgument.into()))
                }
                AsyncOutcome::Completed(Err(error)) => AsyncOutcome::Completed(Err(error)),
                AsyncOutcome::NotApplied(reason) => AsyncOutcome::NotApplied(reason),
                AsyncOutcome::OutcomeUnknown => AsyncOutcome::OutcomeUnknown,
            })
    }
}
impl Future for RegistryClusterHelperInvocation {
    type Output = AsyncOutcome<ClusterHelperExecution, RegistryError>;
    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        self.get_mut()
            .inner
            .poll(context)
            .map(|outcome| match outcome {
                AsyncOutcome::Completed(Ok(InvocationExecution::Helper(value))) => {
                    AsyncOutcome::Completed(Ok(value))
                }
                AsyncOutcome::Completed(Ok(_)) => {
                    AsyncOutcome::Completed(Err(MapServiceError::InvalidArgument.into()))
                }
                AsyncOutcome::Completed(Err(error)) => AsyncOutcome::Completed(Err(error)),
                AsyncOutcome::NotApplied(reason) => AsyncOutcome::NotApplied(reason),
                AsyncOutcome::OutcomeUnknown => AsyncOutcome::OutcomeUnknown,
            })
    }
}
impl Drop for Invocation {
    fn drop(&mut self) {
        self.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value_plane_cluster_auth_075::RuntimeMember;
    use crate::value_plane_loopback_075::LoopbackFault;
    use crate::value_plane_service_cluster_075::ServiceClusterBounds;
    use hydracache_cluster_transport_axum::NodeCredential;
    use std::sync::{atomic::AtomicUsize, mpsc};
    use std::task::Wake;
    use std::time::Duration;

    fn fixture() -> (
        UnifiedServiceRegistry,
        AuthenticatedNamespace,
        RegistryHandle,
    ) {
        let security = SecurityBounds::default();
        let trust = TrustEpochWindow::new(1, 3).unwrap();
        let principal = IngressPrincipal {
            tenant: "tenant".into(),
        };
        let verified = VerifiedMapCommand::new(
            SurfaceId::Hc2Rust,
            &principal,
            "tenant",
            "map",
            1,
            b"k".to_vec(),
            SurfaceOperation::Read,
        )
        .unwrap();
        let claims = RouteClaims {
            authenticated: true,
            authenticated_tenant: "tenant",
            claimed_tenant: "tenant",
            expected_namespace_generation: 1,
            actual_namespace_generation: 1,
            expected_topology_epoch: 1,
            actual_topology_epoch: 1,
            proxy_hops: 0,
            deadline_remaining_ticks: 1000,
            trust_epoch: 1,
            encoded_envelope_bytes: 64,
            key_bytes: 1,
            value_bytes: 0,
        };
        let auth =
            AuthenticatedNamespace::verify(&principal, &verified, &claims, security, trust, 0)
                .unwrap();
        let registry = UnifiedServiceRegistry::new_clustered(
            RegistryBounds {
                partitions: 2,
                ..Default::default()
            },
            CollectionBounds::default(),
            ServiceLimits::default(),
            security,
            trust,
            ClusterRuntimeConfig {
                cluster: "test".into(),
                members: ["a", "b", "c"]
                    .into_iter()
                    .map(|node| RuntimeMember {
                        node: node.into(),
                        generation: 1,
                        credential: NodeCredential::new("key", format!("secret-{node}")),
                    })
                    .collect(),
                entry_node: "c".into(),
                bounds: ServiceClusterBounds {
                    partitions: 2,
                    ..Default::default()
                },
                admission: crate::value_plane_cluster_admission_075::ClusterAdmissionBounds {
                    max_replica_bytes: 64 * 1024 * 1024,
                    ..Default::default()
                },
                max_replays: 4096,
            },
        )
        .unwrap();
        let handle = registry.open(&auth, auth.address()).unwrap();
        (registry, auth, handle)
    }
    fn runtime(
        registry: &UnifiedServiceRegistry,
        handle: &RegistryHandle,
    ) -> ClusterNamespaceRuntime {
        let state = registry.state.lock().unwrap();
        let NamespaceBackend::Cluster(runtime) = &state.maps[&address_key(&handle.address)].service
        else {
            unreachable!()
        };
        runtime.clone()
    }
    fn poll<F: Future + Unpin>(future: &mut F) -> Poll<F::Output> {
        Pin::new(future).poll(&mut Context::from_waker(Waker::noop()))
    }

    #[test]
    fn resumed_helper_refuses_new_transfer_growth_before_backend_or_permit_credits_can_change() {
        let (registry, auth, handle) = fixture();
        let runtime = runtime(&registry, &handle);
        let identity = (0..16)
            .map(|index| MutationIdentity::new(format!("growth-{index}"), 1))
            .find(|identity| runtime.cluster.lock().unwrap().coordinator_for(identity) == 0)
            .unwrap();
        let request =
            ClusterHelperRequest::new(identity, ClusterHelperCommand::ExpireStep { max_work: 1 });
        let mut options = RuntimeCallOptions::default();
        options.call.faults.coordinator.ack = LoopbackFault::Drop;
        let (mut future, control) = registry
            .cluster_helper_invocation_with_retry(
                &handle,
                &auth,
                SurfaceId::Hc2Rust,
                request.clone(),
                options,
                ClusterRetryPolicy::new(3).unwrap(),
            )
            .unwrap();
        assert!(poll(&mut future).is_pending());
        assert!(poll(&mut future).is_pending());
        let reserved =
            registry.state.lock().unwrap().active_invocations[&control.ticket().0].growth;
        let credit = future
            .inner
            .scheduler()
            .unwrap()
            .credit_usage(future.inner.permit.as_ref().unwrap().ticket())
            .unwrap();
        let held_growth =
            credit.0 - future.inner.job.as_ref().unwrap().bytes - handle.address.tenant.len();
        registry
            .enqueue_maintenance(
                &handle,
                &auth,
                RuntimeMaintenance::RepairStep {
                    partition: 1,
                    backup: "a".into(),
                    faults: crate::value_plane_service_cluster_075::ServiceLinkFaults::default(),
                    maximum_work: 1,
                },
            )
            .unwrap();
        let RegistryResult::Maintenance(MaintenanceResult::Online(step)) =
            registry.run_next().unwrap().unwrap().result.unwrap()
        else {
            panic!("actual transfer required")
        };
        assert!(step.work > 0);
        let forecast = runtime.helper_preflight(&auth, &request).unwrap();
        assert!(
            forecast.growth > held_growth,
            "fixture must exceed its signed lane peak: growth={} held={} original={}",
            forecast.growth,
            held_growth,
            reserved
        );
        let before = registry.diagnostics(&handle, &auth).unwrap();
        let usage = registry.usage();
        assert!(poll(&mut future).is_pending());
        assert!(
            matches!(control.last_error(),Some(RegistryError::Quota {dimension:"invocation_growth_credit",maximum,actual}) if maximum==held_growth && actual>held_growth)
        );
        let after = registry.diagnostics(&handle, &auth).unwrap();
        assert_eq!(after.statistics, before.statistics);
        assert_eq!(after.resources, before.resources);
        assert_eq!(after.owners, before.owners);
        assert_eq!(
            registry.state.lock().unwrap().active_invocations[&control.ticket().0].growth,
            reserved
        );
        assert_eq!(registry.usage().reserved_growth, usage.reserved_growth);
        assert_eq!(
            future
                .inner
                .scheduler()
                .unwrap()
                .credit_usage(future.inner.permit.as_ref().unwrap().ticket())
                .unwrap(),
            credit
        );
        assert_eq!(control.cancel(), CancellationDisposition::OutcomeUnknown);
        assert!(matches!(
            poll(&mut future),
            Poll::Ready(AsyncOutcome::OutcomeUnknown)
        ));
        assert_eq!(registry.usage().reserved_growth, 0);
    }

    #[test]
    fn materialized_root_phase_tops_up_global_credit_before_a_concurrent_admission() {
        let (registry, auth, handle) = fixture();
        let runtime = runtime(&registry, &handle);
        let mut command = ServiceCommand::point("put", b"k");
        command.value = Some(b"v".to_vec());
        command.identity = Some(MutationIdentity::new("phase", 1));
        let (mut future, control) = registry
            .cluster_invocation(
                &handle,
                &auth,
                SurfaceId::Resp,
                command,
                RuntimeCallOptions::default(),
            )
            .unwrap();
        assert!(poll(&mut future).is_pending());
        let old = registry.state.lock().unwrap().active_invocations[&control.ticket().0].growth;
        let permit = future.inner.permit.as_ref().unwrap();
        let lane_bytes = permit.work().bytes;
        assert!(poll(&mut future).is_pending());
        assert_eq!(control.phase(), ClusterInvocationPhase::Reserved);
        let work = future.inner.work().unwrap();
        let InvocationWork::Command(command, _) = &work else {
            unreachable!()
        };
        let required = runtime.preflight_registry_growth(command).unwrap();
        assert!(
            required > old,
            "own materialized root must need additional snapshot-copy credit"
        );
        let binding = runtime.admission_binding(&auth, command);
        let receipt = future
            .inner
            .permit
            .as_ref()
            .unwrap()
            .receipt_digest(&binding);
        let mut newcomer = ServiceCommand::point("put", b"other");
        newcomer.value = Some(vec![7; 64]);
        newcomer.identity = Some(MutationIdentity::new("other", 1));
        let newcomer_growth = runtime.preflight_registry_growth(&newcomer).unwrap();
        assert!(required - old <= newcomer_growth);
        let owned = registry.usage().owned_bytes;
        registry.state.lock().unwrap().bounds.max_owned_bytes = owned + old + newcomer_growth;
        future.inner.preflight(&work).unwrap();
        assert_eq!(
            registry.state.lock().unwrap().active_invocations[&control.ticket().0].growth,
            required
        );
        assert_eq!(registry.usage().reserved_growth, required);
        assert_eq!(
            future.inner.permit.as_ref().unwrap().work().bytes,
            lane_bytes
        );
        assert_eq!(
            future
                .inner
                .permit
                .as_ref()
                .unwrap()
                .receipt_digest(&binding),
            receipt
        );
        let concurrent = registry.clone();
        let other_auth = auth.clone();
        let other_handle = handle.clone();
        let refused = std::thread::spawn(move || {
            concurrent.enqueue(&other_handle, &other_auth, SurfaceId::Resp, newcomer)
        })
        .join()
        .unwrap();
        assert!(
            matches!(
                refused,
                Err(RegistryError::Quota {
                    dimension: "owned_bytes",
                    ..
                })
            ),
            "{refused:?}"
        );
        assert_eq!(registry.usage().queued_requests, 1);
        drop(future);
        assert_eq!(registry.usage().reserved_growth, 0);
    }

    #[test]
    fn new_transfer_growth_beyond_a_read_permits_original_peak_refuses_before_dispatch() {
        let (registry, auth, handle) = fixture();
        let mut options = RuntimeCallOptions::default();
        options.call.faults.data.request = LoopbackFault::Drop;
        let (mut future, control) = registry
            .cluster_read_invocation_with_retry(
                &handle,
                &auth,
                SurfaceId::Resp,
                ServiceCommand::point("get", b"k"),
                options,
                ClusterRetryPolicy::new(3).unwrap(),
            )
            .unwrap();
        assert!(poll(&mut future).is_pending());
        assert!(poll(&mut future).is_pending());
        registry
            .enqueue_maintenance(
                &handle,
                &auth,
                RuntimeMaintenance::RepairStep {
                    partition: 1,
                    backup: "a".into(),
                    faults: crate::value_plane_service_cluster_075::ServiceLinkFaults::default(),
                    maximum_work: 1,
                },
            )
            .unwrap();
        registry.run_next().unwrap().unwrap().result.unwrap();
        let before = registry.diagnostics(&handle, &auth).unwrap();
        let owned = registry.usage();
        assert!(poll(&mut future).is_pending());
        assert!(matches!(
            control.last_error(),
            Some(RegistryError::Quota {
                dimension: "invocation_growth_credit",
                ..
            })
        ));
        let after = registry.diagnostics(&handle, &auth).unwrap();
        assert_eq!(before.statistics, after.statistics);
        assert_eq!(before.resources, after.resources);
        assert_eq!(before.owners, after.owners);
        assert_eq!(registry.usage().reserved_growth, owned.reserved_growth);
        assert_eq!(control.cancel(), CancellationDisposition::NotApplied);
        assert!(matches!(
            poll(&mut future),
            Poll::Ready(AsyncOutcome::NotApplied(InvocationRejection::Cancelled))
        ));
        assert_eq!(registry.usage().reserved_growth, 0);
    }

    #[test]
    fn queued_helper_publishes_larger_transfer_growth_before_competing_enqueue_and_releases_it() {
        let (registry, auth, handle) = fixture();
        let runtime = runtime(&registry, &handle);
        let mut put = ServiceCommand::point("put", b"k");
        put.value = Some(b"old".to_vec());
        put.identity = Some(MutationIdentity::new("seed", 1));
        registry
            .enqueue(&handle, &auth, SurfaceId::Resp, put)
            .unwrap();
        registry.run_next().unwrap().unwrap().result.unwrap();
        registry
            .enqueue(
                &handle,
                &auth,
                SurfaceId::Resp,
                ServiceCommand::point("get_entry_view", b"k"),
            )
            .unwrap();
        let RegistryResult::ClusterCommand(view) =
            registry.run_next().unwrap().unwrap().result.unwrap()
        else {
            panic!("actual token required")
        };
        let request = ClusterHelperRequest::new(
            MutationIdentity::new("queued-helper", 1),
            ClusterHelperCommand::VersionBulk {
                operation: crate::value_plane_service_075::VersionBulkOperation::Replace,
                entries: vec![crate::value_plane_service_075::VersionBulkEntry {
                    key: b"k".to_vec(),
                    token: view.result.token.unwrap(),
                    value: Some(b"new".to_vec()),
                    ttl: ServiceTtl::Preserve,
                }],
            },
        );
        let ticket = registry
            .enqueue_helper(
                &handle,
                &auth,
                SurfaceId::Hc2Rust,
                request.clone(),
                RuntimeCallOptions::default(),
            )
            .unwrap();
        let old = registry.state.lock().unwrap().jobs[&ticket.0].growth;
        let partition = runtime.partition_for(b"k");
        let target = if partition == 0 { "c" } else { "a" };
        let repair = registry
            .enqueue_maintenance(
                &handle,
                &auth,
                RuntimeMaintenance::RepairStep {
                    partition,
                    backup: target.into(),
                    faults: crate::value_plane_service_cluster_075::ServiceLinkFaults::default(),
                    maximum_work: 1,
                },
            )
            .unwrap();
        let record = registry.run_next().unwrap().unwrap();
        assert_eq!(record.ticket, repair);
        record.result.unwrap();
        let growth = runtime.helper_preflight(&auth, &request).unwrap().growth;
        assert!(
            growth > old,
            "new transfer must require additional held decisions"
        );
        let newcomer = ServiceCommand::point("get", b"other");
        let newcomer_growth = runtime.preflight_registry_growth(&newcomer).unwrap();
        assert!(growth - old <= newcomer_growth);
        let owned = registry.usage().owned_bytes;
        registry.state.lock().unwrap().bounds.max_owned_bytes = owned + old + newcomer_growth;
        // Explicitly pause at the same metadata publication cut used by dispatch().
        refresh_job_growth(
            &mut registry.state.lock().unwrap(),
            ticket.0,
            &auth.address.tenant,
            growth,
            None,
        )
        .unwrap();
        assert_eq!(
            registry.state.lock().unwrap().jobs[&ticket.0].growth,
            growth
        );
        assert_eq!(registry.usage().reserved_growth, growth);
        let concurrent = registry.clone();
        let other_auth = auth.clone();
        let other_handle = handle.clone();
        let rejected = std::thread::spawn(move || {
            concurrent.enqueue(&other_handle, &other_auth, SurfaceId::Resp, newcomer)
        })
        .join()
        .unwrap();
        assert!(
            matches!(
                rejected,
                Err(RegistryError::Quota {
                    dimension: "owned_bytes",
                    ..
                })
            ),
            "{rejected:?}"
        );
        let record = registry.run_next().unwrap().unwrap();
        assert_eq!(record.ticket, ticket);
        assert!(matches!(
            record.result.unwrap(),
            RegistryResult::ClusterHelper(_)
        ));
        assert_eq!(registry.usage().reserved_growth, 0);
        assert_eq!(
            registry
                .cluster_admission_observations()
                .unwrap()
                .owned_bytes,
            [0; 3]
        );
    }

    struct ReentrantWake {
        registry: UnifiedServiceRegistry,
        calls: AtomicUsize,
    }
    impl Wake for ReentrantWake {
        fn wake(self: Arc<Self>) {
            self.wake_by_ref();
        }
        fn wake_by_ref(self: &Arc<Self>) {
            assert!(
                self.registry.state.try_lock().is_ok(),
                "callback must run after metadata unlock"
            );
            assert!(
                self.registry.dispatching.try_lock().is_ok(),
                "callback must run after dispatcher unlock"
            );
            self.calls.fetch_add(1, Ordering::SeqCst);
        }
    }
    #[test]
    fn dispatcher_release_before_waiter_arm_self_wakes_for_queued_and_claimed_invocations() {
        for claimed in [false, true] {
            let (registry, auth, handle) = fixture();
            let (mut future, _) = registry
                .cluster_read_invocation(
                    &handle,
                    &auth,
                    SurfaceId::Resp,
                    ServiceCommand::point("get", b"k"),
                    RuntimeCallOptions::default(),
                )
                .unwrap();
            if claimed {
                assert!(poll(&mut future).is_pending());
            }
            let scheduler = future.inner.scheduler().unwrap();
            let cut = scheduler.notification_epoch();
            let dispatching = Arc::clone(&registry.dispatching);
            let notifier = scheduler.clone();
            let (held_tx, held_rx) = mpsc::channel();
            let (release_tx, release_rx) = mpsc::channel();
            let worker = std::thread::spawn(move || {
                let guard = dispatching.lock().unwrap();
                held_tx.send(()).unwrap();
                release_rx.recv_timeout(Duration::from_secs(2)).unwrap();
                drop(guard);
                notifier.wake_waiters();
            });
            held_rx.recv_timeout(Duration::from_secs(2)).unwrap();
            if !claimed {
                assert_eq!(
                    future.inner.claim(&Context::from_waker(Waker::noop())),
                    Err(RegistryError::DispatcherBusy)
                );
            } else {
                assert!(registry.dispatching.try_lock().is_err());
            }
            // Force the exact race: the dispatcher completes before the waiter is armed.
            release_tx.send(()).unwrap();
            worker.join().unwrap();
            let observer = Arc::new(ReentrantWake {
                registry: registry.clone(),
                calls: AtomicUsize::new(0),
            });
            let waker = Waker::from(Arc::clone(&observer));
            future
                .inner
                .register_waiter_since(&Context::from_waker(&waker), Some(cut));
            assert_eq!(observer.calls.load(Ordering::SeqCst), 1);
            let mut completed = false;
            for _ in 0..3 {
                if let Poll::Ready(AsyncOutcome::Completed(Ok(_))) = poll(&mut future) {
                    completed = true;
                    break;
                }
            }
            assert!(completed);
            assert_eq!(registry.usage().reserved_growth, 0);
        }
    }
}
