use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use async_trait::async_trait;
use hydracache_cluster_testkit::value_plane_async_075::*;
use hydracache_cluster_testkit::value_plane_collection_075::{
    CollectionBounds, ReferenceCollectionMap, ReplacementEntry, ScanProjection,
    TimedMutationOutcome,
};
use hydracache_cluster_testkit::value_plane_model_075::MutationIdentity;
use tokio::sync::Semaphore;

struct GatedBackend {
    before: Semaphore,
    after: Semaphore,
    applied: AtomicUsize,
    identities: Mutex<Vec<MutationIdentity>>,
}

impl GatedBackend {
    fn new(before: usize, after: usize) -> Self {
        Self {
            before: Semaphore::new(before),
            after: Semaphore::new(after),
            applied: AtomicUsize::new(0),
            identities: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl AsyncMapBackend for GatedBackend {
    type Command = bool;
    type Output = u64;
    type Error = InvocationRejection;

    async fn execute(
        &self,
        reject: &bool,
        context: InvocationContext,
    ) -> Result<u64, InvocationRejection> {
        self.identities
            .lock()
            .unwrap()
            .push(context.identity.clone());
        if *reject {
            return Err(InvocationRejection::InvalidBound);
        }
        self.before.acquire().await.unwrap().forget();
        context.admit()?;
        self.applied.fetch_add(1, Ordering::SeqCst);
        self.after.acquire().await.unwrap().forget();
        Ok(context.identity.sequence)
    }
}

fn identity(sequence: u64) -> MutationIdentity {
    MutationIdentity::new("async-client", sequence)
}

fn is_pending<T, E>(future: &mut InvocationFuture<T, E>) -> bool {
    matches!(
        future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop())),
        Poll::Pending
    )
}

#[tokio::test]
async fn async_facade_preserves_identity_deadline_and_cancellation() {
    let clock = Arc::new(InvocationClock::default());
    let backend = Arc::new(GatedBackend::new(1, 1));
    let facade = AsyncIMapFacade::new(Arc::clone(&backend), Arc::clone(&clock), 1).unwrap();
    let (handle, future) = facade.invoke(identity(1), 10, false).unwrap();
    assert_eq!(handle.identity(), &identity(1));
    let completed = future.await;
    assert_eq!(completed.identity, identity(1));
    assert_eq!(completed.outcome, AsyncOutcome::Completed(Ok(1)));
    assert_eq!(handle.cancel(), CancellationDisposition::AlreadyFinished);
    assert_eq!(facade.inflight(), 0);

    let (cancel, future) = facade.invoke(identity(2), 10, false).unwrap();
    assert_eq!(cancel.cancel(), CancellationDisposition::NotApplied);
    assert_eq!(
        future.await.outcome,
        AsyncOutcome::NotApplied(InvocationRejection::Cancelled)
    );
    assert_eq!(backend.identities.lock().unwrap().len(), 1);

    clock.advance_to(10).unwrap();
    let (_, future) = facade.invoke(identity(3), 10, false).unwrap();
    assert_eq!(
        future.await.outcome,
        AsyncOutcome::NotApplied(InvocationRejection::Deadline)
    );
    assert_eq!(
        clock.advance_to(9),
        Err(InvocationRejection::ClockRegressed)
    );
    assert_eq!(clock.now(), 10);
}

#[tokio::test]
async fn admission_boundary_preserves_uncertainty_and_releases_dropped_work() {
    let clock = Arc::new(InvocationClock::default());
    let backend = Arc::new(GatedBackend::new(1, 0));
    let facade = AsyncIMapFacade::new(Arc::clone(&backend), Arc::clone(&clock), 1).unwrap();
    let (cancel, mut future) = facade.invoke(identity(1), 10, false).unwrap();
    assert!(is_pending(&mut future));
    assert_eq!(backend.applied.load(Ordering::SeqCst), 1);
    assert_eq!(cancel.cancel(), CancellationDisposition::OutcomeUnknown);
    assert_eq!(future.await.outcome, AsyncOutcome::OutcomeUnknown);
    assert_eq!(cancel.cancel(), CancellationDisposition::AlreadyFinished);
    assert_eq!(facade.inflight(), 0);

    backend.before.add_permits(1);
    let (dropped, mut future) = facade.invoke(identity(2), 10, false).unwrap();
    assert!(is_pending(&mut future));
    drop(future);
    assert_eq!(dropped.cancel(), CancellationDisposition::OutcomeUnknown);
    assert_eq!(facade.inflight(), 0);

    let (_, mut future) = facade.invoke(identity(3), 10, false).unwrap();
    assert!(is_pending(&mut future));
    clock.advance_to(10).unwrap();
    assert_eq!(
        future.await.outcome,
        AsyncOutcome::NotApplied(InvocationRejection::Deadline)
    );

    backend.before.add_permits(1);
    let (_, mut future) = facade.invoke(identity(4), 20, false).unwrap();
    assert!(is_pending(&mut future));
    clock.advance_to(20).unwrap();
    assert_eq!(future.await.outcome, AsyncOutcome::OutcomeUnknown);
    assert_eq!(facade.inflight(), 0);
}

#[tokio::test]
async fn facade_bounds_invalid_identity_and_backend_errors_fail_explicitly() {
    let clock = Arc::new(InvocationClock::default());
    let backend = Arc::new(GatedBackend::new(0, 0));
    assert!(matches!(
        AsyncIMapFacade::new(Arc::clone(&backend), Arc::clone(&clock), 0),
        Err(InvocationRejection::InvalidBound)
    ));
    let facade = AsyncIMapFacade::new(Arc::clone(&backend), Arc::clone(&clock), 1).unwrap();
    assert!(matches!(
        facade.invoke(MutationIdentity::new("", 1), 1, false),
        Err(InvocationRejection::InvalidIdentity)
    ));
    assert!(matches!(
        facade.invoke(identity(0), 1, false),
        Err(InvocationRejection::InvalidIdentity)
    ));
    let (_, future) = facade.invoke(identity(1), 10, false).unwrap();
    assert!(matches!(
        facade.invoke(identity(2), 10, false),
        Err(InvocationRejection::Capacity)
    ));
    drop(future);
    assert_eq!(facade.inflight(), 0);
    let (_, future) = facade.invoke(identity(2), 10, true).unwrap();
    assert_eq!(
        future.await.outcome,
        AsyncOutcome::Completed(Err(InvocationRejection::InvalidBound))
    );
}

struct UnadmittedBackend;

#[async_trait]
impl AsyncMapBackend for UnadmittedBackend {
    type Command = ();
    type Output = ();
    type Error = ();
    async fn execute(&self, _: &(), _: InvocationContext) -> Result<(), ()> {
        Ok(())
    }
}

#[tokio::test]
async fn backend_cannot_report_success_without_admission() {
    let facade = AsyncIMapFacade::new(
        Arc::new(UnadmittedBackend),
        Arc::new(InvocationClock::default()),
        1,
    )
    .unwrap();
    let (_, future) = facade.invoke(identity(1), 10, ()).unwrap();
    assert_eq!(
        future.await.outcome,
        AsyncOutcome::NotApplied(InvocationRejection::BackendDidNotAdmit)
    );
}

#[tokio::test]
async fn concrete_async_adapter_uses_the_same_collection_and_replays_mutations() {
    let map = Arc::new(tokio::sync::Mutex::new(
        ReferenceCollectionMap::new(CollectionBounds::default()).unwrap(),
    ));
    let backend = Arc::new(CollectionAsyncBackend {
        map: Arc::clone(&map),
    });
    let clock = Arc::new(InvocationClock::default());
    let facade = AsyncIMapFacade::new(backend, Arc::clone(&clock), 4).unwrap();
    let put = CollectionAsyncCommand::Set {
        key: b"a".to_vec(),
        value: b"v".to_vec(),
        expires_at: Some(50),
    };
    let (_, future) = facade.invoke(identity(1), 10, put.clone()).unwrap();
    let result = future.await;
    assert!(matches!(
        result.outcome,
        AsyncOutcome::Completed(Ok(CollectionAsyncOutput::Mutation(
            TimedMutationOutcome::Applied { changed: true, .. }
        )))
    ));
    let (_, retry) = facade.invoke(identity(1), 10, put).unwrap();
    assert_eq!(retry.await, result);
    clock.advance_to(10).unwrap();
    let (cancelled_retry, expired_retry) = facade
        .invoke(
            identity(1),
            1,
            CollectionAsyncCommand::Set {
                key: b"a".to_vec(),
                value: b"v".to_vec(),
                expires_at: Some(50),
            },
        )
        .unwrap();
    cancelled_retry.cancel();
    assert_eq!(expired_retry.await, result);
    let commands = [
        CollectionAsyncCommand::GetOrDefault {
            key: b"a".to_vec(),
            default: b"fallback".to_vec(),
        },
        CollectionAsyncCommand::ContainsEntry {
            key: b"a".to_vec(),
            value: b"v".to_vec(),
        },
        CollectionAsyncCommand::GetEntryView { key: b"a".to_vec() },
        CollectionAsyncCommand::ReplaceAllDetailed {
            entries: vec![ReplacementEntry {
                key: b"a".to_vec(),
                value: b"new".to_vec(),
            }],
        },
        CollectionAsyncCommand::Scan {
            projection: ScanProjection::Entries,
            cursor: None,
            page_items: 1,
        },
        CollectionAsyncCommand::Remove { key: b"a".to_vec() },
    ];
    for (index, command) in commands.into_iter().enumerate() {
        let (_, future) = facade
            .invoke(identity(index as u64 + 2), 20, command)
            .unwrap();
        assert!(matches!(
            future.await.outcome,
            AsyncOutcome::Completed(Ok(_))
        ));
    }
    assert!(map.lock().await.is_empty(0).unwrap());

    let locked = map.lock().await;
    let (_, mut future) = facade
        .invoke(
            identity(20),
            20,
            CollectionAsyncCommand::Remove { key: b"a".to_vec() },
        )
        .unwrap();
    assert!(is_pending(&mut future));
    clock.advance_to(20).unwrap();
    drop(locked);
    // The queued lock waiter prevents read-only recovery from proving prior absence.
    assert_eq!(future.await.outcome, AsyncOutcome::OutcomeUnknown);
    assert!(map.lock().await.is_empty(20).unwrap());
}

#[tokio::test]
async fn concrete_bulk_replay_preserves_receipts_and_model_errors_are_visible() {
    let map = Arc::new(tokio::sync::Mutex::new(
        ReferenceCollectionMap::new(CollectionBounds {
            max_key_bytes: 1,
            ..CollectionBounds::default()
        })
        .unwrap(),
    ));
    map.lock()
        .await
        .set(b"a".to_vec(), b"old".to_vec(), Some(50), 0)
        .unwrap();
    let facade = AsyncIMapFacade::new(
        Arc::new(CollectionAsyncBackend {
            map: Arc::clone(&map),
        }),
        Arc::new(InvocationClock::default()),
        2,
    )
    .unwrap();
    let command = CollectionAsyncCommand::ReplaceAllDetailed {
        entries: vec![ReplacementEntry {
            key: b"a".to_vec(),
            value: b"new".to_vec(),
        }],
    };
    let (_, future) = facade.invoke(identity(1), 10, command.clone()).unwrap();
    let result = future.await;
    assert!(
        matches!(&result.outcome, AsyncOutcome::Completed(Ok(CollectionAsyncOutput::Replacements(receipts))) if receipts[0].replaced)
    );
    map.lock()
        .await
        .set(b"a".to_vec(), b"later".to_vec(), Some(50), 0)
        .unwrap();
    let revision = map.lock().await.revision();
    let (_, future) = facade.invoke(identity(1), 10, command).unwrap();
    assert_eq!(future.await, result);
    assert_eq!(map.lock().await.revision(), revision);
    let (_, conflict) = facade
        .invoke(
            identity(1),
            10,
            CollectionAsyncCommand::Remove { key: b"a".to_vec() },
        )
        .unwrap();
    assert_eq!(conflict.await.outcome, AsyncOutcome::Completed(Err(CollectionAsyncError::Model(
        hydracache_cluster_testkit::value_plane_collection_075::CollectionError::MutationIdentityConflict,
    ))));
    let (_, oversized) = facade
        .invoke(
            identity(2),
            10,
            CollectionAsyncCommand::GetOrDefault {
                key: b"aa".to_vec(),
                default: Vec::new(),
            },
        )
        .unwrap();
    assert!(matches!(
        oversized.await.outcome,
        AsyncOutcome::Completed(Err(CollectionAsyncError::Model(_)))
    ));
    assert_eq!(facade.inflight(), 0);
}

#[tokio::test]
async fn unavailable_recovery_cannot_invent_not_applied_for_an_existing_identity() {
    let map = Arc::new(tokio::sync::Mutex::new(
        ReferenceCollectionMap::new(CollectionBounds::default()).unwrap(),
    ));
    let facade = AsyncIMapFacade::new(
        Arc::new(CollectionAsyncBackend {
            map: Arc::clone(&map),
        }),
        Arc::new(InvocationClock::default()),
        1,
    )
    .unwrap();
    let locked = map.lock().await;
    let (handle, future) = facade
        .invoke(
            identity(1),
            0,
            CollectionAsyncCommand::Remove { key: b"a".to_vec() },
        )
        .unwrap();
    handle.cancel();
    assert_eq!(future.await.outcome, AsyncOutcome::OutcomeUnknown);
    drop(locked);
    assert_eq!(facade.inflight(), 0);
}
