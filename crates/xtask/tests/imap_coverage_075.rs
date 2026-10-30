use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    process::Command,
};
use xtask::imap_coverage_075::*;
fn source(path: &str) -> Source {
    Source {
        path: path.into(),
        sha256: "a".repeat(64),
        bytes: 100,
        added: [2, 3, 4].into_iter().collect(),
        has_executable: true,
    }
}
fn known() -> BTreeSet<String> {
    ["crates/xtask/src/unit.rs".into()].into_iter().collect()
}
fn target() -> xtask::imap_sbom_075::CargoTarget {
    xtask::imap_sbom_075::CargoTarget {
        triple: "x86_64-pc-windows-msvc".into(),
        cfg: ["windows".into()].into_iter().collect(),
    }
}
#[test]
fn public_workloads_use_fixed_probes_and_distinct_short_resume_segments() {
    let workloads = public_workloads("target/imap-075/fixture");
    assert_eq!(workloads.len(), 6);
    for (arguments, name) in workloads[..3].iter().zip([
        "imap-native-observation",
        "imap-native-concurrent",
        "imap-native-listener-recovery",
    ]) {
        assert_eq!(arguments, &[name, "--release", "0.75"]);
    }
    for (index, arguments) in workloads[3..].iter().enumerate() {
        assert_eq!(
            &arguments[..10],
            [
                "imap-soak",
                "--release",
                "0.75",
                "--cohort",
                if index == 0 { "all" } else { "reference" },
                "--profile",
                "smoke",
                "--host-budget-seconds",
                "180",
                "--output"
            ]
        );
        assert!(arguments[10].starts_with("target/imap-075/fixture/soak-"));
        if index == 0 {
            assert_eq!(arguments.len(), 11);
        } else {
            assert_eq!(arguments[11], "--resume");
            assert_eq!(
                arguments[12],
                format!(
                    "target/imap-075/fixture/soak-cli/reference/{}",
                    if index == 1 {
                        "checkpoint.json"
                    } else {
                        "report.json"
                    }
                )
            );
            assert!(!arguments[12].starts_with(&arguments[10]));
        }
    }
}
#[test]
fn consumer_workspace_refuses_concurrent_or_stale_inputs_and_restarts_empty_at_the_same_path() {
    let root = tempfile::tempdir().unwrap();
    assert!(consumer_workspace(root.path()).is_err());
    std::fs::create_dir_all(root.path().join("target/imap-075")).unwrap();
    let first = consumer_workspace(root.path()).unwrap();
    let path = first.path().to_path_buf();
    assert!(path.ends_with("target/imap-075/coverage-consumer-workspace"));
    std::fs::write(path.join("stale-input"), b"owned fixture").unwrap();
    assert!(consumer_workspace(root.path()).is_err());
    drop(first);
    assert!(!path.exists());
    let next = consumer_workspace(root.path()).unwrap();
    assert_eq!(next.path(), path);
    assert_eq!(std::fs::read_dir(next.path()).unwrap().count(), 0);
    let retained = next.keep();
    assert!(consumer_workspace(root.path()).is_err());
    std::fs::remove_dir(retained).unwrap();
}
fn profile() -> Profile {
    Profile {
        schema: "imap075-coverage-profile-v1".into(),
        baseline: BASELINE.into(),
        os: "windows".into(),
        arch: "x86_64".into(),
        rust_flags: "-C instrument-coverage -C debuginfo=0".into(),
        feature: "imap-075-coverage".into(),
        target: target(),
        tools: vec![Tool {
            name: "fixture".into(),
            sha256: "b".repeat(64),
            bytes: 1,
            version: "fixture".into(),
        }],
    }
}
fn analysis_fixture() -> (Control, Analysis) {
    let artifact = xtask::imap_extended_receipt_075::Artifact {
        path: "target/fixture".into(),
        bytes: 1,
        sha256: "c".repeat(64),
    };
    let control = Control {
        schema: "imap075-coverage-control-v1".into(),
        source_sha: "a".repeat(40),
        run_id: "b".repeat(32),
        production_enabled: false,
        profile: profile(),
        sources: vec![source("crates/xtask/src/unit.rs")],
        producer: artifact.clone(),
    };
    let analysis = Analysis {
        schema: "imap075-scoped-coverage-v1".into(),
        source_sha: control.source_sha.clone(),
        run_id: control.run_id.clone(),
        profile: control.profile.clone(),
        sources: control.sources.clone(),
        raw_profiles: vec![artifact.clone()],
        rust_report: artifact.clone(),
        java_reports: vec![artifact],
        summary: Summary {
            sources: vec![],
            executable_added: 0,
            covered_added: 0,
            compiled_out: vec![],
        },
        qualification: false,
    };
    (control, analysis)
}
#[test]
fn analysis_phase_requires_its_own_live_pid_or_a_closed_source_bound_measurement() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("capture");
    std::fs::create_dir(&directory).unwrap();
    let (control, analysis) = analysis_fixture();
    let write = |name: &str, value: &serde_json::Value| {
        std::fs::write(directory.join(name), serde_json::to_vec(value).unwrap()).unwrap();
    };
    let pid = std::process::id();
    let mut identity = serde_json::json!({"pid":pid,
        "platform_birth":xtask::imap_soak_075::process_identity(pid).unwrap(),
        "incarnation":format!("coverage-{pid}")});
    write("analysis-started.json", &identity);
    assert_eq!(
        analysis_phase(root.path(), &directory, &control, false).unwrap(),
        Some(pid)
    );
    assert!(analysis_phase(root.path(), &directory, &control, true)
        .unwrap_err()
        .to_string()
        .contains("still lives"));
    identity["platform_birth"] = json!("foreign-birth");
    write("analysis-started.json", &identity);
    assert!(analysis_phase(root.path(), &directory, &control, false).is_err());
    identity = json!({"pid":u32::MAX,"platform_birth":"absent-process",
        "incarnation":format!("coverage-{}",u32::MAX)});
    write("analysis-started.json", &identity);
    assert!(analysis_phase(root.path(), &directory, &control, true).is_err());
    write("analysis.json", &serde_json::to_value(&analysis).unwrap());
    assert_eq!(
        analysis_phase(root.path(), &directory, &control, true).unwrap(),
        None
    );
    for field in [
        "schema",
        "source_sha",
        "run_id",
        "profile",
        "sources",
        "qualification",
    ] {
        let mut changed = serde_json::to_value(&analysis).unwrap();
        changed[field] = match field {
            "profile" => {
                let mut profile = changed[field].clone();
                profile["os"] = json!("foreign");
                profile
            }
            "sources" => json!([]),
            "qualification" => json!(true),
            _ => json!("foreign"),
        };
        write("analysis.json", &changed);
        assert!(
            analysis_phase(root.path(), &directory, &control, true)
                .unwrap_err()
                .to_string()
                .contains("incomplete/stale"),
            "{field}"
        );
    }
    identity["pid"] = json!(0);
    write("analysis-started.json", &identity);
    assert!(analysis_phase(root.path(), &directory, &control, true)
        .unwrap_err()
        .to_string()
        .contains("invalid coverage controller"));
}
#[test]
fn reconstructed_analysis_cannot_substitute_any_bound_evidence() {
    let (_, original) = analysis_fixture();
    compare_analysis(&original, &original).unwrap();
    for mode in 0..11 {
        let mut changed = original.clone();
        match mode {
            0 => changed.schema = "future".into(),
            1 => changed.qualification = true,
            2 => changed.source_sha = "d".repeat(40),
            3 => changed.run_id = "e".repeat(32),
            4 => changed.profile.os = "foreign".into(),
            5 => changed.sources.clear(),
            6 => changed.raw_profiles.clear(),
            7 => changed.summary.covered_added += 1,
            8 => changed.rust_report.sha256 = "f".repeat(64),
            9 => changed.rust_report.bytes += 1,
            _ => changed.java_reports.clear(),
        }
        assert!(compare_analysis(&original, &changed).is_err(), "{mode}");
    }
}
#[test]
fn public_checker_and_cli_refuse_an_unqualified_measurement_before_replay() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let scratch = tempfile::tempdir_in(root.join("target/imap-075")).unwrap();
    let path = scratch.path().join("report.json");
    let (_, analysis) = analysis_fixture();
    std::fs::write(&path, serde_json::to_vec(&analysis).unwrap()).unwrap();
    assert!(check(root, &path)
        .unwrap_err()
        .to_string()
        .contains("not finalized qualification"));
    let mut command = Command::new(env!("CARGO_BIN_EXE_xtask"));
    command
        .current_dir(root)
        .args(["imap-coverage-check", "--release", "0.75", "--receipt"])
        .arg(&path);
    let error =
        xtask::imap_proof_process_075::execute(command, &[], Default::default()).unwrap_err();
    assert_eq!(error.kind, xtask::imap_proof_process_075::FailureKind::Exit);
    assert!(error.detail.contains("not finalized qualification"));
    let not_a_repository = tempfile::tempdir().unwrap();
    assert!(capture(not_a_repository.path(), Path::new("target/imap-075/absent")).is_err());
}
#[test]
fn changed_line_intersection_never_counts_comments_or_assumes_coverage() {
    let diff=b"diff --git a/crates/xtask/src/unit.rs b/crates/xtask/src/unit.rs\n--- a/crates/xtask/src/unit.rs\n+++ b/crates/xtask/src/unit.rs\n@@ -1 +2,3 @@\n+code\n+code\n+// comment\n";
    assert_eq!(
        added_lines(diff).unwrap()["crates/xtask/src/unit.rs"],
        [2, 3, 4].into_iter().collect()
    );
    let sources = vec![source("crates/xtask/src/unit.rs")];
    let mut data = Lines::from([(
        "crates/xtask/src/unit.rs".into(),
        [(2, 1), (3, 0)].into_iter().collect(),
    )]);
    let summary = summarize(&sources, &data, "windows", vec![]).unwrap();
    assert_eq!((summary.executable_added, summary.covered_added), (2, 1));
    assert!(require_complete(&summary).is_err());
    data.get_mut("crates/xtask/src/unit.rs")
        .unwrap()
        .insert(3, 2);
    let summary = summarize(&sources, &data, "windows", vec![]).unwrap();
    require_complete(&summary).unwrap();
    assert!(!gate_result(&summary, false).unwrap());
    assert!(gate_result(&summary, true).unwrap());
}
#[test]
fn policy_cannot_silently_weaken_the_denominator_or_platform_claim() {
    let policy: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/testing/imap/0.75/coverage-profile.json"
    ))
    .unwrap();
    validate_policy(&policy).unwrap();
    for key in [
        "baseline",
        "scopes",
        "require_all_changed_executable_lines",
        "production_enabled",
        "claim_branch_coverage",
        "historical_global_ratchet",
    ] {
        let mut changed = policy.clone();
        changed[key] = json!(null);
        assert!(validate_policy(&changed).is_err());
    }
    let mut changed = policy;
    changed["skip_xtask"] = json!(true);
    assert!(validate_policy(&changed).is_err());
}
#[test]
fn atomic_checkpoint_replaces_complete_bytes_and_preserves_previous_on_failure() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("role");
    std::fs::create_dir(&directory).unwrap();
    persist_profile(&directory, 17, b"first fixture; not a valid LLVM report").unwrap();
    persist_profile(&directory, 17, b"second").unwrap();
    let path = directory.join("17.profraw");
    assert_eq!(std::fs::read(&path).unwrap(), b"second");
    assert!(persist_profile(&directory, 0, b"invalid").is_err());
    assert!(persist_profile(&directory, 17, b"").is_err());
    assert!(persist_profile(Path::new("relative"), 17, b"invalid").is_err());
    assert_eq!(std::fs::read(path).unwrap(), b"second");
    assert_eq!(
        std::fs::read_dir(root.path().join("profile-staging"))
            .unwrap()
            .count(),
        0
    );
}
#[test]
fn live_analysis_is_deferred_only_for_measurement_and_every_final_profile_is_required() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("capture");
    for role in ["controller", "tooling", "testkit", "consumer", "analysis"] {
        let path = directory.join("raw").join(role);
        std::fs::create_dir_all(&path).unwrap();
        if role != "analysis" {
            std::fs::write(path.join("17.profraw"), b"metadata fixture, not LLVM data").unwrap();
        }
    }
    let analysis = directory.join("raw/analysis/42.profraw");
    std::fs::write(&analysis, b"").unwrap();
    assert!(collect_profiles(root.path(), &directory, None).is_err());
    assert_eq!(
        collect_profiles(root.path(), &directory, Some(42))
            .unwrap()
            .len(),
        4
    );
    assert!(collect_profiles(root.path(), &directory, Some(0)).is_err());
    std::fs::write(&analysis, b"completed fixture").unwrap();
    assert_eq!(
        collect_profiles(root.path(), &directory, None)
            .unwrap()
            .len(),
        5
    );
    for name in [
        ".profraw",
        "0.profraw",
        "001.profraw",
        "17.profraw.profraw",
        "foreign.tmp",
    ] {
        let path = directory.join("raw/analysis").join(name);
        std::fs::write(&path, b"fixture").unwrap();
        assert!(
            collect_profiles(root.path(), &directory, None).is_err(),
            "{name}"
        );
        std::fs::remove_file(path).unwrap();
    }
    let nonfile = directory.join("raw/analysis/71.profraw");
    std::fs::create_dir(&nonfile).unwrap();
    assert!(collect_profiles(root.path(), &directory, None).is_err());
    std::fs::remove_dir(nonfile).unwrap();
    std::fs::remove_file(directory.join("raw/tooling/17.profraw")).unwrap();
    assert!(collect_profiles(root.path(), &directory, None).is_err());
}
#[test]
fn raw_profile_budget_is_exact_and_refuses_empty_oversized_and_overflow_inputs() {
    let limit = 6 * 1024 * 1024 * 1024;
    assert_eq!(profile_total(0, 1).unwrap(), 1);
    assert_eq!(profile_total(limit - 1, 1).unwrap(), limit);
    assert!(profile_total(limit, 1).is_err());
    assert!(profile_total(0, 0).is_err());
    assert!(profile_total(0, 128 * 1024 * 1024 + 1).is_err());
    assert!(profile_total(u64::MAX, 1)
        .unwrap_err()
        .to_string()
        .contains("overflow"));
}
#[test]
fn missing_empty_foreign_duplicate_and_platform_mixed_components_refuse() {
    let mut sources = vec![source("crates/xtask/src/unit.rs")];
    let data = Lines::from([(
        "crates/xtask/src/unit.rs".into(),
        [(2, 1)].into_iter().collect(),
    )]);
    sources.push(source("crates/xtask/src/new_unreported.rs"));
    assert!(summarize(&sources, &data, "windows", vec![]).is_err());
    sources.pop();
    assert!(summarize(&sources, &Lines::new(), "windows", vec![]).is_err());
    assert!(summarize(&[], &data, "windows", vec![]).is_err());
    assert!(summarize(&sources, &data, "macos", vec![]).is_err());
    let mut duplicate = sources.clone();
    duplicate.push(sources[0].clone());
    assert!(summarize(&duplicate, &data, "windows", vec![]).is_err());
    let mut foreign = data.clone();
    foreign.insert("foreign.rs".into(), BTreeMap::new());
    assert!(summarize(&sources, &foreign, "windows", vec![]).is_err());
    sources.push(source("crates/xtask/src/imap_proof_process_linux_075.rs"));
    let summary = summarize(&sources, &data, "windows", vec![]).unwrap();
    assert_eq!(summary.sources[1].disposition, "compiled_out_platform");
    foreign = data.clone();
    foreign.insert(sources[1].path.clone(), [(2, 1)].into_iter().collect());
    assert!(summarize(&sources, &foreign, "windows", vec![]).is_err());
    let mut comments = source("crates/xtask/src/comment_only.rs");
    comments.has_executable = false;
    sources.push(comments);
    let summary = summarize(&sources, &data, "windows", vec![]).unwrap();
    assert_eq!(
        summary.sources[2].disposition,
        "no_runtime_executable_items"
    );
}
#[test]
fn windows_spelling_normalization_never_hides_traversal_or_foreign_files() {
    assert_eq!(
        normalize_source("C:/Repo", r"\\?\C:\Repo\crates\xtask\src\unit.rs", &known()).unwrap(),
        "crates/xtask/src/unit.rs"
    );
    assert!(normalize_source("C:/Repo", r"c:\repo\CRATES\XTASK\src\unit.rs", &known()).is_ok());
    for bad in [
        "C:/Repo-other/crates/xtask/src/unit.rs",
        "C:/Else/unit.rs",
        "C:unit.rs",
        "C:/Repo/../unit.rs",
        "crates//xtask/src/unit.rs",
        "crates/xtask/src/unknown.rs",
        "C:/éééééééééé",
    ] {
        assert!(normalize_source("C:/Repo", bad, &known()).is_err(), "{bad}");
    }
    assert!(normalize_source("/tmp/repo", r"C:\Repo\crates\xtask\src\unit.rs", &known()).is_err());
    assert_eq!(
        normalize_source("/tmp/repo", "/tmp/repo/crates/xtask/src/unit.rs", &known()).unwrap(),
        "crates/xtask/src/unit.rs"
    );
    assert!(normalize_source("/tmp/repo", "", &known()).is_err());
    assert!(normalize_source("/tmp/repo", &"x".repeat(4097), &known()).is_err());
}
#[test]
fn llvm_lcov_is_bounded_complete_and_aggregates_distinct_object_records() {
    let valid=b"SF:C:/Repo/crates/xtask/src/unit.rs\nFN:2,fixture\nFNDA:1,fixture\nDA:2,1\nDA:3,0\nLF:2\nLH:1\nend_of_record\n";
    let data = parse_lcov(valid, "C:/Repo", &known()).unwrap();
    assert_eq!(data["crates/xtask/src/unit.rs"][&3], 0);
    let mut merged = valid.to_vec();
    merged.extend(b"SF:crates/xtask/src/unit.rs\nDA:3,2\nend_of_record\n");
    assert_eq!(
        parse_lcov(&merged, "C:/Repo", &known()).unwrap()["crates/xtask/src/unit.rs"][&3],
        2
    );
    for bad in [
        b"".as_slice(),
        b"DA:2,1\n",
        b"end_of_record\n",
        b"SF:crates/xtask/src/unit.rs\nDA:2,1\n",
        b"SF:crates/xtask/src/unit.rs\nDA:2,1\nDA:2,2\nend_of_record\n",
        b"SF:crates/xtask/src/unit.rs\nDA:0,1\nend_of_record\n",
        b"SF:crates/xtask/src/unit.rs\npassed:true\nend_of_record\n",
    ] {
        assert!(parse_lcov(bad, "C:/Repo", &known()).is_err());
    }
    assert!(parse_lcov(&vec![0; 32 * 1024 * 1024 + 1], "C:/Repo", &known()).is_err());
}
#[test]
fn future_stale_or_merged_java_sessions_cannot_qualify() {
    let path =
        "tests/java-imap-provenance-075/src/main/java/io/hydracache/imap/provenance/Fixture.java";
    let known = [path.into()].into_iter().collect();
    let valid = json!({"schema":"imap075-java-coverage-v1","sessions":[{"id":"fresh","start":1,"dump":2}],"sources":{"io/hydracache/imap/provenance/Fixture.java":{"2":1,"3":0}}});
    assert_eq!(
        java_lines(
            &valid,
            "java-imap-provenance-075",
            "fresh",
            "C:/Repo",
            &known
        )
        .unwrap()[path][&3],
        0
    );
    for mode in 0..6 {
        let mut value = valid.clone();
        match mode {
            0 => value["schema"] = json!("future"),
            1 => value["sessions"][0]["id"] = json!("stale"),
            2 => {
                value["sessions"]
                    .as_array_mut()
                    .unwrap()
                    .push(valid["sessions"][0].clone());
            }
            3 => value["sources"] = json!({}),
            4 => value["sources"] = json!({"../foreign.java":{"1":1}}),
            _ => value["sources"]["io/hydracache/imap/provenance/Fixture.java"]["0"] = json!(1),
        }
        assert!(
            java_lines(
                &value,
                "java-imap-provenance-075",
                "fresh",
                "C:/Repo",
                &known
            )
            .is_err(),
            "{mode}"
        );
    }
    assert!(java_lines(&valid, "foreign", "fresh", "C:/Repo", &known).is_err());
}
#[test]
fn coverage_header_binds_source_tools_cfg_profile_and_all_source_components() {
    let producer = xtask::imap_extended_receipt_075::Artifact {
        path: "target/fixture.exe".into(),
        bytes: 1,
        sha256: "c".repeat(64),
    };
    let control = Control {
        schema: "imap075-coverage-control-v1".into(),
        source_sha: "a".repeat(40),
        run_id: "b".repeat(32),
        production_enabled: false,
        profile: profile(),
        sources: vec![source("crates/xtask/src/unit.rs")],
        producer,
    };
    validate_control(
        &control,
        &control.source_sha,
        &control.profile,
        &control.sources,
    )
    .unwrap();
    for mode in 0..7 {
        let mut changed = control.clone();
        match mode {
            0 => changed.source_sha = "b".repeat(40),
            1 => changed.production_enabled = true,
            2 => changed.profile.os = "linux".into(),
            3 => changed.profile.tools[0].sha256 = "d".repeat(64),
            4 => changed.sources.clear(),
            5 => changed.schema = "future".into(),
            _ => changed.run_id = "not-unique".into(),
        }
        assert!(
            validate_control(
                &changed,
                &control.source_sha,
                &control.profile,
                &control.sources
            )
            .is_err(),
            "{mode}"
        );
    }
    let mut value = serde_json::to_value(control).unwrap();
    value["passed"] = json!(true);
    assert!(serde_json::from_value::<Control>(value).is_err());
}
#[test]
fn changed_line_ranges_and_cfg_disclosures_are_independent_of_comment_lines() {
    for diff in [
        "+++ b/../bad.rs\n@@ -1 +2 @@\n",
        "+++ b/C:/bad.rs\n@@ -1 +2 @@\n",
        "+++ b/good.rs\n@@ -1 +0 @@\n",
        "+++ b/good.rs\n@@ -1 +1,100001 @@\n",
    ] {
        assert!(added_lines(diff.as_bytes()).is_err());
    }
    assert!(added_lines(&vec![0; 8 * 1024 * 1024 + 1]).is_err());
    let root = tempfile::tempdir().unwrap();
    let path = "crates/xtask/src/cfg.rs";
    std::fs::create_dir_all(root.path().join("crates/xtask/src")).unwrap();
    std::fs::write(
        root.path().join(path),
        "#[cfg(unix)]\nfn compiled_out() { panic!(\"unreachable platform\"); }\nfn active() {}\n",
    )
    .unwrap();
    let mut source = source(path);
    source.added = [1, 2, 3].into_iter().collect();
    let contexts = BTreeMap::from([(path.into(), vec![target()])]);
    let ranges = compiled_out_ranges(root.path(), &[source.clone()], &contexts).unwrap();
    assert_eq!(ranges.len(), 1);
    assert_eq!(ranges[0].first, 1);
    assert!(ranges[0].last >= 2);
    assert!(compiled_out_ranges(root.path(), &[source], &BTreeMap::new()).is_err());
}
#[test]
fn trusted_runner_flags_are_explicit_and_do_not_use_the_live_target() {
    let root = Path::new("C:/Repo");
    let command = runner_command(root, Path::new("C:/Repo/target/raw"));
    let args = command
        .get_args()
        .map(|s| s.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert!(args.contains(&"imap-075-coverage".into()));
    assert!(args.contains(&"--locked".into()));
    let vars = command
        .get_envs()
        .map(|(k, v)| {
            (
                k.to_string_lossy().into_owned(),
                v.map(|v| v.to_string_lossy().into_owned()),
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert!(vars["CARGO_TARGET_DIR"]
        .as_ref()
        .unwrap()
        .ends_with("coverage-runner"));
    assert!(vars["CARGO_ENCODED_RUSTFLAGS"]
        .as_ref()
        .unwrap()
        .contains("instrument-coverage"));
    assert_eq!(vars["RUSTC_WRAPPER"], None);
    assert_eq!(vars["CARGO_INCREMENTAL"], Some("0".into()));
    assert_eq!(vars["RUST_TEST_THREADS"], Some("1".into()));
}
fn git(root: &Path, args: &[&str]) -> String {
    let result = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    String::from_utf8(result.stdout).unwrap().trim().into()
}
fn record(root: &Path, message: &str) {
    git(root, &["add", "."]);
    git(
        root,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-qm",
            message,
        ],
    );
}
#[test]
fn merge_scope_keeps_every_owned_file_before_the_patch_budget_and_binds_all_final_sources() {
    let root = tempfile::tempdir().unwrap();
    git(root.path(), &["init", "-q"]);
    git(root.path(), &["config", "core.autocrlf", "false"]);
    let directory = root.path().join("crates/xtask/src");
    std::fs::create_dir_all(&directory).unwrap();
    let shared = "pub fn inherited() -> u8 { 1 }\n// gap 1\n// gap 2\n// gap 3\n// gap 4\n";
    std::fs::write(directory.join("shared.rs"), shared).unwrap();
    record(root.path(), "common predecessor");
    let common = git(root.path(), &["rev-parse", "HEAD"]);
    std::fs::write(directory.join("foundation.rs"), "pub fn baseline() {}\n").unwrap();
    record(root.path(), "075 coverage baseline");
    let baseline = git(root.path(), &["rev-parse", "HEAD"]);
    git(root.path(), &["checkout", "-qb", "owned"]);
    std::fs::write(
        directory.join("shared.rs"),
        format!("{shared}pub fn added_075() -> u8 {{ 7 }}\n"),
    )
    .unwrap();
    std::fs::write(directory.join("owned.rs"), "pub fn owned() {}\n").unwrap();
    record(root.path(), "075 changes");
    git(root.path(), &["checkout", "--detach", &common]);
    std::fs::write(
        directory.join("shared.rs"),
        shared.replace("{ 1 }", "{ 2 }"),
    )
    .unwrap();
    for name in ["base_large_a.rs", "base_large_b.rs"] {
        std::fs::write(
            directory.join(name),
            format!("//{}\n", "x".repeat(4 * 1024 * 1024 + 1)),
        )
        .unwrap();
    }
    std::fs::write(
        directory.join("base_only.rs"),
        "pub fn predecessor_only() {}\n",
    )
    .unwrap();
    record(root.path(), "new published predecessor");
    git(root.path(), &["branch", "upstream"]);
    git(
        root.path(),
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "merge",
            "--no-ff",
            "--no-edit",
            "owned",
        ],
    );
    let first = source_snapshot_since(root.path(), &baseline).unwrap();
    assert!(first
        .iter()
        .filter(|s| s.path.contains("base_"))
        .all(|s| s.added.is_empty()));
    assert!(
        first
            .iter()
            .find(|s| s.path.ends_with("base_only.rs"))
            .unwrap()
            .has_executable
    );
    assert_eq!(
        first
            .iter()
            .find(|s| s.path.ends_with("owned.rs"))
            .unwrap()
            .added,
        [1].into_iter().collect()
    );
    // In a shared file the final baseline diff is conservative: both the 0.75
    // addition and the predecessor's changed line remain executable obligations.
    assert_eq!(
        first
            .iter()
            .find(|s| s.path.ends_with("shared.rs"))
            .unwrap()
            .added,
        [1, 6].into_iter().collect()
    );
    git(root.path(), &["checkout", "owned"]);
    git(
        root.path(),
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "merge",
            "--no-ff",
            "--no-edit",
            "upstream",
        ],
    );
    assert_eq!(
        source_snapshot_since(root.path(), &baseline).unwrap(),
        first
    );
}
#[test]
fn snapshot_discovers_new_scoped_files_from_real_git_and_hashes_comments() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join(".gitignore"), "target/\n").unwrap();
    git(root.path(), &["init", "-q"]);
    git(root.path(), &["add", ".gitignore"]);
    git(
        root.path(),
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-qm",
            "baseline",
        ],
    );
    let baseline = git(root.path(), &["rev-parse", "HEAD"]);
    // Unrelated source changes must not exhaust the declared scope's Git byte budget.
    std::fs::write(
        root.path().join("large-unrelated.rs"),
        format!("//{}\n", "x".repeat(8 * 1024 * 1024 + 1)),
    )
    .unwrap();
    for (path, text) in [
        ("crates/xtask/src/new.rs", "pub fn added() {}\n"),
        ("crates/xtask/src/comments.rs", "// comment only\n"),
        (
            "tests/java-imap-provenance-075/src/main/java/Fixture.java",
            "/* { comment */ public class Fixture { void run() {} }\n",
        ),
        (
            "tests/java-imap-provenance-075/src/main/java/Comments.java",
            "// { comment\n/* { ignored */\n\"{ quoted\"\n",
        ),
        ("outside.rs", "fn ignored() {}"),
    ] {
        let file = root.path().join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, text).unwrap();
    }
    git(root.path(), &["add", "."]);
    git(
        root.path(),
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-qm",
            "changes",
        ],
    );
    let first = source_snapshot_since(root.path(), &baseline).unwrap();
    assert_eq!(first.len(), 4);
    assert_eq!(first.iter().filter(|s| s.has_executable).count(), 2);
    assert!(first.iter().all(|s| !s.added.is_empty()));
    let path = "crates/xtask/src/comments.rs";
    std::fs::write(root.path().join(path), "// a different comment\n").unwrap();
    let second = source_snapshot_since(root.path(), &baseline).unwrap();
    assert_ne!(
        first.iter().find(|s| s.path == path).unwrap().sha256,
        second.iter().find(|s| s.path == path).unwrap().sha256
    );
    assert!(source_snapshot_since(root.path(), "bad-baseline").is_err());
    assert!(source_snapshot_since(root.path(), &"0".repeat(40)).is_err());
    assert!(source_snapshot_since(root.path(), "HEAD").is_err());
    let current = git(root.path(), &["rev-parse", "HEAD"]);
    assert!(source_snapshot_since(root.path(), &current).is_err());
    for mode in ["capture", "worker", "analysis", "check", "unknown"] {
        assert!(run(vec![], mode).is_err());
        assert!(run(
            vec![
                "--release".into(),
                "0.75".into(),
                "--foreign".into(),
                "target/bad".into()
            ],
            mode
        )
        .is_err());
    }
    for index in 0..2049 {
        std::fs::write(
            root.path()
                .join(format!("crates/xtask/src/bound_{index}.rs")),
            "// bounded source fixture\n",
        )
        .unwrap();
    }
    record(root.path(), "changed source work bound");
    assert!(source_snapshot_since(root.path(), &baseline)
        .unwrap_err()
        .to_string()
        .contains("changed-source work bound"));
    let wide = git(root.path(), &["rev-parse", "HEAD"]);
    std::fs::write(
        root.path().join("crates/xtask/src/new.rs"),
        "pub fn changed_again() {}\n",
    )
    .unwrap();
    record(root.path(), "all tracked source work bound");
    assert!(source_snapshot_since(root.path(), &wide)
        .unwrap_err()
        .to_string()
        .contains("source work bound"));
}
struct FakeDriver {
    calls: usize,
    fail: usize,
}
impl CoverageDriver for FakeDriver {
    fn execute(
        &mut self,
        stage: xtask::imap_build_process_075::BuildStage,
        _: Command,
    ) -> Result<xtask::imap_build_process_075::BuildReport, Box<dyn std::error::Error>> {
        use xtask::imap_build_process_075::*;
        self.calls += 1;
        if self.calls == self.fail {
            return Err("fixture stage failed".into());
        }
        Ok(BuildReport {
            bounds: BuildBounds::default(),
            scope: "fixture-unqualified".into(),
            stage,
            outcome: BuildOutcome::Completed,
            elapsed_ms: 1,
            effective_timeout_ms: 100,
            stdout_path: "fixture".into(),
            stderr_path: "fixture".into(),
            stdout_bytes: 0,
            stderr_bytes: 0,
            stdout_sha256: "a".repeat(64),
            stderr_sha256: "a".repeat(64),
            pid: None,
            root_reaped: true,
            remaining_live_members: Some(0),
            containment: "fixture".into(),
            journal_complete: true,
            logs_complete: true,
        })
    }
}
#[test]
fn pipeline_uses_a_controller_copy_stops_on_failure_and_never_overwrites_output() {
    for fail in [0, 1, 2, 3] {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join(".gitignore"), "target/\n").unwrap();
        git(root.path(), &["init", "-q"]);
        git(root.path(), &["add", ".gitignore"]);
        git(
            root.path(),
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "commit",
                "-qm",
                "fixture",
            ],
        );
        let sha = git(root.path(), &["rev-parse", "HEAD"]);
        let runner = root.path().join("target/imap-075/coverage-runner/debug");
        std::fs::create_dir_all(&runner).unwrap();
        std::fs::write(
            runner.join(if cfg!(windows) { "xtask.exe" } else { "xtask" }),
            "fixture bytes; never executed",
        )
        .unwrap();
        let mut driver = FakeDriver { calls: 0, fail };
        let output = PathBuf::from("target/imap-075/coverage-fixture");
        let result = execute_pipeline(
            root.path(),
            &output,
            sha,
            profile(),
            vec![source("crates/xtask/src/unit.rs")],
            &mut driver,
        );
        if fail == 0 {
            let path = result.unwrap();
            assert!(path.ends_with("capture.json"));
            assert_eq!(driver.calls, 3);
            let control: Control = serde_json::from_slice(
                &std::fs::read(root.path().join(&output).join("control.json")).unwrap(),
            )
            .unwrap();
            assert!(control.producer.path.contains("/controller/"));
            assert!(!control.production_enabled);
        } else {
            assert!(result.is_err());
            assert_eq!(driver.calls, fail);
        }
        let mut driver = FakeDriver { calls: 0, fail: 0 };
        assert!(execute_pipeline(
            root.path(),
            &output,
            "a".repeat(40),
            profile(),
            vec![],
            &mut driver
        )
        .is_err());
        assert_eq!(driver.calls, 0);
    }
}
