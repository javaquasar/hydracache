use hydracache_cluster_testkit::value_plane_cluster_admission_075::*;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::task::{Wake, Waker};

struct CapacityWake {
    queue: ClusterAdmission,
    calls: AtomicUsize,
}
impl Wake for CapacityWake {
    fn wake(self: Arc<Self>) {
        self.calls.fetch_add(1, Ordering::SeqCst);
        // Reentering a notification must run after the admission mutex is released.
        self.queue.wake_waiters();
    }
}

#[test]
fn capacity_waiter_cut_is_atomic_bounded_and_reentrant_progress_is_not_lost() {
    let queue = ClusterAdmission::new(Default::default()).unwrap();
    let ticket = queue
        .admit(work("a", 0, b"key", ClusterLane::Data))
        .unwrap();
    let permit = queue.pop_for(ticket).unwrap().unwrap();
    let wake = Arc::new(CapacityWake {
        queue: queue.clone(),
        calls: AtomicUsize::new(0),
    });
    let waker = Waker::from(Arc::clone(&wake));
    let observed = queue.notification_epoch();
    assert!(queue.wait_for_change(ticket, observed, &waker));
    assert!(!queue.wait_for_change(u64::MAX, observed, &waker));
    let cut = queue.notify_waiters();
    assert_eq!(wake.calls.load(Ordering::SeqCst), 1);
    assert!(queue.notification_epoch() > cut);
    assert!(!queue.wait_for_change(ticket, cut, &waker));
    let observed = queue.notification_epoch();
    assert!(queue.wait_for_change(ticket, observed, &waker));
    drop(permit);
    assert_eq!(
        wake.calls.load(Ordering::SeqCst),
        1,
        "the released ticket's own waiter is removed"
    );
    assert!(!queue.wait_for_change(ticket, queue.notification_epoch(), &waker));
}

#[test]
fn parked_receipt_keeps_logical_owners_but_repair_can_make_progress_before_resume() {
    let queue = ClusterAdmission::new(ClusterAdmissionBounds {
        repair_permits: 1,
        ..Default::default()
    })
    .unwrap();
    let id = queue
        .admit(work("a", 0, b"repair", ClusterLane::Repair))
        .unwrap();
    let permit = queue.pop_for(id).unwrap().unwrap();
    let binding = AdmissionBinding {
        namespace: "map".into(),
        namespace_generation: 1,
        request_digest: [7; 32],
        identity: None,
        deadline: 100,
        grant_digest: None,
    };
    let attempt = permit.begin_invocation(binding.clone()).unwrap();
    assert_eq!(
        permit.park(),
        Err(ClusterAdmissionError::InvocationInFlight)
    );
    assert_eq!(
        permit.try_resume(),
        Err(ClusterAdmissionError::InvocationInFlight)
    );
    drop(attempt);
    permit.park().unwrap();
    assert_eq!(permit.park(), Err(ClusterAdmissionError::PermitParked));
    assert!(matches!(
        permit.begin_invocation(binding.clone()),
        Err(ClusterAdmissionError::PermitParked)
    ));
    assert!(
        !queue.cancel(id).unwrap(),
        "a held logical receipt cannot be cancelled behind its owner"
    );
    assert!(
        queue.pop().unwrap().is_none(),
        "parked work cannot acquire a second permit"
    );
    assert_eq!(queue.observations().owned_bytes[2], 128);
    assert_eq!(queue.observations().inflight[2], 0);
    let repair = queue
        .admit(work("a", 1, b"progress", ClusterLane::Repair))
        .unwrap();
    let physical = queue.pop_for(repair).unwrap().unwrap();
    assert!(!permit.try_resume().unwrap());
    drop(physical);
    assert!(permit.try_resume().unwrap());
    assert!(permit.try_resume().unwrap());
    let mut changed = binding.clone();
    changed.request_digest = [8; 32];
    assert!(matches!(
        permit.begin_invocation(changed),
        Err(ClusterAdmissionError::ReceiptConflict)
    ));
    drop(permit.begin_invocation(binding).unwrap());
    drop(permit);
    assert_eq!(queue.observations().released, 2);
    assert_eq!(queue.observations().owned_bytes, [0; 3]);
}

