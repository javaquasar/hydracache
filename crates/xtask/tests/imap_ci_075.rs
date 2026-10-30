use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use xtask::imap_ci_075::*;
use xtask::imap_qualification_075::digest;
const WORKFLOW: &str = include_str!("../../../.github/workflows/imap-075.yml");
#[test]
fn closed_075_registry_extension_refuses_foreign_metadata_and_missing_workflow() {
    use serde_json::json;
    let root = tempfile::tempdir().unwrap();
    assert!(xtask::imap_gated_registry_075::load(root.path())
        .unwrap()
        .is_empty());
    let path = root
        .path()
        .join(xtask::imap_gated_registry_075::REGISTRY_PATH);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let original: toml::Value = toml::from_str(include_str!(
        "../../../docs/testing/imap/0.75/gated-test-registry.toml"
    ))
    .unwrap();
    let original = serde_json::to_value(original).unwrap();
    fs::write(&path, toml::to_string(&original).unwrap()).unwrap();
    assert!(xtask::imap_gated_registry_075::load(root.path()).is_err());
    let workflow = root.path().join(".github/workflows/imap-075.yml");
    fs::create_dir_all(workflow.parent().unwrap()).unwrap();
    fs::write(&workflow, WORKFLOW).unwrap();
    assert_eq!(
        xtask::imap_gated_registry_075::load(root.path())
            .unwrap()
            .len(),
        4
    );
    for mode in 0..21 {
        let mut value = original.clone();
        match mode {
            0 => value["schema_version"] = json!(2),
            1 => value["release"] = json!("0.74.0"),
            2 => {
                value["gate"].as_array_mut().unwrap().pop();
            }
            3 => value["gate"][0]["id"] = json!("foreign"),
            4 => value["gate"][1] = value["gate"][0].clone(),
            5 => value["gate"][0]["owner_release"] = json!("0.74.0"),
            6 => value["gate"][0]["source"] = json!("outside.rs"),
            7 => value["gate"][0]["command"]["program"] = json!("sh"),
            8 => value["gate"][0]["command"]["args"] = json!(["echo", "pass"]),
            9 => value["gate"][0]["command"]["env"] = json!({"RUSTC_WRAPPER":"foreign"}),
            10 => value["gate"][1]["required_env"] = json!([]),
            11 => value["gate"][0]["timeout_seconds"] = json!(0),
            12 => value["gate"][0]["required_features"] = json!(["foreign"]),
            13 => value["gate"][0]["reason"] = json!(""),
            14 => value["gate"][0]["tier"] = json!("manual"),
            15 => value["gate"][0]["command"]["platform"] = json!("linux"),
            16 => value["gate"][0]["ci"]["workflow"] = json!("foreign.yml"),
            17 => value["gate"][0]["ci"]["step"] = json!("Skipped"),
            18 => value["gate"][0]["ship_mandatory"] = json!(false),
            19 => value["gate"][0]["artifacts"] = json!(["outside"]),
            _ => value["future"] = json!(true),
        }
        fs::write(&path, toml::to_string(&value).unwrap()).unwrap();
        assert!(
            xtask::imap_gated_registry_075::load(root.path()).is_err(),
            "{mode}"
        );
    }
    fs::write(&path, toml::to_string(&original).unwrap()).unwrap();
    fs::write(
        &workflow,
        WORKFLOW.replace("name: Run bounded local proofs", "name: Renamed"),
    )
    .unwrap();
    assert!(xtask::imap_gated_registry_075::load(root.path())
        .unwrap_err()
        .to_string()
        .contains("workflow wiring differs"));
    fs::write(&path, vec![b'x'; 64 * 1024 + 1]).unwrap();
    assert!(xtask::imap_gated_registry_075::load(root.path()).is_err());
}
#[test]
fn actual_preparation_tests_are_registered_and_reach_the_bounded_qualification_step() {
    let root = root();
    let mut registry = xtask::gated_tests::load_registry(&root).unwrap();
    registry
        .gate
        .extend(xtask::imap_gated_registry_075::load(&root).unwrap());
    let entries = registry
        .gate
        .into_iter()
        .filter(|gate| gate.id.starts_with("ignored.xtask.imap075."))
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 4);
    assert!(entries.iter().all(|gate| gate.owner_release == "0.75.0"
        && gate.ship_mandatory
        && gate.command.args.iter().any(|arg| arg == "--ignored")
        && gate.command.args.iter().any(|arg| arg == &gate.test)));
    assert!(
        xtask::release_governance::ci_wiring_problems(&root, &entries)
            .unwrap()
            .is_empty()
    );
    let problems = xtask::gated_tests::check_registry(&root).unwrap();
    assert!(
        !problems
            .iter()
            .any(|problem| problem.contains("imap_soak_075") || problem.contains("imap_sbom_075")),
        "{problems:#?}"
    );
}
#[test]
fn topology_checks_both_075_extensions_and_rejects_duplicate_registration() {
    let root = root();
    let input = root.join("docs/testing/ci-topology.json");
    let input = if input.exists() {
        input
    } else {
        root.join("docs/testing/memory/0.71/ci-topology.json")
    };
    let original: serde_json::Value = serde_json::from_slice(&fs::read(input).unwrap()).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let fixture = temp.path().join("topology.json");
    let mut invalid = original.clone();
    invalid["schema_version"] = serde_json::json!(0);
    fs::write(&fixture, serde_json::to_vec(&invalid).unwrap()).unwrap();
    let error = xtask::ci_topology::check_with_path(&root, "0.71", &fixture)
        .unwrap_err()
        .to_string();
    assert!(error.contains("schema_version must be 1"), "{error}");
    for registry in [
        "ci-topology-extension.json",
        "soak-ci-topology-extension.json",
    ] {
        let extension: serde_json::Value = serde_json::from_slice(
            &fs::read(root.join("docs/testing/imap/0.75").join(registry)).unwrap(),
        )
        .unwrap();
        let mut duplicated = original.clone();
        duplicated["workflows"]
            .as_array_mut()
            .unwrap()
            .push(extension);
        fs::write(&fixture, serde_json::to_vec(&duplicated).unwrap()).unwrap();
        assert!(xtask::ci_topology::check_with_path(&root, "0.71", &fixture)
            .unwrap_err()
            .to_string()
            .contains("unique 0.75 workflow"));
    }
    let isolated = tempfile::tempdir().unwrap();
    let extension_dir = isolated.path().join("docs/testing/imap/0.75");
    fs::create_dir_all(&extension_dir).unwrap();
    fs::write(&fixture, serde_json::to_vec(&original).unwrap()).unwrap();
    for name in [
        "ci-topology-extension.json",
        "soak-ci-topology-extension.json",
    ] {
        let extension = extension_dir.join(name);
        fs::write(&extension, b"{").unwrap();
        assert!(
            xtask::ci_topology::check_with_path(isolated.path(), "0.71", &fixture)
                .unwrap_err()
                .to_string()
                .contains("EOF")
        );
        fs::remove_file(extension).unwrap();
    }
}
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}
fn receipts(mode: &str) -> Vec<CiReceipt> {
    expected_lanes(mode)
        .unwrap()
        .into_iter()
        .map(|lane| CiReceipt {
            release: "0.75".into(),
            receipt_version: 3,
            scope: "local_ci".into(),
            mode: mode.into(),
            source_sha: "a".repeat(40),
            workflow_sha256: digest(WORKFLOW.as_bytes()),
            coverage_sha256: "b".repeat(64),
            seeds: seeds_for(lane.shard).unwrap(),
            steps: 128,
            rust_fingerprint: format!("rustc {} (test fixture)", lane.rust),
            java_fingerprint: format!("openjdk {}.0.1 test fixture", lane.java),
            lane,
            commands: vec![
                "imap-distributed-correctness".into(),
                "imap-capability-audit".into(),
                "imap-migration-dry-run".into(),
                "imap-comparison-check".into(),
                "imap-release-admission".into(),
                "imap-tooling-tests".into(),
                "imap-semantic-audit".into(),
                "imap-cluster-chaos".into(),
                "imap-extended-concurrent".into(),
                "imap-extended-evidence".into(),
                "imap-pre074-preparation".into(),
                "imap-scoped-coverage".into(),
            ],
            completed: true,
            passed: true,
            production_enabled: false,
        })
        .collect()
}
#[test]
fn pr_and_nightly_matrices_have_exact_bounded_toolchains_and_disjoint_seed_shards() {
    check_workflow(&root()).unwrap();
    assert_eq!(expected_lanes("pr").unwrap().len(), 4);
    assert_eq!(expected_lanes("nightly").unwrap().len(), 16);
    for mode in ["pr", "nightly"] {
        validate_receipts(
            &receipts(mode),
            mode,
            &"a".repeat(40),
            &digest(WORKFLOW.as_bytes()),
        )
        .unwrap();
    }
    assert_eq!(seeds_for(3).unwrap(), [123, 124]);
    assert!(seeds_for(4).is_err());
    assert!(expected_lanes("full").is_err());
}
#[test]
fn incomplete_failed_mixed_source_or_weakened_lanes_never_complete_ci() {
    for field in [
        "release",
        "receipt_version",
        "coverage_sha256",
        "scope",
        "mode",
        "source_sha",
        "workflow_sha256",
        "seeds",
        "steps",
        "rust_fingerprint",
        "java_fingerprint",
        "commands",
        "completed",
        "passed",
        "production_enabled",
    ] {
        let mut lanes = receipts("pr");
        let mut value = serde_json::to_value(&lanes[0]).unwrap();
        value[field] = match field {
            "completed" | "passed" => serde_json::json!(false),
            "production_enabled" => serde_json::json!(true),
            "seeds" => serde_json::json!([999]),
            "commands" => serde_json::json!([]),
            "steps" | "receipt_version" => serde_json::json!(0),
            _ => serde_json::json!("wrong"),
        };
        lanes[0] = serde_json::from_value(value).unwrap();
        assert!(
            validate_receipts(&lanes, "pr", &"a".repeat(40), &digest(WORKFLOW.as_bytes())).is_err(),
            "{field}"
        );
    }
    let mut lanes = receipts("pr");
    lanes.pop();
    assert!(
        validate_receipts(&lanes, "pr", &"a".repeat(40), &digest(WORKFLOW.as_bytes())).is_err()
    );
    let mut lanes = receipts("pr");
    lanes.push(lanes[0].clone());
    assert!(
        validate_receipts(&lanes, "pr", &"a".repeat(40), &digest(WORKFLOW.as_bytes())).is_err()
    );
    let mut lanes = receipts("pr");
    lanes[0].lane.rust = "1.99.0".into();
    assert!(
        validate_receipts(&lanes, "pr", &"a".repeat(40), &digest(WORKFLOW.as_bytes())).is_err()
    );
}
#[test]
fn workflow_canaries_reject_unpinned_actions_ignored_failures_and_missing_completion() {
    for changed in [
        WORKFLOW.replace("'fuzz/**', ", ""),
        WORKFLOW.replace("'tests/java-imap-hazelcast-075/**', ", ""),
        WORKFLOW.replace("'tests/rust-imap-contract-consumer-075/**', ", ""),
        WORKFLOW.replace("'.github/workflows/ci.yml', ", ""),
        WORKFLOW.replace("paths: [", "paths-ignore: ["),
        WORKFLOW.replace("fbc6f3992d24b796d5a048ff273f7fcc4a7b6c09", "v5"),
        WORKFLOW.replace("fail-fast: false", "fail-fast: true"),
        WORKFLOW.replace("timeout-minutes: 45", "timeout-minutes: 999"),
        WORKFLOW.replace("if: always()", "if: success()"),
        WORKFLOW.replace("imap-ci-run", "echo skipped"),
        WORKFLOW.replace("test \"$LANES_RESULT\" = success", "true"),
        WORKFLOW
            .replace(
                "retention-days: 14",
                "retention-days: 14\n          unused: true",
            )
            .replace("--receipts", "--missing"),
    ] {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path().join(".github/workflows")).unwrap();
        fs::write(temp.path().join(".github/workflows/imap-075.yml"), changed).unwrap();
        assert!(check_workflow(temp.path()).is_err());
    }
}

