//! Isolated diagnostic controls; frozen source-preservation targets stay unchanged.
#[path = "support/fixed_slice_namespace_guard.rs"]
mod history_namespace;
// The same existing document module, through its original parent chain. Do not
// import it twice: that would duplicate the exported historical macros.
use history_namespace::held_build::launcher::roles::binding::trust::consuming::live::prefix::fresh_status::pid_path::custody_cost::document_scratch as guard;
#[path = "../../../tools/xtask-package-attribution-074/helper_contract.rs"]
mod helper_contract;
#[path = "../../../tools/xtask-package-attribution-074/helper_trace.rs"]
mod helper_trace;
#[path = "../../../tools/normalization-attribution-074/history_probe.rs"]
mod history_profile;
#[path = "../../../tools/normalization-attribution-074/probe.rs"]
mod normalization_profile;
#[path = "../../../tools/authority-runtime-074/gate.rs"]
mod runtime;
#[path = "../../../tools/authority-runtime-074/io.rs"]
mod runtime_io;

#[test]
fn helper_instrumentation_erases_only_exact_observers_and_preserves_twenty_cases() {
    let raw = helper_contract::instrument(helper_contract::BASELINE);
    assert!(raw.starts_with(helper_contract::BASELINE.lines().next().unwrap()));
    let current = std::fs::read_to_string(runtime::root().join(helper_contract::SOURCE))
        .unwrap()
        .replace("\r\n", "\n");
    assert_eq!(current, helper_contract::BASELINE);
    assert_eq!(helper_contract::erase(&raw), current);
    assert_eq!(helper_contract::erase(&raw), helper_contract::BASELINE);
    assert_eq!(raw.matches("\n#[test]\n").count(), 20);
    assert!(helper_contract::SOURCE.ends_with("diagnostic_artifact_fixture_074.rs"));
    for bad in [
        raw.replace(helper_contract::HOOK, ""),
        format!("{raw}{}", helper_contract::HOOK),
        raw.replace("output.status.success()", "!output.status.success()"),
        raw.replace("inventory::project(&raw)", "inventory::project(\"\")"),
        format!("{raw}\nfn added_behavior() {{}}\n"),
    ] {
        assert!(std::panic::catch_unwind(|| helper_contract::erase(&bad)).is_err());
    }
}

#[test]
fn helper_observer_is_default_off_and_does_not_read_fixture_inputs() {
    assert!(std::env::var_os("HC074_PRESERVATION_PROFILE").is_none());
    assert!(helper_trace::read("does-not-exist").is_none());
    assert!(helper_trace::git(&["must-not-spawn"]).is_none());
    assert!(helper_trace::output().is_none());
    helper_trace::finish(helper_trace::decode(), 12, 0);
}

#[test]
fn helper_model_preserves_nested_order_bytes_and_unwind_without_additive_claims() {
    let mut buffer = helper_trace::model::Buffer::new();
    let parent = buffer.begin("git", &["show", "pinned:source"], 1).unwrap();
    let child = buffer.begin("output", &[], 2).unwrap();
    buffer.finish(child, 7, Some((11, 0)), false);
    let decode = buffer.begin("decode", &[], 8).unwrap();
    buffer.finish(decode, 9, None, true);
    buffer.finish(parent, 10, None, true);
    let report = buffer.report("fixture", "one");
    assert_eq!(report["valid"], true);
    assert_eq!(report["started"], 3);
    assert_eq!(report["records"][0]["parent"], parent);
    assert_eq!(report["records"][0]["output_bytes"], 11);
    assert_eq!(report["records"][1]["unwinding"], true);
    assert_eq!(report["records"][2]["returned"], false);
}

