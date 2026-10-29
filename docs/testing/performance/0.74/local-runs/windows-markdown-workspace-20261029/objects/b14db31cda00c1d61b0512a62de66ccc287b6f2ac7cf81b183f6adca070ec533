use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use get_owner_scheduled_controls_074::scheduled::{project, run, Config, Outcome, Sample};
use get_owner_scheduled_controls_074::target::{Target, TargetError, TargetOutcome, TargetRequest};

struct Guard(Arc<AtomicUsize>);
impl Drop for Guard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}
struct Fixture {
    delay: Duration,
    active: Arc<AtomicUsize>,
    panic: bool,
    outcome: TargetOutcome,
}
#[async_trait]
impl Target for Fixture {
    async fn reset(&self) -> Result<String, TargetError> {
        Ok("fixture".to_owned())
    }
    async fn state_digest(&self) -> Result<String, TargetError> {
        Ok("fixture".to_owned())
    }
    async fn execute(&self, _: TargetRequest) -> TargetOutcome {
        self.active.fetch_add(1, Ordering::SeqCst);
        let _guard = Guard(Arc::clone(&self.active));
        assert!(!self.panic, "injected request panic");
        tokio::time::sleep(self.delay).await;
        self.outcome
    }
}
fn fixture(delay_ms: u64) -> Arc<Fixture> {
    Arc::new(Fixture {
        delay: Duration::from_millis(delay_ms),
        active: Arc::new(AtomicUsize::new(0)),
        panic: false,
        outcome: TargetOutcome::Success,
    })
}
fn config() -> Config {
    Config {
        operations: 16,
        offered_rate_per_second: 1000,
        concurrency: 1,
        maximum_queued: 16,
        operation_timeout_ns: 1_000_000_000,
        drain_timeout_ns: 1_000_000_000,
        slo_ns: 2_000_000,
        highest_trackable_ns: 1_000_000_000,
    }
}

#[tokio::test(start_paused = true)]
async fn stall_is_visible_from_original_schedule_not_only_service_time() {
    let target = fixture(10);
    let c = config();
    let result = run(Arc::clone(&target), &c).await.unwrap();
    assert_eq!(result.offered, 16);
    assert_eq!(result.target_completed, 16);
    assert_eq!(result.successes, 16);
    assert_eq!(result.samples[15].scheduled_ns, 15_000_000);
    assert!(
        result.samples[15].scheduled_latency_ns.unwrap()
            > result.samples[15].service_latency_ns.unwrap() * 10
    );
    assert!(result.scheduled_response_latency.p99_ns > result.service_response_latency.p99_ns);
    assert_eq!(result.late_successes, 16);
    assert_eq!(result.good_successes, 0);
    assert_eq!(result.good_fraction_of_all_offers, 0.0);
    assert_eq!(result.goodput_operations_per_second, 0.0);
    assert_eq!(target.active.load(Ordering::SeqCst), 0);
    assert!(result.owned_tasks_drained);
}

#[tokio::test(start_paused = true)]
async fn bounded_overload_preserves_every_offer_and_never_moves_schedule() {
    let mut c = config();
    c.maximum_queued = 0;
    let result = run(fixture(100), &c).await.unwrap();
    assert_eq!(result.target_started, 1);
    assert_eq!(result.admission_rejections, 15);
    assert_eq!(result.pending_high_water, 1);
    assert_eq!(result.samples.len(), 16);
    assert_eq!(result.scheduled_response_latency.samples, 1);
    for (i, sample) in result.samples.iter().enumerate() {
        assert_eq!(sample.scheduled_ns, i as u64 * 1_000_000);
    }
    assert_eq!(result.samples[15].outcome, Outcome::AdmissionRejected);
    assert!(result.samples[15].started_ns.is_none());
}

#[tokio::test(start_paused = true)]
async fn timeout_includes_queue_delay_and_does_not_give_each_request_a_fresh_budget() {
    let mut c = config();
    c.operations = 8;
    c.operation_timeout_ns = 5_000_000;
    let result = run(fixture(100), &c).await.unwrap();
    assert_eq!(result.successes, 0);
    assert_eq!(result.timeouts + result.queue_timeouts, 8);
    assert_eq!(result.target_started, result.timeouts);
    assert_eq!(result.target_completed, result.timeouts);
    assert_eq!(result.scheduled_response_latency.samples, result.timeouts);
    assert_eq!(result.incomplete, 0);
}