#[test]
fn both_evidence_jobs_require_complete_unfiltered_history() {
    for job in ["qualification", "complete"] {
        for weakened in [
            serde_yaml::Value::Null,
            serde_yaml::from_str("fetch-depth: 1").unwrap(),
            serde_yaml::from_str("fetch-depth: 0\nfilter: blob:none").unwrap(),
            serde_yaml::from_str("fetch-depth: 0\nsparse-checkout: crates").unwrap(),
        ] {
            let mut workflow: serde_yaml::Value = serde_yaml::from_str(WORKFLOW).unwrap();
            let checkout = workflow["jobs"][job]["steps"]
                .as_sequence_mut()
                .unwrap()
                .iter_mut()
                .find(|step| {
                    step["uses"]
                        .as_str()
                        .is_some_and(|action| action.starts_with("actions/checkout@"))
                })
                .expect("the evidence job must contain a checkout");
            checkout["with"] = weakened;
            let temp = tempfile::tempdir().unwrap();
            fs::create_dir_all(temp.path().join(".github/workflows")).unwrap();
            fs::write(
                temp.path().join(".github/workflows/imap-075.yml"),
                serde_yaml::to_string(&workflow).unwrap(),
            )
            .unwrap();
            assert!(check_workflow(temp.path()).is_err(), "{job}");
        }
    }
}

