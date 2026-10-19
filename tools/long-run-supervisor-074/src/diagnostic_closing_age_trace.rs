//! Bounded copies of the original age arguments. No clock or admission source.
use std::cell::RefCell;
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
struct Fact {
    issued_ns: u64,
    now_ns: u64,
    age_ns: Option<u64>,
    reason: &'static str,
    file: &'static str,
    line: u32,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
struct Site {
    calls: u64,
    first: Option<Fact>,
    last: Option<Fact>,
    first_refusal: Option<Fact>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub(crate) struct Report {
    // Bounded order: custody-final, causal-window, other (units only).
    sites: [Site; 3],
}
thread_local! { static FRAME: RefCell<Option<Report>> = const { RefCell::new(None) }; }
struct Restore(Option<Report>);
impl Drop for Restore {
    fn drop(&mut self) {
        let report = FRAME.replace(self.0.take());
        if std::thread::panicking() {
            use std::io::Write;
            let _ = writeln!(std::io::stderr().lock(), "HC074-CLOSING-AGE {report:?}");
        }
    }
}
pub(crate) fn capture<T>(call: impl FnOnce() -> T) -> (T, Report) {
    let _restore = Restore(FRAME.replace(Some(Report::default())));
    let value = call();
    (
        value,
        FRAME.with_borrow(|frame| frame.as_ref().unwrap().clone()),
    )
}
#[track_caller]
pub(crate) fn observe(issued: u64, now: u64) {
    let location = std::panic::Location::caller();
    FRAME.with_borrow_mut(|frame| {
        let Some(report) = frame else { return };
        let file = location.file();
        let index = if file.ends_with("diagnostic_original_custody_join.rs") {
            0
        } else if file.ends_with("diagnostic_collector_causal.rs") {
            1
        } else {
            2
        };
        let reason = if issued == 0 {
            "zero-issued"
        } else if now < issued {
            "backward-boottime"
        } else if now - issued >= crate::diagnostic_lease::CONTROLLER_LOSS_SECONDS * 1_000_000_000 {
            "expired"
        } else {
            "within-window"
        };
        let fact = Fact {
            issued_ns: issued,
            now_ns: now,
            age_ns: now.checked_sub(issued),
            reason,
            file,
            line: location.line(),
        };
        let site = &mut report.sites[index];
        site.calls += 1;
        site.first.get_or_insert_with(|| fact.clone());
        if reason != "within-window" {
            site.first_refusal.get_or_insert_with(|| fact.clone());
        }
        site.last = Some(fact);
    });
}
#[test]
fn closing_age_boundary_does_not_replace_original_decision() {
    for (issued, now, expected) in [
        (0, 1, false),
        (5, 4, false),
        (5, 5, true),
        (5, 10_000_000_004, true),
        (5, 10_000_000_005, false),
        (5, u64::MAX, false),
    ] {
        let (result, report) = capture(|| super::super::age(issued, now));
        assert_eq!(result.is_ok(), expected);
        assert_eq!(report.sites[2].calls, 1);
        assert_eq!(report.sites[2].first_refusal.is_none(), expected);
        let last = report.sites[2].last.as_ref().unwrap();
        assert_eq!((last.issued_ns, last.now_ns), (issued, now));
        assert!(last.file.ends_with("diagnostic_closing_age_trace.rs"));
    }
}
#[test]
fn closing_age_storage_is_bounded_and_restores_nested_unwind_and_threads() {
    observe(0, 0);
    assert_eq!(capture(|| {}).1, Report::default());
    let (_, report) = capture(|| {
        for now in 1..=10_000 {
            super::super::age(1, now).unwrap();
        }
        assert_eq!(capture(|| super::super::age(1, 0)).1.sites[2].calls, 1);
        assert!(
            std::panic::catch_unwind(|| capture(|| panic!("closing observer canary"))).is_err()
        );
        assert_eq!(
            std::thread::spawn(|| capture(|| {}).1).join().unwrap(),
            Report::default()
        );
        super::super::age(1, 10_000_000_001).unwrap_err();
    });
    assert_eq!(report.sites[2].calls, 10_001);
    assert_eq!(report.sites[2].first.as_ref().unwrap().now_ns, 1);
    assert_eq!(report.sites[2].last, report.sites[2].first_refusal);
    assert!(FRAME.with_borrow(|frame| frame.is_none()));
}