#[tokio::test(start_paused = true)]
async fn drain_timeout_retains_censored_samples_and_joins_cancelled_owners() {
    let target = fixture(1000);
    let mut c = config();
    c.drain_timeout_ns = 1_000_000;
    let result = run(Arc::clone(&target), &c).await.unwrap();
    assert_eq!(result.incomplete, 16);
    assert_eq!(result.target_started, 1);
    assert_eq!(result.target_completed, 0);
    assert_eq!(result.scheduled_response_latency.samples, 0);
    assert!(result.scheduled_response_latency.p99_ns.is_none());
    assert!(result
        .samples
        .iter()
        .all(|sample| sample.incomplete_lower_bound_ns.is_some()
            && sample.scheduled_latency_ns.is_none()));
    assert_eq!(target.active.load(Ordering::SeqCst), 0);
}

#[tokio::test(start_paused = true)]
async fn panic_is_loud_and_owned_requests_are_drained_before_error() {
    let mut target = fixture(1);
    Arc::get_mut(&mut target).unwrap().panic = true;
    assert!(run(Arc::clone(&target), &config())
        .await
        .unwrap_err()
        .contains("owned request task failed"));
    assert_eq!(target.active.load(Ordering::SeqCst), 0);
}

#[tokio::test(start_paused = true)]
async fn caller_cancellation_aborts_owned_requests() {
    let target = fixture(1000);
    let owned = Arc::clone(&target);
    let handle = tokio::spawn(async move { run(owned, &config()).await });
    tokio::time::sleep(Duration::from_millis(5)).await;
    assert_eq!(target.active.load(Ordering::SeqCst), 1);
    handle.abort();
    assert!(handle.await.unwrap_err().is_cancelled());
    tokio::task::yield_now().await;
    assert_eq!(target.active.load(Ordering::SeqCst), 0);
}

#[tokio::test(start_paused = true)]
async fn result_classification_and_all_offer_denominator_are_not_success_only() {
    for outcome in [
        TargetOutcome::Error,
        TargetOutcome::Rejected,
        TargetOutcome::Timeout,
    ] {
        let mut target = fixture(0);
        Arc::get_mut(&mut target).unwrap().outcome = outcome;
        let mut c = config();
        c.concurrency = 8;
        let result = run(target, &c).await.unwrap();
        assert_eq!(result.successes, 0);
        assert_eq!(result.good_successes, 0);
        assert_eq!(
            result.errors + result.target_rejections + result.timeouts,
            16
        );
        assert!(!result.promotable);
        assert!(!result.product_performance_claim);
    }
}

#[test]
fn bounds_reject_oversized_local_contract() {
    for change in 0..5 {
        let mut c = config();
        match change {
            0 => c.operations = 10_001,
            1 => c.maximum_queued = 1025,
            2 => c.offered_rate_per_second = 0,
            3 => c.concurrency = 2,
            _ => {
                c.offered_rate_per_second = 1;
                c.operations = 17;
            }
        }
        assert!(c.validate().is_err());
    }
}

#[test]
fn projection_rejects_fabricated_samples_and_cleanup_claims() {
    let mut c = config();
    c.operations = 1;
    let valid = Sample {
        sequence: 0,
        scheduled_ns: 0,
        started_ns: Some(1),
        terminal_ns: 10,
        outcome: Outcome::Success,
        scheduled_latency_ns: Some(10),
        service_latency_ns: Some(9),
        incomplete_lower_bound_ns: None,
    };
    for change in 0..9 {
        let mut sample = valid.clone();
        match change {
            0 => sample.sequence = 1,
            1 => sample.scheduled_ns = 1,
            2 => sample.started_ns = None,
            3 => sample.service_latency_ns = Some(10),
            4 => sample.scheduled_latency_ns = Some(9),
            5 => sample.incomplete_lower_bound_ns = Some(10),
            6 => sample.terminal_ns = 21,
            7 => sample.outcome = Outcome::AdmissionRejected,
            _ => sample.outcome = Outcome::Incomplete,
        }
        assert!(project(&c, vec![sample], 20, 1, true).is_err());
    }
    assert!(project(&c, vec![], 20, 0, true).is_err());
    assert!(project(&c, vec![valid.clone()], 20, 18, true).is_err());
    assert!(project(&c, vec![valid], 20, 1, false).is_err());
    let too_early = Sample {
        sequence: 0,
        scheduled_ns: 0,
        started_ns: None,
        terminal_ns: 10,
        outcome: Outcome::QueueTimeout,
        scheduled_latency_ns: None,
        service_latency_ns: None,
        incomplete_lower_bound_ns: None,
    };
    assert!(project(&c, vec![too_early], 20, 1, true)
        .unwrap_err()
        .contains("before original deadline"));
}

