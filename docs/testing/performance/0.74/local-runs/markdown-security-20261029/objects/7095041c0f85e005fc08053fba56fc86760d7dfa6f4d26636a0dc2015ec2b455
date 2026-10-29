//! Test-only copies of existing kernel observations. Never a guard or clock source.
use super::KernelTimePoint;
use std::cell::RefCell;

#[path = "diagnostic_closing_age_trace.rs"]
pub(crate) mod closing;

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub(crate) struct Report {
    observations: u64,
    first: Option<Facts>,
    last: Option<Facts>,
    first_refusal: Option<Facts>,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
struct Facts {
    site: &'static str,
    issued_ns: u64,
    now_ns: u64,
    issued_boot: String,
    now_boot: String,
    age_ns: Option<u64>,
    diagnostic_reason: &'static str,
}
thread_local! {
    static FRAME: RefCell<Option<Report>> = const { RefCell::new(None) };
}
struct Restore(Option<Report>);
impl Drop for Restore {
    fn drop(&mut self) {
        let report = FRAME.replace(self.0.take());
        if std::thread::panicking() {
            use std::io::Write;
            let _ = writeln!(std::io::stderr().lock(), "HC074-CAUSAL-FACTS {report:?}");
        }
    }
}
pub(crate) fn capture<T>(run: impl FnOnce() -> T) -> (T, Report) {
    let _restore = Restore(FRAME.replace(Some(Report::default())));
    let value = run();
    let report = FRAME.with_borrow(|frame| frame.as_ref().unwrap().clone());
    (value, report)
}
pub(crate) fn observe(site: &'static str, issued: &KernelTimePoint, now: &KernelTimePoint) {
    record(
        site,
        issued.boottime_ns(),
        now.boottime_ns(),
        issued.boot_id(),
        now.boot_id(),
    );
}
fn record(site: &'static str, issued: u64, now: u64, issued_boot: &str, now_boot: &str) {
    FRAME.with_borrow_mut(|frame| {
        let Some(report) = frame else { return };
        // Diagnostic classification only; the original caller still decides independently.
        let reason = if issued_boot != now_boot {
            "boot-mismatch"
        } else if issued == 0 {
            "zero-issued"
        } else if now < issued {
            "backward-boottime"
        } else if now - issued >= crate::diagnostic_lease::CONTROLLER_LOSS_SECONDS * 1_000_000_000 {
            "expired"
        } else {
            "within-window"
        };
        let facts = Facts {
            site,
            issued_ns: issued,
            now_ns: now,
            issued_boot: issued_boot.to_owned(),
            now_boot: now_boot.to_owned(),
            age_ns: now.checked_sub(issued),
            diagnostic_reason: reason,
        };
        report.observations += 1;
        report.first.get_or_insert_with(|| facts.clone());
        if reason != "within-window" {
            report.first_refusal.get_or_insert_with(|| facts.clone());
        }
        report.last = Some(facts);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn causal_facts_boundary_classification_does_not_replace_original_guard() {
        for (issued, now, reason) in [
            (0, 1, "zero-issued"),
            (5, 4, "backward-boottime"),
            (5, 5, "within-window"),
            (5, 10_000_000_004, "within-window"),
            (5, 10_000_000_005, "expired"),
            (5, u64::MAX, "expired"),
        ] {
            let (decision, report) = capture(|| {
                record("synthetic-unit-only", issued, now, "boot", "boot");
                super::super::age(issued, now).is_ok()
            });
            assert_eq!(decision, reason == "within-window");
            assert_eq!(report.last.unwrap().diagnostic_reason, reason);
            assert_eq!(report.first_refusal.is_some(), !decision);
        }
        let (_, report) = capture(|| record("synthetic-unit-only", 5, 6, "a", "b"));
        assert_eq!(
            report.first_refusal.unwrap().diagnostic_reason,
            "boot-mismatch"
        );
    }
    #[test]
    fn causal_facts_first_refusal_and_nested_unwind_are_isolated() {
        record("inactive", 0, 0, "a", "b");
        assert_eq!(capture(|| {}).1, Report::default());
        let (_, report) = capture(|| {
            record("first", 1, 2, "a", "a");
            record("failure", 1, 0, "a", "a");
            let (_, inner) = capture(|| record("nested", 0, 0, "a", "b"));
            assert_eq!(inner.observations, 1);
            assert!(
                std::panic::catch_unwind(|| capture(|| panic!("trace restoration canary")))
                    .is_err()
            );
            assert_eq!(
                std::thread::spawn(|| capture(|| {}).1).join().unwrap(),
                Report::default()
            );
            record("last", 1, 3, "a", "a");
        });
        assert_eq!(report.observations, 3);
        assert_eq!(report.first.unwrap().site, "first");
        assert_eq!(report.first_refusal.unwrap().site, "failure");
        assert_eq!(report.last.unwrap().site, "last");
        assert!(FRAME.with_borrow(|f| f.is_none()));
    }
}