#[test]
fn parked_data_resume_honors_partition_and_replica_capacity_without_duplicate_leases() {
    let queue = ClusterAdmission::new(ClusterAdmissionBounds {
        data_permits: 2,
        max_replica_bytes: 64,
        ..Default::default()
    })
    .unwrap();
    let first = queue
        .admit(work("a", 0, b"first", ClusterLane::Data))
        .unwrap();
    let permit = queue.pop_for(first).unwrap().unwrap();
    permit.park().unwrap();
    let cold = queue
        .admit(work("b", 1, b"cold", ClusterLane::Data))
        .unwrap();
    let held = queue.pop_for(cold).unwrap().unwrap();
    assert!(
        !permit.try_resume().unwrap(),
        "actual replica capacity is held by cold partition"
    );
    let same = queue
        .admit(work("a", 0, b"same", ClusterLane::Data))
        .unwrap();
    assert!(
        queue.pop_for(same).unwrap().is_none(),
        "older parked logical request retains partition FIFO"
    );
    drop(held);
    assert!(permit.try_resume().unwrap());
    assert!(queue.pop_for(same).unwrap().is_none());
    drop(permit);
    queue.pop_for(same).unwrap().unwrap().finish();
    assert_eq!(queue.observations().released, 3);
}

#[test]
fn excluded_cooperative_jobs_do_not_block_safety_or_cold_data_but_keep_partition_fifo() {
    let queue = ClusterAdmission::new(Default::default()).unwrap();
    let paused = queue
        .admit(work("a", 0, b"paused", ClusterLane::Data))
        .unwrap();
    let first_safety = queue
        .admit(work("a", 0, b"safety", ClusterLane::Safety))
        .unwrap();
    let permit = queue.pop().unwrap().unwrap();
    assert_eq!(permit.ticket(), first_safety);
    drop(permit);
    let next_safety = queue
        .admit(work("a", 0, b"delete", ClusterLane::Safety))
        .unwrap();
    let permit = queue
        .pop_except(&[paused])
        .unwrap()
        .expect("excluded data cannot block another reserved lane");
    assert_eq!(permit.ticket(), next_safety);
    drop(permit);
    let later_same = queue
        .admit(work("a", 0, b"later", ClusterLane::Data))
        .unwrap();
    let cold = queue
        .admit(work("b", 1, b"cold", ClusterLane::Data))
        .unwrap();
    let permit = queue
        .pop_except(&[paused])
        .unwrap()
        .expect("unrelated partition must remain eligible");
    assert_eq!(permit.ticket(), cold);
    drop(permit);
    assert!(
        queue.pop_except(&[paused]).unwrap().is_none(),
        "later data cannot overtake the paused partition front"
    );
    assert!(
        queue.pop_for(later_same).unwrap().is_none(),
        "explicit selection still preserves scheduler fairness"
    );
    queue.pop_for(paused).unwrap().unwrap().finish();
    queue.pop_for(later_same).unwrap().unwrap().finish();
    assert_eq!(queue.observations().released, 5);
}

fn work(tenant: &str, partition: u32, key: &[u8], lane: ClusterLane) -> ClusterWork {
    ClusterWork {
        tenant: tenant.into(),
        partition,
        key: key.to_vec(),
        lane,
        bytes: 128,
        replica_bytes: 64,
    }
}

#[test]
fn isolated_lanes_and_held_hot_partition_do_not_starve_cold_work() {
    let queue = ClusterAdmission::new(ClusterAdmissionBounds {
        data_permits: 2,
        ..Default::default()
    })
    .unwrap();
    let hot = queue
        .admit(work("a", 1, b"hot", ClusterLane::Data))
        .unwrap();
    let held = queue.pop().unwrap().unwrap();
    assert_eq!(held.ticket(), hot);
    queue
        .admit(work("a", 1, b"hot", ClusterLane::Data))
        .unwrap();
    let cold = queue
        .admit(work("b", 2, b"cold", ClusterLane::Data))
        .unwrap();
    let repair = queue.admit(work("a", 3, b"", ClusterLane::Repair)).unwrap();
    let safety = queue.admit(work("a", 1, b"", ClusterLane::Safety)).unwrap();
    let mut dispatched = Vec::new();
    for _ in 0..3 {
        let permit = queue.pop().unwrap().unwrap();
        dispatched.push(permit.ticket());
        permit.finish();
    }
    assert!(
        dispatched.contains(&cold) && dispatched.contains(&repair) && dispatched.contains(&safety)
    );
    assert!(queue.pop().unwrap().is_none());
    drop(held);
    queue.pop().unwrap().unwrap().finish();
    let seen = queue.observations();
    assert_eq!(seen.queued, [0; 3]);
    assert_eq!(seen.inflight, [0; 3]);
    assert_eq!(seen.owned_bytes, [0; 3]);
    assert_eq!(seen.replica_bytes, 0);
    assert_eq!(seen.released, 5);
    assert!(seen.maximum_wait > 0);
}

