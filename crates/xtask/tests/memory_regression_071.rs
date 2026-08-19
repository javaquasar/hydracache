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
fn release_071_plan_records_all_mandatory_execution_controls() {
    let plan = fs::read_to_string(
        root().join("docs/plans/V0_71_MEMORY_FOOTPRINT_AND_RETENTION_EFFICIENCY_PLAN.md"),
    )
    .unwrap();

    for marker in [
        "S1. Machine-readable memory ownership registry",
        "S2. Phase-correlated allocation-site and lifetime profiles",
        "S3. Evidence-locked stop/go decision gates",
        "S4. Pre-registered statistical and practical-significance contract",
        "S5. Instrumentation overhead and snapshot-coherence budget",
        "S6. Allocator fragmentation, arena and reuse diagnosis",
        "S7. Dedicated-host stability and fingerprint protocol",
        "S8. Upgrade, rollback and mixed-version memory compatibility",
        "S9. CI reliability, deduplication and bounded execution",
        "S10. Minimum releasable result and evidence-based deferral",
        "Required implementation sequence and focused verification",
        "memory-owner-inventory --release 0.71 --check",
        "memory-ownership-check --release 0.71",
        "memory-statistics-check --release 0.71",
        "memory-decision-check --release 0.71",
        "ci-topology-check --release 0.71",
        "observed_non_atomic",
        "measured-no-win",
        "one designated admission",
        "Ship is allowed with zero optional wins",
        "Baseline identities and comparison boundaries",
        "`B0-release`",
        "`B1-instrumented`",
        "`C-candidate`",
        "finite and one-factor",
        "does **not** mean five separate 24-hour campaigns",
        "W2a — mandatory estimator and reporting",
        "W2b — evidence-conditional admission policy",
        "memory-campaign-check --release 0.71 --require-ship",
        "cargo xtask verify",
    ] {
        assert!(
            plan.contains(marker),
            "0.71 plan is missing mandatory control marker {marker:?}"
        );
    }
}

#[test]
fn release_071_records_the_dedicated_reference_dependency_and_no_win_boundary() {
    let releases = fs::read_to_string(root().join("docs/plans/releases.toml")).unwrap();
    let release_start = releases
        .find("version = \"0.71.0\"")
        .expect("0.71 release entry");
    let release = &releases[release_start..];
    assert!(
        release.contains("depends_on = [\"0.70.0\", \"0.67.1\"]"),
        "0.71 must make the dedicated 0.67.1 bootstrap dependency explicit"
    );
    assert!(
        release.contains("A fully evidenced no-win release may ship"),
        "0.71 registry theme must preserve the no-win release boundary"
    );

    let index = fs::read_to_string(root().join("docs/plans/INDEX.md")).unwrap();
    let row = index
        .lines()
        .find(|line| line.starts_with("| [0.71.0]"))
        .expect("0.71 roadmap row");
    for marker in [
        "completed 0.67.1 dedicated-reference bootstrap",
        "unmodified 0.70 release baseline",
        "no-win result may ship",
        "| 0.70.0, 0.67.1 |",
    ] {
        assert!(
            row.contains(marker),
            "0.71 roadmap row is missing {marker:?}"
        );
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
        "evidence-run --release 0.70 --gate fast.memory-regression-070",
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

    let registry = fs::read_to_string(root().join("docs/testing/fast-suite-registry.toml"))
        .expect("fast-suite registry");
    assert!(
        registry.contains("id = \"fast.memory-regression-070\"")
            && registry.contains("program = \"scripts/ci/run-memory-regression-fast-070.sh\""),
        "fast memory evidence gate must resolve to the reviewed runner"
    );

    let runner = fs::read_to_string(root().join("scripts/ci/run-memory-regression-fast-070.sh"))
        .expect("fast memory runner");
    for marker in [
        "--test allocation_profile",
        "--test lock_lease",
        "--test conditional_tombstone",
        "diagnostic_reset_reports_and_clears_every_mutable_data_owner",
        "reset_reconnects_once_repairs_subscription_dedupes_and_loses_session",
        "verify_memory_telemetry_coverage_test.py",
        "--test-threads=1",
    ] {
        assert!(
            runner.contains(marker),
            "fast memory runner is missing {marker:?}"
        );
    }
}
