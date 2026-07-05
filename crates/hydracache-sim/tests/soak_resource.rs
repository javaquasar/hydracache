use hydracache::{StorageOp, StorageOpKind};
use hydracache_sim::{
    BoundedGrowthChecker, InvariantReport, ResourceBudget, ResourceSample, SimConfig, SimStorage,
    SimWorld, MAX_SUBSCRIBER_BUFFER,
};

#[test]
fn footprint_counts_live_bytes_and_pending_delete_markers() {
    let mut storage = SimStorage::new();

    storage
        .apply_checked(write_request(1, "alpha", b"live"))
        .expect("write succeeds");
    let volatile = storage.footprint();
    assert_eq!(volatile.zones, 1);
    assert_eq!(volatile.entries, 1);
    assert_eq!(volatile.live_bytes, 4);
    assert_eq!(volatile.stable_bytes, 0);
    assert_eq!(volatile.volatile_bytes, 4);
    assert_eq!(volatile.pending_delete_markers, 0);
    assert_eq!(volatile.tracked_bytes(), 4);

    storage.fsync();
    let stable = storage.footprint();
    assert_eq!(stable.live_bytes, 4);
    assert_eq!(stable.stable_bytes, 4);
    assert_eq!(stable.volatile_bytes, 0);
    assert_eq!(stable.tracked_bytes(), 4);

    storage
        .apply_checked(write_request(2, "alpha", b"larger"))
        .expect("overwrite succeeds");
    let overwritten = storage.footprint();
    assert_eq!(overwritten.live_bytes, 6);
    assert_eq!(overwritten.stable_bytes, 4);
    assert_eq!(overwritten.volatile_bytes, 6);
    assert_eq!(overwritten.tracked_bytes(), 10);

    storage
        .apply_checked(delete_request(3, "alpha"))
        .expect("delete succeeds");
    let deleted = storage.footprint();
    assert_eq!(deleted.live_bytes, 0);
    assert_eq!(deleted.stable_bytes, 4);
    assert_eq!(deleted.volatile_bytes, 0);
    assert_eq!(deleted.pending_delete_markers, 1);
    assert_eq!(deleted.tracked_bytes(), 4);

    storage.fsync();
    let compacted = storage.footprint();
    assert_eq!(compacted.entries, 0);
    assert_eq!(compacted.live_bytes, 0);
    assert_eq!(compacted.tracked_bytes(), 0);
    assert_eq!(compacted.pending_delete_markers, 0);
}

#[test]
fn bounded_growth_invariant_flags_a_leaky_fixture() {
    let mut checker = BoundedGrowthChecker::new(ResourceBudget {
        max_storage_bytes: 10,
        sample_window: 3,
        ..ResourceBudget::default()
    });
    let mut report = InvariantReport::default();

    for storage_bytes in [4, 8, 16] {
        checker.observe(
            ResourceSample {
                storage_bytes,
                ..ResourceSample::default()
            },
            &mut report,
        );
    }

    assert_eq!(checker.retained_samples(), 3);
    assert!(report
        .violations
        .iter()
        .any(|violation| violation.name == "resource_bounded_growth"));
}

#[test]
fn steady_state_oscillation_within_budget_passes() {
    let mut checker = BoundedGrowthChecker::new(ResourceBudget {
        max_storage_bytes: 32,
        sample_window: 3,
        ..ResourceBudget::default()
    });
    let mut report = InvariantReport::default();

    for storage_bytes in [8, 16, 8, 16, 8] {
        checker.observe(
            ResourceSample {
                storage_bytes,
                ..ResourceSample::default()
            },
            &mut report,
        );
    }

    assert!(report.violations.is_empty(), "{:?}", report.violations);
}

#[test]
fn one_time_stepup_then_plateau_is_not_a_leak() {
    let mut checker = BoundedGrowthChecker::new(ResourceBudget {
        max_storage_bytes: 16,
        sample_window: 3,
        ..ResourceBudget::default()
    });
    let mut report = InvariantReport::default();

    for storage_bytes in [8, 24, 24, 24] {
        checker.observe(
            ResourceSample {
                storage_bytes,
                ..ResourceSample::default()
            },
            &mut report,
        );
    }

    assert!(report.violations.is_empty(), "{:?}", report.violations);
}

#[test]
fn subscriber_pending_stays_bounded_under_slow_consumer() {
    let mut world = SimWorld::new(0x58_20, SimConfig::default());
    world.set_workload_enabled(false);
    world.subscribe("client-a", "ns");

    for index in 0..(MAX_SUBSCRIBER_BUFFER + 8) {
        world
            .push_event("client-a", "ns", format!("key-{index}"), "value")
            .expect("manual push succeeds");
    }

    let snapshot = world.snapshot();
    let lag = snapshot
        .subscribers
        .iter()
        .map(|subscriber| subscriber.lag)
        .max()
        .unwrap_or_default();
    assert!(lag <= MAX_SUBSCRIBER_BUFFER as u64, "{snapshot:?}");
    assert!(world.invariant_report().is_ok());
}

fn write_request(request_id: u64, key: &str, value: &[u8]) -> StorageOp {
    StorageOp {
        request_id,
        kind: StorageOpKind::Write {
            key: key.to_owned(),
            value: value.to_vec(),
        },
    }
}

fn delete_request(request_id: u64, key: &str) -> StorageOp {
    StorageOp {
        request_id,
        kind: StorageOpKind::Delete {
            key: key.to_owned(),
        },
    }
}