#[test]
fn partition_fifo_and_tenant_round_robin_survive_cancellation_and_reactivation() {
    let queue = ClusterAdmission::new(Default::default()).unwrap();
    let a = queue.admit(work("a", 0, b"a", ClusterLane::Data)).unwrap();
    let b = queue.admit(work("b", 0, b"b", ClusterLane::Data)).unwrap();
    let cold = queue.admit(work("b", 1, b"c", ClusterLane::Data)).unwrap();
    let held = queue.pop().unwrap().unwrap();
    assert_eq!(held.ticket(), a);
    let next = queue.pop().unwrap().unwrap();
    assert_eq!(next.ticket(), cold);
    drop(next);
    assert!(!queue.cancel(a).unwrap());
    assert!(queue.pop().unwrap().is_none());
    drop(held);
    assert_eq!(queue.pop().unwrap().unwrap().ticket(), b);
    for _ in 0..100 {
        let id = queue
            .admit(work("returning", 4, b"x", ClusterLane::Data))
            .unwrap();
        assert!(queue.cancel(id).unwrap());
        assert!(!queue.cancel(id).unwrap());
    }
    assert_eq!(queue.observations().queued, [0; 3]);
    assert_eq!(queue.observations().owned_bytes, [0; 3]);
}

#[test]
fn every_budget_rejects_before_admission_and_replica_credits_wait_for_actual_finish() {
    let bounds = ClusterAdmissionBounds {
        max_jobs: 2,
        max_partition_jobs: 1,
        max_hot_key_waiters: 1,
        max_bytes: 256,
        max_tenant_bytes: 128,
        max_replica_bytes: 64,
        ..Default::default()
    };
    let queue = ClusterAdmission::new(bounds).unwrap();
    let id = queue
        .admit(work("a", 0, b"key", ClusterLane::Data))
        .unwrap();
    let before = queue.observations();
    for candidate in [
        work("a", 1, b"other", ClusterLane::Data),
        work("b", 0, b"other", ClusterLane::Data),
    ] {
        assert!(queue.admit(candidate).is_err());
        assert_eq!(queue.observations(), before);
    }
    let held = queue.pop().unwrap().unwrap();
    assert_eq!(held.ticket(), id);
    queue
        .admit(work("b", 1, b"cold", ClusterLane::Data))
        .unwrap();
    assert!(queue.pop().unwrap().is_none());
    queue.admit(work("a", 0, b"", ClusterLane::Safety)).unwrap();
    queue.pop().unwrap().unwrap().finish();
    assert_eq!(queue.observations().replica_bytes, 64);
    held.finish();
    queue.pop().unwrap().unwrap().finish();
    assert_eq!(queue.observations().replica_bytes, 0);
    assert!(queue
        .observations()
        .owned_bytes
        .iter()
        .all(|bytes| *bytes == 0));
}

#[test]
fn invalid_work_lane_bounds_and_oversized_requests_never_retain_metadata() {
    assert!(ClusterAdmission::new(ClusterAdmissionBounds {
        max_jobs: 0,
        ..Default::default()
    })
    .is_err());
    let queue = ClusterAdmission::new(ClusterAdmissionBounds {
        max_safety_jobs: 1,
        max_repair_jobs: 1,
        max_safety_bytes: 128,
        max_repair_bytes: 128,
        ..Default::default()
    })
    .unwrap();
    for tenant in ["", &"x".repeat(129)] {
        assert!(queue
            .admit(work(tenant, 0, b"x", ClusterLane::Data))
            .is_err());
    }
    let mut invalid = work("a", 0, b"x", ClusterLane::Data);
    invalid.bytes = 0;
    assert!(queue.admit(invalid).is_err());
    for lane in [ClusterLane::Safety, ClusterLane::Repair] {
        let id = queue.admit(work("a", 0, b"", lane)).unwrap();
        assert!(queue.admit(work("b", 1, b"", lane)).is_err());
        assert!(queue.cancel(id).unwrap());
    }
    assert_eq!(queue.observations().owned_bytes, [0; 3]);
}

