use hydracache_cluster_testkit::imap_backend_contract_075::*;
use hydracache_cluster_testkit::imap_backend_helper_contract_075::*;
use hydracache_cluster_testkit::provisional_backend_075::{NamespaceIdentity, PartitionRoute};
use hydracache_cluster_testkit::value_plane_async_075::{
    AsyncOutcome, InvocationClock, InvocationRejection,
};
use hydracache_cluster_testkit::value_plane_collection_075::CollectionBounds;
use hydracache_cluster_testkit::value_plane_model_075::MutationIdentity;
use hydracache_cluster_testkit::value_plane_security_075::*;
use hydracache_cluster_testkit::value_plane_service_075::*;
use std::sync::Arc;
#[test]
fn helper_errors_have_stable_actionable_display() {
    let error = HelperBoundaryError::Invocation(InvocationRejection::Cancelled);
    assert_eq!(error.to_string(), format!("{error:?}"));
    assert!(error.to_string().contains("Cancelled"));
}

fn binding() -> BackendBinding {
    BackendBinding {
        namespace: NamespaceIdentity {
            tenant: 1,
            namespace: 2,
            generation: 3,
        },
        route: PartitionRoute {
            partition: 0,
            epoch: 4,
            owner: 5,
        },
        client: "consumer".into(),
    }
}
fn grant(rights: GrantRights, keys: GrantKeys, epoch: u64) -> NamespaceGrant {
    NamespaceGrant::new("1", "2", 3, epoch, rights, keys).unwrap()
}
fn port(
    service: &UnifiedIMapService,
    clock: &Arc<InvocationClock>,
    grant: NamespaceGrant,
) -> ReferenceHelperPort {
    ReferenceHelperPort::new(
        binding(),
        grant,
        service.clone(),
        Arc::clone(clock),
        8,
        DispatchMode::LocalReference,
    )
    .unwrap()
}
fn owner() -> UnifiedIMapService {
    UnifiedIMapService::new(CollectionBounds::default(), 3).unwrap()
}
fn id(n: u64) -> MutationIdentity {
    MutationIdentity::new("consumer", n)
}
async fn run(port: &ReferenceHelperPort, n: u64, command: LocalHelperCommand) -> LocalHelperResult {
    let prepared = port.prepare(&binding(), id(n), 10, command).unwrap();
    match port.invoke(prepared).unwrap().1.await.outcome {
        AsyncOutcome::Completed(Ok(result)) => result,
        other => panic!("{other:?}"),
    }
}
#[tokio::test]
async fn all_seven_helpers_use_original_owner_and_retained_ledger() {
    let service = owner();
    let clock = Arc::new(InvocationClock::default());
    let port = port(&service, &clock, grant(GrantRights::ALL, GrantKeys::All, 8));
    let mut put = ServiceCommand::point("put", b"a");
    put.value = Some(b"v".to_vec());
    put.identity = Some(MutationIdentity::new("writer", 1));
    service.execute(put).unwrap();
    let views = run(
        &port,
        1,
        LocalHelperCommand::EntryViews {
            keys: vec![b"a".to_vec()],
            max_bytes: 1024,
        },
    )
    .await;
    let token = match views {
        LocalHelperResult::EntryViews(v) => v[0].token.unwrap(),
        _ => panic!(),
    };
    assert_eq!(
        service.query_operation_outcome(&id(1)).unwrap(),
        OperationOutcomeQuery::Unknown
    );
    let entry = VersionBulkEntry {
        key: b"a".to_vec(),
        token,
        value: Some(b"new".to_vec()),
        ttl: ServiceTtl::Preserve,
    };
    let result = run(
        &port,
        2,
        LocalHelperCommand::VersionBulk {
            operation: VersionBulkOperation::Replace,
            entries: vec![entry.clone()],
        },
    )
    .await;
    assert!(matches!(result, LocalHelperResult::Versions(_)));
    let revision = service.revision();
    let replay = run(
        &port,
        2,
        LocalHelperCommand::VersionBulk {
            operation: VersionBulkOperation::Replace,
            entries: vec![entry],
        },
    )
    .await;
    assert!(matches!(replay, LocalHelperResult::Versions(_)));
    assert_eq!(revision, service.revision());
    assert!(
        matches!(run(&port, 3, LocalHelperCommand::QueryOutcomes { identities: vec![id(2)], max_bytes: 4096 }).await, LocalHelperResult::Outcomes(v) if matches!(&v[0], OperationOutcomeQuery::Completed(_)))
    );
    assert!(
        matches!(run(&port, 4, LocalHelperCommand::RetireOutcomes { client: "writer".into(), generation: 3, through: 1 }).await, LocalHelperResult::Retirement(r) if r.removed == 1)
    );
    assert!(matches!(
        run(&port, 5, LocalHelperCommand::ExpireStep { max_work: 1 }).await,
        LocalHelperResult::Expiry(_)
    ));
    let checkpoint = run(&port, 6, LocalHelperCommand::Checkpoint).await;
    assert!(matches!(checkpoint, LocalHelperResult::Checkpoint(_)));
    drop(checkpoint);
    let listener = service
        .register_listener(ServiceListenerOptions::default())
        .unwrap();
    let snapshot = run(
        &port,
        7,
        LocalHelperCommand::ListenerSnapshot {
            listener_id: listener.id(),
            max_items: 16,
            max_bytes: 4096,
        },
    )
    .await;
    assert!(matches!(&snapshot, LocalHelperResult::Snapshot(s) if s.entries.len() == 1));
    assert!(service.resource_usage().resync > 0);
    drop(snapshot);
    assert_eq!(service.resource_usage().resync, 0);
    assert_eq!(service.resource_usage().streams, 0);
}
#[tokio::test]
async fn preparation_is_pure_and_cancellation_deadlines_and_cross_port_context_are_fenced() {
    let service = owner();
    let clock = Arc::new(InvocationClock::default());
    let p = port(&service, &clock, grant(GrantRights::ALL, GrantKeys::All, 8));
    let before = service.resource_usage();
    let revision = service.revision();
    let prepared = p
        .prepare(&binding(), id(1), 10, LocalHelperCommand::Checkpoint)
        .unwrap();
    assert_eq!(before, service.resource_usage());
    assert_eq!(revision, service.revision());
    assert_eq!(clock.now(), 0);
    let (handle, future) = p.invoke(prepared).unwrap();
    handle.cancel();
    assert!(matches!(
        future.await.outcome,
        AsyncOutcome::NotApplied(InvocationRejection::Cancelled)
    ));
    let other = port(&service, &clock, grant(GrantRights::ALL, GrantKeys::All, 9));
    let prepared = p
        .prepare(&binding(), id(2), 10, LocalHelperCommand::Checkpoint)
        .unwrap();
    assert!(other.invoke(prepared).is_err());
    let prepared = p
        .prepare(&binding(), id(3), 0, LocalHelperCommand::Checkpoint)
        .unwrap();
    assert!(matches!(
        p.invoke(prepared).unwrap().1.await.outcome,
        AsyncOutcome::NotApplied(InvocationRejection::Deadline)
    ));
    assert_eq!(before, service.resource_usage());
    let mut bad = binding();
    bad.route.owner += 1;
    assert!(matches!(
        p.prepare(&bad, id(4), 10, LocalHelperCommand::Checkpoint),
        Err(HelperBoundaryError::Boundary(
            BoundaryError::ContextMismatch
        ))
    ));
    assert!(p
        .prepare(
            &binding(),
            MutationIdentity::new("other", 1),
            10,
            LocalHelperCommand::Checkpoint
        )
        .is_err());
    assert!(p
        .prepare(&binding(), id(0), 10, LocalHelperCommand::Checkpoint)
        .is_err());
    assert!(p
        .prepare(
            &binding(),
            id(5),
            10,
            LocalHelperCommand::ExpireStep {
                max_work: usize::MAX
            }
        )
        .is_err());
}
#[test]
fn grants_and_production_construction_refuse_without_side_effects() {
    let service = owner();
    let clock = Arc::new(InvocationClock::default());
    let read = GrantRights {
        read: true,
        ..GrantRights::default()
    };
    let p = port(
        &service,
        &clock,
        grant(read, GrantKeys::Prefixes(vec![b"a".to_vec()]), 8),
    );
    assert!(p
        .prepare(
            &binding(),
            id(1),
            10,
            LocalHelperCommand::EntryViews {
                keys: vec![b"a1".to_vec()],
                max_bytes: 1024
            }
        )
        .is_ok());
    assert!(p
        .prepare(
            &binding(),
            id(1),
            10,
            LocalHelperCommand::EntryViews {
                keys: vec![b"b".to_vec()],
                max_bytes: 1024
            }
        )
        .is_err());
    assert!(p
        .prepare(
            &binding(),
            id(1),
            10,
            LocalHelperCommand::QueryOutcomes {
                identities: vec![id(1)],
                max_bytes: 1024
            }
        )
        .is_err());
    assert!(p
        .prepare(&binding(), id(1), 10, LocalHelperCommand::Checkpoint)
        .is_err());
    assert!(ReferenceHelperPort::new(
        binding(),
        grant(GrantRights::ALL, GrantKeys::All, 8),
        service.clone(),
        Arc::clone(&clock),
        1,
        DispatchMode::ProductionMember
    )
    .is_err());
    assert!(ReferenceHelperPort::new(
        binding(),
        NamespaceGrant::new("wrong", "2", 3, 8, read, GrantKeys::All).unwrap(),
        service.clone(),
        clock,
        1,
        DispatchMode::LocalReference
    )
    .is_err());
    assert_eq!(service.resource_usage().streams, 0);
}
