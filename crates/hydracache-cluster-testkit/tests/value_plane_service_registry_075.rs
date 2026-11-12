use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::task::{Context, Poll, Wake, Waker};

use futures_util::Stream;
use hydracache_cluster_testkit::value_plane_admission_075::{
    AdmissionError, AdmissionLimits, DataWork, FairAdmissionQueue, NamespaceError, NamespacePhase,
};
use hydracache_cluster_testkit::value_plane_collection_075::{
    CollectionBounds, ReplacementEntry, ScanProjection,
};
use hydracache_cluster_testkit::value_plane_model_075::{
    MutationIdentity, MutationOperation, TtlDirective,
};
use hydracache_cluster_testkit::value_plane_security_075::{
    RouteClaims, SecurityBounds, SecurityError, TrustEpochWindow,
};
use hydracache_cluster_testkit::value_plane_service_075::{
    ServiceCommand, ServiceLimits, ServiceListenerOptions, ServiceResult, ServiceTtl,
};
use hydracache_cluster_testkit::value_plane_service_registry_075::{
    AuthenticatedNamespace, NamespaceCleanup, RegistryBounds, RegistryError, RegistryHandle,
    RegistryResult, UnifiedServiceRegistry,
};
use hydracache_cluster_testkit::value_plane_stream_075::{
    BoundedStreamSource, DemandStream, DemandStreamError,
};
use hydracache_cluster_testkit::value_plane_surface_075::{
    IngressPrincipal, SurfaceId, SurfaceOperation, VerifiedMapCommand,
};

fn collection() -> CollectionBounds {
    CollectionBounds {
        max_key_bytes: 64,
        max_value_bytes: 64,
        max_entries: 16,
        max_page_items: 4,
        max_scan_items: 16,
        max_response_bytes: 1024,
        max_bulk_mutation_items: 16,
    }
}
fn listener_options() -> ServiceListenerOptions {
    ServiceListenerOptions {
        max_value_bytes: collection().max_value_bytes,
        queue_capacity: 4,
        ..ServiceListenerOptions::default()
    }
}
fn bounds() -> RegistryBounds {
    RegistryBounds {
        max_cleanup_work: 16,
        ..RegistryBounds::default()
    }
}
fn security() -> SecurityBounds {
    SecurityBounds {
        max_envelope_bytes: 4096,
        max_key_bytes: 64,
        max_value_bytes: 64,
        ..SecurityBounds::default()
    }
}
fn registry_with(
    limits: RegistryBounds,
    service: ServiceLimits,
    guards: SecurityBounds,
) -> UnifiedServiceRegistry {
    UnifiedServiceRegistry::new(
        limits,
        collection(),
        service,
        guards,
        TrustEpochWindow::new(1, 3).unwrap(),
    )
    .unwrap()
}
fn new_registry() -> UnifiedServiceRegistry {
    registry_with(bounds(), ServiceLimits::default(), security())
}
fn verified(
    tenant: &str,
    namespace: &str,
    generation: u64,
    surface: SurfaceId,
    key: &[u8],
    operation: SurfaceOperation,
) -> VerifiedMapCommand {
    VerifiedMapCommand::new(
        surface,
        &IngressPrincipal {
            tenant: tenant.into(),
        },
        tenant,
        namespace,
        generation,
        key.to_vec(),
        operation,
    )
    .unwrap()
}
fn claims(tenant: &str, generation: u64) -> RouteClaims<'_> {
    RouteClaims {
        authenticated: true,
        authenticated_tenant: tenant,
        claimed_tenant: tenant,
        expected_namespace_generation: generation,
        actual_namespace_generation: generation,
        expected_topology_epoch: 1,
        actual_topology_epoch: 1,
        proxy_hops: 0,
        deadline_remaining_ticks: 1000,
        trust_epoch: 1,
        encoded_envelope_bytes: 100,
        key_bytes: 1,
        value_bytes: 0,
    }
}
fn authenticated(tenant: &str, namespace: &str, generation: u64) -> AuthenticatedNamespace {
    AuthenticatedNamespace::verify(
        &IngressPrincipal {
            tenant: tenant.into(),
        },
        &verified(
            tenant,
            namespace,
            generation,
            SurfaceId::Hc2Rust,
            b"x",
            SurfaceOperation::Read,
        ),
        &claims(tenant, generation),
        security(),
        TrustEpochWindow::new(1, 3).unwrap(),
        0,
    )
    .unwrap()
}
fn open(
    registry: &UnifiedServiceRegistry,
    tenant: &str,
    namespace: &str,
) -> (AuthenticatedNamespace, RegistryHandle) {
    let context = authenticated(tenant, namespace, 1);
    let handle = registry.open(&context, context.address()).unwrap();
    (context, handle)
}
fn write(operation: &str, key: &[u8], value: &[u8], sequence: u64) -> ServiceCommand {
    let mut command = ServiceCommand::point(operation, key);
    command.value = Some(value.to_vec());
    command.identity = Some(MutationIdentity::new("q", sequence));
    command
}
fn execute(
    registry: &UnifiedServiceRegistry,
    handle: &RegistryHandle,
    context: &AuthenticatedNamespace,
    command: ServiceCommand,
) -> Result<ServiceResult, RegistryError> {
    let ticket = registry.enqueue(handle, context, SurfaceId::Hc2Rust, command)?;
    let record = registry.run_next()?.expect("queued request");
    assert_eq!(record.ticket, ticket);
    match record.result? {
        RegistryResult::Command(result) => Ok(result),
        other => panic!("unexpected dispatch {other:?}"),
    }
}
fn begin_delete(
    registry: &UnifiedServiceRegistry,
    handle: &RegistryHandle,
    context: &AuthenticatedNamespace,
) {
    let ticket = registry.enqueue_delete(handle, context).unwrap();
    let record = registry.run_next().unwrap().unwrap();
    assert_eq!(record.ticket, ticket);
    assert_eq!(record.result.unwrap(), RegistryResult::DeleteStarted);
}
fn reclaim(
    registry: &UnifiedServiceRegistry,
    handle: &RegistryHandle,
    context: &AuthenticatedNamespace,
    maximum: usize,
) -> NamespaceCleanup {
    let ticket = registry.enqueue_reclaim(handle, context, maximum).unwrap();
    let record = registry.run_next().unwrap().unwrap();
    assert_eq!(record.ticket, ticket);
    match record.result.unwrap() {
        RegistryResult::Cleanup(progress) => progress,
        other => panic!("unexpected cleanup {other:?}"),
    }
}