#[test]
fn bulk_reservations_preserve_fifo_on_every_affected_partition() {
    let queue = ClusterAdmission::new(Default::default()).unwrap();
    let bulk = queue
        .admit_for_partitions(work("a", 0, b"", ClusterLane::Data), &[0, 1])
        .unwrap();
    let held = queue.pop().unwrap().unwrap();
    assert_eq!(held.ticket(), bulk);
    assert_eq!(held.partitions(), &[0, 1]);
    let secondary = queue
        .admit(work("b", 1, b"key", ClusterLane::Data))
        .unwrap();
    let cold = queue
        .admit(work("c", 2, b"key", ClusterLane::Data))
        .unwrap();
    assert_eq!(queue.pop().unwrap().unwrap().ticket(), cold);
    assert!(queue.pop().unwrap().is_none());
    drop(held);
    assert_eq!(queue.pop().unwrap().unwrap().ticket(), secondary);
    for partitions in [&[][..], &[0, 0], &[1]] {
        assert!(queue
            .admit_for_partitions(work("a", 0, b"", ClusterLane::Data), partitions)
            .is_err());
    }
}

#[test]
fn bulk_secondary_hot_keys_and_permit_key_binding_cannot_bypass_admission() {
    let queue = ClusterAdmission::new(ClusterAdmissionBounds {
        max_hot_key_waiters: 1,
        ..Default::default()
    })
    .unwrap();
    let keys = vec![b"first".to_vec(), b"hot".to_vec()];
    queue
        .admit_for_resources(work("a", 0, b"first", ClusterLane::Data), &[0, 1], &keys)
        .unwrap();
    assert!(queue
        .admit(work("a", 1, b"hot", ClusterLane::Data))
        .is_err());
    let held = queue.pop().unwrap().unwrap();
    assert!(held.keys_match(&keys));
    assert!(!held.keys_match(&[b"first".to_vec()]));
    assert!(!held.keys_match(&[vec![0; 129]]));
    drop(held);
    queue
        .admit(work("a", 1, b"hot", ClusterLane::Data))
        .unwrap();
    assert!(queue
        .admit_for_resources(work("b", 2, b"point", ClusterLane::Data), &[2], &[])
        .is_err());
    queue
        .admit_for_resources(work("empty-bulk", 2, b"", ClusterLane::Data), &[2], &[])
        .unwrap();
    let empty = queue.pop().unwrap().unwrap();
    assert!(empty.keys_match(&[]));
    assert!(!empty.keys_match(&[Vec::new()]));
}

#[test]
fn admission_ticket_and_logical_tick_never_wrap_into_reused_work() {
    // Repeated completed leases exercise monotonically fresh tickets and single-release ownership.
    let queue = ClusterAdmission::new(Default::default()).unwrap();
    let mut previous = 0;
    for _ in 0..256 {
        let id = queue.admit(work("a", 0, b"x", ClusterLane::Data)).unwrap();
        assert!(id > previous);
        previous = id;
        queue.pop().unwrap().unwrap().finish();
    }
    assert_eq!(queue.observations().released, 256);
    assert_eq!(queue.observations().owned_bytes, [0; 3]);
}

#[test]
fn repair_tenant_churn_cannot_reset_the_data_lane_round_robin() {
    let queue = ClusterAdmission::new(Default::default()).unwrap();
    queue
        .admit(work("a", 0, b"hot", ClusterLane::Data))
        .unwrap();
    let cold = queue
        .admit(work("b", 1, b"cold", ClusterLane::Data))
        .unwrap();
    queue.admit(work("z", 2, b"", ClusterLane::Repair)).unwrap();
    let mut cold_seen = false;
    for _ in 0..8 {
        let lease = queue.pop().unwrap().unwrap();
        let ticket = lease.ticket();
        let lane = lease.work().lane;
        let tenant = lease.work().tenant.clone();
        drop(lease);
        if ticket == cold {
            cold_seen = true;
            break;
        }
        if lane == ClusterLane::Repair {
            queue.admit(work("z", 2, b"", ClusterLane::Repair)).unwrap();
        } else if tenant == "a" {
            queue
                .admit(work("a", 0, b"hot", ClusterLane::Data))
                .unwrap();
        }
    }
    assert!(
        cold_seen,
        "cold data must progress despite continuous repair and hot data"
    );
}