#[test]
fn helper_model_refuses_unknown_operations_budgets_reverse_time_and_bad_retirement() {
    for op in ["unknown", "read"] {
        let mut buffer = helper_trace::model::Buffer::new();
        let large = "x".repeat(4097);
        assert!(buffer.begin(op, &[&large], 0).is_err());
        assert_eq!(buffer.report("fixture", "one")["valid"], false);
    }
    for mode in 0..3 {
        let mut buffer = helper_trace::model::Buffer::new();
        let id = buffer.begin("read", &["source"], 5).unwrap();
        buffer.finish(
            if mode == 0 { id + 1 } else { id },
            if mode == 1 { 4 } else { 6 },
            Some((1, 0)),
            false,
        );
        if mode == 2 {
            buffer.finish(id, 7, Some((1, 0)), false);
        }
        assert_eq!(buffer.report("fixture", "one")["valid"], false);
    }
    let mut buffer = helper_trace::model::Buffer::new();
    for tick in 0..helper_trace::model::LIMIT {
        let id = buffer.begin("read", &[], tick).unwrap();
        buffer.finish(id, tick, Some((0, 0)), false);
    }
    assert!(buffer.begin("read", &[], 0).is_err());
    let mut buffer = helper_trace::model::Buffer::new();
    assert!(buffer.begin("read", &[""; 65], 0).is_err());
    let mut buffer = helper_trace::model::Buffer::new();
    for _ in 0..4 {
        buffer.begin("git", &[], 0).unwrap();
    }
    assert!(buffer.begin("git", &[], 0).is_err());
}

#[test]
fn helper_observer_fixture_child() {
    let Some(mode) = std::env::var_os("HC074_HELPER_FIXTURE") else {
        return;
    };
    assert_eq!(
        std::env::var_os("HC074_PRESERVATION_PROFILE").unwrap(),
        "v1"
    );
    if mode == "healthy" {
        let children: Vec<_> = (0..4)
            .map(|index| {
                std::thread::Builder::new()
                    .name(format!("helper-fixture-{index}"))
                    .spawn(|| {
                        let helper = helper_trace::git(&["show", "pinned:fixture"]);
                        helper_trace::finish(helper_trace::output(), 12, 0);
                        helper_trace::finish(helper_trace::decode(), 11, 0);
                        helper_trace::finish(helper, 11, 0);
                        helper_trace::finish(helper_trace::read("fixture"), 11, 0);
                    })
                    .unwrap()
            })
            .collect();
        for child in children {
            child.join().unwrap();
        }
    } else {
        assert_eq!(mode, "panic");
        let result = std::panic::catch_unwind(|| {
            let _helper = helper_trace::git(&["show", "pinned:fixture"]);
            let _output = helper_trace::output();
            panic!("original-helper-fixture-panic");
        });
        let error = result.unwrap_err();
        assert_eq!(
            error.downcast_ref::<&str>(),
            Some(&"original-helper-fixture-panic")
        );
    }
}

#[test]
fn history_prefix_protocol_and_unrelated_input_are_exact() {
    assert_eq!(history_profile::PATHS.len(), 3);
    assert_eq!(history_profile::ITERATIONS, 16);
    assert_eq!(history_profile::order(0), history_profile::OPERATIONS);
    let mut reverse = history_profile::order(1);
    reverse.reverse();
    assert_eq!(reverse, history_profile::order(0));
    for operation in history_profile::OPERATIONS {
        for raw in ["", "unchanged", "\u{0}λ\r\n// unmatched hook"] {
            assert_eq!(history_profile::normalize(operation, "unrelated", raw), raw);
        }
    }
    assert!(
        std::panic::catch_unwind(|| history_profile::normalize("unknown", "unrelated", ""))
            .is_err()
    );
}

#[test]
fn history_prefix_cost_child() {
    let Some(input) = std::env::var_os("HC074_HISTORY_CHAIN_INPUT") else {
        return;
    };
    history_profile::run(std::path::Path::new(&input));
}

#[test]
fn history_namespace_protocol_and_unrelated_input_are_exact() {
    for block in 0..6 {
        assert_eq!(history_profile::namespace_order(block), ["namespace-chain"]);
    }
    for raw in ["", "unchanged", "\u{0}λ\r\n// unmatched hook"] {
        assert_eq!(
            history_profile::normalize("namespace-chain", "unrelated", raw),
            raw
        );
    }
}

#[test]
fn history_namespace_matches_independent_frozen_sources_and_keeps_body_drift() {
    for path in history_profile::PATHS {
        let raw = runtime_io::read_to_string(runtime::root().join(path))
            .unwrap()
            .replace('\r', "");
        let expected = runtime::frozen("a33bd9a4d0e910ec71b78f640f19b8404517ce2f", path);
        assert_eq!(history_namespace::normalize(path, &raw), expected);
        let changed = format!("{raw}\nfn hc074_unreviewed_body() {{ panic!(\"drift\"); }}\n");
        assert_ne!(history_namespace::normalize(path, &changed), expected);
    }
}

