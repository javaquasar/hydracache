//! Exact local observations and bounded-parser substitutions only.
#[macro_export]
macro_rules! hc074_leaf_requests_view {
    ($path:expr, $source:expr) => {{
        let path: &str = $path;
        let mut raw: String = $source.to_owned();
        let hook = "\n#[path = \"../tests/support/validation_leaf_requests.rs\"]\nmod validation_leaf_requests;\n";
        let gate = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/support/validation_leaf_requests_gate.txt")).replace('\r', "");
        for (file, exact, marker) in [
            ("tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs", hook, "validation_leaf_requests"),
            ("docs/testing/gated-test-registry.toml", gate.as_str(), "# BEGIN VALIDATION LEAF REQUEST GATE"),
        ] {
            if path == file && raw.contains(marker) {
                assert_eq!(raw.matches(exact).count(), 1, "exact leaf request test hook");
                raw = raw.replacen(exact, "", 1);
            }
        }
        raw
    }};
}
#[macro_export]
macro_rules! hc074_round_cost_view {
    ($path:expr, $source:expr) => {{
        let path: &str = $path;
        let mut raw: String = $crate::hc074_leaf_requests_view!(path, $source);
        for (file, hooks) in [
            ("tools/long-run-supervisor-074/src/diagnostic_validation_trace.rs", &[
                "#[cfg(target_os = \"linux\")]\n#[path = \"../tests/support/round_cost.rs\"]\npub(crate) mod round_cost;\n\n",
                "#[cfg(target_os = \"linux\")]\npub(crate) fn numeric_snapshot() -> Option<Report> {\n    FRAME.with_borrow(|frame| frame.as_ref().map(|frame| frame.borrow().report.clone()))\n}\n\n",
            ][..]),
            ("tools/long-run-supervisor-074/src/diagnostic_kernel_walk_trace.rs", &[
                "pub(crate) fn numeric_snapshot() -> Option<Report> {\n    FRAME.with_borrow(|frame| frame.as_ref().map(|frame| frame.borrow().report.clone()))\n}\n",
            ][..]),
            ("tools/long-run-supervisor-074/src/diagnostic_live_walk_trace.rs", &[
                "pub(crate) fn numeric_snapshot() -> Option<[Stats; 6]> {\n    FRAME.with_borrow(|frame| frame.as_ref().map(|frame| frame.borrow().stats))\n}\n",
            ][..]),
            ("tools/long-run-supervisor-074/src/diagnostic_validation_frame.rs", &[
                "            #[cfg(test)]\n            let step_scope =\n                crate::diagnostic_ipc::validation_trace::round_cost::span(step as usize);\n",
                "            #[cfg(test)]\n            step_scope.complete();\n",
            ][..]),
            ("tools/long-run-supervisor-074/tests/support/consuming_held_cost.rs", &[
                "    let round_scope = trace::round_cost::Capture::new();\n",
                "    let round_steps = round_scope.finish();\n",
                "        \"round_steps\":round_steps,\n",
            ][..]),
        ] {
            if path == file && (raw.contains("round_cost::") || raw.contains("fn numeric_snapshot()")) {
                for hook in hooks {
                    assert_eq!(raw.matches(hook).count(), 1, "exact round observer hook");
                    raw = raw.replacen(hook, "", 1);
                }
            }
        }
        raw
    }};
}
#[macro_export]
macro_rules! hc074_starting_effects_view {
    ($path:expr, $source:expr) => {{
        let path: &str = $path;
        let mut raw: String = $crate::hc074_round_cost_view!(path, $source);
        let active = raw.contains("::mark(\"st-");
        for (file, indent, names) in [
            ("tools/long-run-supervisor-074/src/diagnostic_original_output_join.rs", 8, &["st-enter", "st-encoded", "st-begun", "st-final", "st-finaled", "st-acked"][..]),
            ("tools/long-run-supervisor-074/src/diagnostic_original_output_join.rs", 12, &["st-open", "st-opened", "st-stepped", "st-checked", "st-closed"][..]),
            ("tools/long-run-supervisor-074/src/diagnostic_collector_starting_storage.rs", 8, &["st-store", "st-inspected", "st-effect", "st-effected"][..]),
        ] {
            if path == file && active {
                for name in names {
                    let exact = format!("{space}#[cfg(test)]\n{space}crate::diagnostic_ipc::validation_trace::custody_boundaries::mark(\"{name}\");\n", space=" ".repeat(indent));
                    assert_eq!(raw.matches(&exact).count(), 1, "exact Starting cost mark: {name}");
                    raw = raw.replacen(&exact, "", 1);
                }
            }
        }
        raw
    }};
}
#[macro_export]
macro_rules! hc074_remaining_preparation_view {
    ($path:expr, $source:expr) => {{
        let path: &str = $path;
        let mut raw: String = $crate::hc074_starting_effects_view!(path, $source);
        for (file, hook) in [
            ("tools/long-run-supervisor-074/src/diagnostic_kernel_admission_tests.rs", "#[path = \"../tests/support/projection_cost.rs\"]\npub(super) mod projection_cost;\n\n"),
            ("tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs", "\n#[path = \"../tests/support/projection_alloc.rs\"]\nmod projection_fields;\n"),
        ] {
            if path == file && raw.contains(hook) {
                assert_eq!(raw.matches(hook).count(), 1, "exact projection test hook");
                raw = raw.replacen(hook, "", 1);
            }
        }
        if path == "tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs"
            && raw.contains("    let scalar = |prefix:")
        {
            let before = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/long-run-supervisor-074/tests/support/projection_reference.rs")).replace('\r', "");
            let after = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/long-run-supervisor-074/tests/support/projection_candidate.rs")).replace('\r', "");
            let function = |text: &str| format!("fn projection(bytes: &[u8], credentials: &libc::ucred) -> Result<Projection, KernelError> {{{}", text.split_once(") -> Result<Projection, KernelError> {").unwrap().1.trim_end());
            let after = function(&after);
            assert_eq!(raw.matches(&after).count(), 1, "exact bounded projection parser");
            let old_body = before.split_once(") -> Result<Projection, KernelError> {").unwrap().1;
            let before = format!("fn projection(bytes: &[u8], credentials: &libc::ucred) -> Result<Projection, KernelError> {{{}", old_body.trim_end());
            raw = raw.replacen(&after, &before, 1);
        }
        let has_marks = raw.contains("::mark(\"outputs-") || raw.contains("::mark(\"owned-");
        for (file, indent, names) in [
            ("tools/long-run-supervisor-074/src/diagnostic_original_output_join.rs", 12, &["outputs-enter", "outputs-starting", "outputs-pipes", "outputs-sinks"][..]),
            ("tools/long-run-supervisor-074/src/diagnostic_owned_root_manager.rs", 8, &["owned-enter"][..]),
            ("tools/long-run-supervisor-074/src/diagnostic_owned_root_manager.rs", 16, &["owned-context"][..]),
            ("tools/long-run-supervisor-074/src/diagnostic_original_root_guardians.rs", 4, &["owned-opening", "owned-controls", "owned-closing"][..]),
        ] {
            if path == file && has_marks {
                for name in names {
                    let exact = format!("{space}#[cfg(test)]\n{space}crate::diagnostic_ipc::validation_trace::custody_boundaries::mark(\"{name}\");\n", space=" ".repeat(indent));
                    assert_eq!(raw.matches(&exact).count(), 1, "exact remaining preparation mark: {name}");
                    raw = raw.replacen(&exact, "", 1);
                }
            }
        }
        if path == "tools/long-run-supervisor-074/src/diagnostic_original_output_join.rs"
            && raw.contains("let result = held.prepare(|at| checkpoint(OutputJoinStage::Sinks(at)));")
        {
            let exact = "            let result = held.prepare(|at| checkpoint(OutputJoinStage::Sinks(at)));\n            result";
            assert_eq!(raw.matches(exact).count(), 1, "exact output result observation");
            raw = raw.replacen(exact, "            held.prepare(|at| checkpoint(OutputJoinStage::Sinks(at)))", 1);
        }
        raw
    }};
}
