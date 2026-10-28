//! Cooperative actual-cluster invocation phases; no task, timer, or hidden value store.
use super::*;
use crate::value_plane_async_075::{AsyncOutcome, CancellationDisposition, InvocationRejection};
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
struct Control {
    phase: ClusterInvocationPhase,
    cancelled: bool,
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
    pub fn cancel(&self) -> CancellationDisposition {
        let mut control = self.control.lock().unwrap();
        if control.phase == ClusterInvocationPhase::Finished {
            return CancellationDisposition::AlreadyFinished;
        }
        control.cancelled = true;
        let outcome = if matches!(
            control.phase,
            ClusterInvocationPhase::Queued | ClusterInvocationPhase::Claimed
        ) {
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
pub struct RegistryClusterInvocation {
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
impl UnifiedServiceRegistry {
    pub fn cluster_invocation(
        &self,
        handle: &RegistryHandle,
        authenticated: &AuthenticatedNamespace,
        surface: SurfaceId,
        command: ServiceCommand,
        options: RuntimeCallOptions,
    ) -> Result<(RegistryClusterInvocation, ClusterInvocationHandle), RegistryError> {
        if !is_mutating_operation(&command.operation) || command.identity.is_none() {
            return Err(MapServiceError::InvalidArgument.into());
        }
        if command.operation == "destroy" {
            return Err(RegistryError::SafetyLaneRequired);
        }
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
        let ticket = self.enqueue_kind(
            handle,
            authenticated,
            JobKind::Execute {
                cooperative: true,
                surface,
                command,
                options: Some(options),
            },
        )?;
        let control = Arc::new(Mutex::new(Control {
            phase: ClusterInvocationPhase::Queued,
            cancelled: false,
            waker: None,
            error: None,
        }));
        Ok((
            RegistryClusterInvocation {
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
impl RegistryClusterInvocation {
    fn runtime(&self) -> Result<ClusterNamespaceRuntime, RegistryError> {
        let state = self.registry.state.lock().unwrap();
        validate_handle(&state, &self.handle, &self.authenticated, false)?;
        match &state.maps[&address_key(&self.handle.address)].service {
            NamespaceBackend::Cluster(runtime) => Ok(runtime.clone()),
            _ => Err(ServiceClusterError::UnsupportedScope.into()),
        }
    }
    fn command(&self) -> Result<(ServiceCommand, Option<RuntimeCallOptions>), RegistryError> {
        let state = self.registry.state.lock().unwrap();
        validate_handle(&state, &self.handle, &self.authenticated, false)?;
        let job = self
            .job
            .as_ref()
            .or_else(|| state.jobs.get(&self.ticket.0))
            .ok_or(RegistryError::UnknownNamespace)?;
        authorize_job(&self.authenticated, &job.kind)?;
        let JobKind::Execute {
            command, options, ..
        } = &job.kind
        else {
            return Err(MapServiceError::InvalidArgument.into());
        };
        let mut command = command.clone();
        command.now = state.now;
        command.deadline = command.deadline.min(self.authenticated.deadline);
        Ok((command, options.clone()))
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
        state.active_invocations.insert(
            self.ticket.0,
            (job.handle.address.clone(), job.bytes, job.growth),
        );
        self.job = Some(job);
        self.permit = Some(permit);
        Ok(true)
    }
    fn preflight(&self, command: &ServiceCommand) -> Result<(), RegistryError> {
        let state = self.registry.state.lock().unwrap();
        validate_handle(&state, &self.handle, &self.authenticated, false)?;
        self.authenticated.authorize_command(command)?;
        let growth = state.maps[&address_key(&self.handle.address)]
            .service
            .preflight_registry_growth(command)?;
        check_owned_growth(
            &state,
            &self.handle.address.tenant,
            growth,
            Some(self.ticket.0),
        )
    }
    fn register_waiter(&self, context: &Context<'_>) {
        let state = self.registry.state.lock().unwrap();
        if let Some(scheduler) = &state.cluster_admission {
            if let Some(ticket) = self
                .permit
                .as_ref()
                .map(ClusterPermit::ticket)
                .or_else(|| state.jobs.get(&self.ticket.0).and_then(|job| job.admission))
            {
                scheduler.register_waiter(ticket, context.waker());
            }
        }
    }
    fn wake_scheduler(&self) {
        let scheduler = self
            .registry
            .state
            .lock()
            .unwrap()
            .cluster_admission
            .clone();
        if let Some(scheduler) = scheduler {
            scheduler.wake_waiters();
        }
    }
    fn cleanup(&mut self) {
        if self.finished {
            return;
        }
        if self.reserved {
            if let Some(job) = &self.job {
                if let JobKind::Execute { command, .. } = &job.kind {
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
                    if let Some(runtime) = runtime {
                        let _ = runtime.cancel_pending(command);
                    }
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
        if let Some((_, bytes, growth)) = state.active_invocations.remove(&self.ticket.0) {
            state.queued_bytes -= bytes;
            state.reserved_growth -= growth;
        }
        let scheduler = state.cluster_admission.clone();
        drop(state);
        let permit = self.permit.take();
        let released_permit = permit.is_some();
        drop(permit);
        self.job = None;
        self.finished = true;
        let mut control = self.control.lock().unwrap();
        control.phase = ClusterInvocationPhase::Finished;
        control.waker = None;
        drop(control);
        if !released_permit {
            if let Some(scheduler) = scheduler {
                scheduler.wake_waiters();
            }
        }
    }
    fn complete(
        &mut self,
        outcome: AsyncOutcome<ServiceClusterExecution, RegistryError>,
    ) -> Poll<AsyncOutcome<ServiceClusterExecution, RegistryError>> {
        self.cleanup();
        Poll::Ready(outcome)
    }
    fn interrupted(
        &mut self,
        reason: InvocationRejection,
    ) -> AsyncOutcome<ServiceClusterExecution, RegistryError> {
        match self.command().and_then(|(command, _)| {
            let runtime = self.runtime()?;
            Ok((
                runtime.clone(),
                command.clone(),
                runtime.recover_invocation(&self.authenticated, &command)?,
            ))
        }) {
            Ok((runtime, command, ServiceRecovery::Completed(result)))
                if result.kind != "outcome_unknown" =>
            {
                let execution = runtime.recovered_execution(&command, *result);
                if execution.backup_proved && execution.failure.is_none() {
                    AsyncOutcome::Completed(Ok(execution))
                } else {
                    AsyncOutcome::OutcomeUnknown
                }
            }
            Ok((_, _, ServiceRecovery::Unknown)) if !self.reserved => {
                AsyncOutcome::NotApplied(reason)
            }
            Ok(_) => AsyncOutcome::OutcomeUnknown,
            Err(error) if !self.reserved => AsyncOutcome::Completed(Err(error)),
            Err(_) => AsyncOutcome::OutcomeUnknown,
        }
    }
}
impl Future for RegistryClusterInvocation {
    type Output = AsyncOutcome<ServiceClusterExecution, RegistryError>;
    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if this.finished {
            return Poll::Ready(AsyncOutcome::Completed(Err(RegistryError::InvalidBound(
                "invocation_finished",
            ))));
        }
        let phase = {
            let mut control = this.control.lock().unwrap();
            control.waker = Some(context.waker().clone());
            if control.cancelled {
                drop(control);
                let outcome = this.interrupted(InvocationRejection::Cancelled);
                return this.complete(outcome);
            }
            control.phase
        };
        let (command, options) = match this.command() {
            Ok(command) => command,
            Err(error) => {
                let outcome = if this.reserved {
                    AsyncOutcome::OutcomeUnknown
                } else {
                    AsyncOutcome::Completed(Err(error))
                };
                return this.complete(outcome);
            }
        };
        if command.now >= command.deadline {
            let outcome = this.interrupted(InvocationRejection::Deadline);
            return this.complete(outcome);
        }
        if phase == ClusterInvocationPhase::Queued {
            match this.claim(context) {
                Ok(false) => return Poll::Pending,
                Err(RegistryError::DispatcherBusy) => {
                    this.register_waiter(context);
                    return Poll::Pending;
                }
                Err(error) => return this.complete(AsyncOutcome::Completed(Err(error))),
                Ok(true) => {}
            }
            this.control.lock().unwrap().phase = ClusterInvocationPhase::Claimed;
            context.waker().wake_by_ref();
            return Poll::Pending;
        }
        let dispatching = Arc::clone(&this.registry.dispatching);
        let Ok(dispatch) = dispatching.try_lock() else {
            this.register_waiter(context);
            return Poll::Pending;
        };
        if let Err(error) = this.preflight(&command) {
            let outcome = if this.reserved {
                AsyncOutcome::OutcomeUnknown
            } else {
                AsyncOutcome::Completed(Err(error))
            };
            drop(dispatch);
            return this.complete(outcome);
        }
        let runtime = match this.runtime() {
            Ok(runtime) => runtime,
            Err(error) => {
                drop(dispatch);
                return this.complete(AsyncOutcome::Completed(Err(error)));
            }
        };
        let result = if phase == ClusterInvocationPhase::Claimed {
            this.reserved = true;
            this.control.lock().unwrap().phase = ClusterInvocationPhase::Reserving;
            runtime
                .reserve_invocation(
                    &this.authenticated,
                    this.permit.as_ref().expect("claimed permit"),
                    &command,
                    options.as_ref(),
                )
                .map(|_| None)
        } else {
            this.control.lock().unwrap().phase = ClusterInvocationPhase::Dispatching;
            runtime
                .execute(
                    &this.authenticated,
                    this.permit.as_ref().expect("claimed permit"),
                    command,
                    options.as_ref(),
                )
                .map(Some)
        };
        drop(dispatch);
        this.wake_scheduler();
        match result {
            Err(error) => {
                this.control.lock().unwrap().error = Some(error.into());
                this.complete(AsyncOutcome::OutcomeUnknown)
            }
            Ok(None) => {
                this.control.lock().unwrap().phase = ClusterInvocationPhase::Reserved;
                context.waker().wake_by_ref();
                Poll::Pending
            }
            Ok(Some(execution))
                if execution.result.kind == "outcome_unknown"
                    || execution.failure.is_some()
                    || !execution.backup_proved =>
            {
                this.complete(AsyncOutcome::OutcomeUnknown)
            }
            Ok(Some(execution)) => this.complete(AsyncOutcome::Completed(Ok(execution))),
        }
    }
}
impl Drop for RegistryClusterInvocation {
    fn drop(&mut self) {
        self.cleanup();
    }
}