#[test]
fn build_deadline_cannot_be_removed_extended_or_started_after_checkout() {
    for mode in 0..3 {
        let mut workflow: serde_yaml::Value = serde_yaml::from_str(WORKFLOW).unwrap();
        let steps = workflow["jobs"]["qualification"]["steps"]
            .as_sequence_mut()
            .unwrap();
        let at = steps
            .iter()
            .position(|step| step["name"].as_str() == Some("Qualification budget"))
            .unwrap();
        match mode {
            0 => {
                steps.remove(at);
            }
            1 => {
                steps[at]["run"] = serde_yaml::Value::String("echo unbounded".into());
            }
            _ => {
                let step = steps.remove(at);
                steps.push(step);
            }
        }
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join(".github/workflows")).unwrap();
        fs::write(
            root.path().join(".github/workflows/imap-075.yml"),
            serde_yaml::to_string(&workflow).unwrap(),
        )
        .unwrap();
        assert!(check_workflow(root.path()).is_err());
    }
}

#[test]
fn retained_evidence_is_missing_in_a_real_shallow_checkout_and_restored_with_history() {
    fn git(directory: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .current_dir(directory)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }
    let temp = tempfile::tempdir().unwrap();
    let origin = temp.path().join("origin");
    fs::create_dir(&origin).unwrap();
    git(&origin, &["init", "-b", "retained-history"]);
    fs::write(
        origin.join("evidence.txt"),
        "accepted historical evidence\n",
    )
    .unwrap();
    git(&origin, &["add", "evidence.txt"]);
    let commit = [
        "-c",
        "user.name=IMap fixture",
        "-c",
        "user.email=fixture@example.invalid",
        "commit",
        "-m",
        "retain evidence",
    ];
    git(&origin, &commit);
    let retained = git(&origin, &["rev-parse", "HEAD"]);
    fs::write(origin.join("evidence.txt"), "current candidate\n").unwrap();
    git(&origin, &["add", "evidence.txt"]);
    git(&origin, &commit);
    git(
        temp.path(),
        &[
            "clone",
            "--no-local",
            "--depth",
            "1",
            origin.to_str().unwrap(),
            "checkout",
        ],
    );
    let checkout = temp.path().join("checkout");
    assert_eq!(
        git(&checkout, &["rev-parse", "--is-shallow-repository"]),
        "true"
    );
    assert!(!Command::new("git")
        .current_dir(&checkout)
        .args(["cat-file", "-e", &format!("{retained}^{{commit}}")])
        .output()
        .unwrap()
        .status
        .success());
    git(&checkout, &["fetch", "--unshallow", "origin"]);
    assert_eq!(
        git(&checkout, &["rev-parse", "--is-shallow-repository"]),
        "false"
    );
    assert_eq!(
        git(&checkout, &["show", &format!("{retained}:evidence.txt")]),
        "accepted historical evidence"
    );
}
#[test]
fn receipt_writes_are_bounded_idempotent_and_never_overwrite_conflicting_evidence() {
    let temp = tempfile::tempdir().unwrap();
    let receipt = receipts("pr").remove(0);
    let path = "target/ci/ci-receipt-test.json";
    write_receipt(temp.path(), path, &receipt).unwrap();
    write_receipt(temp.path(), path, &receipt).unwrap();
    let mut changed = receipt.clone();
    changed.passed = false;
    assert!(write_receipt(temp.path(), path, &changed).is_err());
    let stored: CiReceipt =
        serde_json::from_slice(&fs::read(temp.path().join(path)).unwrap()).unwrap();
    assert!(stored.passed);
    for path in [
        "",
        "../escape.json",
        "/absolute.json",
        "C:\\escape.json",
        "target\\ci\\ambiguous.json",
    ] {
        assert!(write_receipt(temp.path(), path, &receipt).is_err());
    }
    assert!(check_receipts(temp.path(), "../outside", "pr").is_err());
}
#[test]
fn cli_rejects_ambiguous_modes_and_false_lane_identity_before_running_work() {
    for command in ["imap-ci-check", "imap-ci-run"] {
        for args in [
            vec!["--release", "0.74"],
            vec!["--release"],
            vec!["--release", "0.75", "--mode", "invalid"],
            vec!["--release", "0.75", "--java", "99"],
            vec!["--release", "0.75", "--release", "0.75"],
        ] {
            assert!(!Command::new(env!("CARGO_BIN_EXE_xtask"))
                .current_dir(root())
                .arg(command)
                .args(args)
                .output()
                .unwrap()
                .status
                .success());
        }
    }
}

