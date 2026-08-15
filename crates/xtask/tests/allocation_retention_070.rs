use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const WORK_ITEMS: [(&str, &str, &str); 8] = [
    (
        "W0",
        "crates/hydracache/tests/allocation_profile.rs",
        "struct AllocationSnapshot",
    ),
    (
        "W1",
        "crates/hydracache/src/tag_index.rs",
        "MAX_GENERATION_TOMBSTONES",
    ),
    (
        "W2",
        "crates/hydracache/src/load_breaker.rs",
        "max_tracked_keys",
    ),
    (
        "W3",
        "crates/hydracache-client-transport-axum/src/lib.rs",
        "ClientSurfaceRetainedState",
    ),
    (
        "W4",
        "crates/hydracache/src/cache.rs",
        "run_pending_tasks().await",
    ),
    (
        "W5",
        "crates/hydracache-client-hc2/src/types.rs",
        "ClientRetainedStateSnapshot",
    ),
    (
        "W6",
        "scripts/perf/run-memory-leak-stage.sh",
        "ship_evidence_eligible=false",
    ),
    (
        "W7",
        ".github/workflows/ci.yml",
        "canary-sweep --release 0.70 --tier fast",
    ),
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}

fn closure_problems(omitted: Option<&str>) -> Vec<String> {
    let root = root();
    let mut problems = Vec::new();
    for (work_item, source, marker) in WORK_ITEMS {
        if omitted == Some(work_item) {
            problems.push(format!("{work_item} closure evidence was removed"));
            continue;
        }
        match fs::read_to_string(root.join(source)) {
            Ok(text) if text.contains(marker) => {}
            Ok(_) => problems.push(format!(
                "{work_item} marker {marker:?} is absent from {source}"
            )),
            Err(error) => problems.push(format!(
                "{work_item} source {source} cannot be read: {error}"
            )),
        }
    }

    let plan = fs::read_to_string(
        root.join("docs/plans/V0_70_ALLOCATION_PATH_AND_RETENTION_AUDIT_PLAN.md"),
    )
    .unwrap_or_default();
    for (work_item, _, _) in WORK_ITEMS {
        if !plan.contains(&format!("## {work_item}.")) {
            problems.push(format!("release plan omits {work_item}"));
        }
    }
    problems
}

#[test]
fn release_070_allocation_retention_closure_is_fail_closed() {
    let problems = closure_problems(None);
    assert!(problems.is_empty(), "0.70 closure problems: {problems:#?}");
}

#[test]
fn hosted_memory_diagnostic_is_explicitly_non_promotable() {
    let root = root();
    let workflow = fs::read_to_string(root.join(".github/workflows/ci.yml")).unwrap();
    for marker in [
        "run_memory_diagnostic:",
        "memory-diagnostic-hosted:",
        "runs-on: ubuntu-24.04",
        "MEMORY_DIAGNOSTIC_ENVIRONMENT: github-hosted",
        "MEMORY_DIAGNOSTIC_TARGETS: hydra redis",
        "scripts/perf/run-memory-leak-stage.sh",
        "Upload raw memory evidence",
    ] {
        assert!(workflow.contains(marker), "workflow is missing {marker:?}");
    }

    let runner = fs::read_to_string(root.join("scripts/perf/run-memory-leak-stage.sh")).unwrap();
    for marker in [
        "diagnostic_environment=\"${MEMORY_DIAGNOSTIC_ENVIRONMENT-bare-metal}\"",
        "diagnostic_environment=$diagnostic_environment",
        "output_dir=\"$(cd \"$output_dir\" && pwd -P)\"",
        "ship_evidence_eligible=false",
        "bare_metal_checks=not_applicable",
        "irq_isolation_checks=not_applicable",
        "if [[ \"$diagnostic_environment\" == bare-metal ]]",
        "memory diagnostics contain incomplete cases",
    ] {
        assert!(runner.contains(marker), "runner is missing {marker:?}");
    }
}

#[test]
fn canary_release_070_accepts_missing_work_item_evidence() {
    let Ok(defect) = env::var("HYDRACACHE_CANARY_DEFECT") else {
        return;
    };
    assert!(WORK_ITEMS.iter().any(|(item, _, _)| *item == defect));
    let problems = closure_problems(Some(&defect));
    assert!(
        problems.is_empty(),
        "HC-CANARY-RED:{defect}: release closure correctly rejected missing evidence: {problems:#?}"
    );
}
