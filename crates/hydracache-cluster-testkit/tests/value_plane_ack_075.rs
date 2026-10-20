use std::collections::BTreeSet;

use hydracache_cluster_testkit::value_plane_ack_075::{
    AckContract, AckProgress, AckTrackerBounds, AckTrackerError, ReplicaAckTracker,
    ReplicaApplyAck, ReplicaGeneration,
};
use hydracache_cluster_testkit::value_plane_model_075::MutationIdentity;

fn mutation(sequence: u64) -> MutationIdentity {
    MutationIdentity::new("client-a", sequence)
}

fn contract(sequence: u64, retained_bytes: usize) -> AckContract {
    AckContract {
        partition: 7,
        epoch: 11,
        version: sequence,
        checksum: 900 + sequence,
        mutation: mutation(sequence),
        expected_replicas: BTreeSet::from([
            ReplicaGeneration::new("backup-b", 3),
            ReplicaGeneration::new("backup-c", 4),
        ]),
        required: 2,
        retained_bytes,
    }
}

fn ack(sequence: u64, node: &str, generation: u64) -> ReplicaApplyAck {
    ReplicaApplyAck {
        partition: 7,
        epoch: 11,
        version: sequence,
        checksum: 900 + sequence,
        mutation: mutation(sequence),
        replica: ReplicaGeneration::new(node, generation),
    }
}

fn tracker() -> ReplicaAckTracker {
    ReplicaAckTracker::new(AckTrackerBounds {
        max_in_flight: 2,
        max_in_flight_bytes: 32,
        max_expected_replicas: 2,
    })
    .unwrap()
}

#[test]
fn exact_unique_replica_generations_satisfy_the_contract() {
    let mut tracker = tracker();
    tracker.register(contract(1, 8)).unwrap();
    assert_eq!(
        tracker.record_ack(ack(1, "backup-b", 3)).unwrap(),
        AckProgress::Pending {
            received: 1,
            required: 2
        }
    );
    assert_eq!(
        tracker.record_ack(ack(1, "backup-c", 4)).unwrap(),
        AckProgress::Satisfied {
            received: 2,
            required: 2
        }
    );
    assert_eq!(tracker.finish(&mutation(1)).unwrap().version, 1);
    assert!(tracker.is_empty());
    assert_eq!(tracker.retained_bytes(), 0);
}

#[test]
fn duplicate_wrong_generation_and_false_claims_fail_loud() {
    let mut tracker = tracker();
    tracker.register(contract(1, 8)).unwrap();
    tracker.record_ack(ack(1, "backup-b", 3)).unwrap();
    assert_eq!(
        tracker.record_ack(ack(1, "backup-b", 3)),
        Err(AckTrackerError::DuplicateAck)
    );
    assert_eq!(
        tracker.record_ack(ack(1, "backup-b", 2)),
        Err(AckTrackerError::UnexpectedReplica)
    );
    let mut false_ack = ack(1, "backup-c", 4);
    false_ack.epoch = 10;
    assert_eq!(
        tracker.record_ack(false_ack),
        Err(AckTrackerError::ClaimMismatch("epoch"))
    );
    assert_eq!(
        tracker.finish(&mutation(1)),
        Err(AckTrackerError::NotSatisfied)
    );
}

#[test]
fn timeout_cancel_and_success_release_all_ownership() {
    let mut tracker = tracker();
    tracker.register(contract(1, 8)).unwrap();
    tracker.register(contract(2, 9)).unwrap();
    assert_eq!(tracker.retained_bytes(), 17);
    tracker.timeout(&mutation(1)).unwrap();
    tracker.cancel(&mutation(2)).unwrap();
    assert_eq!(tracker.len(), 0);
    assert_eq!(tracker.retained_bytes(), 0);
}

#[test]
fn count_byte_replica_and_contract_bounds_are_enforced() {
    assert_eq!(
        ReplicaAckTracker::new(AckTrackerBounds {
            max_in_flight: 0,
            max_in_flight_bytes: 1,
            max_expected_replicas: 1,
        })
        .unwrap_err(),
        AckTrackerError::InvalidBound("in_flight")
    );
    let mut tracker = tracker();
    tracker.register(contract(1, 24)).unwrap();
    assert_eq!(
        tracker.register(contract(2, 9)),
        Err(AckTrackerError::CapacityExceeded("in_flight_bytes"))
    );
    let mut invalid = contract(3, 1);
    invalid.required = 3;
    assert_eq!(
        tracker.register(invalid),
        Err(AckTrackerError::InvalidContract("required_replicas"))
    );
}
