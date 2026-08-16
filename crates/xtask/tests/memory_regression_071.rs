use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}

#[test]
fn release_071_plan_records_the_hosted_handoff_and_ci_tiers() {
    let plan = fs::read_to_string(
        root().join("docs/plans/V0_71_MEMORY_FOOTPRINT_AND_RETENTION_EFFICIENCY_PLAN.md"),
    )
    .unwrap();

    for marker in [
        "Investigation hand-off from 0.70",
        "Allocator high-water or live ownership",
        "Correct TTL tail",
        "Allocator A/B",
        "CI detection tiers",
        "Memory Regression Fast",
        "intentionally has no absolute RSS/PSS threshold",
        "three fresh processes",
    ] {
        assert!(plan.contains(marker), "0.71 plan is missing {marker:?}");
    }
}

#[test]
fn fast_memory_regression_job_checks_stable_owners_not_host_rss() {
    let workflow = fs::read_to_string(root().join(".github/workflows/ci.yml")).unwrap();
    let start = workflow
        .find("  memory-regression-fast:")
        .expect("fast memory job");
    let end = workflow[start..]
        .find("\n  memory-diagnostic-hosted:")
        .map(|offset| start + offset)
        .expect("hosted diagnostic follows fast job");
    let job = &workflow[start..end];

    for marker in [
        "name: Memory Regression Fast",
        "runs-on: ubuntu-24.04",
        "timeout-minutes: 30",
        "--test allocation_profile",
        "--test lock_lease",
        "--test conditional_tombstone",
        "diagnostic_reset_reports_and_clears_every_mutable_data_owner",
        "reset_reconnects_once_repairs_subscription_dedupes_and_loses_session",
        "verify_memory_telemetry_coverage_test.py",
        "--test-threads=1",
    ] {
        assert!(
            job.contains(marker),
            "fast memory job is missing {marker:?}"
        );
    }

    for unstable_gate in ["VmRSS", "smaps_rollup", "run-memory-leak-stage.sh"] {
        assert!(
            !job.contains(unstable_gate),
            "fast memory job must not gate on {unstable_gate:?}"
        );
    }
}
