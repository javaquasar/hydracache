use hydracache::{AdmissionController, AdmissionError, AdmissionLimits, AdmissionRejectionReason};

#[test]
fn admission_overload_is_shed_with_retryable_backpressure_not_unbounded_queue() {
    let limits = AdmissionLimits::new(1, 100, 1).retry_after_ms(10);
    let mut admission = AdmissionController::new(limits);
    let _active = admission.try_acquire("active", 50).unwrap();

    admission.enqueue("queued", 10).unwrap();
    let rejected = admission.enqueue("overflow", 10).unwrap_err();

    assert_eq!(
        rejected,
        AdmissionError::Backpressure {
            reason: AdmissionRejectionReason::QueueFull,
            retry_after_ms: 20
        }
    );
    assert_eq!(admission.snapshot().queue_depth, 1);
    assert_eq!(admission.snapshot().rejected_total, 1);
}

#[test]
fn admission_count_and_memory_permits_are_released() {
    let limits = AdmissionLimits::new(2, 100, 0).retry_after_ms(5);
    let mut admission = AdmissionController::new(limits);
    let first = admission.try_acquire("first", 70).unwrap();

    assert_eq!(
        admission.try_acquire("too-large", 40),
        Err(AdmissionError::Backpressure {
            reason: AdmissionRejectionReason::MemoryLimit,
            retry_after_ms: 5
        })
    );
    admission.release(first);
    let second = admission.try_acquire("second", 40).unwrap();

    assert_eq!(second.request_id, "second");
    assert_eq!(admission.snapshot().memory_bytes, 40);
}

#[test]
fn admission_fifo_backlog_promotes_oldest_first() {
    let limits = AdmissionLimits::new(1, 100, 2);
    let mut admission = AdmissionController::new(limits);

    let first = admission.try_acquire("active", 10).unwrap();
    admission.enqueue("queued-a", 10).unwrap();
    admission.enqueue("queued-b", 10).unwrap();
    assert!(admission.admit_next().is_none());

    admission.release(first);
    let next = admission.admit_next().unwrap();

    assert_eq!(next.request_id, "queued-a");
    assert_eq!(admission.snapshot().queue_depth, 1);
}

#[test]
fn admission_rejects_single_request_larger_than_memory_budget() {
    let limits = AdmissionLimits::new(1, 10, 1);
    let mut admission = AdmissionController::new(limits);

    assert_eq!(
        admission.try_acquire("huge", 11),
        Err(AdmissionError::Backpressure {
            reason: AdmissionRejectionReason::MemoryLimit,
            retry_after_ms: 25
        })
    );
    assert_eq!(admission.snapshot().in_flight, 0);
}
