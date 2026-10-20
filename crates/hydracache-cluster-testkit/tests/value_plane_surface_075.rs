use hydracache_cluster_testkit::value_plane_model_075::{
    MutationIdentity, MutationOperation, TtlDirective, ValuePlaneBounds,
};
use hydracache_cluster_testkit::value_plane_surface_075::{
    CrossSurfaceReferenceMap, IngressPrincipal, SurfaceCapability, SurfaceError, SurfaceId,
    SurfaceOperation, SurfaceOutcome, VerifiedMapCommand,
};

fn principal() -> IngressPrincipal {
    IngressPrincipal {
        tenant: "tenant-a".into(),
    }
}

fn command(surface: SurfaceId, operation: SurfaceOperation) -> VerifiedMapCommand {
    VerifiedMapCommand::new(
        surface,
        &principal(),
        "tenant-a",
        "orders",
        1,
        b"key".to_vec(),
        operation,
    )
    .unwrap()
}

fn read(surface: SurfaceId) -> VerifiedMapCommand {
    command(surface, SurfaceOperation::Read)
}

#[test]
fn every_surface_projects_into_one_value_and_accounting_owner() {
    let mut map = CrossSurfaceReferenceMap::new(17, ValuePlaneBounds::default()).unwrap();
    let surfaces = [
        SurfaceId::Resp,
        SurfaceId::Hc1,
        SurfaceId::Hc2Rust,
        SurfaceId::Hc2Java,
    ];
    for (index, writer) in surfaces.into_iter().enumerate() {
        let value = vec![b'v', index as u8];
        map.execute(command(
            writer,
            SurfaceOperation::Mutation {
                identity: MutationIdentity::new("surface-client", index as u64 + 1),
                operation: MutationOperation::Put {
                    value: value.clone(),
                    ttl: TtlDirective::Eternal,
                },
            },
        ))
        .unwrap();
        for reader in surfaces {
            assert_eq!(
                map.execute(read(reader)).unwrap(),
                SurfaceOutcome::Read(Some(value.clone()))
            );
        }
    }
    assert_eq!(map.event_count(), 4);
    assert_eq!(map.accounted_value_bytes(), 2);
}

#[test]
fn unsupported_surface_fails_before_shared_state_mutation() {
    let mut map = CrossSurfaceReferenceMap::new(3, ValuePlaneBounds::default()).unwrap();
    map.disable(SurfaceId::Resp, SurfaceCapability::Mutate);
    assert_eq!(
        map.execute(command(
            SurfaceId::Resp,
            SurfaceOperation::Mutation {
                identity: MutationIdentity::new("client", 1),
                operation: MutationOperation::Put {
                    value: b"value".to_vec(),
                    ttl: TtlDirective::Eternal,
                },
            },
        )),
        Err(SurfaceError::Unsupported)
    );
    assert_eq!(
        map.execute(read(SurfaceId::Hc2Rust)).unwrap(),
        SurfaceOutcome::Read(None)
    );
    assert_eq!(map.event_count(), 0);
}

#[test]
fn ingress_principal_prevents_cross_tenant_projection() {
    assert_eq!(
        VerifiedMapCommand::new(
            SurfaceId::Hc2Java,
            &principal(),
            "tenant-b",
            "orders",
            1,
            b"key".to_vec(),
            SurfaceOperation::Read,
        ),
        Err(SurfaceError::TenantSubstitution)
    );
}

#[test]
fn replay_through_another_surface_returns_one_retained_outcome() {
    let mut map = CrossSurfaceReferenceMap::new(5, ValuePlaneBounds::default()).unwrap();
    let identity = MutationIdentity::new("client", 9);
    let operation = MutationOperation::GetAndPut {
        value: b"value".to_vec(),
        ttl: TtlDirective::Eternal,
    };
    let first = map
        .execute(command(
            SurfaceId::Hc1,
            SurfaceOperation::Mutation {
                identity: identity.clone(),
                operation: operation.clone(),
            },
        ))
        .unwrap();
    let replay = map
        .execute(command(
            SurfaceId::Hc2Java,
            SurfaceOperation::Mutation {
                identity,
                operation,
            },
        ))
        .unwrap();
    assert_eq!(first, replay);
    assert_eq!(map.event_count(), 1);
}
