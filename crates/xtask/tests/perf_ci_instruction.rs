use std::fs;

use serde_json::Value;

fn read(path: &str) -> String {
    let root = xtask::doc_check::find_repo_root().unwrap();
    fs::read_to_string(root.join(path)).unwrap()
}

#[test]
fn ci_instruction_profile_is_paired_and_cannot_be_ship_evidence() {
    let policy: Value =
        serde_json::from_str(&read("docs/testing/perf-policies/ci-instruction-v1.json")).unwrap();
    assert_eq!(policy["schema_version"], 1);
    assert_eq!(policy["profile"], "ci-instruction-v1");
    assert_eq!(policy["measurement"]["blocking_metric"], "Ir");
    assert_eq!(policy["measurement"]["harness_version"], "0.19.4");
    assert_eq!(policy["measurement"]["toolchain"], "rust-1.94.0");
    assert_eq!(
        policy["measurement"]["aslr"],
        "enabled-identically-for-base-and-head"
    );
    assert_eq!(
        policy["comparison"]["strategy"],
        "paired-base-head-same-job"
    );
    assert_eq!(policy["comparison"]["rolling_baseline"], false);
    assert_eq!(policy["comparison"]["cross_runner_comparison"], false);
    assert_eq!(policy["claim_boundary"]["relative_work_regression"], true);
    for forbidden in [
        "qualification_evidence",
        "bootstrap_evidence",
        "ship_evidence_eligible",
        "latency_claim",
        "throughput_claim",
        "capacity_claim",
    ] {
        assert_eq!(policy["claim_boundary"][forbidden], false, "{forbidden}");
    }
}

#[test]
fn workflow_runs_real_callgrind_work_without_touching_reference_lanes() {
    let workflow = read(".github/workflows/ci.yml");
    let script = read("scripts/perf/run-ci-instruction-pair.sh");
    let harness = read("scripts/perf/ci-instruction-harness/benches/cache_work.rs");
    let guide = read("docs/testing/PERF_CI_INSTRUCTION_PROFILE.md");

    for required in [
        "performance-ci-instruction-v1:",
        "runs-on: ubuntu-24.04",
        "ref: ${{ github.event.pull_request.head.sha || github.sha }}",
        "cargo install --locked gungraun-runner --version 0.19.4",
        "scripts/perf/run-ci-instruction-pair.sh",
        "Upload instruction profiles and receipt",
    ] {
        assert!(workflow.contains(required), "workflow lacks {required}");
    }
    for required in [
        "git archive",
        "--save-baseline=base",
        "--baseline=base",
        "--callgrind-limits='ir=5.0%'",
        "--allow-aslr=yes",
        "--parallel=1",
        "sync_subject_lock base",
        "sync_subject_lock head",
        "cargo update --offline",
        "subject lock synchronization changed a registry package",
        "report.json",
        "contract-sha256.txt",
    ] {
        assert!(script.contains(required), "runner lacks {required}");
    }
    for required in [
        "cache_get_hit",
        "cache_get_miss",
        "OPERATIONS_PER_SAMPLE: usize = 64",
    ] {
        assert!(harness.contains(required), "harness lacks {required}");
    }
    for required in [
        "does not replace `reference-v1`",
        "does not replace `memory-only-v1`",
        "not latency",
        "not throughput",
    ] {
        assert!(guide.contains(required), "guide lacks {required}");
    }
}