#[test]
fn authenticated_context_binds_every_route_identity() {
    let registry = new_registry();
    let (context, handle) = open(&registry, "a", "orders");
    for (field, requested) in [
        ("tenant", {
            let mut address = context.address().clone();
            address.tenant = "b".into();
            address
        }),
        ("namespace", {
            let mut address = context.address().clone();
            address.namespace = "other".into();
            address
        }),
        ("generation", {
            let mut address = context.address().clone();
            address.generation = 2;
            address
        }),
    ] {
        assert_eq!(
            registry.open(&context, &requested).unwrap_err(),
            RegistryError::IdentitySubstitution(field)
        );
    }
    let mut request = verified(
        "a",
        "orders",
        1,
        SurfaceId::Resp,
        b"k",
        SurfaceOperation::Read,
    );
    request.namespace = "other".into();
    assert_eq!(
        registry
            .enqueue_verified(&handle, &context, &request)
            .unwrap_err(),
        RegistryError::IdentitySubstitution("namespace")
    );
    request.namespace = "orders".into();
    request.tenant = "b".into();
    assert_eq!(
        registry
            .enqueue_verified(&handle, &context, &request)
            .unwrap_err(),
        RegistryError::IdentitySubstitution("tenant")
    );
    request.tenant = "a".into();
    request.namespace_generation = 2;
    assert_eq!(
        registry
            .enqueue_verified(&handle, &context, &request)
            .unwrap_err(),
        RegistryError::IdentitySubstitution("generation")
    );
    let other = new_registry();
    assert_eq!(
        other
            .enqueue(
                &handle,
                &context,
                SurfaceId::Resp,
                ServiceCommand::point("get", b"k")
            )
            .unwrap_err(),
        RegistryError::WrongRegistry
    );
    assert_eq!(
        registry
            .diagnostics(&handle, &context)
            .unwrap()
            .statistics
            .requests,
        0
    );

    let principal = IngressPrincipal { tenant: "a".into() };
    let request = verified(
        "a",
        "orders",
        1,
        SurfaceId::Hc1,
        b"x",
        SurfaceOperation::Read,
    );
    let mut route = claims("a", 1);
    route.authenticated = false;
    assert!(matches!(
        AuthenticatedNamespace::verify(
            &principal,
            &request,
            &route,
            security(),
            TrustEpochWindow::new(1, 3).unwrap(),
            0
        ),
        Err(RegistryError::Security(SecurityError::UnauthenticatedRoute))
    ));
    route.authenticated = true;
    route.claimed_tenant = "b";
    assert!(matches!(
        AuthenticatedNamespace::verify(
            &principal,
            &request,
            &route,
            security(),
            TrustEpochWindow::new(1, 3).unwrap(),
            0
        ),
        Err(RegistryError::Security(SecurityError::TenantSubstitution))
    ));
    let mut changed = request.clone();
    changed.namespace_generation = 2;
    assert!(matches!(
        AuthenticatedNamespace::verify(
            &principal,
            &changed,
            &claims("a", 1),
            security(),
            TrustEpochWindow::new(1, 3).unwrap(),
            0
        ),
        Err(RegistryError::IdentitySubstitution("generation"))
    ));
    changed = request.clone();
    changed.namespace.clear();
    assert!(AuthenticatedNamespace::verify(
        &principal,
        &changed,
        &claims("a", 1),
        security(),
        TrustEpochWindow::new(1, 3).unwrap(),
        0
    )
    .is_err());
    assert!(AuthenticatedNamespace::verify(
        &IngressPrincipal { tenant: "b".into() },
        &request,
        &claims("a", 1),
        security(),
        TrustEpochWindow::new(1, 3).unwrap(),
        0
    )
    .is_err());
    let mut route = claims("a", 1);
    route.proxy_hops = 2;
    let weak_ingress = AuthenticatedNamespace::verify(
        &principal,
        &request,
        &route,
        SecurityBounds {
            max_proxy_hops: 2,
            ..security()
        },
        TrustEpochWindow::new(1, 3).unwrap(),
        0,
    )
    .unwrap();
    assert!(matches!(
        registry.open(&weak_ingress, weak_ingress.address()),
        Err(RegistryError::Security(SecurityError::RedirectLoop {
            limit: 1,
            actual: 2
        }))
    ));
}

#[test]
fn binary_keys_point_bulk_and_cross_surface_share_only_their_namespace_owner() {
    let registry = new_registry();
    let (a, ah) = open(&registry, "a", "same");
    let (b, bh) = open(&registry, "b", "same");
    let (other, oh) = open(&registry, "a", "other");
    let key = [0, 255, 0, 1];
    for (context, handle, value) in [(&a, &ah, b"a"), (&b, &bh, b"b"), (&other, &oh, b"c")] {
        execute(&registry, handle, context, write("put", &key, value, 1)).unwrap();
    }
    for surface in [
        SurfaceId::Resp,
        SurfaceId::Hc1,
        SurfaceId::Hc2Rust,
        SurfaceId::Hc2Java,
    ] {
        let request = verified("a", "same", 1, surface, &key, SurfaceOperation::Read);
        let ticket = registry.enqueue_verified(&ah, &a, &request).unwrap();
        let record = registry.run_next().unwrap().unwrap();
        assert_eq!(record.ticket, ticket);
        assert_eq!(record.surface, Some(surface));
        assert!(
            matches!(record.result.unwrap(), RegistryResult::Command(result) if result.value.as_deref() == Some(b"a"))
        );
    }
    let mut bulk = ServiceCommand::point("put_all_detailed", b"");
    bulk.identity = Some(MutationIdentity::new("q", 2));
    bulk.entries = vec![
        ReplacementEntry {
            key: vec![0, 255],
            value: b"one".to_vec(),
        },
        ReplacementEntry {
            key: vec![0, 0],
            value: b"two".to_vec(),
        },
    ];
    assert_eq!(execute(&registry, &ah, &a, bulk).unwrap().items.len(), 2);
    let mut get_all = ServiceCommand::point("get_all", b"");
    get_all.keys = vec![vec![0, 255], vec![0, 0], key.to_vec()];
    assert_eq!(
        execute(&registry, &ah, &a, get_all.clone()).unwrap().items[0]
            .value
            .as_deref(),
        Some(b"one".as_slice())
    );
    let foreign = execute(&registry, &bh, &b, get_all).unwrap();
    assert_eq!(foreign.items[0].kind, "absent");
    assert_eq!(foreign.items[2].value.as_deref(), Some(b"b".as_slice()));
    let mut page = DemandStream::new(
        registry
            .page_source(&ah, &a, ScanProjection::Entries)
            .unwrap(),
        2,
    )
    .unwrap();
    let mut cx = Context::from_waker(Waker::noop());
    let mut keys = Vec::new();
    while let Poll::Ready(Some(Ok(entry))) = Pin::new(&mut page).poll_next(&mut cx) {
        keys.push(entry.key);
    }
    assert_eq!(keys, vec![vec![0, 0], vec![0, 255], key.to_vec()]);
    assert_eq!(registry.diagnostics(&ah, &a).unwrap().owners.streams, 0);
    assert_eq!(
        execute(&registry, &oh, &other, ServiceCommand::point("get", &key))
            .unwrap()
            .value
            .as_deref(),
        Some(b"c".as_slice())
    );
}

