use hydracache::{
    AdmissionController, AdmissionError, AdmissionLimits, AdmissionPermit,
    AdmissionRejectionReason, AdmissionSnapshot,
};

mod sustained_overload {
    use super::*;

    #[test]
    fn sustained_overload_rejects_are_counted_and_queue_is_bounded() {
        let limits = AdmissionLimits::new(2, 120, 4).retry_after_ms(10);
        let mut controller = AdmissionController::new(limits);
        let active = fill_in_flight(&mut controller, limits, 40);

        for index in 0..1_000 {
            let _ = controller.enqueue(format!("queued-{index}"), 20);
            let snapshot = controller.snapshot();
            assert_bounded(snapshot, limits);
        }

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.in_flight, limits.max_in_flight);
        assert_eq!(snapshot.queue_depth, limits.max_queue_depth);
        assert!(
            snapshot.rejected_total >= 996,
            "sustained overload should be counted, got {snapshot:?}"
        );

        release_all(&mut controller, active);
    }

    #[test]
    fn node_recovers_to_healthy_after_overload_subsides() {
        let limits = AdmissionLimits::new(1, 64, 2).retry_after_ms(5);
        let mut controller = AdmissionController::new(limits);
        let active = controller.try_acquire("active", 32).unwrap();

        controller.enqueue("queued-a", 16).unwrap();
        controller.enqueue("queued-b", 16).unwrap();
        assert!(matches!(
            controller.enqueue("overflow", 16),
            Err(AdmissionError::Backpressure {
                reason: AdmissionRejectionReason::QueueFull,
                ..
            })
        ));

        controller.release(active);
        while let Some(permit) = controller.admit_next() {
            assert_bounded(controller.snapshot(), limits);
            controller.release(permit);
        }

        let healthy = controller.try_acquire("healthy", 8).unwrap();
        let snapshot = controller.snapshot();
        assert_eq!(snapshot.in_flight, 1);
        assert_eq!(snapshot.memory_bytes, 8);
        assert_eq!(snapshot.queue_depth, 0);
        assert_eq!(snapshot.rejected_total, 1);
        controller.release(healthy);

        let drained = controller.snapshot();
        assert_eq!(drained.in_flight, 0);
        assert_eq!(drained.memory_bytes, 0);
        assert_eq!(drained.queue_depth, 0);
    }

    #[test]
    fn oversized_request_is_rejected_and_counted() {
        let limits = AdmissionLimits::new(4, 128, 4).retry_after_ms(7);
        let mut controller = AdmissionController::new(limits);

        assert_eq!(
            controller.try_acquire("oversized", 129),
            Err(AdmissionError::Backpressure {
                reason: AdmissionRejectionReason::MemoryLimit,
                retry_after_ms: 7
            })
        );

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.in_flight, 0);
        assert_eq!(snapshot.memory_bytes, 0);
        assert_eq!(snapshot.queue_depth, 0);
        assert_eq!(snapshot.rejected_total, 1);
    }

    #[test]
    fn slow_downstream_does_not_grow_in_flight_unbounded() {
        let limits = AdmissionLimits::new(4, 256, 4).retry_after_ms(10);
        let mut controller = AdmissionController::new(limits);
        let mut active = fill_in_flight(&mut controller, limits, 32);

        for index in 0..10_000 {
            let _ = controller.try_acquire(format!("immediate-{index}"), 32);
            let _ = controller.enqueue(format!("queued-{index}"), 32);
            assert_bounded(controller.snapshot(), limits);
        }

        assert_eq!(controller.snapshot().in_flight, limits.max_in_flight);
        assert_eq!(controller.snapshot().queue_depth, limits.max_queue_depth);
        assert!(controller.snapshot().rejected_total > 0);

        release_all(&mut controller, active.drain(..));
    }
}

fn fill_in_flight(
    controller: &mut AdmissionController,
    limits: AdmissionLimits,
    bytes: usize,
) -> Vec<AdmissionPermit> {
    (0..limits.max_in_flight)
        .map(|index| {
            controller
                .try_acquire(format!("active-{index}"), bytes)
                .expect("initial capacity admits")
        })
        .collect()
}

fn release_all(
    controller: &mut AdmissionController,
    permits: impl IntoIterator<Item = AdmissionPermit>,
) {
    for permit in permits {
        controller.release(permit);
    }
}

fn assert_bounded(snapshot: AdmissionSnapshot, limits: AdmissionLimits) {
    assert!(
        snapshot.in_flight <= limits.max_in_flight,
        "in_flight exceeded limit: {snapshot:?}"
    );
    assert!(
        snapshot.memory_bytes <= limits.max_memory_bytes,
        "memory exceeded limit: {snapshot:?}"
    );
    assert!(
        snapshot.queue_depth <= limits.max_queue_depth,
        "queue exceeded limit: {snapshot:?}"
    );
}
