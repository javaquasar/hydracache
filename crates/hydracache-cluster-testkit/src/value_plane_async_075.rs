//! Lazy, bounded async API over test-only backends, with explicit admission and cancellation.

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio::sync::Notify;

use crate::value_plane_collection_075::{
    CollectionError, EntryView, ReferenceCollectionMap, ReplacementEntry, ReplacementReceipt,
    ScanCursor, ScanPage, ScanProjection, TimedMutationOptions, TimedMutationOutcome,
};
use crate::value_plane_model_075::{identity_blank, MutationIdentity};

/// Only explicit logical signals advance invocation deadlines in this reference facade.
#[derive(Debug, Default)]
pub struct InvocationClock {
    now: AtomicU64,
    changed: Notify,
}

impl InvocationClock {
    pub fn now(&self) -> u64 {
        self.now.load(Ordering::SeqCst)
    }

    pub fn advance_to(&self, now: u64) -> Result<(), InvocationRejection> {
        self.now
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                (now >= current).then_some(now)
            })
            .map_err(|_| InvocationRejection::ClockRegressed)?;
        self.changed.notify_waiters();
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvocationRejection {
    InvalidIdentity,
    InvalidBound,
    Capacity,
    Deadline,
    Cancelled,
    ClockRegressed,
    BackendDidNotAdmit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AsyncOutcome<T, E> {
    Completed(Result<T, E>),
    NotApplied(InvocationRejection),
    OutcomeUnknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvocationResult<T, E> {
    pub identity: MutationIdentity,
    pub outcome: AsyncOutcome<T, E>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancellationDisposition {
    NotApplied,
    OutcomeUnknown,
    AlreadyFinished,
}

/// Recovery is read-only. A busy store cannot prove that an earlier invocation was absent.
pub enum BackendRecovery<T> {
    NotRetained,
    Retained(T),
    Unavailable,
}

#[derive(Debug, Clone, Copy)]
enum Phase {
    Pending,
    Admitted,
    Interrupted {
        admitted: bool,
        reason: InvocationRejection,
    },
    Finished,
}

#[derive(Debug)]
struct InvocationState {
    phase: Mutex<Phase>,
    changed: Notify,
}

/// A cloned handle can interrupt the invocation without discarding its caller-owned identity.
#[derive(Debug, Clone)]
pub struct InvocationHandle {
    identity: MutationIdentity,
    state: Arc<InvocationState>,
}

impl InvocationHandle {
    pub fn identity(&self) -> &MutationIdentity {
        &self.identity
    }

    /// Describes interruption of this attempt; the future can still recover a prior completion
    /// carrying the same identity, which always takes precedence over a not-applied observation.
    pub fn cancel(&self) -> CancellationDisposition {
        self.interrupt(InvocationRejection::Cancelled)
    }

    fn interrupt(&self, reason: InvocationRejection) -> CancellationDisposition {
        let mut phase = self.state.phase.lock().expect("invocation state poisoned");
        let disposition = match *phase {
            Phase::Pending => {
                *phase = Phase::Interrupted {
                    admitted: false,
                    reason,
                };
                CancellationDisposition::NotApplied
            }
            Phase::Admitted => {
                *phase = Phase::Interrupted {
                    admitted: true,
                    reason,
                };
                CancellationDisposition::OutcomeUnknown
            }
            Phase::Interrupted { admitted, .. } => {
                if admitted {
                    CancellationDisposition::OutcomeUnknown
                } else {
                    CancellationDisposition::NotApplied
                }
            }
            Phase::Finished => CancellationDisposition::AlreadyFinished,
        };
        drop(phase);
        self.state.changed.notify_waiters();
        disposition
    }
}

/// Backends must cross this fence immediately before they may mutate their authoritative state.
#[derive(Debug, Clone)]
pub struct InvocationContext {
    pub identity: MutationIdentity,
    pub deadline: u64,
    clock: Arc<InvocationClock>,
    state: Arc<InvocationState>,
}

impl InvocationContext {
    pub fn now(&self) -> u64 {
        self.clock.now()
    }

    pub fn admit(&self) -> Result<(), InvocationRejection> {
        let mut phase = self.state.phase.lock().expect("invocation state poisoned");
        match *phase {
            Phase::Pending if self.now() < self.deadline => {
                *phase = Phase::Admitted;
                Ok(())
            }
            Phase::Pending => {
                *phase = Phase::Interrupted {
                    admitted: false,
                    reason: InvocationRejection::Deadline,
                };
                Err(InvocationRejection::Deadline)
            }
            Phase::Admitted => Ok(()),
            Phase::Interrupted { reason, .. } => Err(reason),
            Phase::Finished => Err(InvocationRejection::Cancelled),
        }
    }
}

#[async_trait]
pub trait AsyncMapBackend: Send + Sync + 'static {
    type Command: Send + Sync + 'static;
    type Output: Send + 'static;
    type Error: Send + 'static;

    fn recover(
        &self,
        _command: &Self::Command,
        _identity: &MutationIdentity,
    ) -> Result<BackendRecovery<Self::Output>, Self::Error> {
        Ok(BackendRecovery::NotRetained)
    }

    async fn execute(
        &self,
        command: &Self::Command,
        context: InvocationContext,
    ) -> Result<Self::Output, Self::Error>;
}

pub type InvocationFuture<T, E> = Pin<Box<dyn Future<Output = InvocationResult<T, E>> + Send>>;
pub type StartedInvocation<T, E> =
    Result<(InvocationHandle, InvocationFuture<T, E>), InvocationRejection>;

struct InflightGuard {
    inflight: Arc<AtomicUsize>,
    handle: InvocationHandle,
}

impl Drop for InflightGuard {
    fn drop(&mut self) {
        self.handle.cancel();
        self.inflight.fetch_sub(1, Ordering::SeqCst);
    }
}

pub struct AsyncIMapFacade<B: AsyncMapBackend> {
    backend: Arc<B>,
    clock: Arc<InvocationClock>,
    max_inflight: usize,
    inflight: Arc<AtomicUsize>,
}

impl<B: AsyncMapBackend> AsyncIMapFacade<B> {
    pub fn new(
        backend: Arc<B>,
        clock: Arc<InvocationClock>,
        max_inflight: usize,
    ) -> Result<Self, InvocationRejection> {
        if max_inflight == 0 {
            return Err(InvocationRejection::InvalidBound);
        }
        Ok(Self {
            backend,
            clock,
            max_inflight,
            inflight: Arc::new(AtomicUsize::new(0)),
        })
    }

    pub fn inflight(&self) -> usize {
        self.inflight.load(Ordering::SeqCst)
    }

    /// Allocation is bounded; backend work starts only when the returned future is polled.
    pub fn invoke(
        &self,
        identity: MutationIdentity,
        deadline: u64,
        command: B::Command,
    ) -> StartedInvocation<B::Output, B::Error> {
        if identity_blank(&identity.client) || identity.client.len() > 128 || identity.sequence == 0
        {
            return Err(InvocationRejection::InvalidIdentity);
        }
        self.inflight
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |count| {
                (count < self.max_inflight).then_some(count.saturating_add(1))
            })
            .map_err(|_| InvocationRejection::Capacity)?;
        let state = Arc::new(InvocationState {
            phase: Mutex::new(Phase::Pending),
            changed: Notify::new(),
        });
        let handle = InvocationHandle {
            identity: identity.clone(),
            state: Arc::clone(&state),
        };
        let guard = InflightGuard {
            inflight: Arc::clone(&self.inflight),
            handle: handle.clone(),
        };
        let context = InvocationContext {
            identity: identity.clone(),
            deadline,
            clock: Arc::clone(&self.clock),
            state: Arc::clone(&state),
        };
        let backend = Arc::clone(&self.backend);
        let clock = Arc::clone(&self.clock);
        let future_handle = handle.clone();
        let future = Box::pin(async move {
            let _guard = guard;
            let work = backend.execute(&command, context);
            tokio::pin!(work);
            loop {
                let interrupted = state.changed.notified();
                let advanced = clock.changed.notified();
                tokio::pin!(interrupted, advanced);
                interrupted.as_mut().enable();
                advanced.as_mut().enable();
                let recovery_available = match backend.recover(&command, &identity) {
                    Ok(BackendRecovery::NotRetained) => true,
                    Ok(BackendRecovery::Unavailable) => false,
                    recovery => {
                        let result = match recovery {
                            Ok(BackendRecovery::Retained(value)) => Ok(value),
                            Err(error) => Err(error),
                            _ => unreachable!("nonterminal recovery handled above"),
                        };
                        *state.phase.lock().expect("invocation state poisoned") = Phase::Finished;
                        return InvocationResult {
                            identity,
                            outcome: AsyncOutcome::Completed(result),
                        };
                    }
                };
                if clock.now() >= deadline {
                    future_handle.interrupt(InvocationRejection::Deadline);
                }
                let interruption = {
                    let phase = state.phase.lock().expect("invocation state poisoned");
                    match *phase {
                        Phase::Interrupted { admitted: true, .. } => {
                            Some(AsyncOutcome::OutcomeUnknown)
                        }
                        Phase::Interrupted {
                            admitted: false,
                            reason,
                        } => Some(if recovery_available {
                            AsyncOutcome::NotApplied(reason)
                        } else {
                            AsyncOutcome::OutcomeUnknown
                        }),
                        _ => None,
                    }
                };
                if let Some(outcome) = interruption {
                    *state.phase.lock().expect("invocation state poisoned") = Phase::Finished;
                    return InvocationResult { identity, outcome };
                }
                tokio::select! {
                    biased;
                    _ = &mut interrupted => {},
                    _ = &mut advanced => {},
                    result = &mut work => {
                        let mut phase = state.phase.lock().expect("invocation state poisoned");
                        let outcome = match *phase {
                            Phase::Interrupted { admitted: true, .. } => AsyncOutcome::OutcomeUnknown,
                            Phase::Interrupted { admitted: false, reason } => AsyncOutcome::NotApplied(reason),
                            Phase::Pending if result.is_ok() => AsyncOutcome::NotApplied(InvocationRejection::BackendDidNotAdmit),
                            _ => AsyncOutcome::Completed(result),
                        };
                        *phase = Phase::Finished;
                        return InvocationResult { identity, outcome };
                    }
                }
            }
        });
        Ok((handle, future))
    }
}

#[derive(Debug, Clone)]
pub enum CollectionAsyncCommand {
    Set {
        key: Vec<u8>,
        value: Vec<u8>,
        expires_at: Option<u64>,
    },
    Remove {
        key: Vec<u8>,
    },
    GetOrDefault {
        key: Vec<u8>,
        default: Vec<u8>,
    },
    ContainsEntry {
        key: Vec<u8>,
        value: Vec<u8>,
    },
    GetEntryView {
        key: Vec<u8>,
    },
    ReplaceAllDetailed {
        entries: Vec<ReplacementEntry>,
    },
    Scan {
        projection: ScanProjection,
        cursor: Option<ScanCursor>,
        page_items: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectionAsyncOutput {
    Mutation(TimedMutationOutcome),
    Value(Vec<u8>),
    Contains(bool),
    Entry(Option<EntryView>),
    Replacements(Vec<ReplacementReceipt>),
    Page(ScanPage),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectionAsyncError {
    Admission(InvocationRejection),
    Model(CollectionError),
}

/// Async access to the same collection model; waiting for its lock happens before admission.
pub struct CollectionAsyncBackend {
    pub map: Arc<tokio::sync::Mutex<ReferenceCollectionMap>>,
}

#[async_trait]
impl AsyncMapBackend for CollectionAsyncBackend {
    type Command = CollectionAsyncCommand;
    type Output = CollectionAsyncOutput;
    type Error = CollectionAsyncError;

    fn recover(
        &self,
        command: &Self::Command,
        identity: &MutationIdentity,
    ) -> Result<BackendRecovery<Self::Output>, Self::Error> {
        if !matches!(
            command,
            CollectionAsyncCommand::Set { .. }
                | CollectionAsyncCommand::Remove { .. }
                | CollectionAsyncCommand::ReplaceAllDetailed { .. }
        ) {
            return Ok(BackendRecovery::NotRetained);
        }
        let Ok(map) = self.map.try_lock() else {
            return Ok(BackendRecovery::Unavailable);
        };
        let recovered = match command {
            CollectionAsyncCommand::Set {
                key,
                value,
                expires_at,
            } => map
                .recover_timed_put(identity, key, value, *expires_at)
                .map(|value| value.map(CollectionAsyncOutput::Mutation)),
            CollectionAsyncCommand::Remove { key } => map
                .recover_timed_remove(identity, key)
                .map(|value| value.map(CollectionAsyncOutput::Mutation)),
            CollectionAsyncCommand::ReplaceAllDetailed { entries } => map
                .recover_replacements(identity, entries)
                .map(|value| value.map(CollectionAsyncOutput::Replacements)),
            _ => unreachable!("read commands returned above"),
        }
        .map_err(CollectionAsyncError::Model)?;
        Ok(match recovered {
            Some(value) => BackendRecovery::Retained(value),
            None => BackendRecovery::NotRetained,
        })
    }

    async fn execute(
        &self,
        command: &Self::Command,
        context: InvocationContext,
    ) -> Result<Self::Output, Self::Error> {
        let mut map = self.map.lock().await;
        context.admit().map_err(CollectionAsyncError::Admission)?;
        let now = context.now();
        let options = TimedMutationOptions {
            now,
            deadline: context.deadline,
            cancelled: false,
            lose_response: false,
        };
        let result = match command {
            CollectionAsyncCommand::Set {
                key,
                value,
                expires_at,
            } => map
                .try_put(
                    context.identity,
                    key.clone(),
                    value.clone(),
                    *expires_at,
                    options,
                )
                .map(CollectionAsyncOutput::Mutation),
            CollectionAsyncCommand::Remove { key } => map
                .try_remove(context.identity, key, options)
                .map(CollectionAsyncOutput::Mutation),
            CollectionAsyncCommand::GetOrDefault { key, default } => map
                .get_or_default(key, default, now)
                .map(CollectionAsyncOutput::Value),
            CollectionAsyncCommand::ContainsEntry { key, value } => map
                .contains_entry(key, value, now)
                .map(CollectionAsyncOutput::Contains),
            CollectionAsyncCommand::GetEntryView { key } => map
                .get_entry_view(key, now)
                .map(CollectionAsyncOutput::Entry),
            CollectionAsyncCommand::ReplaceAllDetailed { entries } => map
                .replace_all_detailed_idempotent(context.identity, entries, now)
                .map(CollectionAsyncOutput::Replacements),
            CollectionAsyncCommand::Scan {
                projection,
                cursor,
                page_items,
            } => map
                .scan(*projection, *cursor, *page_items, now)
                .map(CollectionAsyncOutput::Page),
        };
        result.map_err(CollectionAsyncError::Model)
    }
}