#[test]
fn verified_point_projection_preserves_conditional_return_and_ttl_contracts() {
    let registry = new_registry();
    let (context, handle) = open(&registry, "a", "map");
    let operations = [
        MutationOperation::Put {
            value: b"v".to_vec(),
            ttl: TtlDirective::ExpireAfter(5),
        },
        MutationOperation::GetAndPut {
            value: b"w".to_vec(),
            ttl: TtlDirective::Preserve,
        },
        MutationOperation::PutIfAbsent {
            value: b"ignored".to_vec(),
            ttl: TtlDirective::Eternal,
        },
        MutationOperation::ReplaceIfPresent {
            value: b"x".to_vec(),
            ttl: TtlDirective::Eternal,
        },
        MutationOperation::ReplaceIfValue {
            expected: b"x".to_vec(),
            value: b"y".to_vec(),
            ttl: TtlDirective::ExpireAfter(7),
        },
        MutationOperation::RemoveIfValue {
            expected: b"wrong".to_vec(),
        },
        MutationOperation::RemoveIfValue {
            expected: b"y".to_vec(),
        },
        MutationOperation::GetAndRemove,
        MutationOperation::Put {
            value: b"z".to_vec(),
            ttl: TtlDirective::Eternal,
        },
        MutationOperation::GetAndRemove,
        MutationOperation::Put {
            value: b"q".to_vec(),
            ttl: TtlDirective::Eternal,
        },
        MutationOperation::Delete,
    ];
    let mut results = Vec::new();
    for (index, operation) in operations.into_iter().enumerate() {
        let request = verified(
            "a",
            "map",
            1,
            SurfaceId::Hc2Java,
            b"k",
            SurfaceOperation::Mutation {
                identity: MutationIdentity::new("verified", index as u64 + 1),
                operation,
            },
        );
        registry
            .enqueue_verified(&handle, &context, &request)
            .unwrap();
        if let RegistryResult::Command(result) =
            registry.run_next().unwrap().unwrap().result.unwrap()
        {
            results.push(result);
        }
    }
    assert_eq!(results[1].value.as_deref(), Some(b"v".as_slice()));
    assert_eq!(results[1].remaining_ttl, Some(5));
    assert_eq!(results[2].kind, "present");
    assert_eq!(results[3].remaining_ttl, None);
    assert_eq!(results[4].remaining_ttl, Some(7));
    assert_eq!(results[5].kind, "mismatch");
    assert_eq!(results[6].value.as_deref(), Some(b"y".as_slice()));
    assert_eq!(results[7].kind, "absent");
    assert_eq!(results[9].value.as_deref(), Some(b"z".as_slice()));
    assert_eq!(results[11].value, None);
}

#[test]
fn tenant_fair_admission_and_reserved_safety_lane() {
    let registry = registry_with(
        RegistryBounds {
            admission: AdmissionLimits {
                total: 4,
                per_tenant: 3,
                per_partition: 3,
                safety: 1,
            },
            ..bounds()
        },
        ServiceLimits::default(),
        security(),
    );
    let (a, ah) = open(&registry, "a", "map");
    let (b, bh) = open(&registry, "b", "map");
    for _ in 0..3 {
        registry
            .enqueue(&ah, &a, SurfaceId::Resp, ServiceCommand::point("get", b"k"))
            .unwrap();
    }
    let cold = registry
        .enqueue(&bh, &b, SurfaceId::Hc1, ServiceCommand::point("get", b"k"))
        .unwrap();
    assert!(matches!(
        registry.enqueue(&ah, &a, SurfaceId::Resp, ServiceCommand::point("get", b"k")),
        Err(RegistryError::Admission(AdmissionError::TotalQueueFull))
    ));
    let delete = registry.enqueue_delete(&ah, &a).unwrap();
    assert!(matches!(
        registry.enqueue_delete(&ah, &a),
        Err(RegistryError::Admission(AdmissionError::SafetyQueueFull))
    ));
    assert_eq!(registry.run_next().unwrap().unwrap().ticket, delete);
    let first = registry.run_next().unwrap().unwrap();
    let second = registry.run_next().unwrap().unwrap();
    assert!(first.ticket == cold || second.ticket == cold);
    while registry.run_next().unwrap().is_some() {}
    assert_eq!(
        registry.diagnostics(&ah, &a).unwrap().statistics.requests,
        0
    );
    assert_eq!(
        registry.diagnostics(&bh, &b).unwrap().statistics.requests,
        1
    );
    assert_eq!(
        reclaim(&registry, &ah, &a, 1).phase,
        NamespacePhase::Reclaimable
    );

    let registry = registry_with(
        RegistryBounds {
            max_queued_bytes: 132,
            admission: AdmissionLimits {
                total: 1,
                per_tenant: 1,
                per_partition: 1,
                safety: 1,
            },
            ..bounds()
        },
        ServiceLimits::default(),
        security(),
    );
    let (context, handle) = open(&registry, "a", "map");
    registry
        .enqueue(
            &handle,
            &context,
            SurfaceId::Resp,
            ServiceCommand::point("get", b"k"),
        )
        .unwrap();
    let delete = registry.enqueue_delete(&handle, &context).unwrap();
    assert_eq!(registry.usage().queued_bytes, 260);
    assert_eq!(registry.run_next().unwrap().unwrap().ticket, delete);
}

#[test]
fn admission_scheduler_metadata_is_bounded_across_repeated_reactivation() {
    let mut queue = FairAdmissionQueue::new(AdmissionLimits {
        total: 8,
        per_tenant: 4,
        per_partition: 4,
        safety: 1,
    })
    .unwrap();
    for request in 0..10000 {
        queue
            .admit_data(DataWork {
                tenant: format!("tenant-{}", request % 7),
                partition: 0,
                request_id: request,
            })
            .unwrap();
        assert_eq!(queue.active_tenants(), 1);
        queue.pop().unwrap();
        assert_eq!(queue.active_tenants(), 0);
    }
    assert!(queue.pop().is_none());
    for (tenant, request) in [("a", 1), ("a", 2), ("b", 3), ("b", 4)] {
        queue
            .admit_data(DataWork {
                tenant: tenant.into(),
                partition: 0,
                request_id: request,
            })
            .unwrap();
    }
    let mut tenants = Vec::new();
    while let Some(hydracache_cluster_testkit::value_plane_admission_075::WorkItem::Data(work)) =
        queue.pop()
    {
        tenants.push(work.tenant);
    }
    assert_eq!(tenants, ["a", "b", "a", "b"]);
}

