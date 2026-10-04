use hydracache_cluster_testkit::value_plane_admission_075::{
    AdmissionError, AdmissionLimits, DataWork, FairAdmissionQueue, NamespaceError,
    NamespaceLifecycle, NamespacePhase, ReclamationOwners, ReclamationWatermarks, WorkItem,
};

fn data(tenant: &str, partition: u32, request_id: u64) -> DataWork {
    DataWork {
        tenant: tenant.into(),
        partition,
        request_id,
    }
}

#[test]
fn noisy_tenant_and_hot_partition_cannot_starve_cold_tenant() {
    let mut queue = FairAdmissionQueue::new(AdmissionLimits {
        total: 16,
        per_tenant: 8,
        per_partition: 4,
        safety: 2,
    })
    .unwrap();
    for request_id in 0..4 {
        queue.admit_data(data("noisy", 7, request_id)).unwrap();
    }
    assert!(matches!(
        queue.admit_data(data("noisy", 7, 99)),
        Err(AdmissionError::PartitionQueueFull { .. })
    ));
    queue.admit_data(data("cold", 1, 100)).unwrap();

    let first_two = [queue.pop().unwrap(), queue.pop().unwrap()];
    assert!(first_two.iter().any(|item| matches!(
        item,
        WorkItem::Data(work) if work.tenant == "cold"
    )));
}

#[test]
fn safety_lane_is_reserved_and_served_first() {
    let mut queue = FairAdmissionQueue::new(AdmissionLimits {
        total: 1,
        per_tenant: 1,
        per_partition: 1,
        safety: 1,
    })
    .unwrap();
    queue.admit_data(data("tenant", 1, 1)).unwrap();
    assert_eq!(
        queue.admit_data(data("other", 2, 2)),
        Err(AdmissionError::TotalQueueFull)
    );
    queue.admit_safety("promotion").unwrap();
    assert!(matches!(queue.pop(), Some(WorkItem::Safety { .. })));
}

#[test]
fn all_admission_dimensions_are_finite_and_enforced() {
    assert!(matches!(
        FairAdmissionQueue::new(AdmissionLimits {
            total: 0,
            per_tenant: 1,
            per_partition: 1,
            safety: 1,
        }),
        Err(AdmissionError::InvalidBound("total"))
    ));
}

#[test]
fn namespace_reuse_waits_for_delete_and_listener_watermarks() {
    let mut lifecycle = NamespaceLifecycle::new(3).unwrap();
    lifecycle.begin_delete().unwrap();
    assert_eq!(lifecycle.phase(), NamespacePhase::Draining);
    lifecycle
        .commit_delete(
            10,
            ReclamationOwners {
                entries: 2,
                ttl_tasks: 1,
                tombstones: 1,
                dedup_results: 1,
                subscriptions: 1,
                staging_files: 1,
                quota_owners: 1,
            },
        )
        .unwrap();
    lifecycle.advance_reclamation(ReclamationWatermarks {
        replica_applied: 10,
        listener_cutover: 9,
        dedup_expired: 10,
        transfer_closed: 10,
        tombstone_gc: 10,
    });
    assert_eq!(lifecycle.phase(), NamespacePhase::DeleteCommitted);
    assert_eq!(lifecycle.recreate(), Err(NamespaceError::NotReclaimable));
    lifecycle.advance_reclamation(ReclamationWatermarks {
        replica_applied: 8,
        listener_cutover: 10,
        dedup_expired: 8,
        transfer_closed: 8,
        tombstone_gc: 8,
    });
    assert_eq!(lifecycle.phase(), NamespacePhase::Reclaimable);
    assert_eq!(lifecycle.reclaim_step(3).unwrap(), 3);
    assert_eq!(lifecycle.recreate(), Err(NamespaceError::OwnersRemain(5)));
    assert_eq!(lifecycle.reclaim_step(5).unwrap(), 5);
    assert_eq!(lifecycle.recreate().unwrap(), 4);
}

#[test]
fn old_namespace_generation_never_crosses_recreate() {
    let mut lifecycle = NamespaceLifecycle::new(1).unwrap();
    lifecycle.begin_delete().unwrap();
    lifecycle
        .commit_delete(5, ReclamationOwners::default())
        .unwrap();
    lifecycle.advance_reclamation(ReclamationWatermarks {
        replica_applied: 5,
        listener_cutover: 5,
        dedup_expired: 5,
        transfer_closed: 5,
        tombstone_gc: 5,
    });
    lifecycle.recreate().unwrap();
    assert!(matches!(
        lifecycle.validate_request(1),
        Err(NamespaceError::StaleGeneration {
            expected: 2,
            actual: 1
        })
    ));
    lifecycle.validate_request(2).unwrap();
}

#[test]
fn namespace_reclamation_requires_every_owner_watermark_and_bounded_cleanup() {
    let mut lifecycle = NamespaceLifecycle::new(9).unwrap();
    lifecycle.begin_delete().unwrap();
    lifecycle
        .commit_delete(
            20,
            ReclamationOwners {
                entries: 1,
                ttl_tasks: 1,
                tombstones: 1,
                dedup_results: 1,
                subscriptions: 1,
                staging_files: 1,
                quota_owners: 1,
            },
        )
        .unwrap();
    lifecycle.advance_reclamation(ReclamationWatermarks {
        replica_applied: 20,
        listener_cutover: 20,
        dedup_expired: 20,
        transfer_closed: 19,
        tombstone_gc: 20,
    });
    assert_eq!(lifecycle.phase(), NamespacePhase::DeleteCommitted);
    assert_eq!(
        lifecycle.reclaim_step(1),
        Err(NamespaceError::NotReclaimable)
    );
    lifecycle.advance_reclamation(ReclamationWatermarks {
        transfer_closed: 20,
        ..ReclamationWatermarks::default()
    });
    assert_eq!(
        lifecycle.reclaim_step(0),
        Err(NamespaceError::InvalidBound("reclaim_items"))
    );
    assert_eq!(lifecycle.reclaim_step(2).unwrap(), 2);
    assert_eq!(lifecycle.owners().total(), 5);
}
