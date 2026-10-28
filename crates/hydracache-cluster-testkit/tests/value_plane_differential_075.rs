use std::collections::BTreeSet;

use hydracache_cluster_testkit::value_plane_differential_075::{
    execute_rust, first_difference, generated_trace, shrink_trace, ApiTrace, DifferentialError,
    TraceStep, MAX_TRACE_BYTES, MAX_TRACE_LINE_BYTES, MAX_TRACE_STEPS,
};

#[test]
fn trace_generation_roundtrips_and_covers_every_contract_operation() {
    let trace = generated_trace(117, 128).unwrap();
    assert_eq!(ApiTrace::parse(&trace.encode()).unwrap(), trace);
    assert_eq!(generated_trace(117, 128).unwrap(), trace);
    assert_ne!(generated_trace(118, 128).unwrap(), trace);
    let contract: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/testing/imap/0.75/operation-contract.json"
    ))
    .unwrap();
    let covered = trace
        .steps
        .iter()
        .map(|step| step.operation.as_str())
        .collect::<BTreeSet<_>>();
    for operation in contract["operations"].as_array().unwrap() {
        assert!(
            covered.contains(operation["id"].as_str().unwrap()),
            "{}",
            operation["id"]
        );
    }
    let results = execute_rust(&trace).unwrap();
    assert_eq!(results.len(), trace.steps.len());
    assert!(results
        .iter()
        .all(|line| line.is_ascii() && !line.contains('\n')));
    assert!(!results
        .iter()
        .any(|line| line.starts_with("error:unsupported_operation")));
    assert!(results.iter().any(|line| line.starts_with("pending|")));
    assert!(results.iter().any(|line| line.starts_with("completed|")));
    assert!(results.iter().any(|line| line.starts_with("unknown|")));
    assert!(results
        .iter()
        .any(|line| line.starts_with("error:scan_cursor_stale|")));
    assert!(results.iter().any(|line| line.contains("/expired/")));
    assert!(results.iter().any(|line| line.contains("/destroyed/")));
    assert!(results.iter().any(|line| line.contains("/cleared/")));
    assert!(results
        .iter()
        .any(|line| line.starts_with("removed|74776f|")));
    let empty_clear = trace
        .steps
        .iter()
        .enumerate()
        .filter(|(_, step)| step.operation == "clear_detailed")
        .nth(1)
        .unwrap()
        .0;
    assert!(results[empty_clear].starts_with("cleared|~|~|~|0|0|"));
    assert_eq!(results[empty_clear].split('|').nth(8), Some("[]"));
    for operation in [
        "put_all_detailed",
        "replace_all_detailed",
        "remove_all_detailed",
    ] {
        let index = trace
            .steps
            .iter()
            .position(|step| step.operation == operation)
            .unwrap();
        let events = results[index].split('|').nth(8).unwrap();
        let key_c = events.find("/63/").unwrap();
        let key_b = events.find("/62/").unwrap();
        assert!(
            key_c < key_b,
            "{operation}: event order must preserve request order"
        );
    }
    assert!(first_difference(&trace, &results, &results).is_none());
}

#[test]
fn malformed_or_oversized_trace_inputs_fail_before_execution() {
    let valid = TraceStep::point("get", b"", Some(b""));
    assert_eq!(TraceStep::parse(&valid.encode()).unwrap(), valid);
    assert!(TraceStep::parse("get\t0\t~\t~\teternal\t0\t0\t9\t\t").is_err());
    assert!(TraceStep::parse("get\tzz\t~\t~\teternal\t0\t0\t9\t\t").is_err());
    assert!(TraceStep::parse("get\t\t~\t~\tafter:0\t0\t0\t9\t\t").is_err());
    assert!(TraceStep::parse("get\t\t~\t~\tunknown\t0\t0\t9\t\t").is_err());
    assert!(TraceStep::parse("get\t\t~\t~\teternal\t-1\t0\t9\t\t").is_err());
    assert!(TraceStep::parse("get\t\t~\t~\teternal\t0\t0\t9\t;\t").is_err());
    assert!(TraceStep::parse("GET\t\t~\t~\teternal\t0\t0\t9\t\t").is_err());
    assert!(TraceStep::parse(&"x".repeat(MAX_TRACE_LINE_BYTES + 1)).is_err());
    assert!(ApiTrace::parse("").is_err());
    assert!(ApiTrace::parse("imap-api-trace-v2\t1\n").is_err());
    assert!(ApiTrace::parse("imap-api-trace-v1\t1\n").is_err());
    assert!(ApiTrace::parse(&"x".repeat(MAX_TRACE_BYTES + 1)).is_err());
    let excessive = format!(
        "imap-api-trace-v1\t1\n{}",
        format!("{}\n", valid.encode()).repeat(MAX_TRACE_STEPS + 1)
    );
    assert!(ApiTrace::parse(&excessive).is_err());
    assert!(generated_trace(1, MAX_TRACE_STEPS).is_err());
    assert!(generated_trace(u64::MAX, 0).is_err());
}