#[test]
fn all_quotas_fail_before_actual_dispatch() {
    for (limits, expected) in [
        (
            RegistryBounds {
                max_tenants: 1,
                ..bounds()
            },
            "tenants",
        ),
        (
            RegistryBounds {
                max_maps: 1,
                ..bounds()
            },
            "maps",
        ),
        (
            RegistryBounds {
                max_maps_per_tenant: 1,
                ..bounds()
            },
            "maps_per_tenant",
        ),
    ] {
        let registry = registry_with(limits, ServiceLimits::default(), security());
        open(&registry, "a", "one");
        let context = if expected == "tenants" {
            authenticated("b", "two", 1)
        } else {
            authenticated("a", "two", 1)
        };
        assert!(
            matches!(registry.open(&context, context.address()), Err(RegistryError::Quota { dimension, .. }) if dimension == expected)
        );
        assert_eq!(registry.usage().maps, 1);
    }
    for (limits, expected) in [
        (
            RegistryBounds {
                max_metadata_bytes: 1,
                ..bounds()
            },
            "metadata_bytes",
        ),
        (
            RegistryBounds {
                max_identifier_bytes: 1,
                ..bounds()
            },
            "identifier_bytes",
        ),
    ] {
        let registry = registry_with(limits, ServiceLimits::default(), security());
        let context = authenticated("a", "long", 1);
        assert!(
            matches!(registry.open(&context, context.address()), Err(RegistryError::Quota { dimension, .. }) if dimension == expected)
        );
        assert_eq!(registry.usage().maps, 0);
    }
    for (limits, expected) in [
        (
            RegistryBounds {
                max_command_bytes: 132,
                ..bounds()
            },
            "command_bytes",
        ),
        (
            RegistryBounds {
                max_owned_bytes: 180,
                ..bounds()
            },
            "owned_bytes",
        ),
        (
            RegistryBounds {
                max_tenant_owned_bytes: 180,
                ..bounds()
            },
            "tenant_owned_bytes",
        ),
    ] {
        let registry = registry_with(limits, ServiceLimits::default(), security());
        let (context, handle) = open(&registry, "a", "map");
        assert!(
            matches!(registry.enqueue(&handle, &context, SurfaceId::Resp, write("put", b"k", b"v", 1)), Err(RegistryError::Quota { dimension, .. }) if dimension == expected)
        );
        let actual = registry.diagnostics(&handle, &context).unwrap();
        assert_eq!(actual.statistics.requests, 0);
        assert_eq!(actual.owners.total(), 0);
    }
    let registry = registry_with(
        RegistryBounds {
            max_queued_bytes: 132,
            ..bounds()
        },
        ServiceLimits::default(),
        security(),
    );
    let (context, handle) = open(&registry, "a", "map");
    registry
        .enqueue(
            &handle,
            &context,
            SurfaceId::Resp,
            ServiceCommand::point("get", b"k"),
        )
        .unwrap();
    assert!(matches!(
        registry.enqueue(
            &handle,
            &context,
            SurfaceId::Resp,
            ServiceCommand::point("get", b"k")
        ),
        Err(RegistryError::Quota {
            dimension: "queued_bytes",
            ..
        })
    ));
    assert_eq!(
        registry
            .diagnostics(&handle, &context)
            .unwrap()
            .statistics
            .requests,
        0
    );
    let registry = registry_with(
        bounds(),
        ServiceLimits {
            value_bytes: 25,
            ..ServiceLimits::default()
        },
        security(),
    );
    let (context, handle) = open(&registry, "a", "map");
    assert!(
        matches!(registry.enqueue(&handle, &context, SurfaceId::Resp, write("put", b"k", b"v", 1)), Err(RegistryError::Service(error)) if error.kind() == "resource_budget_exhausted")
    );
    assert_eq!(
        registry
            .diagnostics(&handle, &context)
            .unwrap()
            .statistics
            .requests,
        0
    );
    for (guards, command, expected) in [
        (
            SecurityBounds {
                max_key_bytes: 2,
                ..security()
            },
            write("put", b"key", b"v", 1),
            "key",
        ),
        (
            SecurityBounds {
                max_value_bytes: 2,
                ..security()
            },
            write("put", b"k", b"val", 1),
            "value",
        ),
        (
            SecurityBounds {
                max_envelope_bytes: 131,
                ..security()
            },
            write("put", b"k", b"v", 1),
            "envelope",
        ),
    ] {
        let registry = registry_with(bounds(), ServiceLimits::default(), guards);
        let (context, handle) = open(&registry, "a", "map");
        let error = registry
            .enqueue(&handle, &context, SurfaceId::Resp, command)
            .unwrap_err();
        assert!(matches!(
            (&error, expected),
            (
                RegistryError::Security(SecurityError::KeyTooLarge { .. }),
                "key"
            ) | (
                RegistryError::Security(SecurityError::ValueTooLarge { .. }),
                "value"
            ) | (
                RegistryError::Security(SecurityError::EnvelopeTooLarge { .. }),
                "envelope"
            )
        ));
        assert_eq!(
            registry
                .diagnostics(&handle, &context)
                .unwrap()
                .statistics
                .requests,
            0
        );
    }
    let registry = registry_with(
        RegistryBounds {
            max_owned_bytes: 500,
            ..bounds()
        },
        ServiceLimits::default(),
        security(),
    );
    let (context, handle) = open(&registry, "a", "map");
    assert!(matches!(
        registry.page_source(&handle, &context, ScanProjection::Entries),
        Err(RegistryError::Quota { .. })
    ));
    assert!(matches!(
        registry.listener_source(&handle, &context, listener_options()),
        Err(RegistryError::Quota { .. })
    ));
    assert_eq!(
        registry
            .diagnostics(&handle, &context)
            .unwrap()
            .owners
            .total(),
        0
    );
}

#[test]
fn namespace_reuse_waits_for_actual_pending_dedup_and_leases() {
    let registry = new_registry();
    let (context, handle) = open(&registry, "a", "map");
    execute(&registry, &handle, &context, write("put", b"a", b"1", 1)).unwrap();
    execute(&registry, &handle, &context, write("put", b"b", b"2", 2)).unwrap();
    let pending = write("put", b"p", b"p", 3);
    registry
        .enqueue_reservation(&handle, &context, pending)
        .unwrap();
    registry.run_next().unwrap().unwrap().result.unwrap();
    let listener = registry
        .listener_source(
            &handle,
            &context,
            ServiceListenerOptions {
                include_value: true,
                ..listener_options()
            },
        )
        .unwrap();
    let listener_id = listener.registration_id().unwrap();
    let page = registry
        .page_source(&handle, &context, ScanProjection::Entries)
        .unwrap();
    let mut snapshot = registry
        .resync_listener(&handle, &context, listener_id, 16, 128)
        .unwrap();
    let actual = registry.diagnostics(&handle, &context).unwrap().owners;
    assert_eq!(
        (
            actual.values,
            actual.completed_outcomes,
            actual.pending,
            actual.streams,
            actual.resync_leases
        ),
        (2, 2, 1, 2, 1)
    );
    let queued = registry
        .enqueue(
            &handle,
            &context,
            SurfaceId::Resp,
            ServiceCommand::point("get", b"a"),
        )
        .unwrap();
    begin_delete(&registry, &handle, &context);
    let first = reclaim(&registry, &handle, &context, 2);
    assert!(first.reclaimed <= 2);
    assert_eq!(first.queued_requests, 1);
    assert_eq!(first.phase, NamespacePhase::Draining);
    assert!(registry.recreate(&handle, &context).is_err());
    let rejected = registry.run_next().unwrap().unwrap();
    assert_eq!(rejected.ticket, queued);
    assert!(matches!(
        rejected.result,
        Err(RegistryError::Namespace(NamespaceError::NotActive))
    ));
    let mut progress = first;
    for _ in 0..16 {
        progress = reclaim(&registry, &handle, &context, 1);
        if progress.remaining == 0 {
            break;
        }
    }
    assert_eq!(progress.remaining, 0);
    assert_eq!(progress.blocked_owners, 4);
    assert_eq!(progress.owners.pending, 1);
    assert_eq!(progress.owners.completed_outcomes, 0);
    assert!(registry
        .cancel_pending(&handle, &context, &MutationIdentity::new("q", 3))
        .unwrap());
    assert!(!registry
        .cancel_pending(&handle, &context, &MutationIdentity::new("q", 3))
        .unwrap());
    drop(page);
    drop(listener);
    snapshot.close();
    let complete = reclaim(&registry, &handle, &context, 1);
    assert_eq!(complete.phase, NamespacePhase::Reclaimable);
    assert_eq!(complete.owners.total(), 0);
    let next = registry.recreate(&handle, &context).unwrap();
    assert_eq!(next.address().generation, 2);
    assert!(matches!(
        registry.enqueue(
            &handle,
            &context,
            SurfaceId::Resp,
            ServiceCommand::point("get", b"a")
        ),
        Err(RegistryError::Namespace(NamespaceError::StaleGeneration {
            expected: 2,
            actual: 1
        }))
    ));
    assert!(matches!(
        registry.enqueue(
            &next,
            &context,
            SurfaceId::Resp,
            ServiceCommand::point("get", b"a")
        ),
        Err(RegistryError::IdentitySubstitution("generation"))
    ));
    let fresh = authenticated("a", "map", 2);
    registry.open(&fresh, fresh.address()).unwrap();
    assert_eq!(
        execute(&registry, &next, &fresh, ServiceCommand::point("get", b"a"))
            .unwrap()
            .kind,
        "absent"
    );
}

