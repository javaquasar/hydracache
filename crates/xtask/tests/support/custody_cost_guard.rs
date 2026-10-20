//! Exact test-only observation hooks, never a validator-body exclusion.
pub(crate) const HOOKS: [(&str, &str); 4] = [
    ("tools/long-run-supervisor-074/src/diagnostic_validation_trace.rs", "\n#[cfg(target_os = \"linux\")]\n#[path = \"../tests/support/custody_boundaries.rs\"]\npub(crate) mod custody_boundaries;\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_original_enrollment_frame_build.rs", "            #[cfg(test)]\n            crate::diagnostic_ipc::validation_trace::custody_boundaries::mark(\n                \"before-first-custody\",\n            );\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_original_enrollment_frame_build.rs", "            #[cfg(test)]\n            crate::diagnostic_ipc::validation_trace::custody_boundaries::mark(\"closing-enter\");\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_original_enrollment_frame_build.rs", "            #[cfg(test)]\n            crate::diagnostic_ipc::validation_trace::custody_boundaries::mark(\"closing-exit\");\n"),
];
pub(crate) const OBSERVER: &str =
    "tools/long-run-supervisor-074/tests/support/consuming_held_cost.rs";
pub(crate) const NEW: &str = "    let ((((result, auth), live), kernel), boundaries) = trace::custody_boundaries::capture(|| {\n        trace::kernel_walk::capture(|| {\n            trace::live_walk::capture(|| {\n                trace::capture(|| std::panic::catch_unwind(std::panic::AssertUnwindSafe(call)))\n            })\n        })\n    });\n";
const OLD: &str = "    let (((result, auth), live), kernel) = trace::kernel_walk::capture(|| {\n        trace::live_walk::capture(|| {\n            trace::capture(|| std::panic::catch_unwind(std::panic::AssertUnwindSafe(call)))\n        })\n    });\n";
pub(crate) fn normalize(path: &str, raw: &str) -> String {
    let mut raw = raw.to_owned();
    for (file, exact, old) in HOOKS.into_iter().map(|(f, s)| (f, s, "")).chain([
        (OBSERVER, NEW, OLD),
        (
            OBSERVER,
            "\"kernel\":kernel,\"live\":live,\"boundaries\":boundaries,",
            "\"kernel\":kernel,\"live\":live,",
        ),
    ]) {
        if path == file {
            assert_eq!(
                raw.matches(exact).count(),
                1,
                "exact custody observation hook"
            );
            raw = raw.replacen(exact, old, 1);
        }
    }
    raw
}
