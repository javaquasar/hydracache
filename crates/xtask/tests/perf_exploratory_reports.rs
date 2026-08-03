use std::fs;

fn read(path: &str) -> String {
    let root = xtask::doc_check::find_repo_root().unwrap();
    fs::read_to_string(root.join(path)).unwrap().replace("\r\n", "\n")
}

#[test]
fn archive_identity_and_release_boundary_are_explicit() {
    let archive = read("docs/testing/perf-scenarios/0.67/EXPLORATORY_ARCHIVE.md");

    for required in [
        "explore-0.67-telemetry-20260803",
        "dbc2f82f7f303528b3cca7842818730c82232b9c",
        "1ce50cb455742395d303f46cb81866efa513c664",
        "30613577155",
        "8786642124",
        "30614325548",
        "8787545365",
        "did not count as a bootstrap sample",
        "five accepted, serialized, same-fingerprint bootstrap",
        "5,469",
        "not satisfy a release gate",
    ] {
        assert!(archive.contains(required), "archive index lacks {required}");
    }
}

#[test]
fn curated_index_covers_every_retained_report() {
    let index = read("docs/testing/perf-scenarios/0.67/results/README.md");
    let reports = [
        "relative-eight-cases-20260801.md",
        "comparative-memory-20260802.md",
        "target-aggregate-20260802.md",
        "development-six-20260802.md",
        "memory-leak-report-20260803.md",
        "memory-leak-analysis-20260803.md",
        "metric-expansion-report-20260803.md",
        "metric-expansion-analysis-20260803.md",
        "memory-investigations-report-20260804.md",
        "memory-optimization-analysis-20260804.md",
    ];

    for report in reports {
        assert!(index.contains(report), "result index lacks {report}");
        let body = read(&format!(
            "docs/testing/perf-scenarios/0.67/results/{report}"
        ));
        assert!(body.starts_with("# "), "{report} lacks a title");
        assert!(body.len() > 500, "{report} is unexpectedly small");
        assert!(!body.contains("62.210.158.50"));
        assert!(!body.contains("PRIVATE KEY"));
        assert!(!body.contains("RUNNER_TOKEN="));
    }
}

#[test]
fn preparation_report_preserves_failures_without_exposing_retired_address() {
    let report = read(
        "docs/testing/perf-scenarios/0.67/exploratory-preparation-and-measurement-report.md",
    );

    assert!(report.contains("retired public address is intentionally omitted"));
    assert!(report.contains("The run is **rejected**"));
    assert!(report.contains("No HydraCache-versus-Redis-versus-Hazelcast ranking"));
    assert!(report.contains("Harness changes and problems corrected"));
    assert!(report.contains("Noise controls used"));
    assert!(!report.contains("62.210.158.50"));
}