#[test]
fn tooling_isolates_the_live_producer_on_every_platform_and_preserves_gate_arguments() {
    let root = root();
    let isolated = root.join("target/imap-075/tooling");
    for windows in [false, true] {
        let command = tooling_command(&root, windows);
        assert_eq!(command.get_program(), "cargo");
        assert_eq!(command.get_current_dir(), Some(root.as_path()));
        assert_eq!(
            command
                .get_args()
                .map(|arg| arg.to_str().unwrap())
                .collect::<Vec<_>>(),
            [
                "test",
                "-p",
                "xtask",
                "--lib",
                "--test",
                "imap_contract_075",
                "--test",
                "imap_capability_audit_075",
                "--test",
                "imap_migration_dry_run_075",
                "--test",
                "imap_qualification_075",
                "--test",
                "imap_ci_075",
                "--test",
                "imap_semantic_audit075",
                "--test",
                "imap_cluster_campaign_075",
                "--test",
                "imap_cluster_fuzz_policy075",
                "--test",
                "imap_foundation_generate_075",
                "--test",
                "imap_extended_differential_075",
                "--test",
                "imap_extended_concurrent_075",
                "--test",
                "imap_proof_process_075",
                "--test",
                "imap_build_process_075",
                "--test",
                "imap_extended_receipt_075",
                "--test",
                "imap_daemon_probe_075",
                "--test",
                "imap_storage_probe_075",
                "--test",
                "imap_pre074_075",
                "--test",
                "imap_native_probe_075",
                "--test",
                "imap_native_concurrent_075",
                "--test",
                "imap_native_recovery_075",
                "--test",
                "imap_soak_075",
                "--test",
                "imap_sbom_075",
                "--test",
                "imap_coverage_075",
                "--locked",
            ]
        );
        let child_target = command
            .get_envs()
            .find(|(name, _)| name == &"CARGO_TARGET_DIR")
            .map(|(_, value)| value);
        assert_eq!(child_target, Some(Some(isolated.as_os_str())));
        for (name, value) in [
            ("CARGO_PROFILE_DEV_DEBUG", "0"),
            ("CARGO_PROFILE_TEST_DEBUG", "0"),
            ("CARGO_INCREMENTAL", "0"),
        ] {
            assert_eq!(
                command
                    .get_envs()
                    .find(|(key, _)| key == &name)
                    .and_then(|(_, v)| v)
                    .and_then(|v| v.to_str()),
                Some(value)
            );
        }
    }
}