#[test]
fn histogram_overflow_is_loud_and_all_offers_conserve_across_outcomes() {
    let mut c = config();
    c.operations = 7;
    c.highest_trackable_ns = 2_000_000;
    c.operation_timeout_ns = 3_000_000;
    let outcomes = [
        Outcome::Success,
        Outcome::Error,
        Outcome::Timeout,
        Outcome::TargetRejected,
        Outcome::AdmissionRejected,
        Outcome::QueueTimeout,
        Outcome::Incomplete,
    ];
    let samples = outcomes
        .into_iter()
        .enumerate()
        .map(|(i, outcome)| {
            let scheduled_ns = i as u64 * 1_000_000;
            let completed = i < 4;
            Sample {
                sequence: i as u64,
                scheduled_ns,
                started_ns: completed.then_some(scheduled_ns),
                terminal_ns: scheduled_ns + 3_000_000,
                outcome,
                scheduled_latency_ns: completed.then_some(3_000_000),
                service_latency_ns: completed.then_some(3_000_000),
                incomplete_lower_bound_ns: (outcome == Outcome::Incomplete).then_some(3_000_000),
            }
        })
        .collect();
    let result = project(&c, samples, 10_000_000, 7, true).unwrap();
    assert_eq!(result.scheduled_response_latency.overflow_count, 4);
    assert_eq!(result.service_response_latency.overflow_count, 4);
    assert_eq!(result.target_completed, 4);
    assert_eq!(
        result.offered,
        result.successes
            + result.errors
            + result.timeouts
            + result.target_rejections
            + result.admission_rejections
            + result.queue_timeouts
            + result.incomplete
    );
    assert_eq!(result.good_fraction_of_all_offers, 0.0);
}

#[tokio::test(start_paused = true)]
async fn logical_concurrency_grid_stays_bounded_and_keeps_fixed_schedule() {
    for concurrency in [1, 8, 32, 128] {
        let mut c = config();
        c.operations = 128;
        c.concurrency = concurrency;
        c.maximum_queued = 0;
        let result = run(fixture(10), &c).await.unwrap();
        assert!(result.pending_high_water <= concurrency);
        assert_eq!(
            result.offered,
            result.target_completed + result.admission_rejections
        );
        for (sequence, sample) in result.samples.iter().enumerate() {
            assert_eq!(sample.scheduled_ns, sequence as u64 * 1_000_000);
        }
    }
}

#[test]
fn on_time_success_uses_all_offer_fraction_and_full_drain_interval() {
    let mut c = config();
    c.operations = 2;
    let samples = vec![
        Sample {
            sequence: 0,
            scheduled_ns: 0,
            started_ns: Some(0),
            terminal_ns: 1_000_000,
            outcome: Outcome::Success,
            scheduled_latency_ns: Some(1_000_000),
            service_latency_ns: Some(1_000_000),
            incomplete_lower_bound_ns: None,
        },
        Sample {
            sequence: 1,
            scheduled_ns: 1_000_000,
            started_ns: None,
            terminal_ns: 1_000_000,
            outcome: Outcome::AdmissionRejected,
            scheduled_latency_ns: None,
            service_latency_ns: None,
            incomplete_lower_bound_ns: None,
        },
    ];
    let result = project(&c, samples, 2_000_000, 1, true).unwrap();
    assert_eq!(result.good_successes, 1);
    assert_eq!(result.good_fraction_of_all_offers, 0.5);
    assert_eq!(result.goodput_operations_per_second, 500.0);
}
