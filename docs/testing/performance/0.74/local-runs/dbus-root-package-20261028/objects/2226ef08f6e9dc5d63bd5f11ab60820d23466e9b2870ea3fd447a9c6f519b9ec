use super::*;

fn fake() -> GuardPair {
    let origin = Origin::current();
    let make = || {
        let file = || File::open("/dev/null").unwrap();
        Watchdog {
            pid: origin.pid,
            pinned: file(),
            peer: file(),
            alarm: file(),
            origin,
            pending: true,
            consumed: false,
            armed: false,
            refused: false,
        }
    };
    GuardPair {
        guardians: [make(), make()],
        origin,
        consumed: false,
        armed: false,
        refused: false,
    }
}
#[test]
fn redundant_guardian_retry_and_missing_readiness_are_sticky() {
    let target = File::open("/dev/null").unwrap();
    for state in [
        (true, false, false),
        (false, false, true),
        (true, true, true),
    ] {
        let mut pair = fake();
        (pair.consumed, pair.armed, pair.refused) = state;
        assert!(pair.enroll_once(&target).is_err());
        assert!(pair.refused);
        assert!(pair.guardians.iter().all(|g| !g.consumed));
        assert!(pair.ready().is_err());
    }
    let mut pair = fake();
    assert!(pair.ready().is_err());
    assert!(pair.enroll_once(&target).is_err());
    assert_eq!(pair.test_state(), (false, false, true));
}
#[test]
fn redundant_guardian_thread_drift_refuses_before_first_send() {
    let mut pair = fake();
    std::thread::spawn(move || {
        let target = File::open("/dev/null").unwrap();
        assert!(pair.enroll_once(&target).is_err());
        assert!(pair.guardians.iter().all(|g| !g.consumed));
        assert_eq!(pair.test_state(), (false, false, true));
    })
    .join()
    .unwrap();
}
#[test]
fn redundant_guardian_closing_check_never_accepts_readable_surrogates() {
    let mut pair = fake();
    pair.consumed = true;
    pair.armed = true;
    for g in &mut pair.guardians {
        g.armed = true;
    }
    assert!(pair.ready().is_err());
    assert!(pair.refused);
}
#[test]
fn redundant_guardian_invalid_fence_refuses_before_any_child() {
    let file = File::open("/dev/null").unwrap();
    assert!(GuardPair::spawn(file, 1, false).is_err());
}