#[test]
fn history_namespace_still_refuses_missing_and_duplicate_namespace_hook() {
    let path = "tools/long-run-supervisor-074/src/diagnostic_fixed_slice_tests.rs";
    let raw = runtime_io::read_to_string(runtime::root().join(path))
        .unwrap()
        .replace('\r', "");
    let hook = "\n#[path = \"../tests/support/fixed_slice_namespace.rs\"]\nmod namespace;\n";
    assert_eq!(raw.matches(hook).count(), 1);
    let delegated = history_namespace::held_build::normalize(path, &raw);
    assert_eq!(
        history_namespace::normalize(path, &raw),
        delegated.replacen(hook, "", 1)
    );
    for bad in [raw.replacen(hook, "", 1), format!("{raw}{hook}")] {
        assert!(std::panic::catch_unwind(|| history_namespace::normalize(path, &bad)).is_err());
    }
}

#[test]
fn history_namespace_cost_child() {
    let Some(input) = std::env::var_os("HC074_HISTORY_CHAIN_INPUT") else {
        return;
    };
    history_profile::run_namespace(std::path::Path::new(&input));
}

#[test]
fn normalization_cost_protocol_keeps_separate_scopes() {
    assert_eq!(normalization_profile::PATHS.len(), 3);
    assert_eq!(normalization_profile::OPERATIONS.len(), 8);
    assert_eq!(
        normalization_profile::order(0),
        normalization_profile::OPERATIONS
    );
    let mut reverse = normalization_profile::order(1);
    reverse.reverse();
    assert_eq!(reverse, normalization_profile::order(0));
}

#[test]
fn normalization_cost_child() {
    let Some(input) = std::env::var_os("HC074_NORMALIZATION_INPUT") else {
        return;
    };
    normalization_profile::run(std::path::Path::new(&input));
}

#[test]
fn guard_path_comparison_is_isolated_and_original_installation_stays_exact() {
    let before = include_str!("../../../tools/guard-path-comparison-074/fixed-slice-before.txt")
        .replace("\r\n", "\n");
    let current = std::fs::read_to_string(runtime::root().join(guard_path::SOURCE))
        .unwrap()
        .replace("\r\n", "\n");
    assert_eq!(current, before);
    let candidate = guard_path::candidate(&before);
    assert_eq!(candidate.matches(guard_path::NEW).count(), 1);
    assert!(!runtime::accepts(guard_path::SOURCE, &candidate));
    assert_eq!(runtime::expected(guard_path::SOURCE), Some(current.clone()));
    assert!(runtime::accepts(guard_path::SOURCE, &current));
    for bad in [
        current.replace(guard_path::OLD, ""),
        format!("{current}{}", guard_path::OLD),
        current.replace("assert_eq!", "assert_ne!"),
        format!("{current}\nconst UNRELATED: bool = true;\n"),
    ] {
        assert!(!runtime::accepts(guard_path::SOURCE, &bad));
    }
    // The pre-existing source gate compares Rust tokens, not comments/formatting.
    assert!(runtime::accepts(
        guard_path::SOURCE,
        &format!("{current}\n// comment\n")
    ));
}

#[test]
fn guard_path_comparison_refuses_missing_duplicate_and_already_changed_baselines() {
    let before = include_str!("../../../tools/guard-path-comparison-074/fixed-slice-before.txt")
        .replace("\r\n", "\n");
    for bad in [
        before.replace(guard_path::OLD, ""),
        format!("{before}{}", guard_path::OLD),
        guard_path::candidate(&before),
    ] {
        assert!(std::panic::catch_unwind(|| guard_path::candidate(&bad)).is_err());
    }
}

#[test]
fn guard_path_comparison_leaves_other_reader_expectations_unchanged() {
    for path in [
        "crates/xtask/tests/support/performance_contract_074_closing_validation.rs",
        "crates/xtask/tests/support/performance_contract_074_held_placement.rs",
    ] {
        let (_, read) = runtime::performance::READERS
            .iter()
            .find(|(file, _)| *file == path)
            .unwrap();
        let original = runtime::frozen(runtime::BASE, path);
        assert_eq!(
            runtime::expected(path),
            Some(runtime::performance::reader(&original, read))
        );
    }
    assert!(runtime::expected("not/an/approved/source.rs").is_none());
}
#[path = "../../../tools/guard-path-comparison-074/contract.rs"]
mod guard_path;
#[path = "../../../tools/authority-storage-source-074/contract.rs"]
mod storage_contract;