#[test]
fn differential_comparison_detects_value_errors_missing_and_extra_steps() {
    let trace = ApiTrace {
        seed: 1,
        steps: vec![TraceStep::point("get", b"key", None)],
    };
    let correct = vec!["present|31|2|~|0|0|[]|[]|[]|[]".to_owned()];
    let divergent = vec!["present|32|2|~|0|0|[]|[]|[]|[]".to_owned()];
    let mismatch = first_difference(&trace, &correct, &divergent).unwrap();
    assert_eq!(mismatch.step, 0);
    assert_eq!(mismatch.operation, "get");
    assert_ne!(mismatch.rust, mismatch.java);
    assert!(first_difference(&trace, &correct, &[]).is_some());
    assert!(first_difference(&trace, &[], &[]).is_some());
    assert!(first_difference(&trace, &correct, &[correct[0].clone(), "extra".into()]).is_some());
}

#[test]
fn differential_failures_shrink_deterministically_with_a_budget() {
    let trace = ApiTrace {
        seed: 117,
        steps: vec![
            TraceStep::point("get", b"unrelated", None),
            TraceStep::point("set", b"trigger", Some(b"v")),
            TraceStep::point("get", b"unrelated", None),
        ],
    };
    let predicate = |candidate: &ApiTrace| -> Result<bool, DifferentialError> {
        Ok(candidate.steps.iter().any(|step| step.key == b"trigger"))
    };
    let minimized = shrink_trace(&trace, 16, predicate).unwrap();
    assert_eq!(minimized.steps.len(), 1);
    assert_eq!(minimized.steps[0].key, b"trigger");
    assert_eq!(shrink_trace(&trace, 16, predicate).unwrap(), minimized);
    assert_eq!(shrink_trace(&trace, 0, predicate).unwrap(), trace);
    assert_eq!(shrink_trace(&trace, 16, |_| Ok(false)).unwrap(), trace);
    assert!(shrink_trace(&trace, 16, |_| Err(DifferentialError("failure".into()))).is_err());
    let mut long = ApiTrace {
        seed: 117,
        steps: vec![TraceStep::point("get", b"unrelated", None); 200],
    };
    long.steps[70] = TraceStep::point("set", b"trigger", Some(b"v"));
    assert_eq!(shrink_trace(&long, 16, predicate).unwrap().steps.len(), 1);
    let mut calls = 0;
    let limited = shrink_trace(&long, 3, |candidate| {
        calls += 1;
        predicate(candidate)
    })
    .unwrap();
    assert!(calls <= 3);
    assert!(!limited.steps.is_empty());
    assert!(limited.steps.len() < long.steps.len());
}

#[test]
fn state_observations_respect_per_step_and_aggregate_retention_budgets() {
    let trace = |count: usize, reads: usize| {
        let mut steps = (0..count)
            .map(|index| {
                let mut key = vec![0; 1024];
                key[0] = index as u8;
                let mut step = TraceStep::point("set", &key, Some(&vec![255; 1024]));
                step.identity = index as u64 + 1;
                step
            })
            .collect::<Vec<_>>();
        steps.extend((0..reads).map(|_| TraceStep::point("get", b"", None)));
        ApiTrace { seed: 117, steps }
    };
    assert!(execute_rust(&trace(17, 0))
        .unwrap_err()
        .to_string()
        .contains("output bound"));
    assert!(execute_rust(&trace(15, 100))
        .unwrap_err()
        .to_string()
        .contains("output bound"));
}
