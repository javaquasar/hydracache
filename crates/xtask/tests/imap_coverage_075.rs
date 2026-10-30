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
    assert!(source_snapshot_since(root.path(), "HEAD").is_err());
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