#[test]
fn deleting_generation_fences_old_handles_cursors_and_streams() {
    let registry = new_registry();
    let (context, handle) = open(&registry, "a", "map");
    for (index, key) in [b"a", b"b", b"c"].into_iter().enumerate() {
        execute(
            &registry,
            &handle,
            &context,
            write("put", key, b"v", index as u64 + 1),
        )
        .unwrap();
    }
    let token = execute(
        &registry,
        &handle,
        &context,
        ServiceCommand::point("get_entry_view", b"a"),
    )
    .unwrap()
    .token;
    let mut command = ServiceCommand::point("key_set_page", b"");
    command.limit = 1;
    let old_cursor = execute(&registry, &handle, &context, command)
        .unwrap()
        .cursor;
    let mut pages = DemandStream::new(
        registry
            .page_source(&handle, &context, ScanProjection::Entries)
            .unwrap(),
        2,
    )
    .unwrap();
    let mut cx = Context::from_waker(Waker::noop());
    assert!(matches!(
        Pin::new(&mut pages).poll_next(&mut cx),
        Poll::Ready(Some(Ok(_)))
    ));
    assert_eq!(pages.retained_items(), 1);
    begin_delete(&registry, &handle, &context);
    assert!(matches!(
        Pin::new(&mut pages).poll_next(&mut cx),
        Poll::Ready(Some(Err(DemandStreamError::Source(
            RegistryError::Namespace(NamespaceError::NotActive)
        ))))
    ));
    assert_eq!(pages.retained_items(), 0);
    for _ in 0..16 {
        if reclaim(&registry, &handle, &context, 2).phase == NamespacePhase::Reclaimable {
            break;
        }
    }
    let next = registry.recreate(&handle, &context).unwrap();
    let fresh = authenticated("a", "map", 2);
    let mut resumed = ServiceCommand::point("key_set_page", b"");
    resumed.cursor = old_cursor;
    resumed.limit = 1;
    assert!(
        matches!(execute(&registry, &next, &fresh, resumed), Err(RegistryError::Service(error)) if error.kind() == "cursor_mismatch")
    );
    execute(&registry, &next, &fresh, write("put", b"a", b"new", 1)).unwrap();
    let mut conditional = write("replace_if_version", b"a", b"wrong", 2);
    conditional.token = token;
    assert_eq!(
        execute(&registry, &next, &fresh, conditional).unwrap().kind,
        "mismatch"
    );
    assert_eq!(
        execute(&registry, &next, &fresh, ServiceCommand::point("get", b"a"))
            .unwrap()
            .value
            .as_deref(),
        Some(b"new".as_slice())
    );
}

#[test]
fn listener_gap_resync_and_empty_snapshot_leases_use_actual_owners() {
    let registry = new_registry();
    let (context, handle) = open(&registry, "a", "map");
    let mut listener = registry
        .listener_source(
            &handle,
            &context,
            ServiceListenerOptions {
                include_value: true,
                queue_capacity: 1,
                ..listener_options()
            },
        )
        .unwrap();
    let id = listener.registration_id().unwrap();
    let mut cx = Context::from_waker(Waker::noop());
    let mut bulk = write("put_all_detailed", b"", b"", 1);
    bulk.value = None;
    bulk.entries = vec![
        ReplacementEntry {
            key: b"a".to_vec(),
            value: b"1".to_vec(),
        },
        ReplacementEntry {
            key: b"b".to_vec(),
            value: b"2".to_vec(),
        },
    ];
    execute(&registry, &handle, &context, bulk).unwrap();
    assert!(
        matches!(listener.poll_batch(&mut cx, 1), Poll::Ready(Ok(Some(events))) if events.len() == 1 && events[0].kind == "gap")
    );
    let mut snapshot = registry
        .resync_listener(&handle, &context, id, 16, 128)
        .unwrap();
    assert_eq!(snapshot.entries.len(), 2);
    assert_eq!(
        registry
            .diagnostics(&handle, &context)
            .unwrap()
            .owners
            .resync_leases,
        1
    );
    snapshot.close();
    execute(
        &registry,
        &handle,
        &context,
        write("put", b"a", b"fresh", 2),
    )
    .unwrap();
    assert!(
        matches!(listener.poll_batch(&mut cx, 1), Poll::Ready(Ok(Some(events))) if events[0].kind == "updated" && events[0].value.as_deref() == Some(b"fresh".as_slice()))
    );
    drop(listener);
    let (empty, empty_handle) = open(&registry, "a", "empty");
    let listener = registry
        .listener_source(&empty_handle, &empty, listener_options())
        .unwrap();
    let mut empty_snapshot = registry
        .resync_listener(
            &empty_handle,
            &empty,
            listener.registration_id().unwrap(),
            1,
            64,
        )
        .unwrap();
    assert!(empty_snapshot.entries.is_empty());
    assert_eq!(
        registry
            .diagnostics(&empty_handle, &empty)
            .unwrap()
            .resources
            .resync,
        0
    );
    begin_delete(&registry, &empty_handle, &empty);
    drop(listener);
    let blocked = reclaim(&registry, &empty_handle, &empty, 16);
    assert_eq!(blocked.remaining, 0);
    assert_eq!(blocked.blocked_owners, 1);
    assert_eq!(blocked.owners.resync_leases, 1);
    assert_eq!(blocked.phase, NamespacePhase::Draining);
    empty_snapshot.close();
    assert_eq!(
        reclaim(&registry, &empty_handle, &empty, 1).phase,
        NamespacePhase::Reclaimable
    );
}

#[test]
fn concurrent_registry_requests_preserve_isolation_and_atomic_quotas() {
    let probe = new_registry();
    let (context, handle) = open(&probe, "a", "map");
    probe
        .enqueue(
            &handle,
            &context,
            SurfaceId::Resp,
            write("put_if_absent", b"k", b"v", 1),
        )
        .unwrap();
    let maximum = probe.usage().reserved_growth;
    let registry = registry_with(
        RegistryBounds {
            max_owned_bytes: maximum,
            ..bounds()
        },
        ServiceLimits::default(),
        security(),
    );
    let (context, handle) = open(&registry, "a", "map");
    let barrier = Arc::new(Barrier::new(8));
    let workers = (1..=8)
        .map(|sequence| {
            let registry = registry.clone();
            let context = context.clone();
            let handle = handle.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                registry.enqueue(
                    &handle,
                    &context,
                    SurfaceId::Hc2Java,
                    write("put_if_absent", b"k", &[sequence as u8], sequence),
                )
            })
        })
        .collect::<Vec<_>>();
    let outcomes = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);
    assert!(outcomes
        .iter()
        .filter_map(|outcome| outcome.as_ref().err())
        .all(|error| matches!(
            error,
            RegistryError::Quota {
                dimension: "owned_bytes",
                ..
            }
        )));
    assert_eq!(
        registry
            .diagnostics(&handle, &context)
            .unwrap()
            .statistics
            .requests,
        0
    );
    assert_eq!(registry.usage().queued_requests, 1);
    assert!(registry.usage().reserved_growth <= maximum);
    registry.run_next().unwrap().unwrap().result.unwrap();
    assert!(registry.usage().owned_bytes <= maximum);
    let isolated = new_registry();
    let mut scopes = Vec::new();
    for index in 0..8 {
        scopes.push(open(&isolated, &format!("t{index}"), "same"));
    }
    let workers = scopes
        .iter()
        .enumerate()
        .map(|(index, (context, handle))| {
            let registry = isolated.clone();
            let context = context.clone();
            let handle = handle.clone();
            std::thread::spawn(move || {
                registry
                    .enqueue(
                        &handle,
                        &context,
                        SurfaceId::Resp,
                        write("put", &[0, 255], &[index as u8], 1),
                    )
                    .unwrap()
            })
        })
        .collect::<Vec<_>>();
    for worker in workers {
        worker.join().unwrap();
    }
    while isolated.run_next().unwrap().is_some() {}
    for (index, (context, handle)) in scopes.iter().enumerate() {
        assert_eq!(
            execute(
                &isolated,
                handle,
                context,
                ServiceCommand::point("get", &[0, 255])
            )
            .unwrap()
            .value,
            Some(vec![index as u8])
        );
    }
}

