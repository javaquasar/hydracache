use hydracache_cluster_testkit::imap_backend_contract_075::*;
use hydracache_cluster_testkit::provisional_backend_075::{NamespaceIdentity, PartitionRoute};
use hydracache_cluster_testkit::value_plane_capability_075::UNIFIED_OPERATIONS;
use hydracache_cluster_testkit::value_plane_collection_075::CollectionBounds;
use hydracache_cluster_testkit::value_plane_model_075::MutationIdentity;
use hydracache_cluster_testkit::value_plane_service_075::*;
use std::collections::BTreeSet;

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
fn service() -> UnifiedIMapService {
    UnifiedIMapService::new(
        CollectionBounds {
            max_key_bytes: 1024,
            ..CollectionBounds::default()
        },
        3,
    )
    .unwrap()
}
fn put(sequence: u64) -> ServiceCommand {
    let mut c = ServiceCommand::point("put", &[0, 255]);
    c.value = Some(vec![0, 128, 255]);
    c.identity = Some(MutationIdentity::new("consumer", sequence));
    c
}
fn port(service: UnifiedIMapService) -> ReferenceBackendPort {
    ReferenceBackendPort::new(binding(), service, DispatchMode::LocalReference).unwrap()
}

#[test]
fn closed_command_catalog_is_exact_and_scopes_namespace_work_explicitly() {
    let actual = BackendOperation::ALL
        .iter()
        .map(|op| {
            assert_eq!(BackendOperation::parse(op.name()).unwrap(), *op);
            op.name()
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(actual, UNIFIED_OPERATIONS.iter().copied().collect());
    assert_eq!(
        BackendOperation::Clear.scope(),
        OperationScope::NamespaceReferenceOnly
    );
    assert_eq!(
        BackendOperation::PutAll.scope(),
        OperationScope::ExplicitKeys
    );
    assert_eq!(
        BackendOperation::QueryOutcome.scope(),
        OperationScope::Handle
    );
    assert_eq!(
        BackendOperation::parse("entry_processor"),
        Err(BoundaryError::UnsupportedOperation)
    );
}

#[test]
fn preparation_is_read_only_and_owned_execution_preserves_binary_values_and_replay() {
    let service = service();
    let port = port(service.clone());
    let before = service.statistics();
    let before_usage = service.resource_usage();
    let mut original = put(1);
    original.ttl = ServiceTtl::ExpireAfter(10);
    original.deadline = 7;
    let prepared = port.prepare(&binding(), original.clone()).unwrap();
    assert_eq!(prepared.operation(), BackendOperation::Put);
    assert_eq!(prepared.deadline(), 7);
    original.value = Some(vec![42]);
    assert_eq!(service.statistics(), before);
    assert_eq!(service.resource_usage(), before_usage);
    let result = port.execute(prepared).unwrap();
    let view = service
        .execute(ServiceCommand::point("get_entry_view", &[0, 255]))
        .unwrap();
    assert_eq!(view.value, Some(vec![0, 128, 255]));
    assert_eq!(view.remaining_ttl, Some(10));
    let mut replay = put(1);
    replay.ttl = ServiceTtl::ExpireAfter(10);
    replay.deadline = 7;
    assert_eq!(
        port.execute(port.prepare(&binding(), replay).unwrap())
            .unwrap(),
        result
    );
}

#[test]
fn context_and_identity_substitution_is_refused_without_owner_side_effects() {
    let service = service();
    let port = port(service.clone());
    let before = service.statistics();
    for field in 0..7 {
        let mut wrong = binding();
        match field {
            0 => wrong.namespace.tenant += 1,
            1 => wrong.namespace.namespace += 1,
            2 => wrong.namespace.generation += 1,
            3 => wrong.route.partition += 1,
            4 => wrong.route.epoch += 1,
            5 => wrong.route.owner += 1,
            _ => wrong.client = "other".into(),
        }
        assert!(matches!(
            port.prepare(&wrong, put(1)),
            Err(BoundaryError::ContextMismatch)
        ));
    }
    for id in [None, Some(MutationIdentity::new("other", 1))] {
        let mut c = put(1);
        c.identity = id;
        assert!(matches!(
            port.prepare(&binding(), c),
            Err(BoundaryError::IdentityMismatch)
        ));
    }
    assert_eq!(service.statistics(), before);
    assert!(service.snapshot(0).unwrap().is_empty());
}

#[test]
fn malformed_requests_are_refused_before_time_events_and_outcome_reservation() {
    let service = service();
    let port = port(service.clone());
    let before = service.statistics();
    let before_usage = service.resource_usage();
    for field in 0..4 {
        let mut c = put(1);
        c.now = 100;
        match field {
            0 => c.value = None,
            1 => c.ttl = ServiceTtl::ExpireAfter(0),
            2 => c.key = vec![0; 8192],
            _ => c.operation = "future_operation".into(),
        }
        assert!(port.prepare(&binding(), c).is_err());
    }
    assert_eq!(service.statistics(), before);
    assert_eq!(service.resource_usage(), before_usage);
    assert_eq!(service.logical_time(), 0);
}

#[test]
fn prepared_request_cannot_cross_a_different_port_binding() {
    let first = port(service());
    let prepared = first.prepare(&binding(), put(1)).unwrap();
    let other = service();
    let mut changed = binding();
    changed.route.epoch += 1;
    let second =
        ReferenceBackendPort::new(changed, other.clone(), DispatchMode::LocalReference).unwrap();
    assert_eq!(
        second.execute(prepared),
        Err(BoundaryError::ContextMismatch)
    );
    assert_eq!(other.statistics().requests, 0);
}

#[test]
fn production_mode_and_invalid_bindings_cannot_construct_a_port() {
    assert!(matches!(
        ReferenceBackendPort::new(binding(), service(), DispatchMode::ProductionMember),
        Err(BoundaryError::DisabledBlockedBy074)
    ));
    for field in 0..7 {
        let mut b = binding();
        match field {
            0 => b.namespace.tenant = 0,
            1 => b.namespace.namespace = 0,
            2 => b.namespace.generation = 0,
            3 => b.route.epoch = 0,
            4 => b.route.owner = 0,
            5 => b.client = "\u{2003}".into(),
            _ => b.client = "a".repeat(129),
        }
        assert!(matches!(
            ReferenceBackendPort::new(b, service(), DispatchMode::LocalReference),
            Err(BoundaryError::InvalidBinding)
        ));
    }
}

#[test]
fn every_catalog_operation_executes_through_the_same_reference_owner() {
    use hydracache_cluster_testkit::value_plane_collection_075::ReplacementEntry;
    for operation in BackendOperation::ALL {
        let service = service();
        service.execute(put(1)).unwrap();
        let token = service
            .execute(ServiceCommand::point("get_entry_view", &[0, 255]))
            .unwrap()
            .token;
        let port = port(service);
        let mut command = put(2);
        command.operation = operation.name().into();
        command.expected = Some(vec![0, 128, 255]);
        command.keys = vec![vec![0, 255]];
        command.entries = vec![ReplacementEntry {
            key: vec![0, 255],
            value: vec![9],
        }];
        command.token = token;
        let prepared = port.prepare(&binding(), command).unwrap();
        assert!(port.execute(prepared).is_ok(), "{}", operation.name());
    }
}

#[test]
fn owner_generation_and_execution_revalidation_fence_prepared_work() {
    let mut wrong = binding();
    wrong.namespace.generation += 1;
    assert!(matches!(
        ReferenceBackendPort::new(wrong, service(), DispatchMode::LocalReference),
        Err(BoundaryError::InvalidBinding)
    ));
    let service = service();
    let port = port(service.clone());
    let first = port.prepare(&binding(), put(1)).unwrap();
    let second = port.prepare(&binding(), put(1)).unwrap();
    port.execute(first).unwrap();
    let mut conflict = put(1);
    conflict.value = Some(vec![42]);
    let conflict = port.prepare(&binding(), conflict).unwrap();
    assert!(matches!(
        port.execute(conflict),
        Err(BoundaryError::Service(_))
    ));
    assert!(port.execute(second).is_ok());
}

#[test]
fn deadline_cancellation_and_namespace_fencing_preserve_owner_certainty() {
    let service = service();
    let port = port(service.clone());
    let mut cancelled = put(1);
    cancelled.cancelled = true;
    assert_eq!(
        port.execute(port.prepare(&binding(), cancelled).unwrap())
            .unwrap()
            .kind,
        "not_applied"
    );
    let original = port
        .execute(port.prepare(&binding(), put(1)).unwrap())
        .unwrap();
    let mut replay = put(1);
    replay.cancelled = true;
    replay.now = 100;
    replay.deadline = 0;
    assert_eq!(
        port.execute(port.prepare(&binding(), replay).unwrap())
            .unwrap(),
        original
    );
    let prepared = port.prepare(&binding(), put(2)).unwrap();
    service.fence_namespace().unwrap();
    assert!(matches!(
        port.prepare(&binding(), put(3)),
        Err(BoundaryError::Service(MapServiceError::NamespaceDeleted))
    ));
    assert!(matches!(
        port.execute(prepared),
        Err(BoundaryError::Service(MapServiceError::NamespaceDeleted))
    ));
}
