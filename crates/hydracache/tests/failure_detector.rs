use std::time::Duration;

use hydracache::{
    liveness_allows_ownership_change, liveness_allows_repair_or_handoff, Liveness,
    PhiAccrualConfig, PhiAccrualDetector,
};

fn detector(threshold: f64, interval: Duration) -> PhiAccrualDetector {
    PhiAccrualDetector::with_config(PhiAccrualConfig::new(10, threshold, interval))
}

#[test]
fn failure_detector_steady_heartbeats_keep_phi_low() {
    let mut detector = detector(8.0, Duration::from_millis(100));
    for now in [0, 100, 200, 300] {
        detector.heartbeat(now);
    }

    assert!(detector.phi(350) < 1.0);
    assert!(detector.is_available(350));
}

#[test]
fn failure_detector_missed_heartbeats_raise_phi_past_threshold() {
    let mut detector = detector(4.0, Duration::from_millis(100));
    for now in [0, 100, 200, 300] {
        detector.heartbeat(now);
    }

    let liveness = detector.liveness(900);

    assert!(matches!(liveness, Liveness::Suspect { .. }));
    assert!(!detector.is_available(900));
}

#[test]
fn failure_detector_adapts_to_slower_but_regular_links() {
    let mut detector = detector(3.0, Duration::from_millis(1000));
    for now in [0, 1000, 2000, 3000] {
        detector.heartbeat(now);
    }

    assert!(detector.is_available(4500));
    assert!(detector.phi(4500) < 3.0);
}

#[test]
fn failure_detector_flapping_does_not_change_ownership_before_commit_topology() {
    let liveness = Liveness::Suspect { phi: 9.0 };

    assert!(!liveness_allows_ownership_change(liveness, false));
    assert!(liveness_allows_ownership_change(liveness, true));
}

#[test]
fn failure_detector_gates_hint_replay_and_repair() {
    let mut detector = detector(4.0, Duration::from_millis(100));
    for now in [0, 100, 200, 300] {
        detector.heartbeat(now);
    }

    assert!(liveness_allows_repair_or_handoff(&detector, 350));
    assert!(!liveness_allows_repair_or_handoff(&detector, 900));
}

#[test]
fn failure_detector_false_suspect_metric_is_counted() {
    let mut detector = PhiAccrualDetector::new();

    detector.record_false_suspect();

    assert_eq!(detector.metrics(0).false_suspect_total, 1);
}
