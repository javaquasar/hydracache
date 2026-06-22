use hydracache::{
    replicated_slot_version, ClusterEpoch, ClusterGeneration, ReplicatedSlot, TombstoneAdmission,
    TombstoneBudget, TombstoneTracker,
};
use proptest::prelude::*;

proptest! {
    #[test]
    fn tombstone_beats_stale_value_replication(value_version in 0_u64..1_000_000, delta in 1_u64..1000) {
        let stale_value = ReplicatedSlot::Value {
            value: b"stale".to_vec(),
            version: value_version,
        };
        let tombstone = ReplicatedSlot::Tombstone {
            version: value_version + delta,
            gc_eligible_after: None,
        };

        prop_assert!(stale_value.clone().merge(tombstone.clone()).is_tombstone());
        prop_assert!(tombstone.merge(stale_value).is_tombstone());
    }
}

#[test]
fn tombstone_wins_on_version_tie() {
    let value = ReplicatedSlot::Value {
        value: "old",
        version: 42,
    };
    let tombstone = ReplicatedSlot::Tombstone {
        version: 42,
        gc_eligible_after: None,
    };

    assert!(value.merge(tombstone).is_tombstone());
}

#[test]
fn concurrent_value_and_tombstone_resolve_by_version() {
    let older_tombstone = ReplicatedSlot::Tombstone::<String> {
        version: 3,
        gc_eligible_after: None,
    };
    let newer_value = ReplicatedSlot::Value {
        value: "fresh".to_owned(),
        version: 4,
    };

    assert_eq!(
        older_tombstone.merge(newer_value),
        ReplicatedSlot::Value {
            value: "fresh".to_owned(),
            version: 4,
        }
    );
}

#[test]
fn gc_blocked_until_repair_confirmation() {
    let mut tracker = TombstoneTracker::new(TombstoneBudget::new(2, 1024));

    assert_eq!(
        tracker.admit("user:1", 1, 64, None),
        TombstoneAdmission::Stored
    );
    assert_eq!(
        tracker.admit("user:2", 2, 64, None),
        TombstoneAdmission::Stored
    );
    assert!(tracker.contains_key("user:1"));
    assert!(tracker.contains_key("user:2"));

    tracker.confirm_repair("user:1", ClusterEpoch::new(3));
    assert_eq!(
        tracker.admit("user:3", 3, 64, None),
        TombstoneAdmission::EvictedEligible { freed: 1 }
    );
    assert!(!tracker.contains_key("user:1"));
    assert!(tracker.contains_key("user:2"));
    assert!(tracker.contains_key("user:3"));
}

#[test]
fn eligible_tombstones_evicted_oldest_first_under_budget() {
    let mut tracker = TombstoneTracker::new(TombstoneBudget::new(2, 1024));
    tracker.admit("old", 1, 64, Some(ClusterEpoch::new(1)));
    tracker.admit("middle", 2, 64, Some(ClusterEpoch::new(1)));

    let admission = tracker.admit("new", 3, 64, Some(ClusterEpoch::new(1)));

    assert_eq!(admission, TombstoneAdmission::EvictedEligible { freed: 1 });
    assert!(!tracker.contains_key("old"));
    assert!(tracker.contains_key("middle"));
    assert!(tracker.contains_key("new"));
}

#[test]
fn blocking_tombstone_never_silently_dropped() {
    let mut tracker = TombstoneTracker::new(TombstoneBudget::new(1, 128));
    tracker.admit("blocking-a", 1, 64, None);

    assert_eq!(
        tracker.admit("blocking-b", 2, 64, None),
        TombstoneAdmission::RepairDebt
    );
    assert_eq!(tracker.len(), 2);
    assert!(tracker.repair_debt());
}

#[test]
fn replicated_version_is_generation_plus_message_id() {
    assert_eq!(
        replicated_slot_version(ClusterGeneration::new(7), 9),
        (7_u64 << 32) | 9
    );
}