struct ReentrantWake {
    registry: UnifiedServiceRegistry,
    observed: AtomicUsize,
}
impl Wake for ReentrantWake {
    fn wake(self: Arc<Self>) {
        let _ = self.registry.usage();
        assert_eq!(
            self.registry.run_next().unwrap_err(),
            RegistryError::DispatcherBusy
        );
        self.observed.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn parked_listener_wakes_outside_registry_mutex_and_rechecks_rotated_auth() {
    let registry = new_registry();
    let (context, handle) = open(&registry, "a", "map");
    let mut listener = registry
        .listener_source(&handle, &context, listener_options())
        .unwrap();
    let wake = Arc::new(ReentrantWake {
        registry: registry.clone(),
        observed: AtomicUsize::new(0),
    });
    let waker = Waker::from(Arc::clone(&wake));
    let mut cx = Context::from_waker(&waker);
    assert!(listener.poll_batch(&mut cx, 1).is_pending());
    execute(&registry, &handle, &context, write("put", b"a", b"1", 1)).unwrap();
    assert_eq!(wake.observed.load(Ordering::SeqCst), 1);
    assert!(matches!(
        listener.poll_batch(&mut cx, 1),
        Poll::Ready(Ok(Some(_)))
    ));
    registry
        .enqueue(
            &handle,
            &context,
            SurfaceId::Resp,
            ServiceCommand::point("get", b"a"),
        )
        .unwrap();
    registry.rotate_trust(2).unwrap();
    registry.advance_clock(4).unwrap();
    assert!(matches!(
        registry.run_next().unwrap().unwrap().result,
        Err(RegistryError::Security(SecurityError::UnknownTrustEpoch(1)))
    ));
    assert!(matches!(
        listener.validate_buffered(),
        Err(RegistryError::Security(SecurityError::UnknownTrustEpoch(1)))
    ));
    let request = verified("a", "map", 1, SurfaceId::Resp, b"a", SurfaceOperation::Read);
    let mut route = claims("a", 1);
    route.trust_epoch = 2;
    let fresh = AuthenticatedNamespace::verify(
        &IngressPrincipal { tenant: "a".into() },
        &request,
        &route,
        security(),
        TrustEpochWindow::new(2, 3).unwrap(),
        4,
    )
    .unwrap();
    assert_eq!(
        registry
            .diagnostics(&handle, &fresh)
            .unwrap()
            .statistics
            .requests,
        1
    );
    assert!(registry.advance_clock(3).is_err());
    assert!(registry.rotate_trust(2).is_err());
}

#[test]
fn invalid_bounds_expired_contexts_and_dispatch_errors_release_admission() {
    for limits in [
        RegistryBounds {
            max_tenants: 0,
            ..bounds()
        },
        RegistryBounds {
            max_owned_bytes: usize::MAX,
            ..bounds()
        },
    ] {
        assert!(matches!(
            UnifiedServiceRegistry::new(
                limits,
                collection(),
                ServiceLimits::default(),
                security(),
                TrustEpochWindow::new(1, 3).unwrap()
            ),
            Err(RegistryError::InvalidBound(_))
        ));
    }
    assert!(matches!(
        UnifiedServiceRegistry::new(
            bounds(),
            collection(),
            ServiceLimits::default(),
            SecurityBounds {
                max_key_bytes: 0,
                ..security()
            },
            TrustEpochWindow::new(1, 3).unwrap()
        ),
        Err(RegistryError::InvalidBound("security_key"))
    ));
    assert!(UnifiedServiceRegistry::new(
        RegistryBounds {
            admission: AdmissionLimits {
                total: 0,
                ..bounds().admission
            },
            ..bounds()
        },
        collection(),
        ServiceLimits::default(),
        security(),
        TrustEpochWindow::new(1, 3).unwrap()
    )
    .is_err());
    let registry = new_registry();
    let (context, handle) = open(&registry, "a", "map");
    assert!(matches!(
        registry.recreate(&handle, &context),
        Err(RegistryError::Namespace(NamespaceError::NotReclaimable))
    ));
    assert!(registry.enqueue_reclaim(&handle, &context, 0).is_err());
    assert!(registry.enqueue_reclaim(&handle, &context, 17).is_err());
    registry.enqueue_reclaim(&handle, &context, 1).unwrap();
    assert!(matches!(
        registry.run_next().unwrap().unwrap().result,
        Err(RegistryError::Namespace(NamespaceError::NotDraining))
    ));
    assert!(registry
        .enqueue_reservation(&handle, &context, ServiceCommand::point("put", b"k"))
        .is_err());
    assert_eq!(
        registry
            .enqueue(
                &handle,
                &context,
                SurfaceId::Resp,
                ServiceCommand::point("destroy", b"")
            )
            .unwrap_err(),
        RegistryError::SafetyLaneRequired
    );
    assert_eq!(
        registry
            .enqueue_reservation(&handle, &context, ServiceCommand::point("destroy", b""))
            .unwrap_err(),
        RegistryError::SafetyLaneRequired
    );
    assert!(
        matches!(execute(&registry, &handle, &context, ServiceCommand::point("unsupported", b"k")), Err(RegistryError::Service(error)) if error.kind() == "unsupported_operation")
    );
    let mut invalid_page = ServiceCommand::point("key_set_page", b"");
    invalid_page.limit = 0;
    assert!(
        matches!(execute(&registry, &handle, &context, invalid_page), Err(RegistryError::Service(error)) if error.kind() == "request_too_large")
    );
    assert_eq!(registry.usage().queued_requests, 0);
    assert_eq!(registry.usage().reserved_growth, 0);
    let mut route = claims("a", 1);
    route.deadline_remaining_ticks = 1;
    let request = verified("a", "map", 1, SurfaceId::Resp, b"k", SurfaceOperation::Read);
    let expiring = AuthenticatedNamespace::verify(
        &IngressPrincipal { tenant: "a".into() },
        &request,
        &route,
        security(),
        TrustEpochWindow::new(1, 3).unwrap(),
        0,
    )
    .unwrap();
    registry
        .enqueue(
            &handle,
            &expiring,
            SurfaceId::Resp,
            ServiceCommand::point("get", b"k"),
        )
        .unwrap();
    registry.advance_clock(1).unwrap();
    assert!(matches!(
        registry.run_next().unwrap().unwrap().result,
        Err(RegistryError::Security(SecurityError::DeadlineExpired))
    ));
    assert_eq!(
        registry
            .diagnostics(&handle, &context)
            .unwrap()
            .statistics
            .requests,
        1
    );
    route = claims("a", 1);
    route.expected_topology_epoch = 2;
    route.actual_topology_epoch = 2;
    let stale = AuthenticatedNamespace::verify(
        &IngressPrincipal { tenant: "a".into() },
        &request,
        &route,
        security(),
        TrustEpochWindow::new(1, 3).unwrap(),
        0,
    )
    .unwrap();
    assert!(matches!(
        registry.open(&stale, stale.address()),
        Err(RegistryError::Security(SecurityError::StaleTopologyEpoch {
            expected: 1,
            actual: 2
        }))
    ));
    let invalid_generation = authenticated("a", "new", 2);
    assert!(matches!(
        registry.open(&invalid_generation, invalid_generation.address()),
        Err(RegistryError::Namespace(NamespaceError::InvalidGeneration))
    ));
}

#[test]
fn tenant_partition_and_snapshot_quotas_remain_tied_to_actual_owners() {
    for (admission, expected) in [
        (
            AdmissionLimits {
                total: 8,
                per_tenant: 1,
                per_partition: 4,
                safety: 1,
            },
            "tenant",
        ),
        (
            AdmissionLimits {
                total: 8,
                per_tenant: 4,
                per_partition: 1,
                safety: 1,
            },
            "partition",
        ),
    ] {
        let registry = registry_with(
            RegistryBounds {
                admission,
                ..bounds()
            },
            ServiceLimits::default(),
            security(),
        );
        let (context, handle) = open(&registry, "a", "map");
        registry
            .enqueue(
                &handle,
                &context,
                SurfaceId::Resp,
                ServiceCommand::point("get", b"k"),
            )
            .unwrap();
        let error = registry
            .enqueue(
                &handle,
                &context,
                SurfaceId::Resp,
                ServiceCommand::point("get", b"k"),
            )
            .unwrap_err();
        assert!(matches!(
            (&error, expected),
            (
                RegistryError::Admission(AdmissionError::TenantQueueFull(_)),
                "tenant"
            ) | (
                RegistryError::Admission(AdmissionError::PartitionQueueFull { .. }),
                "partition"
            )
        ));
        assert_eq!(
            registry
                .diagnostics(&handle, &context)
                .unwrap()
                .statistics
                .requests,
            0
        );
    }
    let registry = registry_with(
        RegistryBounds {
            max_owned_bytes: 5000,
            ..bounds()
        },
        ServiceLimits::default(),
        security(),
    );
    let (context, handle) = open(&registry, "a", "map");
    let listener = registry
        .listener_source(&handle, &context, listener_options())
        .unwrap();
    assert!(
        matches!(registry.listener_source(&handle, &context, ServiceListenerOptions { max_value_bytes: 129, ..listener_options() }), Err(RegistryError::Service(error)) if error.kind() == "invalid_argument")
    );
    assert!(matches!(
        registry.resync_listener(
            &handle,
            &context,
            listener.registration_id().unwrap(),
            16,
            4096
        ),
        Err(RegistryError::Quota { .. })
    ));
    assert_eq!(
        registry
            .diagnostics(&handle, &context)
            .unwrap()
            .owners
            .resync_leases,
        0
    );
    assert!(registry
        .resync_listener(&handle, &context, u64::MAX, 16, 64)
        .is_err());
    assert_eq!(registry.usage().reserved_growth, 0);
    drop(listener);
}

#[test]
fn stream_cutover_and_registry_drop_discard_buffered_and_orphaned_access() {
    let registry = new_registry();
    let (context, handle) = open(&registry, "a", "map");
    let source = registry
        .listener_source(
            &handle,
            &context,
            ServiceListenerOptions {
                include_value: true,
                queue_capacity: 4,
                ..listener_options()
            },
        )
        .unwrap();
    let id = source.registration_id().unwrap();
    let mut stream = DemandStream::new(source, 2).unwrap();
    execute(&registry, &handle, &context, write("put", b"k", b"one", 1)).unwrap();
    execute(&registry, &handle, &context, write("put", b"k", b"two", 2)).unwrap();
    let mut cx = Context::from_waker(Waker::noop());
    assert!(matches!(
        Pin::new(&mut stream).poll_next(&mut cx),
        Poll::Ready(Some(Ok(_)))
    ));
    assert_eq!(stream.retained_items(), 1);
    let snapshot = registry
        .resync_listener(&handle, &context, id, 16, 128)
        .unwrap();
    assert_eq!(snapshot.entries[0].value, b"two");
    drop(snapshot);
    assert!(Pin::new(&mut stream).poll_next(&mut cx).is_pending());
    assert_eq!(stream.retained_items(), 0);
    execute(
        &registry,
        &handle,
        &context,
        write("put", b"k", b"fresh", 3),
    )
    .unwrap();
    assert!(
        matches!(Pin::new(&mut stream).poll_next(&mut cx), Poll::Ready(Some(Ok(event))) if event.value.as_deref() == Some(b"fresh".as_slice()))
    );
    drop(registry);
    assert!(matches!(
        Pin::new(&mut stream).poll_next(&mut cx),
        Poll::Ready(Some(Err(DemandStreamError::Source(
            RegistryError::RegistryGone
        ))))
    ));
    assert!(matches!(
        Pin::new(&mut stream).poll_next(&mut cx),
        Poll::Ready(None)
    ));
}

#[test]
fn resync_reserves_expiry_fanout_before_time_queues_or_values_change() {
    let setup = |limits: RegistryBounds| {
        let registry = registry_with(limits, ServiceLimits::default(), security());
        let (context, handle) = open(&registry, "a", "map");
        let mut command = write("put", b"k", b"v", 1);
        command.ttl = ServiceTtl::ExpireAfter(1);
        execute(&registry, &handle, &context, command).unwrap();
        let target = registry
            .listener_source(&handle, &context, listener_options())
            .unwrap();
        let other = registry
            .listener_source(&handle, &context, listener_options())
            .unwrap();
        (registry, context, handle, target, other)
    };
    let (probe, _, _, _target, _other) = setup(bounds());
    let baseline = probe.usage().owned_bytes;
    for (limits, dimension) in [
        (
            RegistryBounds {
                max_owned_bytes: baseline + 1,
                ..bounds()
            },
            "owned_bytes",
        ),
        (
            RegistryBounds {
                max_tenant_owned_bytes: baseline + 1,
                ..bounds()
            },
            "tenant_owned_bytes",
        ),
    ] {
        let (registry, context, handle, target, _other) = setup(limits);
        registry.advance_clock(1).unwrap();
        let before = registry.diagnostics(&handle, &context).unwrap();
        assert_eq!(before.logical_time, 0);
        assert_eq!(before.owners.values, 1);
        assert_eq!(before.statistics.queued_events, 0);
        let error = registry
            .resync_listener(&handle, &context, target.registration_id().unwrap(), 1, 1)
            .unwrap_err();
        assert!(
            matches!(error, RegistryError::Quota { dimension: actual, .. } if actual == dimension)
        );
        assert_eq!(
            registry
                .resync_listener(
                    &handle,
                    &context,
                    target.registration_id().unwrap(),
                    1,
                    usize::MAX
                )
                .unwrap_err(),
            RegistryError::InvalidBound("resync_growth")
        );
        assert_eq!(registry.diagnostics(&handle, &context).unwrap(), before);
        assert_eq!(registry.usage().owned_bytes, baseline);
        assert_eq!(registry.usage().reserved_growth, 0);
    }
    let (registry, context, handle, target, mut other) = setup(bounds());
    registry.advance_clock(1).unwrap();
    let mut snapshot = registry
        .resync_listener(&handle, &context, target.registration_id().unwrap(), 1, 1)
        .unwrap();
    assert!(snapshot.entries.is_empty());
    assert_eq!(registry.usage().reserved_growth, 0);
    let after = registry.diagnostics(&handle, &context).unwrap();
    assert_eq!(after.logical_time, 1);
    assert_eq!(after.owners.resync_leases, 1);
    assert_eq!(after.statistics.queued_events, 1);
    let mut cx = Context::from_waker(Waker::noop());
    assert!(
        matches!(other.poll_batch(&mut cx, 1), Poll::Ready(Ok(Some(events))) if events.len() == 1 && events[0].kind == "expired")
    );
    snapshot.close();
    assert_eq!(
        registry
            .diagnostics(&handle, &context)
            .unwrap()
            .owners
            .resync_leases,
        0
    );
}

#[test]
fn resync_maintenance_bound_covers_large_expired_keys_with_tiny_values() {
    let registry = UnifiedServiceRegistry::new(
        RegistryBounds {
            max_owned_bytes: 5310,
            ..bounds()
        },
        CollectionBounds {
            max_key_bytes: 1024,
            max_value_bytes: 1,
            ..collection()
        },
        ServiceLimits {
            expiry_work: 1,
            ..ServiceLimits::default()
        },
        SecurityBounds {
            max_key_bytes: 1024,
            max_value_bytes: 1,
            ..security()
        },
        TrustEpochWindow::new(1, 3).unwrap(),
    )
    .unwrap();
    let (context, handle) = open(&registry, "a", "map");
    let mut command = write("put", &[b'k'; 1024], b"v", 1);
    command.ttl = ServiceTtl::ExpireAfter(1);
    execute(&registry, &handle, &context, command).unwrap();
    let mut listeners = Vec::new();
    for _ in 0..3 {
        listeners.push(
            registry
                .listener_source(
                    &handle,
                    &context,
                    ServiceListenerOptions {
                        max_value_bytes: 1,
                        ..listener_options()
                    },
                )
                .unwrap(),
        );
    }
    let baseline = registry.usage().owned_bytes;
    assert_eq!(baseline, 4810);
    registry.advance_clock(1).unwrap();
    let before = registry.diagnostics(&handle, &context).unwrap();
    assert!(matches!(
        registry.resync_listener(
            &handle,
            &context,
            listeners[0].registration_id().unwrap(),
            1,
            1
        ),
        Err(RegistryError::Quota {
            dimension: "owned_bytes",
            ..
        })
    ));
    assert_eq!(registry.diagnostics(&handle, &context).unwrap(), before);
    assert_eq!(before.logical_time, 0);
    assert_eq!(registry.usage().owned_bytes, baseline);
    assert_eq!(registry.usage().reserved_growth, 0);
}

struct RevocationWake {
    registry: UnifiedServiceRegistry,
    observed: AtomicUsize,
}
impl Wake for RevocationWake {
    fn wake(self: Arc<Self>) {
        let _ = self.registry.usage();
        assert!(self.registry.run_next().unwrap().is_none());
        self.observed.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn parked_auth_deadlines_and_trust_revocation_wake_and_release_actual_stream_owners() {
    let registry = new_registry();
    let (context, handle) = open(&registry, "a", "map");
    let source = registry
        .listener_source(&handle, &context, listener_options())
        .unwrap();
    let mut stream = DemandStream::new(source, 1).unwrap();
    let wake = Arc::new(RevocationWake {
        registry: registry.clone(),
        observed: AtomicUsize::new(0),
    });
    let waker = Waker::from(Arc::clone(&wake));
    let mut cx = Context::from_waker(&waker);
    assert!(Pin::new(&mut stream).poll_next(&mut cx).is_pending());
    registry.rotate_trust(2).unwrap();
    assert_eq!(wake.observed.load(Ordering::SeqCst), 1);
    assert!(
        Pin::new(&mut stream).poll_next(&mut cx).is_pending(),
        "previous trust epoch remains valid during overlap"
    );
    registry.advance_clock(4).unwrap();
    assert_eq!(wake.observed.load(Ordering::SeqCst), 2);
    assert!(matches!(
        Pin::new(&mut stream).poll_next(&mut cx),
        Poll::Ready(Some(Err(DemandStreamError::Source(
            RegistryError::Security(SecurityError::UnknownTrustEpoch(1))
        ))))
    ));
    let mut route = claims("a", 1);
    route.trust_epoch = 2;
    let fresh = AuthenticatedNamespace::verify(
        &IngressPrincipal { tenant: "a".into() },
        &verified("a", "map", 1, SurfaceId::Resp, b"x", SurfaceOperation::Read),
        &route,
        security(),
        TrustEpochWindow::new(2, 3).unwrap(),
        4,
    )
    .unwrap();
    let released = registry.diagnostics(&handle, &fresh).unwrap();
    assert_eq!(released.owners.streams, 0);
    assert_eq!(released.owners.listeners, 0);
    assert_eq!(released.resources.streams, 0);

    let registry = new_registry();
    let (context, handle) = open(&registry, "a", "map");
    let mut route = claims("a", 1);
    route.deadline_remaining_ticks = 1;
    let expiring = AuthenticatedNamespace::verify(
        &IngressPrincipal { tenant: "a".into() },
        &verified("a", "map", 1, SurfaceId::Resp, b"x", SurfaceOperation::Read),
        &route,
        security(),
        TrustEpochWindow::new(1, 3).unwrap(),
        0,
    )
    .unwrap();
    let source = registry
        .listener_source(&handle, &expiring, listener_options())
        .unwrap();
    let mut stream = DemandStream::new(source, 1).unwrap();
    let wake = Arc::new(RevocationWake {
        registry: registry.clone(),
        observed: AtomicUsize::new(0),
    });
    let waker = Waker::from(Arc::clone(&wake));
    let mut cx = Context::from_waker(&waker);
    assert!(Pin::new(&mut stream).poll_next(&mut cx).is_pending());
    registry.advance_clock(0).unwrap();
    assert_eq!(wake.observed.load(Ordering::SeqCst), 0);
    registry.advance_clock(1).unwrap();
    assert_eq!(wake.observed.load(Ordering::SeqCst), 1);
    assert!(matches!(
        Pin::new(&mut stream).poll_next(&mut cx),
        Poll::Ready(Some(Err(DemandStreamError::Source(
            RegistryError::Security(SecurityError::DeadlineExpired)
        ))))
    ));
    assert_eq!(
        registry
            .diagnostics(&handle, &context)
            .unwrap()
            .owners
            .total(),
        0
    );
    assert_eq!(registry.usage().reserved_growth, 0);
}
