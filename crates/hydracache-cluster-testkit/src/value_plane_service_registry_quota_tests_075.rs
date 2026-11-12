//! Private fixture may tighten an already-populated registry budget without a public config API.
use super::*;
use crate::value_plane_cluster_admission_075::ClusterAdmissionBounds;
use crate::value_plane_cluster_auth_075::RuntimeMember;
use crate::value_plane_cluster_runtime_075::ClusterHelperResult;
use crate::value_plane_loopback_075::LoopbackFault;
use crate::value_plane_service_075::{VersionBulkEntry, VersionBulkOperation};
use crate::value_plane_service_cluster_075::{
    ServiceClusterBounds, ServiceClusterCall, ServiceLinkFaults,
};
use hydracache_cluster_transport_axum::NodeCredential;

fn fixture() -> (
    UnifiedServiceRegistry,
    AuthenticatedNamespace,
    RegistryHandle,
) {
    let security = SecurityBounds::default();
    let registry = UnifiedServiceRegistry::new_clustered(
        RegistryBounds::default(),
        CollectionBounds {
            max_entries: 64,
            max_key_bytes: 64,
            max_value_bytes: 128,
            max_page_items: 16,
            max_scan_items: 128,
            max_response_bytes: 4096,
            max_bulk_mutation_items: 32,
        },
        ServiceLimits::default(),
        security,
        TrustEpochWindow::new(1, 3).unwrap(),
        ClusterRuntimeConfig {
            cluster: "quota".into(),
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
                max_frame_bytes: 1024 * 1024,
                ..ServiceClusterBounds::default()
            },
            admission: ClusterAdmissionBounds {
                max_replica_bytes: 64 * 1024 * 1024,
                max_repair_bytes: 64 * 1024 * 1024,
                ..ClusterAdmissionBounds::default()
            },
            max_replays: 128,
        },
    )
    .unwrap();
    let principal = IngressPrincipal {
        tenant: "tenant".into(),
    };
    let verified = VerifiedMapCommand::new(
        SurfaceId::Hc2Rust,
        &principal,
        "tenant",
        "map",
        1,
        b"key".to_vec(),
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
        key_bytes: 3,
        value_bytes: 0,
    };
    let auth = AuthenticatedNamespace::verify(
        &principal,
        &verified,
        &claims,
        security,
        TrustEpochWindow::new(1, 3).unwrap(),
        0,
    )
    .unwrap();
    let handle = registry.open(&auth, auth.address()).unwrap();
    (registry, auth, handle)
}
fn execute(
    registry: &UnifiedServiceRegistry,
    handle: &RegistryHandle,
    auth: &AuthenticatedNamespace,
    command: ServiceCommand,
) -> ServiceResult {
    registry
        .enqueue(handle, auth, SurfaceId::Hc2Rust, command)
        .unwrap();
    let RegistryResult::ClusterCommand(result) =
        registry.run_next().unwrap().unwrap().result.unwrap()
    else {
        panic!("cluster owner expected");
    };
    result.result
}
fn forecast(
    registry: &UnifiedServiceRegistry,
    handle: &RegistryHandle,
    auth: &AuthenticatedNamespace,
    request: &ClusterHelperRequest,
) -> usize {
    let state = registry.state.lock().unwrap();
    state.maps[&address_key(handle.address())]
        .service
        .helper_preflight(auth, request)
        .unwrap()
        .growth
}

#[test]
fn typed_helper_transfer_holds_are_admitted_by_global_quota_before_any_pending_root() {
    let (registry, auth, handle) = fixture();
    let mut entries = Vec::new();
    for (index, key) in [vec![0, 0], vec![0, 255]].into_iter().enumerate() {
        let mut put = ServiceCommand::point("put", &key);
        put.value = Some(vec![77; 128]);
        put.identity = Some(MutationIdentity::new("seed", index as u64 + 1));
        execute(&registry, &handle, &auth, put);
        let token = execute(
            &registry,
            &handle,
            &auth,
            ServiceCommand::point("get_entry_view", &key),
        )
        .token
        .unwrap();
        entries.push(VersionBulkEntry {
            key,
            token,
            value: None,
            ttl: ServiceTtl::ExpireAfter(20),
        });
    }
    let request = ClusterHelperRequest::new(
        MutationIdentity::new("root-b", 1),
        ClusterHelperCommand::VersionBulk {
            operation: VersionBulkOperation::SetTtl,
            entries,
        },
    );
    let plain = forecast(&registry, &handle, &auth, &request);
    for (partition, target) in [(0, "c"), (1, "a")] {
        registry
            .enqueue_maintenance(
                &handle,
                &auth,
                RuntimeMaintenance::RepairStep {
                    partition,
                    backup: target.into(),
                    faults: ServiceLinkFaults::default(),
                    maximum_work: 1,
                },
            )
            .unwrap();
        registry.run_next().unwrap().unwrap().result.unwrap();
    }
    let with_holds = forecast(&registry, &handle, &auth, &request);
    assert!(
        with_holds > plain,
        "active transfer holds must add admission credit"
    );
    let before = registry.diagnostics(&handle, &auth).unwrap();
    let owned = usage_bytes(before.resources);
    let old_bounds = {
        let mut state = registry.state.lock().unwrap();
        let old = state.bounds;
        state.bounds.max_owned_bytes = owned + plain;
        state.bounds.max_tenant_owned_bytes = owned + plain;
        old
    };
    assert!(matches!(
        registry.enqueue_helper(
            &handle,
            &auth,
            SurfaceId::Hc2Rust,
            request.clone(),
            RuntimeCallOptions::default()
        ),
        Err(RegistryError::Quota { .. })
    ));
    assert_eq!(registry.diagnostics(&handle, &auth).unwrap(), before);
    assert_eq!(registry.usage().queued_requests, 0);
    assert_eq!(registry.usage().reserved_growth, 0);
    registry.state.lock().unwrap().bounds = old_bounds;
    let mut call = ServiceClusterCall::default();
    call.faults.coordinator.replica = LoopbackFault::Drop;
    registry
        .enqueue_helper(
            &handle,
            &auth,
            SurfaceId::Hc2Rust,
            request.clone(),
            RuntimeCallOptions {
                call,
                ..RuntimeCallOptions::default()
            },
        )
        .unwrap();
    let RegistryResult::ClusterHelper(result) =
        registry.run_next().unwrap().unwrap().result.unwrap()
    else {
        panic!("typed owner expected");
    };
    assert!(!result.backup_proved);
    let ClusterHelperResult::Versions(partial) = result.result else {
        panic!("version helper must preserve ordered unknown items")
    };
    assert_eq!(partial.kind, "partial");
    assert_eq!(partial.items.len(), 2);
    assert!(partial
        .items
        .iter()
        .all(|item| item.kind == "outcome_unknown" && item.version.is_none()));
    let after = registry.diagnostics(&handle, &auth).unwrap();
    assert!(after.owners.pending > 0);
    let measured = usage_bytes(after.resources)
        .checked_sub(owned)
        .expect("pending root and holds own additional payload");
    assert!(
        with_holds >= measured,
        "helper forecast must cover actual pending and transfer credit owners"
    );
    registry
        .cancel_pending(&handle, &auth, &request.identity)
        .unwrap();
    assert_eq!(
        registry.diagnostics(&handle, &auth).unwrap().owners.pending,
        0
    );
    assert_eq!(registry.usage().reserved_growth, 0);
}
