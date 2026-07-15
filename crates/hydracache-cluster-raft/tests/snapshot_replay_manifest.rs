use serde_json::{json, Value};

fn manifest() -> Value {
    serde_json::from_str(include_str!("vectors/snapshot_replay_manifest.json"))
        .expect("snapshot replay manifest fixture must be valid JSON")
}

fn string_field<'a>(manifest: &'a Value, field: &str, problems: &mut Vec<String>) -> &'a str {
    let Some(value) = manifest.get(field).and_then(Value::as_str) else {
        problems.push(format!("{field} must be a non-empty string"));
        return "";
    };
    if value.trim().is_empty() {
        problems.push(format!("{field} must be a non-empty string"));
    }
    value
}

fn array_field<'a>(manifest: &'a Value, field: &str, problems: &mut Vec<String>) -> &'a Vec<Value> {
    let Some(value) = manifest.get(field).and_then(Value::as_array) else {
        problems.push(format!("{field} must be an array"));
        return empty_array();
    };
    value
}

fn empty_array() -> &'static Vec<Value> {
    static EMPTY: std::sync::OnceLock<Vec<Value>> = std::sync::OnceLock::new();
    EMPTY.get_or_init(Vec::new)
}

fn validate_manifest(manifest: &Value) -> Result<(), Vec<String>> {
    let mut problems = Vec::new();
    string_field(manifest, "schema", &mut problems);
    string_field(manifest, "current_hypothesis", &mut problems);
    string_field(manifest, "replay_seed", &mut problems);
    string_field(manifest, "trace_artifact", &mut problems);

    let decision = string_field(manifest, "decision", &mut problems);
    if !matches!(decision, "fixed" | "explained" | "blocked") {
        problems.push(format!(
            "decision must be fixed, explained, or blocked; got {decision:?}"
        ));
    }

    let supporting = array_field(manifest, "supporting_evidence", &mut problems);
    let contradicting = array_field(manifest, "contradicting_evidence", &mut problems);
    let unexplained = array_field(manifest, "unexplained_state_machine_errors", &mut problems);
    let schedule = array_field(manifest, "schedule", &mut problems);
    if supporting.is_empty() {
        problems.push("supporting_evidence must not be empty".to_owned());
    }
    if schedule.is_empty() {
        problems.push("schedule must not be empty".to_owned());
    }

    let closure = manifest.get("closure").unwrap_or(&Value::Null);
    let environmental = closure
        .get("environmental")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let fix_strategy = closure
        .get("fix_strategy")
        .and_then(Value::as_str)
        .unwrap_or("unknown");

    if environmental && (!unexplained.is_empty() || !contradicting.is_empty()) {
        problems.push(
            "environmental closure is forbidden while contradictions or unexplained state-machine errors remain"
                .to_owned(),
        );
    }
    if fix_strategy == "log_level_downgrade"
        && (!unexplained.is_empty() || !contradicting.is_empty())
    {
        problems.push("log-level downgrade cannot close a correctness contradiction".to_owned());
    }

    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems)
    }
}

#[test]
fn snapshot_replay_manifest_contains_contradiction_ledger_fields() {
    validate_manifest(&manifest()).expect("manifest should satisfy the contradiction ledger shape");
}

#[test]
fn snapshot_replay_manifest_rejects_environmental_closure_with_unexplained_apply_error() {
    let mut manifest = manifest();
    manifest["contradicting_evidence"] = json!(["restored member set missed committed tail"]);
    manifest["unexplained_state_machine_errors"] = json!(["raft snapshot apply error"]);
    manifest["closure"]["environmental"] = json!(true);

    let problems = validate_manifest(&manifest).expect_err("environmental closure must fail");
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W5") {
        assert!(
            problems.is_empty(),
            "HC-CANARY-RED:W5 contradiction was closed as environmental"
        );
    }
    assert!(
        problems
            .iter()
            .any(|problem| problem.contains("environmental closure is forbidden")),
        "unexpected validation problems: {problems:?}"
    );
}

#[test]
fn snapshot_replay_manifest_rejects_log_level_downgrade_as_fix() {
    let mut manifest = manifest();
    manifest["contradicting_evidence"] = json!(["apply error was hidden from the guard"]);
    manifest["closure"]["fix_strategy"] = json!("log_level_downgrade");

    let problems = validate_manifest(&manifest).expect_err("log downgrade closure must fail");
    assert!(
        problems
            .iter()
            .any(|problem| problem.contains("log-level downgrade")),
        "unexpected validation problems: {problems:?}"
    );
}
