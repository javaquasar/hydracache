use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn enrolled_public_builder_policy_keeps_execution_admission_closed() {
    use hydracache_long_run_supervisor_074::diagnostic_builder::{decode_key, load_policy};
    let c = contract("diagnostic-builder-enrollment-contract.toml");
    assert_eq!(c["github_reviewer_login"].as_str(), Some("javaquasar"));
    assert_eq!(c["github_reviewer_id"].as_integer(), Some(22050429));
    assert_eq!(
        c["technical_review"].as_str(),
        Some("agent-delegated-by-user")
    );
    assert_eq!(c["required_reviewer_gate"].as_bool(), Some(true));
    assert_eq!(c["fresh_separate_seed_required"].as_bool(), Some(true));
    assert_eq!(c["manual_workflow_only"].as_bool(), Some(true));
    assert_eq!(
        c["default_branch_registration_requires_separate_permission"].as_bool(),
        Some(true)
    );
    for field in [
        "independent_human_review",
        "prevent_self_review",
        "admin_bypass_used",
        "admin_bypass_permitted",
        "private_key_in_repository_allowed",
        "private_key_on_performance_host_allowed",
        "host_install_allowed",
        "product_workload_allowed",
        "qualification_allowed",
        "admission_allowed",
    ] {
        assert_eq!(c[field].as_bool(), Some(false), "{field}");
    }
    let raw = std::fs::read(
        root().join("docs/testing/performance/0.74/diagnostic-builder-public-policy.json"),
    )
    .unwrap();
    let pin = "dfe81cf770088029e03f04faa4da429ef163583c0ba1b7002b8ec5606e766e22";
    let actual: String = Sha256::digest(&raw)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(actual, pin);
    let controller = decode_key(c["controller_public_key_hex"].as_str().unwrap()).unwrap();
    assert!(load_policy(&raw, pin, &controller).is_ok());
    assert!(load_policy(&raw, &"0".repeat(64), &controller).is_err());
    let directory = root()
        .join("docs/testing/performance/0.74/local-runs/diagnostic-builder-enrollment-20261009");
    let packet: Value =
        serde_json::from_slice(&std::fs::read(directory.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(
        packet["schema_version"],
        "diagnostic-builder-enrollment-evidence-074-v1"
    );
    assert_eq!(packet["public_policy_sha256"], pin);
    assert_eq!(
        packet["checked_source_commit"],
        "b4b62e1e4a21ab3c6e23d2d57cb3c5412e78c30a"
    );
    assert_eq!(packet["performance_contract_tests"], 78);
    for field in [
        "real_builder_key_generated",
        "real_builder_key_registered",
        "required_reviewer_gate",
    ] {
        assert_eq!(packet[field], true, "{field}");
    }
    for field in [
        "can_admins_bypass",
        "real_build_signature_verified",
        "workflow_dispatched",
        "github_pending_deployment_approved",
        "main_modified",
        "host_modified",
        "host_install_allowed",
        "product_workload_run",
        "qualification_run",
        "admission_allowed",
    ] {
        assert_eq!(packet[field], false, "{field}");
    }
    let api: Value = serde_json::from_slice(
        &std::fs::read(directory.join("github-public-metadata.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(api["environment"]["can_admins_bypass"], false);
    assert_eq!(api["branches"]["total_count"], 1);
    assert_eq!(api["branches"]["branch_policies"][0]["type"], "branch");
    assert_eq!(
        api["branches"]["branch_policies"][0]["name"],
        "feat/0.74-resp-native-throughput"
    );
    assert_eq!(api["workflow_lookup"]["status"], "404");
    assert_eq!(api["default_branch_file_lookup"]["status"], "404");
    assert_eq!(api["secret_metadata"]["total_count"], 1);
    assert_eq!(
        api["secret_metadata"]["secrets"][0]["name"],
        "HYDRACACHE_074_BUILDER_SIGNING_KEY_HEX"
    );
    for artifact in packet["raw_files"].as_array().unwrap() {
        let name = artifact["name"].as_str().unwrap();
        assert_eq!(Path::new(name).components().count(), 1);
        let bytes = std::fs::read(directory.join(name)).unwrap();
        assert_eq!(artifact["bytes"].as_u64().unwrap(), bytes.len() as u64);
        let actual: String = Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(artifact["sha256"], actual);
    }
}

#[test]
fn separate_diagnostic_builder_does_not_enroll_host_execution() {
    let c = contract("diagnostic-builder-local-contract.toml");
    assert_eq!(c["repository_id"].as_integer(), Some(1217101761));
    assert_eq!(
        c["builder_id"].as_str(),
        Some("hydracache-linux-observer-074-v1")
    );
    for field in [
        "separate_provisioning_key_required",
        "external_policy_digest_required",
    ] {
        assert_eq!(c[field].as_bool(), Some(true));
    }
    for field in [
        "build_job_has_signing_secret",
        "sign_job_builds_observer",
        "automatic_workflow_trigger",
        "host_install_allowed",
        "product_workload_allowed",
        "qualification_allowed",
        "production_route_enabled",
        "real_key_enrolled_by_local_tests",
        "admission_allowed",
        "old_unsigned_build_promotable",
    ] {
        assert_eq!(c[field].as_bool(), Some(false));
    }
    for name in ["main.rs", "server.rs", "protocol.rs", "config.rs"] {
        let code =
            std::fs::read_to_string(root().join("tools/long-run-supervisor-074/src").join(name))
                .unwrap();
        assert!(!code.contains("diagnostic_builder"), "{name}");
    }
    let workflow =
        std::fs::read_to_string(root().join(".github/workflows/diagnostic-builder-074.yml"))
            .unwrap();
    let parsed: serde_yaml::Value = serde_yaml::from_str(&workflow).unwrap();
    assert_eq!(parsed["on"].as_mapping().unwrap().len(), 1);
    assert!(parsed["on"]
        .as_mapping()
        .unwrap()
        .contains_key("workflow_dispatch"));
    assert_eq!(parsed["jobs"].as_mapping().unwrap().len(), 2);
    assert_eq!(parsed["jobs"]["sign"]["needs"].as_str(), Some("build"));
    let (build, sign) = workflow.split_once("  sign:").unwrap();
    assert!(!build.contains("secrets."));
    assert!(sign.contains("environment: performance-diagnostic-builder-074"));
    assert!(sign.contains("unset BUILDER_SIGNING_KEY_HEX"));
    for forbidden in [
        "self-hosted",
        "sudo",
        "HYDRACACHE_074_AUTH_SIGNING_KEY_HEX",
        "--run ",
        "workflow_call:",
        "  push:",
    ] {
        assert!(!workflow.contains(forbidden));
    }
    assert_eq!(workflow.matches("overwrite: false").count(), 2);
    assert_eq!(workflow.matches("check-protection").count(), 2);
    let coordinator =
        std::fs::read_to_string(root().join("scripts/perf/diagnostic_builder_074.py")).unwrap();
    assert!(coordinator.contains("environment.get(\"can_admins_bypass\") is not False"));
    let provisioning = std::fs::read_to_string(
        root().join(".github/workflows/performance-long-run-host-provision-074.yml"),
    )
    .unwrap();
    assert!(!provisioning.contains("BUILDER_SIGNING_KEY"));
    assert!(!provisioning.contains("diagnostic_builder"));
    let manifest =
        std::fs::read_to_string(root().join("tools/long-run-supervisor-074/Cargo.toml")).unwrap();
    assert!(manifest.contains("default-run = \"hydracache-long-run-supervisor-074\""));
    let directory =
        root().join("docs/testing/performance/0.74/local-runs/diagnostic-builder-6118219f");
    let packet: Value =
        serde_json::from_slice(&std::fs::read(directory.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(
        packet["schema_version"],
        "diagnostic-builder-local-evidence-074-v1"
    );
    assert_eq!(
        packet["implementation_source_commit"],
        "6118219fb39353ca9007f0581423e965b0e11cf9"
    );
    assert_eq!(
        packet["implementation_source_tree"],
        "d78bf6b3406ec834a469626d8deb789a39fe78e0"
    );
    for flag in [
        "real_builder_key_generated",
        "real_builder_key_enrolled",
        "real_signed_build_produced",
        "workflow_dispatched",
        "host_install_allowed",
        "product_workload_run",
        "qualification_run",
        "admission_allowed",
        "full_workspace_verify_run",
    ] {
        assert_eq!(packet[flag], false, "{flag}");
    }
    assert_eq!(packet["windows_rust_tests"], 23);
    assert_eq!(packet["linux_rust_tests"], 31);
    assert_eq!(packet["python_tests_per_os"], 7);
    assert_eq!(packet["ship_check_expected_exit"], 1);
    let expected = [
        (
            "linux-format-preparation.log",
            "08c5046690afc08044c6d47823da0877136c965186f692a9220433b47ce0948e",
        ),
        (
            "linux.log",
            "e8e4916e17b97dd8146c616359d71e066829159c75ecf0d7e6a9738b2d06684d",
        ),
        (
            "root.log",
            "6d5cbddf256c8e6fcbb7cb9370de25d60a0bb060f84079b152ad5e40a5be2b4c",
        ),
        (
            "test-first.log",
            "1e01a76f24b823ac110d788ca19938e2c899cc496dc912522f4409e400a751c5",
        ),
        (
            "windows.log",
            "bb1917048b546522bf44bcd2908695622ed5c45dfea1e704e66c65b3beb918c9",
        ),
    ];
    let rows = packet["raw_files"].as_array().unwrap();
    assert_eq!(rows.len(), expected.len());
    for (name, digest) in expected {
        let row = rows.iter().find(|r| r["name"] == name).unwrap();
        let bytes = std::fs::read(directory.join(name)).unwrap();
        assert_eq!(row["bytes"].as_u64(), Some(bytes.len() as u64));
        assert_eq!(row["sha256"], digest);
        let actual: String = Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(actual, digest);
    }
    assert!(std::fs::read_to_string(directory.join("test-first.log"))
        .unwrap()
        .contains("E0432"));
    assert!(
        std::fs::read_to_string(directory.join("linux-format-preparation.log"))
            .unwrap()
            .contains("cargo-fmt' is not installed")
    );
}

#[test]
fn unsigned_local_build_audit_cannot_authorize_a_diagnostic_launch() {
    let c = contract("diagnostic-local-build-contract.toml");
    assert_eq!(
        c["source_commit"].as_str(),
        Some("62114be0f5da3218706e30d7424acfb5d0579d07")
    );
    assert_eq!(
        c["source_tree"].as_str(),
        Some("a9f059d752f5497c8dc5f07fbee4db6b22a366f4")
    );
    assert_eq!(
        c["real_local_build_allowed_by_this_slice"].as_bool(),
        Some(true)
    );
    assert_eq!(c["validation_only"].as_bool(), Some(true));
    assert_eq!(c["features"].as_array().unwrap().len(), 0);
    for flag in [
        "counting_allocator",
        "trusted_builder_enrolled",
        "production_artifact_enrolled",
        "host_install_allowed",
        "product_workload_allowed",
        "qualification_allowed",
        "promotable",
        "admission_allowed",
        "audit_can_construct_verified_build",
        "audit_can_execute_binary",
    ] {
        assert_eq!(c[flag].as_bool(), Some(false), "{flag}");
    }
    let source = std::fs::read_to_string(
        root().join("tools/long-run-supervisor-074/src/diagnostic_artifacts.rs"),
    )
    .unwrap();
    let audit = source
        .split("pub struct UnsignedContentInspection")
        .nth(1)
        .unwrap()
        .split("impl VerifiedBuild")
        .next()
        .unwrap();
    for required in [
        "attestation_verified: false",
        "source_git_identity_verified: false",
        "execution_authorized: false",
        "admission_allowed: false",
        "MAX_BINARY_BYTES",
        "MAX_LOCK_BYTES",
        "MAX_CONFIG_BYTES",
        "MAX_LOG_BYTES",
        "check_cargo_log(contents.build_log)?",
        "read_local_file",
        "O_NOFOLLOW",
        "O_NONBLOCK",
    ] {
        assert!(audit.contains(required), "{required}");
    }
    for forbidden in [
        "Command::",
        "VerifiedBuild {",
        "SigningKey",
        "write_all",
        "remove_file",
        "StartTransientUnit",
    ] {
        assert!(!audit.contains(forbidden), "{forbidden}");
    }
    assert!(!source.contains("impl From<UnsignedContentInspection"));
    assert!(source.contains("path.starts_with(\"path+file:///\")"));
    assert!(source.contains("path.ends_with(\"/tools/get-owner-scheduled-controls-074\")"));
    let directory =
        root().join("docs/testing/performance/0.74/local-runs/diagnostic-local-build-62114be0");
    let m: Value =
        serde_json::from_slice(&std::fs::read(directory.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(
        m["schema_version"],
        "diagnostic-unsigned-local-build-evidence-074-v1"
    );
    assert_eq!(
        m["build_source_commit"],
        "62114be0f5da3218706e30d7424acfb5d0579d07"
    );
    assert_eq!(
        m["build_source_tree"],
        "a9f059d752f5497c8dc5f07fbee4db6b22a366f4"
    );
    assert_eq!(
        m["audit_source_commit"],
        "cee62b37e978fa29277f58e14a3051eaeaa24c7d"
    );
    assert_eq!(
        m["audit_source_tree"],
        "63bb3c2e4451b1755e53c14f4d914bde651b38ad"
    );
    for field in [
        "build_source_clean_before",
        "build_source_clean_after",
        "audit_source_clean_before",
        "audit_source_clean_after",
        "native_cargo_stdout_retained",
        "compiler_stderr_separate",
        "local_real_linux_release_build_completed",
        "real_elf_content_audit_passed_on_windows",
        "real_elf_content_audit_passed_on_linux",
        "validation_only",
        "formatting_checked",
        "doc_check_passed",
        "local_contract_passed",
        "links_passed",
        "mdbook_passed",
    ] {
        assert_eq!(m[field], true, "{field}");
    }
    for field in [
        "attestation_verified",
        "trusted_builder_enrolled",
        "production_artifact_enrolled",
        "independent_compilation_reproduced",
        "installed_paths_verified",
        "execution_authorized",
        "unit_mutation_performed",
        "writer_revocation_proven",
        "watchdog_implemented",
        "pending_intent_reconciliation_implemented",
        "production_route_enabled",
        "host_install_performed",
        "host_ssh_performed",
        "fixture_started",
        "product_workload_started",
        "pilot_executed",
        "qualification_started",
        "performance_measured",
        "numerical_performance_claim_allowed",
        "full_workspace_verify_run",
        "promotable",
        "admission_allowed",
        "counting_allocator",
    ] {
        assert_eq!(m[field], false, "{field}");
    }
    assert_eq!(m["features"].as_array().unwrap().len(), 0);
    let expected = [
        (
            "audit-linux.json",
            838u64,
            "b644f82e61699aef9eb918f87e4d5e18d171a73eff93a72d5444ba1b64b67102",
            "127033dae7e120955d3ccec5f3c1bbe356107abe",
        ),
        (
            "audit-windows.json",
            839u64,
            "3ec2b73b436eaa6ea24850f28bf6e0c50f1bfbbfe9224374e68d461dd5bfa07e",
            "4875dfc641acd5db3d6d1eff838153b6c2ff4784",
        ),
        (
            "binary.sha256",
            165u64,
            "970699e0b74df528dc5f2a6bbec5d4d12eb259224d9dc5c22cf858dea247e748",
            "beba2a4c451c09f896d3eee3a38facc3199074e7",
        ),
        (
            "cargo.jsonl",
            357108u64,
            "6210e77ba735a0c79eb9056706174d0de28bcab6b052516e2e20226ded5028cb",
            "dda187beaa3f606871a6f9e51df73a918b4c133c",
        ),
        (
            "cargo.stderr.log",
            12037u64,
            "88a6d81c86ea87a70d21429007488f46f08cd591b6afc8ab0e7f874dab205296",
            "111524215c8212836e89aa170fd849271c7f2753",
        ),
        (
            "elf-header.bin",
            64u64,
            "0010fbadccb2360e3bb8197f7a43b16d2266497d75a318aa97aa2a1de55cde58",
            "ebdc7cfc9c25d4290a1ba21797bf6521db36fc1c",
        ),
        (
            "inputs-after.sha256",
            881u64,
            "20ae55f0f598409be2aab9b45be98e901e6694522c7e24d88c4c6b81c60723b2",
            "e540582abf4e52a9d43cd9f1c376fc8961225ac8",
        ),
        (
            "inputs-before.sha256",
            881u64,
            "20ae55f0f598409be2aab9b45be98e901e6694522c7e24d88c4c6b81c60723b2",
            "e540582abf4e52a9d43cd9f1c376fc8961225ac8",
        ),
        (
            "linux-cee62b37.log",
            3182u64,
            "ad327f20377d35dec4162bda72967457570913e4329d1e11ff316d3bf680df63",
            "aa011159062b20edf59bf09c2aa0c6334e0785a2",
        ),
        (
            "package-id-test-first.log",
            1109u64,
            "a1512fcb6c5492f3d3add383d4225d7a2070c47e0f120705a9830d717bedf1fe",
            "611b36c22d618090f4cd3a55775330df4ed1e339",
        ),
        (
            "root-cee62b37.log",
            11711u64,
            "aef1f1ed01b393ac98774c200b1863b817bae8f5235ea6ca2327ceef081fb1f7",
            "e4dca51e54c53527a773f7f246fcecb7f3d182ea",
        ),
        (
            "source-after.log",
            84u64,
            "f86ebfbd50e1a7dc8913633b62b64aa7afc306bcbdd0b084b995a2cea4201fd7",
            "14a687cd181e6bcddd217f32f5fb208d573a8f18",
        ),
        (
            "source-before.log",
            84u64,
            "f86ebfbd50e1a7dc8913633b62b64aa7afc306bcbdd0b084b995a2cea4201fd7",
            "14a687cd181e6bcddd217f32f5fb208d573a8f18",
        ),
        (
            "toolchain.log",
            72u64,
            "413557cd8449617eac0f2dd4d9af0fdf007b2353eb6b683a87630841bb26528c",
            "67ffa857d61e1276d5b035fc8feafb8f9a7cb203",
        ),
        (
            "unsigned-audit-test-first.log",
            2423u64,
            "ffd94d78978ac0f6eca74e9abbff2f33aaa56427fb00c12438bf241674ffe6dd",
            "a31034bda5a7afeb0a9add806c43b1ade1e98e6f",
        ),
        (
            "validate-direct.json",
            150u64,
            "614f09d348dc3c08a18306bce4d3672722db28cb16bf89c2c311bf6c83706805",
            "366f5a80e7298535d9c1cf6b5289486e76c0412a",
        ),
        (
            "validate-embedded.json",
            150u64,
            "81dab3616b9d7e288a04db8d9706d29561a5e2d2099275621cd9b29fe5485aaa",
            "f425b91bcf37d846426cc3cf48af06c1b1fac00c",
        ),
        (
            "validate-resp2.json",
            150u64,
            "cb300bbb75c28866a64bbd83d402aa9c3e26e0a612ace4fcd6cdd0001e811135",
            "ef1ebd4e33a54b5f063c5fd009b6847614ebcee4",
        ),
        (
            "validate-resp3.json",
            150u64,
            "74dffa0adf6f0de4cf4c0db4bc08cda95f49371864b8accec5eca9eda2525170",
            "f505649dd4a67445434bf4b62652c2c17de9fa72",
        ),
        (
            "windows-cee62b37.log",
            2273u64,
            "176158eb8ff7ad6d613fd338f5f791e1eba29d2eafdd5103fe05c97886dbbd93",
            "0c250db5282490a3b6f066ae9b8ccb401e2558c2",
        ),
    ];
    assert_eq!(m["files"].as_array().unwrap().len(), expected.len());
    for (index, (path, size, digest, blob)) in expected.into_iter().enumerate() {
        let entry = &m["files"][index];
        let raw = std::fs::read(directory.join(path)).unwrap();
        assert_eq!(entry["path"], path);
        assert_eq!(entry["bytes"], size);
        assert_eq!(entry["sha256"], digest);
        assert_eq!(entry["git_blob"], blob);
        assert_eq!(raw.len() as u64, size);
        let hash: String = Sha256::digest(raw)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(hash, digest);
    }
    let windows: Value =
        serde_json::from_slice(&std::fs::read(directory.join("audit-windows.json")).unwrap())
            .unwrap();
    let linux: Value =
        serde_json::from_slice(&std::fs::read(directory.join("audit-linux.json")).unwrap())
            .unwrap();
    assert_eq!(windows, linux);
    assert_eq!(
        linux["binary"]["sha256"],
        "2a497dfcf8c5ec8ab32d55fe620ef96c554d55e1e31437f31c6474d2aa6625eb"
    );
    assert_eq!(linux["binary"]["bytes"], 26492624);
    assert_eq!(
        linux["build_log"]["sha256"],
        "6210e77ba735a0c79eb9056706174d0de28bcab6b052516e2e20226ded5028cb"
    );
    for flag in [
        "attestation_verified",
        "source_git_identity_verified",
        "installed_paths_verified",
        "execution_authorized",
        "admission_allowed",
    ] {
        assert_eq!(linux[flag], false, "{flag}");
    }
    assert_eq!(
        std::fs::read(directory.join("inputs-before.sha256")).unwrap(),
        std::fs::read(directory.join("inputs-after.sha256")).unwrap()
    );
    assert_eq!(
        std::fs::read(directory.join("source-before.log")).unwrap(),
        std::fs::read(directory.join("source-after.log")).unwrap()
    );
    let cargo = std::fs::read_to_string(directory.join("cargo.jsonl")).unwrap();
    let events: Vec<Value> = cargo
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(
        events.last().unwrap(),
        &json!({"reason":"build-finished","success":true})
    );
    let artifacts: Vec<&Value> = events
        .iter()
        .filter(|event| event["target"]["name"] == "timing-controls-074")
        .collect();
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0]["features"], json!([]));
    assert_eq!(artifacts[0]["profile"]["opt_level"], "3");
    assert_eq!(artifacts[0]["profile"]["debug_assertions"], false);
    assert_eq!(artifacts[0]["profile"]["test"], false);
    assert!(artifacts[0]["package_id"]
        .as_str()
        .unwrap()
        .ends_with("/tools/get-owner-scheduled-controls-074#0.0.0"));
    for surface in ["embedded", "direct", "resp2", "resp3"] {
        let value: Value = serde_json::from_slice(
            &std::fs::read(directory.join(format!("validate-{surface}.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(value["valid"], true);
        assert_eq!(value["fixture_started"], false);
        assert_eq!(value["admission_allowed"], false);
        assert_eq!(
            value["workload_sha256"],
            m["validation_workload_sha256"][surface]
        );
    }
    for (path, count) in [
        ("windows-cee62b37.log", 13),
        ("linux-cee62b37.log", 19),
        ("root-cee62b37.log", 112),
    ] {
        let text = std::fs::read_to_string(directory.join(path)).unwrap();
        assert!(!text.contains("test result: FAILED"));
        let total: u64 = text
            .lines()
            .filter_map(|l| l.strip_prefix("test result: ok. "))
            .map(|l| l.split_whitespace().next().unwrap().parse::<u64>().unwrap())
            .sum();
        assert_eq!(total, count);
    }
    let header = std::fs::read(directory.join("elf-header.bin")).unwrap();
    assert_eq!(&header[..7], b"\x7fELF\x02\x01\x01");
    assert_eq!(u16::from_le_bytes(header[18..20].try_into().unwrap()), 62);
    assert!(
        std::fs::read_to_string(directory.join("package-id-test-first.log"))
            .unwrap()
            .contains("test result: FAILED. 0 passed; 1 failed")
    );
    assert!(
        std::fs::read_to_string(directory.join("unsigned-audit-test-first.log"))
            .unwrap()
            .contains("cannot find function")
    );
}

#[test]
fn proc_documents_use_one_bounded_read_without_retry_or_parser_relaxation() {
    let c = contract("diagnostic-proc-document-contract.toml");
    assert_eq!(c["maximum_document_bytes"].as_integer(), Some(65536));
    assert_eq!(
        c["maximum_positional_reads_per_document"].as_integer(),
        Some(1)
    );
    assert_eq!(c["offset"].as_integer(), Some(0));
    assert_eq!(c["original_parallel_readers"].as_integer(), Some(8));
    assert_eq!(c["original_parallel_iterations"].as_integer(), Some(16));
    for flag in [
        "partial_document_retry_allowed",
        "new_pid_capture_allowed",
        "parser_or_identity_checks_weakened",
        "production_backend_enrolled",
        "host_install_allowed",
        "product_workload_allowed",
        "qualification_allowed",
        "promotable",
        "admission_allowed",
    ] {
        assert_eq!(c[flag].as_bool(), Some(false), "{flag}");
    }
    let code = std::fs::read_to_string(
        root().join("tools/long-run-supervisor-074/src/diagnostic_process.rs"),
    )
    .unwrap();
    let reader = code
        .split("fn bounded_from(")
        .nth(1)
        .unwrap()
        .split("fn child(")
        .next()
        .unwrap();
    assert!(reader.contains("let mut buffer = [0; DOCUMENT_BYTES + 1];"));
    assert!(reader.contains("let n = read(&mut buffer, 0)?;"));
    assert_eq!(reader.matches("read(&mut").count(), 1);
    for forbidden in ["while ", "loop {", "\n    for ", ".seek(", "read_to_end"] {
        assert!(!reader.contains(forbidden), "{forbidden}");
    }
    assert!(code.contains("fn one_positional_read_never_splices_a_regenerated_document_tail()"));
    assert!(code.contains("fn single_read_budget_empty_error_and_declared_size_fail_closed()"));
    assert!(code.contains("for _ in 0..8"));
    assert!(code.contains("for _ in 0..16"));
}

#[test]
fn bounded_diagnostic_manager_cannot_mutate_units_or_release_reservations() {
    let c = contract("diagnostic-manager-local-contract.toml");
    assert_eq!(
        c["contract_id"].as_str(),
        Some("diagnostic-manager-local-074-v1")
    );
    for flag in [
        "automatic_retry",
        "arbitrary_commands_allowed",
        "unit_mutation_allowed",
        "production_diagnostic_route_enabled",
        "host_install_allowed",
        "product_workload_allowed",
        "qualification_allowed",
        "promotable",
        "admission_allowed",
        "snapshot_is_cleanup_authority",
    ] {
        assert_eq!(c[flag].as_bool(), Some(false), "{flag}");
    }
    assert_eq!(
        c["pending_helper_blocks_new_operation"].as_bool(),
        Some(true)
    );
    for (name, value) in [
        ("request_bytes", 4096),
        ("combined_output_bytes", 65536),
        ("operation_timeout_ms", 2000),
        ("cleanup_timeout_ms", 1000),
        ("method_timeout_ms", 250),
        ("manager_uid", 0),
        ("manager_pid", 1),
        ("helper_tokio_threads", 1),
        ("helper_address_space_bytes", 536870912),
        ("helper_cpu_seconds", 2),
        ("helper_core_bytes", 0),
    ] {
        assert_eq!(c[name].as_integer(), Some(value), "{name}");
    }
    let code = std::fs::read_to_string(
        root().join("tools/long-run-supervisor-074/src/diagnostic_manager.rs"),
    )
    .unwrap();
    let production = code.split("#[cfg(test)]\nmod tests").next().unwrap();
    for forbidden in [
        "StartTransientUnit",
        "StopUnit",
        "KillUnit",
        "ResetFailed",
        ".wait()",
        "Command::new(\"sh\"",
        "Command::new(\"systemctl\"",
        "fs::remove_file",
        "fs::rename",
        "HostExecutionClaim",
        "DiagnosticBackend",
    ] {
        assert!(!production.contains(forbidden), "{forbidden}");
    }
    for required in [
        "Command::new(\"/proc/self/exe\")",
        "env_clear()",
        "SYS_close_range",
        "CLOSE_RANGE_CLOEXEC",
        "poll_pending_cleanup",
        "GetNameOwner",
        "GetConnectionUnixUser",
        "GetConnectionUnixProcessID",
        "CacheProperties::No",
        "org.freedesktop.systemd1.NoSuchUnit",
        "unix:path=/run/dbus/system_bus_socket",
        "OUTPUT_BYTES.saturating_sub",
        "RLIMIT_AS",
        "RLIMIT_CPU",
        "RLIMIT_CORE",
        "restrict_worker_resources()?",
        "TOKIO_WORKER_THREADS",
    ] {
        assert!(production.contains(required), "{required}");
    }
    let campaign =
        std::fs::read_to_string(root().join("tools/long-run-supervisor-074/src/systemd_unit.rs"))
            .unwrap();
    assert!(!campaign.contains("hydracache-diagnostic-074-"));
    let directory =
        root().join("docs/testing/performance/0.74/local-runs/diagnostic-manager-c2fdc3f4");
    let m: Value =
        serde_json::from_slice(&std::fs::read(directory.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(
        m["schema_version"],
        "diagnostic-manager-local-evidence-074-v1"
    );
    assert_eq!(
        m["implementation_source_commit"],
        "c2fdc3f48878bd931788218fec3573b137def3ba"
    );
    assert_eq!(
        m["implementation_source_tree"],
        "f61e80525d4742b2d2fa3ab1f6f91507ddcdf44d"
    );
    for field in [
        "source_clean_before",
        "source_clean_after",
        "formatting_checked",
        "doc_check_passed",
        "local_performance_contract_passed",
        "documentation_links_passed",
        "mdbook_passed",
    ] {
        assert_eq!(m[field], true, "{field}");
    }
    for field in [
        "snapshot_is_cleanup_authority",
        "loaded_unit_policy_authenticated",
        "original_start_authenticated",
        "exec_identity_proven",
        "tree_membership_authenticated",
        "writer_revocation_proven",
        "durable_first_failure_implemented",
        "watchdog_implemented",
        "pending_intent_reconciliation_implemented",
        "production_route_enabled",
        "host_install_allowed",
        "unit_mutation_performed",
        "host_ssh_performed",
        "installed_service_operations_performed",
        "cgroup_operations_performed",
        "observer_build_proof_in_this_packet",
        "product_workload_started",
        "pilot_executed",
        "qualification_started",
        "full_workspace_verify_run",
        "performance_measured",
        "numerical_performance_claim_allowed",
        "promotable",
        "admission_allowed",
    ] {
        assert_eq!(m[field], false, "{field}");
    }
    for (field, count) in [
        ("windows_passed", 62),
        ("linux_passed", 154),
        ("root_passed", 88),
        ("release_governance_tests_passed", 23),
        ("release_governance_cli_checks", 17),
        ("ship_check_expected_exit", 1),
        ("local_real_manager_absence_checks", 4),
        ("wrong_boot_refused_exit", 9),
    ] {
        assert_eq!(m[field], count);
    }
    let expected = [
        (
            "224f6ed3/linux.log",
            17328u64,
            "c963ae72b5157c4c3faa3ddd4636723bf651ca7f9e88dee023e43da2a140fb67",
            "ffc7f203beb87a5c5033c808ff7562e05bcae760",
            152u64,
            0u64,
        ),
        (
            "224f6ed3/root.log",
            11920u64,
            "4550c535b1d70a18092ab0f791ebdba245bd8044ed608359d1a9fa1d12eede96",
            "cba18941be7d648dff830f87796f0333b6a46888",
            106u64,
            4u64,
        ),
        (
            "224f6ed3/windows.log",
            7726u64,
            "7477619933fb7a60625cb4597351c643898f149fc9f8ea2f716dd1bf6892a5e0",
            "5bdc7a74d463e479935ada77687b670063ed7f8b",
            62u64,
            0u64,
        ),
        (
            "9a225347/linux.log",
            16440u64,
            "1589e1736a72e36fde81a33c68acd058be6fe56223a981944792ab75cba053a8",
            "dee9717abfb133497c2a95d433d87ba0eaf5a435",
            151u64,
            1u64,
        ),
        (
            "9a225347/proc-splice-test-first.log",
            2356u64,
            "ec363d999d0ae4d5e921ac3a302bda83a9c6ca187be8de803485618397c060fb",
            "5fc05e116ba5ba5e7b6bf9ed8e0453ffb956dc0c",
            0u64,
            1u64,
        ),
        (
            "9a225347/root.log",
            11015u64,
            "4ffa86fd1dadf0c0fd38431703d93ce59497cdcae7f5e2f6fab8f067939c5dec",
            "67d70cddb3fa4531391c15313b53b9093d2c4321",
            110u64,
            0u64,
        ),
        (
            "9a225347/windows.log",
            7253u64,
            "6c7e5cfb25f9b9b2b0ebb3f6b1e878db2735508cc19a632ace7452bbac48d672",
            "e22afce941c279c6a8ef1a497742afb219d1e68a",
            62u64,
            0u64,
        ),
        (
            "097ab6fa/linux.log",
            16633u64,
            "80c036ebfb3c0eb20b32c3e874ee160d7612923922bbf517b9e7d67b1c0cab2d",
            "9ee80532711424a304d62a90d325a53690d8c23c",
            153u64,
            1u64,
        ),
        (
            "097ab6fa/root.log",
            11373u64,
            "534e792ba816c402e8624407f12fc1439ffd7d5054ecf9c3c82f06e5b313426c",
            "e5812b2e891785ed259c6093732ed1474b2bd3ca",
            111u64,
            0u64,
        ),
        (
            "097ab6fa/windows.log",
            7217u64,
            "bee4ca825d9b8445f27fe7aef4fe635e717e6403afac737f1e8377fca086cb67",
            "c0314ae8553ab504197011c7fbe37cc216a79e40",
            62u64,
            0u64,
        ),
        (
            "c2fdc3f4/linux.log",
            17679u64,
            "897d5fa0a336a4c7e4d7210953517b97831b2dfbe1f45d7fd9936205651f0d6e",
            "3038456cd5e888c89f590055e45163731e63c7a8",
            154u64,
            0u64,
        ),
        (
            "c2fdc3f4/root.log",
            11289u64,
            "49590042d5a6f79d58da6585a0d537be0b3c902fb5710b21f878af90151c57db",
            "f6390b64ce381a439a5738b127a1026b0c43275d",
            111u64,
            0u64,
        ),
        (
            "c2fdc3f4/windows.log",
            7217u64,
            "674e64b20199dce82148e305e7cfed1b9ce9960277d347e854aeeba4d64dc54c",
            "f67a68e79ba8b0a6486d6ac99fa638ef91dfdff3",
            62u64,
            0u64,
        ),
    ];
    assert_eq!(m["files"].as_array().unwrap().len(), expected.len());
    for (index, (path, size, digest, blob, passed, failed)) in expected.into_iter().enumerate() {
        let entry = &m["files"][index];
        let raw = std::fs::read(directory.join(path)).unwrap();
        assert_eq!(entry["path"], path);
        assert_eq!(entry["bytes"], size);
        assert_eq!(entry["sha256"], digest);
        assert_eq!(entry["git_blob"], blob);
        assert_eq!(entry["passed"], passed);
        assert_eq!(entry["failed"], failed);
        assert_eq!(raw.len() as u64, size);
        let actual: String = Sha256::digest(&raw)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(actual, digest);
        let text = String::from_utf8(raw).unwrap();
        let mut counts = (0u64, 0u64);
        for tail in text.lines().filter_map(|l| l.strip_prefix("test result: ")) {
            let (_, numbers) = tail.split_once(". ").unwrap();
            let mut parts = numbers.split(';');
            counts.0 += parts
                .next()
                .unwrap()
                .split_whitespace()
                .next()
                .unwrap()
                .parse::<u64>()
                .unwrap();
            counts.1 += parts
                .next()
                .unwrap()
                .split_whitespace()
                .next()
                .unwrap()
                .parse::<u64>()
                .unwrap();
        }
        assert_eq!(counts, (passed, failed));
    }
    let linux = std::fs::read_to_string(directory.join("c2fdc3f4/linux.log")).unwrap();
    let snapshots: Vec<Value> = linux
        .lines()
        .filter(|l| l.starts_with("{\"schema_version\":1,\"scope\":"))
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(snapshots.len(), 4);
    for (snapshot, surface) in snapshots
        .iter()
        .zip(["embedded", "direct", "resp2", "resp3"])
    {
        assert_eq!(snapshot["scope"]["surface"], surface);
        assert_eq!(snapshot["scope"]["boot_id"], m["local_boot_id"]);
        assert_eq!(snapshot["manager_owner"], m["local_manager_unique_owner"]);
        assert_eq!(snapshot["manager_uid"], 0);
        assert_eq!(snapshot["manager_pid"], 1);
        assert!(snapshot["unit"].is_null());
    }
    assert!(linux.contains("expected wrong-boot refusal: exit 9; no unit mutation"));
    assert!(linux.contains("manager observation refused: Exit; helper cleanup=true"));
}

#[test]
fn diagnostic_live_execution_composition_keeps_enrollment_and_admission_closed() {
    let c = contract("diagnostic-live-execution-local-contract.toml");
    assert_eq!(
        c["contract_id"].as_str(),
        Some("diagnostic-live-execution-local-074-v1")
    );
    for field in [
        "original_identity_process_only",
        "original_state_revision_and_clocks_required",
        "production_material_and_outputs_required",
        "first_failure_latches_three_borrowed_guards",
        "typed_first_error_preserved",
        "no_observation_after_refusal",
    ] {
        assert_eq!(c[field].as_bool(), Some(true), "{field}");
    }
    for field in [
        "positive_local_production_composition_proved",
        "worker_account_authenticated",
        "original_start_authenticated",
        "atomic_snapshot_proved",
        "same_open_file_description_proved",
        "whole_operation_deadline_proved",
        "production_output_preparation_allowed",
        "unit_mutation_allowed",
        "host_install_allowed",
        "product_workload_allowed",
        "qualification_allowed",
        "admission_allowed",
        "writer_revocation_proved",
        "durable_refusal_journal",
    ] {
        assert_eq!(c[field].as_bool(), Some(false), "{field}");
    }
    let order: Vec<_> = c["revalidation_order"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(
        order,
        ["identity", "outputs", "material", "io", "material", "outputs", "identity"]
    );
    assert_eq!(
        c["manager_observations_per_revalidation"].as_integer(),
        Some(4)
    );
    let source = std::fs::read_to_string(
        root().join("tools/long-run-supervisor-074/src/diagnostic_live_execution.rs"),
    )
    .unwrap();
    let code = source
        .split("#[cfg(test)]")
        .next()
        .unwrap()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    for required in ["pub fn pin_live_execution<'guard, 'process>", "let process = identity.original_process();",
        "identity.refuse(); outputs.refuse(); material.refuse();", "outputs.matches_production_state(&state)",
        "material.matches_production_state(&state)", "self.finish_observation(result)",
        "Step::Identity, Step::Outputs, Step::Material, Step::Io, Step::Material, Step::Outputs, Step::Identity,"] {
        assert!(code.contains(required), "{required}");
    }
    for forbidden in [
        "Serialize",
        "StartTransientUnit",
        "kill(",
        "create_dir",
        "write_all(",
        "pub fn streams",
        "pub fn process",
        "pub fn material",
        "pub fn outputs",
        "pin_kernel_process(",
    ] {
        assert!(!code.contains(forbidden), "{forbidden}");
    }
    // Shape guard only; runtime refusal and ordering tests supply behavior evidence.
}

#[test]
fn production_output_inspection_keeps_preparation_and_worker_enrollment_closed() {
    let c = contract("diagnostic-production-output-local-contract.toml");
    assert_eq!(
        c["contract_id"].as_str(),
        Some("diagnostic-production-output-local-074-v1")
    );
    assert_eq!(
        c["fixed_root"].as_str(),
        Some("/var/lib/hydracache-performance/diagnostics")
    );
    assert_eq!(c["maximum_stream_bytes"].as_integer(), Some(8388608));
    for field in [
        "root_owned_ancestors_required",
        "root_owned_private_lease_required",
        "nonroot_ancestor_search_required",
        "nonroot_worker_cell_required",
        "distinct_production_capability",
        "first_refusal_is_sticky",
        "readonly_stream_descriptors",
    ] {
        assert_eq!(c[field].as_bool(), Some(true), "{field}");
    }
    for field in [
        "worker_identity_authenticated",
        "positive_production_ancestry_observed",
        "production_output_preparation_allowed",
        "process_identity_composed",
        "original_start_authenticated",
        "unit_mutation_allowed",
        "host_install_allowed",
        "product_workload_allowed",
        "qualification_allowed",
        "admission_allowed",
        "atomic_namespace_snapshot_proved",
        "writer_revocation_proved",
        "durable_refusal_journal",
    ] {
        assert_eq!(c[field].as_bool(), Some(false), "{field}");
    }
    assert_eq!(c["root_owned_lease_mode"].as_str(), Some("0711"));
    let source = std::fs::read_to_string(
        root().join("tools/long-run-supervisor-074/src/diagnostic_named_output.rs"),
    )
    .unwrap();
    let code = source
        .split("#[cfg(test)]")
        .next()
        .unwrap()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    for forbidden in [
        "O_CREAT",
        "O_TRUNC",
        "chown(",
        "create_dir",
        "write_all(",
        "StartTransientUnit",
        "Serialize",
        "pub fn streams",
    ] {
        assert!(!code.contains(forbidden), "{forbidden}");
    }
    for required in [
        "pub struct ProductionOutputRead",
        "pub fn pin_production_outputs(",
        "Path::new(PRODUCTION_ROOT)",
        "s.uid == 0 && s.gid == 0",
        "s.mode & 0o7022 == 0",
        "s.mode & 0o7777 == 0o700",
        "s.mode & 0o7777 == 0o711",
        "s.mode & 0o001 != 0",
        "ProductionLevel::Lease",
        "self.inner.revalidate()",
        "if self.production || state != &self.state",
        "*id == 0 || *id == u32::MAX",
    ] {
        assert!(code.contains(required), "{required}");
    }
}

#[test]
fn diagnostic_live_identity_composition_keeps_lifecycle_and_admission_closed() {
    let c = contract("diagnostic-live-identity-local-contract.toml");
    assert_eq!(
        c["contract_id"].as_str(),
        Some("diagnostic-live-identity-local-074-v1")
    );
    for field in [
        "first_refusal_is_sticky",
        "original_readers_borrowed",
        "kernel_tree_required",
        "fixed_boot_and_tree_scope_required",
        "original_main_pid_required",
        "exact_member_node_required",
        "frozen_tree_refused",
    ] {
        assert_eq!(c[field].as_bool(), Some(true), "{field}");
    }
    for field in [
        "positive_local_kernel_composition_proved",
        "original_start_authenticated",
        "root_inode_authenticated_by_manager",
        "atomic_snapshot_proved",
        "dynamic_tree_refresh_allowed",
        "output_and_executable_composed",
        "production_output_preparation_allowed",
        "unit_mutation_allowed",
        "host_install_allowed",
        "product_workload_allowed",
        "qualification_allowed",
        "admission_allowed",
        "writer_revocation_proved",
        "durable_refusal_journal",
    ] {
        assert_eq!(c[field].as_bool(), Some(false), "{field}");
    }
    for field in [
        "manager_observations_per_revalidation",
        "source_checks_per_revalidation",
    ] {
        assert_eq!(c[field].as_integer(), Some(2));
    }
    for (field, value) in [
        ("active_state", "active"),
        ("sub_state", "running"),
        ("result", "success"),
    ] {
        assert_eq!(c[field].as_str(), Some(value));
    }
    let source = std::fs::read_to_string(
        root().join("tools/long-run-supervisor-074/src/diagnostic_live_identity.rs"),
    )
    .unwrap();
    let code = source.split("#[cfg(test)]").next().unwrap();
    let code = code.split_whitespace().collect::<Vec<_>>().join(" ");
    for forbidden in [
        "Command::new",
        "StartTransientUnit",
        "StopUnit",
        "Serialize",
        "SYS_pidfd_open",
        "read_fixture_tree",
        "pin_owned_test_helper",
        "TreeObservation",
        "kill(",
        "refresh(",
    ] {
        assert!(!code.contains(forbidden), "{forbidden}");
    }
    for required in [
        ".pin_for(state)",
        "tree.is_kernel_scope(scope)",
        "self.process.revalidate()?",
        "self.tree.revalidate()?",
        "manager.inspect_original(invocation)",
        "self.invocation.refuse()",
        "unit.main_pid != self.generation.pid",
        "self.generation.cgroup_path != expected_path",
        "node.frozen",
        "members.next().is_some()",
        "process: &'a ProcessRead",
        "tree: &'a TreeRead",
    ] {
        assert!(code.contains(required), "{required}");
    }
}

#[test]
fn named_fixture_outputs_keep_production_preparation_and_execution_closed() {
    let c = contract("diagnostic-named-output-local-contract.toml");
    assert_eq!(
        c["contract_id"].as_str(),
        Some("diagnostic-named-output-local-074-v1")
    );
    for (field, expected) in [
        ("maximum_stream_bytes", 8388608),
        ("maximum_path_bytes", 4096),
        ("maximum_directory_depth", 64),
    ] {
        assert_eq!(c[field].as_integer(), Some(expected), "{field}");
    }
    for field in [
        "first_refusal_is_sticky",
        "fixed_names_from_validated_state",
        "retained_ancestor_bindings",
        "readonly_stream_descriptors",
        "append_growth_allowed",
        "observed_shrink_refused",
        "fixture_origin_explicit",
    ] {
        assert_eq!(c[field].as_bool(), Some(true), "{field}");
    }
    for field in [
        "production_ancestry_proved",
        "production_output_preparation_allowed",
        "unit_mutation_allowed",
        "host_install_allowed",
        "product_workload_allowed",
        "qualification_allowed",
        "admission_allowed",
        "atomic_namespace_snapshot_proved",
        "same_open_file_description_proved",
        "continuous_exec_identity_proved",
        "writer_revocation_proved",
        "durable_refusal_journal",
        "original_start_authenticated",
    ] {
        assert_eq!(c[field].as_bool(), Some(false), "{field}");
    }
    let source = std::fs::read_to_string(
        root().join("tools/long-run-supervisor-074/src/diagnostic_named_output.rs"),
    )
    .unwrap();
    let code = source.split("#[cfg(test)]").next().unwrap();
    let code = code.split_whitespace().collect::<Vec<_>>().join(" ");
    for forbidden in [
        "Command::new",
        "O_CREAT",
        "O_TRUNC",
        "write_all(",
        "read_to_end(",
        "seek(",
        "StartTransientUnit",
        "StopUnit",
        "Serialize",
        "pub fn streams",
        "pub fn file",
        "pin_fixed_outputs",
    ] {
        assert!(!code.contains(forbidden), "{forbidden}");
    }
    for required in [
        "diagnostic_start_intent(state)",
        "O_RDONLY | libc::O_NOFOLLOW",
        "self.check_directories()",
        "check_entries(leaf)",
        "meta.len() < self.high_water[i]",
        "self.refused = true",
        "self.outputs.refused = true",
        "self.io.revalidate()?",
        "!material.is_fixture()",
        "STREAM_BYTES: u64 = crate::diagnostic_receipts::STREAM_BYTES as u64",
    ] {
        assert!(code.contains(required), "{required}");
    }
}

#[test]
fn original_process_io_binding_keeps_start_and_pathname_proof_closed() {
    assert_eq!(
        hydracache_long_run_supervisor_074::diagnostic_receipts::STREAM_BYTES,
        8_388_608
    );
    let c = contract("diagnostic-io-local-contract.toml");
    assert_eq!(
        c["contract_id"].as_str(),
        Some("diagnostic-io-local-074-v1")
    );
    assert_eq!(c["maximum_document_bytes"].as_integer(), Some(65536));
    assert_eq!(c["maximum_stream_bytes"].as_integer(), Some(8388608));
    for field in [
        "first_refusal_is_sticky",
        "readonly_expected_descriptors",
        "fixed_proc_targets_only",
        "observed_shrink_refused",
    ] {
        assert_eq!(c[field].as_bool(), Some(true), "{field}");
    }
    for field in [
        "unit_mutation_allowed",
        "host_install_allowed",
        "product_workload_allowed",
        "qualification_allowed",
        "admission_allowed",
        "output_pathnames_verified",
        "same_open_file_description_proved",
        "continuous_exec_identity_proved",
        "effective_environment_proved",
        "writer_revocation_proved",
        "durable_refusal_journal",
        "original_start_authenticated",
    ] {
        assert_eq!(c[field].as_bool(), Some(false), "{field}");
    }
    let source = std::fs::read_to_string(
        root().join("tools/long-run-supervisor-074/src/diagnostic_process_io.rs"),
    )
    .unwrap();
    let production = source.split("#[cfg(test)]").next().unwrap();
    for forbidden in [
        "SYS_pidfd_open",
        "Command::new",
        "kill(",
        "O_CREAT",
        "O_TRUNC",
        "O_WRONLY |",
        "write_all(",
        "read_link(",
        "StartTransientUnit",
        "StopUnit",
    ] {
        assert!(!production.contains(forbidden), "{forbidden}");
    }
    for required in [
        "O_PATH | libc::O_CLOEXEC",
        "readonly_clone",
        "observe_stream",
        "parse_fdinfo",
        "O_APPEND",
        "self.process.revalidate()",
        "self.refused = true",
        "STREAM_BYTES: u64 = crate::diagnostic_receipts::STREAM_BYTES as u64",
    ] {
        assert!(production.contains(required), "{required}");
    }
    let artifact = std::fs::read_to_string(
        root().join("tools/long-run-supervisor-074/src/diagnostic_artifacts_linux.rs"),
    )
    .unwrap();
    assert!(artifact.contains("pub fn bind_asserted_process_io"));
    assert!(artifact.contains("self.revalidate_for(state)?"));
    assert!(!artifact.contains("pub fn binary_file"));
}

#[test]
fn loaded_diagnostic_settings_projection_keeps_execution_closed() {
    let c = contract("diagnostic-loaded-local-contract.toml");
    assert_eq!(
        c["contract_id"].as_str(),
        Some("diagnostic-loaded-local-074-v1")
    );
    for flag in [
        "unit_mutation_allowed",
        "product_workload_allowed",
        "qualification_allowed",
        "admission_allowed",
        "complete_policy_proof",
        "append_destinations_verified",
        "effective_process_environment_verified",
        "original_executable_verified",
        "runtime_guard_is_durable_journal",
        "snapshot_is_cleanup_authority",
    ] {
        assert_eq!(c[flag].as_bool(), Some(false), "{flag}");
    }
    for (key, value) in [
        ("request_schema", 1),
        ("response_schema", 1),
        ("projected_settings", 53),
        ("operation_timeout_ms", 2000),
        ("combined_output_bytes", 65536),
        ("cleanup_timeout_ms", 1000),
    ] {
        assert_eq!(c[key].as_integer(), Some(value), "{key}");
    }
    assert_eq!(c["helper_mode"].as_str(), Some("diagnostic-loaded-worker"));
    assert_eq!(c["first_refusal_is_sticky"].as_bool(), Some(true));
    let source = std::fs::read_to_string(
        root().join("tools/long-run-supervisor-074/src/diagnostic_loaded.rs"),
    )
    .unwrap();
    let production = source.split("#[cfg(test)]").next().unwrap();
    for forbidden in [
        "StartTransientUnit",
        "StopUnit",
        "KillUnit",
        "DiagnosticBackend",
        "fs::remove_file",
        "Command::new",
    ] {
        assert!(!production.contains(forbidden), "{forbidden}");
    }
    for required in [
        "pin_for",
        "revalidate",
        "original.invocation_id",
        "same_manager",
        "ExecStartEx",
        "StandardOutput",
        "EnvironmentFiles",
        "LimitNOFILESoft",
        "LimitFSIZESoft",
    ] {
        assert!(production.contains(required), "{required}");
    }
}

#[test]
fn diagnostic_process_binding_cannot_signal_recapture_or_release_host() {
    let c = contract("diagnostic-process-local-contract.toml");
    assert_eq!(c["schema_version"].as_integer(), Some(1));
    assert_eq!(
        c["contract_id"].as_str(),
        Some("diagnostic-process-local-074-v1")
    );
    assert_eq!(
        c["source_commit"].as_str(),
        Some("62114be0f5da3218706e30d7424acfb5d0579d07")
    );
    for field in [
        "production_route_enabled",
        "host_install_allowed",
        "product_workload_allowed",
        "qualification_allowed",
        "promotable",
        "admission_allowed",
        "systemd_identity_authenticated",
        "tree_membership_authenticated",
        "writer_revocation_proven",
        "host_release_allowed",
        "watchdog_implemented",
        "automatic_pid_recapture",
        "signals_allowed",
        "exit_status_proven",
        "exec_identity_proven",
        "fixture_can_claim_kernel_origin",
    ] {
        assert_eq!(c[field].as_bool(), Some(false), "{field}");
    }
    assert_eq!(c["maximum_document_bytes"].as_integer(), Some(65536));
    assert_eq!(c["maximum_cgroup_depth"].as_integer(), Some(8));
    assert_eq!(c["maximum_cgroup_component_bytes"].as_integer(), Some(128));
    assert_eq!(c["pidfd_poll_timeout_ms"].as_integer(), Some(0));
    let code = std::fs::read_to_string(
        root().join("tools/long-run-supervisor-074/src/diagnostic_process.rs"),
    )
    .unwrap();
    let production = code.split("#[cfg(test)]").next().unwrap();
    for forbidden in [
        "Command::",
        "pidfd_send_signal",
        "libc::kill",
        "waitpid",
        "waitid",
        "impl DiagnosticBackend",
        "release_host",
        "zbus::",
    ] {
        assert!(!production.contains(forbidden), "{forbidden}");
    }
    assert!(production.contains("SYS_pidfd_open"));
    assert!(production.contains("const DOCUMENT_BYTES: usize = 65_536;"));
    assert!(production.contains("const MAX_DEPTH: usize = 8;"));
    assert!(production.contains("const MAX_COMPONENT: usize = 128;"));
    let directory =
        root().join("docs/testing/performance/0.74/local-runs/diagnostic-process-b8c626cf");
    let m: Value =
        serde_json::from_slice(&std::fs::read(directory.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(
        m["schema_version"],
        "diagnostic-process-local-evidence-074-v1"
    );
    assert_eq!(
        m["implementation_source_commit"],
        "b8c626cf52c84bc3ea2899ce33acde894bf02556"
    );
    assert_eq!(
        m["implementation_source_tree"],
        "da1c32a17d01be69e096fb20f767c894cda164a6"
    );
    assert_eq!(
        m["preregistered_contract_commit"],
        "5b5cd52fdb409d0a88c105a6d1b1dc9d3c4d4c13"
    );
    for field in [
        "public_diagnostic_positive_path_observed",
        "live_diagnostic_kernel_tree_observed",
        "real_pid_reuse_induced",
        "real_process_migration_induced",
        "host_reboot_induced",
        "pid_namespace_created_or_switched",
        "original_start_identity_authenticated",
        "systemd_identity_authenticated",
        "tree_membership_authenticated",
        "cgroup_inode_binding_proven",
        "writer_revocation_proven",
        "exec_identity_proven",
        "exit_status_proven",
        "reader_latches_terminal_failures",
        "uninterrupted_lifetime_after_supervisor_restart_proven",
        "host_release_allowed",
        "watchdog_implemented",
        "pending_intent_reconciliation_implemented",
        "production_route_enabled",
        "signals_sent",
        "host_ssh_performed",
        "installed_service_operations_performed",
        "cgroup_operations_performed",
        "real_product_binary_built",
        "product_workload_started",
        "pilot_executed",
        "qualification_started",
        "full_workspace_verify_run",
        "performance_measured",
        "numerical_performance_claim_allowed",
        "promotable",
        "admission_allowed",
    ] {
        assert_eq!(m[field], false, "{field}");
    }
    for field in [
        "source_clean_before",
        "source_clean_after",
        "local_self_pidfd_observed",
        "nonproduct_helper_started",
        "helper_exited_on_stdin_eof",
        "parallel_positional_revalidation_checked",
        "seeded_stat_property_checked",
    ] {
        assert_eq!(m[field], true, "{field}");
    }
    for (field, count) in [
        ("windows_passed", 62),
        ("linux_passed", 130),
        ("root_passed", 86),
        ("release_governance_tests_passed", 23),
        ("release_governance_cli_checks", 17),
        ("ship_check_expected_exit", 1),
    ] {
        assert_eq!(m[field], count);
    }
    assert_eq!(m["windows_checks"]["process_library"], 0);
    assert_eq!(m["linux_checks"]["process_library"], 13);
    assert_eq!(
        m["qualification_manifest_sha256"],
        "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
    );
    assert_eq!(
        m["root_cargo_lock_sha256"],
        "be46eaf8e97e507bda5e3f2b671d0d622e6a639ce1d4613769a4693723d62b19"
    );
    assert_eq!(
        m["observer_cargo_lock_sha256"],
        "e3be470f5a1bff4e917e841fc5bfbd257ff12559206bbc1481c3c4bca528eb1d"
    );
    assert_eq!(m["files"].as_array().unwrap().len(), 3);
    for (index, (name, size, digest, blob, passed)) in [
        (
            "windows.log",
            7502u64,
            "102d0cd615ce7f4f8cdfc3760894416be75ced99ab40f979f70c69685af3e8cf",
            "c5d9b3937ede68f0222f11d08c3992a78dc641af",
            62u64,
        ),
        (
            "linux.log",
            21466,
            "13ef01da02c9f610c9f04927c8ad9ae0d9dc6a93cc6b20c01016bd16a0c7277f",
            "423b11478c7c01fe9da3d1e6be973c7d2b29c3a1",
            130,
        ),
        (
            "root.log",
            11180,
            "9af40cafff88853c234773093185572013b4040d4e14d81b03121cb29e841bda",
            "dd8bf842ea747e66dc21235dd8bc1628917972a4",
            109,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let entry = &m["files"][index];
        let raw = std::fs::read(directory.join(name)).unwrap();
        assert_eq!(entry["path"], name);
        assert_eq!(entry["bytes"], size);
        assert_eq!(entry["sha256"], digest);
        assert_eq!(entry["git_blob"], blob);
        assert_eq!(entry["passed"], passed);
        assert_eq!(raw.len() as u64, size);
        let actual: String = Sha256::digest(&raw)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(actual, digest);
        let text = String::from_utf8(raw).unwrap();
        assert!(!text.contains("test result: FAILED"));
        let total: u64 = text
            .lines()
            .filter_map(|line| line.strip_prefix("test result: ok. "))
            .map(|tail| {
                tail.split_whitespace()
                    .next()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap()
            })
            .sum();
        assert_eq!(total, passed);
    }
    for name in ["main.rs", "server.rs", "protocol.rs", "config.rs"] {
        assert!(!std::fs::read_to_string(
            root().join("tools/long-run-supervisor-074/src").join(name)
        )
        .unwrap()
        .contains("diagnostic_process"));
    }
}

#[test]
fn diagnostic_recursive_tree_reader_cannot_authorize_cleanup_or_execution() {
    let c = contract("diagnostic-tree-local-contract.toml");
    assert_eq!(
        c["source_commit"].as_str(),
        Some("62114be0f5da3218706e30d7424acfb5d0579d07")
    );
    for field in [
        "production_route_enabled",
        "host_install_allowed",
        "host_pilot_execution_allowed",
        "product_workload_allowed",
        "qualification_allowed",
        "promotable",
        "admission_allowed",
        "process_generation_proven",
        "systemd_identity_authenticated",
        "writer_revocation_proven",
        "host_release_allowed",
        "watchdog_implemented",
        "pending_intent_reconciliation_implemented",
        "fixture_is_kernel_cgroup",
        "empty_snapshot_is_cleanup_authority",
        "old_inputs_or_packets_mutable",
        "automatic_observation_retry",
    ] {
        assert_eq!(c[field].as_bool(), Some(false), "{field}");
    }
    for (field, n) in [
        ("maximum_nodes", 32),
        ("maximum_depth", 8),
        ("maximum_processes", 256),
        ("maximum_entries_per_node", 256),
        ("document_bytes", 65536),
        ("total_document_bytes", 4194304),
    ] {
        assert_eq!(c[field].as_integer(), Some(n), "{field}");
    }
    let code = std::fs::read_to_string(
        root().join("tools/long-run-supervisor-074/src/diagnostic_tree.rs"),
    )
    .unwrap();
    for forbidden in [
        "Command::new",
        "std::process::",
        "zbus::",
        "libc::kill",
        "release_host",
        "impl DiagnosticBackend",
    ] {
        assert!(!code.contains(forbidden), "{forbidden}");
    }
    for name in ["main.rs", "server.rs", "protocol.rs", "config.rs"] {
        assert!(!std::fs::read_to_string(
            root().join("tools/long-run-supervisor-074/src").join(name)
        )
        .unwrap()
        .contains("diagnostic_tree"));
    }
    let directory =
        root().join("docs/testing/performance/0.74/local-runs/diagnostic-tree-f555f32a");
    let m: Value =
        serde_json::from_slice(&std::fs::read(directory.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(m["schema_version"], "diagnostic-tree-local-evidence-074-v1");
    assert_eq!(
        m["implementation_source_commit"],
        "f555f32a49c940304f11e1154ace6ddbcc72c2f9"
    );
    assert_eq!(
        m["implementation_source_tree"],
        "8dc9543bc81b655c50f9b1e1320e72f41a6e3699"
    );
    assert_eq!(
        m["preregistered_contract_commit"],
        "57ed62552031ba1f33ae37d1de947d5db9f8d678"
    );
    for (field, digest) in [
        (
            "qualification_manifest_sha256",
            "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc",
        ),
        (
            "root_cargo_lock_sha256",
            "be46eaf8e97e507bda5e3f2b671d0d622e6a639ce1d4613769a4693723d62b19",
        ),
        (
            "observer_cargo_lock_sha256",
            "e3be470f5a1bff4e917e841fc5bfbd257ff12559206bbc1481c3c4bca528eb1d",
        ),
    ] {
        assert_eq!(m[field], digest);
    }
    for field in [
        "live_diagnostic_kernel_tree_observed",
        "systemd_identity_authenticated",
        "process_generation_proven",
        "writer_revocation_proven",
        "empty_snapshot_is_cleanup_authority",
        "host_release_allowed",
        "watchdog_implemented",
        "pending_intent_reconciliation_implemented",
        "production_route_enabled",
        "host_ssh_performed",
        "service_operations_performed",
        "cgroup_operations_performed",
        "real_product_binary_built",
        "product_workload_started",
        "pilot_executed",
        "qualification_started",
        "full_workspace_verify_run",
        "performance_measured",
        "numerical_performance_claim_allowed",
        "promotable",
        "admission_allowed",
    ] {
        assert_eq!(m[field], false, "{field}");
    }
    for field in [
        "source_clean_before",
        "source_clean_after",
        "synthetic_positive_fixture",
        "recursive_fixture_tree_checked",
        "retained_descriptor_revalidation_checked",
        "aggregate_bounds_checked",
        "seeded_pid_property_checked",
    ] {
        assert_eq!(m[field], true, "{field}");
    }
    for (field, count) in [
        ("windows_passed", 62),
        ("linux_passed", 117),
        ("root_passed", 85),
        ("release_governance_tests_passed", 23),
        ("release_governance_cli_checks", 17),
        ("ship_check_expected_exit", 1),
    ] {
        assert_eq!(m[field], count);
    }
    assert_eq!(m["windows_checks"]["diagnostic_tree"], 0);
    assert_eq!(m["linux_checks"]["diagnostic_tree"], 8);
    assert_eq!(m["linux_checks"]["tree_library"], 4);
    assert_eq!(m["files"].as_array().unwrap().len(), 3);
    for (index, (name, size, digest, blob)) in [
        (
            "windows.log",
            7348u64,
            "1dadbe36353e48fd516579cfd3e20c354606891457198d6ec93f31dd0fc58c4d",
            "d277fe24a8e056f0831e525a7304e106446690e0",
        ),
        (
            "linux.log",
            19818,
            "3231e2e59a1e95e2e85e1d79d1d1167265d7fdeb15ac5b355f298270957c1521",
            "8d93cdc053034dd309d32debf542ff1f40b64d01",
        ),
        (
            "root.log",
            11063,
            "3db26fd5153cda4823006f5153e6a19c6068bac67afc410f8b45392595e231d6",
            "7b0bfc4ae875ab15c76cc588022bd399872f6eb0",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let entry = &m["files"][index];
        let raw = std::fs::read(directory.join(name)).unwrap();
        assert_eq!(entry["path"], name);
        assert_eq!(entry["bytes"], size);
        assert_eq!(entry["sha256"], digest);
        assert_eq!(entry["git_blob"], blob);
        assert_eq!(raw.len() as u64, size);
        let actual: String = Sha256::digest(&raw)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(actual, digest);
        let text = String::from_utf8(raw).unwrap();
        assert!(!text.contains("test result: FAILED"));
        assert!(text.contains("test result: ok."));
    }
}

#[test]
fn local_diagnostic_spool_cannot_open_host_execution_or_claim_power_loss_proof() {
    let c = contract("diagnostic-spool-local-contract.toml");
    for field in [
        "production_route_enabled",
        "host_install_allowed",
        "host_pilot_execution_allowed",
        "real_product_build_allowed_by_this_slice",
        "qualification_allowed",
        "promotable",
        "admission_allowed",
        "live_cgroup_cleanup_proven",
        "fixture_is_production_spool",
        "power_loss_durability_proven",
        "automatic_pending_recovery_allowed",
        "workload_retry_allowed",
        "old_inputs_or_packets_mutable",
    ] {
        assert_eq!(c[field].as_bool(), Some(false), "{field}");
    }
    for (field, value) in [
        ("stream_prefix_bytes", 8388608),
        ("combined_prefix_bytes", 16777216),
        ("manifest_bytes", 65536),
    ] {
        assert_eq!(c[field].as_integer(), Some(value));
    }
    for (field, value) in [
        ("source_directory_mode", "0700"),
        ("source_file_mode", "0600"),
        ("sealed_directory_mode", "0500"),
        ("sealed_file_mode", "0400"),
    ] {
        assert_eq!(c[field].as_str(), Some(value));
    }
    assert_eq!(c["fault_points"].as_array().unwrap().len(), 6);
    assert_eq!(
        c["source_commit"].as_str(),
        Some("62114be0f5da3218706e30d7424acfb5d0579d07")
    );
    for name in ["main.rs", "server.rs", "protocol.rs", "config.rs"] {
        let code =
            std::fs::read_to_string(root().join("tools/long-run-supervisor-074/src").join(name))
                .unwrap();
        assert!(
            !code.contains("diagnostic_spool"),
            "production enrollment: {name}"
        );
    }
    let code = std::fs::read_to_string(
        root().join("tools/long-run-supervisor-074/src/diagnostic_spool.rs"),
    )
    .unwrap();
    for prohibited in [
        "std::process::Command",
        "Command::new",
        "zbus::",
        "tokio::",
        "release_host",
        "remove_dir_all",
    ] {
        assert!(
            !code.contains(prohibited),
            "fixture-only spool: {prohibited}"
        );
    }
    for required in ["RENAME_NOREPLACE", "O_NOFOLLOW", "O_EXCL", "sync_all"] {
        assert!(code.contains(required));
    }
    let directory =
        root().join("docs/testing/performance/0.74/local-runs/diagnostic-spool-88b9b835");
    let m: Value =
        serde_json::from_slice(&std::fs::read(directory.join("manifest.json")).unwrap()).unwrap();
    for (field, expected) in [
        ("schema_version", "diagnostic-spool-local-evidence-074-v1"),
        (
            "implementation_source_commit",
            "88b9b8356b44669a49a52f7dfabb8bc2c4a892a1",
        ),
        (
            "implementation_source_tree",
            "a19041f8eea17cb1b34e98b8e0e96559a17471db",
        ),
        (
            "preregistered_contract_commit",
            "5aab718e37d3f6cc02c38556570eadfa4f181b73",
        ),
        (
            "qualification_manifest_sha256",
            "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc",
        ),
        (
            "root_cargo_lock_sha256",
            "be46eaf8e97e507bda5e3f2b671d0d622e6a639ce1d4613769a4693723d62b19",
        ),
        (
            "observer_cargo_lock_sha256",
            "e3be470f5a1bff4e917e841fc5bfbd257ff12559206bbc1481c3c4bca528eb1d",
        ),
    ] {
        assert_eq!(m[field], expected, "{field}");
    }
    for field in [
        "fixture_is_production_spool",
        "power_loss_durability_proven",
        "live_cgroup_cleanup_proven",
        "autonomous_watchdog_proven",
        "real_builder_enrolled",
        "production_route_enabled",
        "real_product_binary_built",
        "raw_workload_receipt_packet_sealed",
        "pending_intent_reconciliation_implemented",
        "automatic_pending_recovery_allowed",
        "workload_retry_allowed",
        "host_ssh_performed",
        "service_operations_performed",
        "product_workload_started",
        "pilot_executed",
        "qualification_started",
        "full_workspace_verify_run",
        "numerical_performance_claim_allowed",
        "promotable",
        "admission_allowed",
    ] {
        assert_eq!(m[field], false, "{field}");
    }
    for field in [
        "source_clean_before",
        "source_clean_after",
        "synthetic_positive_fixture",
        "local_fixture_spool_publication_checked",
        "bounded_overflow_prefix_checked",
        "no_replace_publication_checked",
        "six_logical_crash_windows_checked",
        "offline_external_digest_reconciliation_checked",
    ] {
        assert_eq!(m[field], true, "{field}");
    }
    for (field, count) in [
        ("windows_passed", 62),
        ("linux_passed", 105),
        ("root_passed", 84),
        ("release_governance_checks_passed", 23),
        ("release_governance_cli_checks", 17),
        ("ship_check_expected_exit", 1),
    ] {
        assert_eq!(m[field], count);
    }
    assert_eq!(m["windows_checks"]["diagnostic_spool"], 0);
    assert_eq!(m["linux_checks"]["diagnostic_spool"], 7);
    assert_eq!(m["files"].as_array().unwrap().len(), 3);
    for (index, (name, size, digest, blob)) in [
        (
            "windows.log",
            6875u64,
            "a47119963e80750fdd6a38d1ab3d8f0e5128963736239ed1ebf2eb8834f34893",
            "82212184d8179a472a40b5f691606cda54f5444a",
        ),
        (
            "linux.log",
            18249,
            "9066a14e928d7788bdcdee675c730a53799ac35d0240698132d6bcb45b327ae4",
            "32e3d26ca3ef1bdc0567370cf562e0f25389c822",
        ),
        (
            "root.log",
            10987,
            "7b418f99865a4135757e92fa77d393ca48c7d547349109b06ae72294727f304a",
            "d2ec793d937680a56d3b2d47f1870187f2d7c7b8",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let entry = &m["files"][index];
        let raw = std::fs::read(directory.join(name)).unwrap();
        assert_eq!(entry["path"], name);
        assert_eq!(entry["bytes"], size);
        assert_eq!(entry["sha256"], digest);
        assert_eq!(entry["git_blob"], blob);
        assert_eq!(raw.len() as u64, size);
        let actual: String = Sha256::digest(&raw)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(actual, digest);
        let text = String::from_utf8(raw).unwrap();
        assert!(!text.contains("test result: FAILED"));
        assert!(text.contains("test result: ok."));
    }
}

#[test]
fn local_raw_diagnostic_receipts_cannot_claim_spool_or_live_execution() {
    let c = contract("diagnostic-receipts-local-contract.toml");
    for field in [
        "production_route_enabled",
        "host_install_allowed",
        "host_pilot_execution_allowed",
        "real_product_build_allowed_by_this_slice",
        "qualification_allowed",
        "promotable",
        "admission_allowed",
        "packet_is_durable_spool_proof",
        "caller_terminal_claim_is_live_cgroup_proof",
        "synthetic_report_is_measurement",
        "retry_allowed",
        "old_inputs_or_packets_mutable",
    ] {
        assert_eq!(c[field].as_bool(), Some(false), "{field}");
    }
    for (field, value) in [
        ("stream_bytes", 8388608),
        ("combined_bytes", 16777216),
        ("minimum_usable_cpu_ns", 1000000000),
        ("minimum_usable_measurement_wall_ns", 1000000000),
        ("samples", 10000),
        ("sample_calendar_step_ns", 200000),
    ] {
        assert_eq!(c[field].as_integer(), Some(value), "{field}");
    }
    assert_eq!(
        c["source_commit"].as_str(),
        Some("62114be0f5da3218706e30d7424acfb5d0579d07")
    );
    assert_eq!(c["preset"].as_str(), Some("baseline-get-four-cells-p0-v1"));
    for name in ["main.rs", "server.rs", "protocol.rs", "config.rs"] {
        let code =
            std::fs::read_to_string(root().join("tools/long-run-supervisor-074/src").join(name))
                .unwrap();
        assert!(
            !code.contains("diagnostic_receipts"),
            "production enrollment: {name}"
        );
    }
    let code = std::fs::read_to_string(
        root().join("tools/long-run-supervisor-074/src/diagnostic_receipts.rs"),
    )
    .unwrap();
    for prohibited in [
        "std::process::Command",
        "Command::new",
        "std::fs::",
        "zbus::",
        "tokio::",
    ] {
        assert!(
            !code.contains(prohibited),
            "byte-only verifier: {prohibited}"
        );
    }
    let directory =
        root().join("docs/testing/performance/0.74/local-runs/diagnostic-receipts-ceaa8b07");
    let m: Value =
        serde_json::from_slice(&std::fs::read(directory.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(
        m["schema_version"],
        "diagnostic-receipts-local-evidence-074-v1"
    );
    assert_eq!(
        m["implementation_source_commit"],
        "ceaa8b07ffef714fe9b3662791941c565aa65e93"
    );
    assert_eq!(
        m["implementation_source_tree"],
        "b565b47562154123593c6517cbb90abf70aec20a"
    );
    assert_eq!(
        m["preregistered_contract_commit"],
        "db2219e7b7751b1b12fde4774ebb3e7afeb4cf41"
    );
    for field in [
        "packet_is_durable_spool_proof",
        "caller_terminal_claim_is_live_cgroup_proof",
        "real_builder_enrolled",
        "production_route_enabled",
        "real_product_binary_built",
        "raw_workload_receipt_packet_sealed",
        "pending_intent_reconciliation_implemented",
        "live_cgroup_cleanup_proven",
        "autonomous_watchdog_proven",
        "host_ssh_performed",
        "service_operations_performed",
        "product_workload_started",
        "pilot_executed",
        "qualification_started",
        "full_workspace_verify_run",
        "numerical_performance_claim_allowed",
        "promotable",
        "admission_allowed",
    ] {
        assert_eq!(m[field], false, "{field}");
    }
    for field in [
        "source_clean_before",
        "source_clean_after",
        "synthetic_positive_fixture",
        "local_raw_receipt_verifier_checked",
        "packet_byte_reconciliation_checked",
    ] {
        assert_eq!(m[field], true, "{field}");
    }
    for (field, count) in [
        ("windows_passed", 62),
        ("linux_passed", 96),
        ("root_passed", 83),
        ("release_governance_checks_passed", 23),
        ("ship_check_expected_exit", 1),
    ] {
        assert_eq!(m[field], count);
    }
    assert_eq!(m["files"].as_array().unwrap().len(), 3);
    for (index, (name, size, digest, blob)) in [
        (
            "windows.log",
            6850u64,
            "648cb8fe2a4175ff9a28455f4777279e58a6405eb483b4126f8c4e0515d57d83",
            "2b663a617f3f6d274c13d2edaefc0b4aa42cdb25",
        ),
        (
            "linux.log",
            16837,
            "82bd768ca8369d65469f73b1e678ef862f9c88f0cfcd671866f287ab625753ad",
            "34f62dd29df2bab2022f11e359df1214e9f39f46",
        ),
        (
            "root.log",
            11570,
            "7b8e800b4c82b5de4ffe53c0424c648b2dce3d98cb8529a063bdf25782f1f3ee",
            "bdf4da05bafebaf081c0ea663d997d81d78643d1",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let entry = &m["files"][index];
        let raw = std::fs::read(directory.join(name)).unwrap();
        assert_eq!(entry["path"], name);
        assert_eq!(entry["bytes"], size);
        assert_eq!(entry["sha256"], digest);
        assert_eq!(entry["git_blob"], blob);
        assert_eq!(raw.len() as u64, size);
        let actual: String = Sha256::digest(&raw)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(actual, digest);
        let text = String::from_utf8(raw).unwrap();
        assert!(!text.contains("test result: FAILED"));
        assert!(text.contains("test result: ok."));
    }
}

#[test]
fn local_diagnostic_artifact_verification_keeps_execution_and_build_claims_closed() {
    let c = contract("diagnostic-artifacts-local-contract.toml");
    for field in [
        "production_route_enabled",
        "host_install_allowed",
        "host_pilot_execution_allowed",
        "real_product_build_allowed_by_this_slice",
        "real_trusted_builder_enrolled",
        "qualification_allowed",
        "promotable",
        "admission_allowed",
        "fixture_root_is_production_root",
        "synthetic_signature_is_real_compilation_proof",
        "path_revalidation_is_exec_race_closure",
        "old_inspection_seal_or_packets_mutable",
    ] {
        assert_eq!(c[field].as_bool(), Some(false), "{field}");
    }
    assert_eq!(
        c["signature_domain"].as_str(),
        Some("hydracache-diagnostic-build-074-v1")
    );
    assert_eq!(
        c["source_commit"].as_str(),
        Some("62114be0f5da3218706e30d7424acfb5d0579d07")
    );
    assert_eq!(
        c["source_tree"].as_str(),
        Some("a9f059d752f5497c8dc5f07fbee4db6b22a366f4")
    );
    assert_eq!(c["preset"].as_str(), Some("baseline-get-four-cells-p0-v1"));
    assert_eq!(
        c["install_root"].as_str(),
        Some("/opt/hydracache-performance/0.74/diagnostic-pilot")
    );
    for (field, value) in [
        ("binary_bytes", 134217728),
        ("receipt_bytes", 65536),
        ("config_bytes", 65536),
        ("build_log_bytes", 16777216),
        ("lock_bytes", 1048576),
        ("production_uid", 0),
        ("production_gid", 0),
    ] {
        assert_eq!(c[field].as_integer(), Some(value), "{field}");
    }
    for (field, value) in [
        ("binary_mode", "0555"),
        ("data_mode", "0444"),
        ("bundle_mode", "0555"),
    ] {
        assert_eq!(c[field].as_str(), Some(value));
    }
    for name in ["main.rs", "server.rs", "protocol.rs", "config.rs"] {
        let mut code =
            std::fs::read_to_string(root().join("tools/long-run-supervisor-074/src").join(name))
                .unwrap();
        if name == "main.rs" {
            // The separately preregistered operator-only unsigned audit is not
            // installed bundle/BuildTrust enrollment. Admit this exact read
            // function once, not the artifacts module in other production paths.
            let audit = "diagnostic_artifacts::inspect_unsigned_files(";
            assert_eq!(code.matches(audit).count(), 1);
            code = code.replace(audit, "unsigned_operator_content_audit(");
        }
        for enrollment in [
            "diagnostic_artifacts",
            "inspect_fixed_install",
            "BuildTrust",
        ] {
            assert!(!code.contains(enrollment), "{name}: {enrollment}");
        }
    }
    let digest: String = Sha256::digest(
        std::fs::read(root().join("docs/testing/performance/0.74/qualification-manifest.toml"))
            .unwrap(),
    )
    .iter()
    .map(|b| format!("{b:02x}"))
    .collect();
    assert_eq!(
        digest,
        "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
    );
    let packet =
        root().join("docs/testing/performance/0.74/local-runs/diagnostic-artifacts-70ca60eb");
    let m: Value =
        serde_json::from_slice(&std::fs::read(packet.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(
        m["schema_version"],
        "diagnostic-artifacts-local-evidence-074-v1"
    );
    assert_eq!(
        m["implementation_source_commit"],
        "70ca60eb3f2f75a326e9f6430467749b76440a04"
    );
    assert_eq!(
        m["implementation_source_tree"],
        "4ab1648c5cdf09e6a7fc0a425e884028b300bad2"
    );
    assert_eq!(
        m["preregistered_contract_commit"],
        "127405fc3300d2e8e6274c546c7132426cab12b2"
    );
    assert_eq!(m["qualification_manifest_sha256"], digest);
    assert_eq!(m["ship_check_expected_exit"], 1);
    for flag in [
        "source_clean_before",
        "source_clean_after",
        "local_attestation_verifier_checked",
        "synthetic_positive_fixture",
        "linux_temporary_descriptor_checks_passed",
    ] {
        assert_eq!(m[flag], true, "{flag}");
    }
    for flag in [
        "production_fixed_root_inspected",
        "production_route_enabled",
        "real_trusted_builder_enrolled",
        "real_product_binary_built",
        "build_provenance_proven",
        "path_revalidation_is_exec_race_closure",
        "pending_intent_reconciliation_implemented",
        "live_cgroup_cleanup_proven",
        "autonomous_watchdog_proven",
        "raw_workload_receipt_packet_sealed",
        "host_ssh_performed",
        "service_operations_performed",
        "product_workload_started",
        "pilot_executed",
        "qualification_started",
        "full_workspace_verify_run",
        "numerical_performance_claim_allowed",
        "promotable",
        "admission_allowed",
    ] {
        assert_eq!(m[flag], false, "{flag}");
    }
    for (path, field, expected) in [
        (
            "Cargo.lock",
            "root_cargo_lock_sha256",
            "be46eaf8e97e507bda5e3f2b671d0d622e6a639ce1d4613769a4693723d62b19",
        ),
        (
            "tools/get-owner-scheduled-controls-074/Cargo.lock",
            "observer_cargo_lock_sha256",
            "e3be470f5a1bff4e917e841fc5bfbd257ff12559206bbc1481c3c4bca528eb1d",
        ),
    ] {
        let actual: String = Sha256::digest(std::fs::read(root().join(path)).unwrap())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(actual, expected);
        assert_eq!(m[field], expected);
    }
    assert_eq!(m["files"].as_array().unwrap().len(), 3);
    for (name, bytes_expected, hash, passed, summaries) in [
        (
            "windows.log",
            4923,
            "2a4432c3b029d320231392af8000bcfd7970136ff1a7cdd70127eafdf33fc8a3",
            51,
            4,
        ),
        (
            "linux.log",
            15211,
            "e0e804026e015b3ed2b50dd5b3b0329e256e8ef81151099a2c663bbd606c80cb",
            85,
            6,
        ),
        (
            "root.log",
            8632,
            "a3d317a3c82bc486cb99e932ace4f2010f47265912e17cd3d97b61de8efb7594",
            82,
            2,
        ),
    ] {
        let entry = m["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["path"] == name)
            .unwrap();
        let bytes = std::fs::read(packet.join(name)).unwrap();
        assert_eq!(bytes.len(), bytes_expected);
        assert_eq!(entry["bytes"], bytes_expected);
        let actual: String = Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(actual, hash);
        assert_eq!(entry["sha256"], hash);
        let text = String::from_utf8(bytes).unwrap();
        assert_eq!(
            text.lines()
                .filter(|s| s.starts_with("test ") && s.trim_end().ends_with(" ... ok"))
                .count(),
            passed
        );
        let rows: Vec<_> = text
            .lines()
            .filter(|s| s.starts_with("test result:"))
            .collect();
        assert_eq!(rows.len(), summaries);
        assert!(rows.iter().all(|s| s.contains("0 failed; 0 ignored;")));
    }
    let log = std::fs::read_to_string(packet.join("root.log")).unwrap();
    assert!(log.contains("ship admission is closed while candidate identity and release qualification are incomplete"));
}

#[test]
fn local_diagnostic_ipc_keeps_production_and_numerical_admission_closed() {
    let c = contract("diagnostic-ipc-local-contract.toml");
    for flag in [
        "production_route_enabled",
        "host_install_allowed",
        "host_pilot_execution_allowed",
        "qualification_allowed",
        "promotable",
        "admission_allowed",
        "ledger_eviction_allowed",
        "pending_request_reexecution_allowed",
        "replayed_heartbeat_extends_lease",
        "arbitrary_paths_argv_or_workload_allowed",
        "old_campaign_protocol_changed",
    ] {
        assert_eq!(c[flag].as_bool(), Some(false), "{flag}");
    }
    assert_eq!(
        c["revision_check_and_mutation_share_host_fence"].as_bool(),
        Some(true)
    );
    for (field, value) in [
        ("packet_bytes", 16384),
        ("authorization_seconds", 60),
        ("future_clock_skew_seconds", 0),
        ("ledger_bytes", 262144),
        ("ledger_entries", 128),
    ] {
        assert_eq!(c[field].as_integer(), Some(value), "{field}");
    }
    assert_eq!(
        c["signature_domain"].as_str(),
        Some("hydracache-diagnostic-request-074-v1")
    );
    assert_eq!(c["preset"].as_str(), Some("baseline-get-four-cells-p0-v1"));
    for name in ["main.rs", "server.rs", "protocol.rs", "config.rs"] {
        let code =
            std::fs::read_to_string(root().join("tools/long-run-supervisor-074/src").join(name))
                .unwrap();
        for enrollment in [
            "diagnostic_ipc",
            "handle_local_connection",
            "SignedDiagnosticRequest",
        ] {
            assert!(!code.contains(enrollment), "{name}: {enrollment}");
        }
    }
    let p0 = contract("rental-diagnostic-pilot-contract.toml");
    assert_eq!(p0["numerical_execution_allowed"].as_bool(), Some(false));
    let digest: String = Sha256::digest(
        std::fs::read(root().join("docs/testing/performance/0.74/qualification-manifest.toml"))
            .unwrap(),
    )
    .iter()
    .map(|b| format!("{b:02x}"))
    .collect();
    assert_eq!(
        digest,
        "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
    );
    let packet = root().join("docs/testing/performance/0.74/local-runs/diagnostic-ipc-403696a7");
    let m: Value =
        serde_json::from_slice(&std::fs::read(packet.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(
        m["implementation_source_commit"],
        "403696a7ab3e61870e5346592e5aa7cc9ed08b88"
    );
    assert_eq!(
        m["preregistered_contract_commit"],
        "cf110eda1702c2db01adc438762394514ef52930"
    );
    for flag in [
        "source_clean_before",
        "source_clean_after",
        "local_authenticated_ipc_checked",
    ] {
        assert_eq!(m[flag], true, "{flag}");
    }
    for flag in [
        "production_route_enabled",
        "pending_intent_reconciliation_implemented",
        "ledger_eviction_allowed",
        "live_cgroup_cleanup_proven",
        "autonomous_watchdog_proven",
        "build_provenance_proven",
        "raw_workload_receipt_packet_sealed",
        "host_ssh_performed",
        "service_operations_performed",
        "product_workload_started",
        "pilot_executed",
        "qualification_started",
        "full_workspace_verify_run",
        "numerical_performance_claim_allowed",
        "promotable",
        "admission_allowed",
    ] {
        assert_eq!(m[flag], false, "{flag}");
    }
    assert_eq!(m["qualification_manifest_sha256"], digest);
    assert_eq!(m["ship_check_expected_exit"], 1);
    assert_eq!(m["files"].as_array().unwrap().len(), 3);
    for (name, passed, summaries) in [
        ("windows.log", 42, 3),
        ("linux.log", 70, 5),
        ("root.log", 81, 2),
    ] {
        let file = m["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["path"] == name)
            .unwrap();
        let bytes = std::fs::read(packet.join(name)).unwrap();
        assert_eq!(bytes.len() as u64, file["bytes"].as_u64().unwrap());
        let hex: String = Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(hex, file["sha256"].as_str().unwrap());
        let log = String::from_utf8(bytes).unwrap();
        assert_eq!(
            log.lines()
                .filter(|s| s.starts_with("test ") && s.trim_end().ends_with(" ... ok"))
                .count(),
            passed
        );
        let rows: Vec<_> = log
            .lines()
            .filter(|s| s.starts_with("test result:"))
            .collect();
        assert_eq!(rows.len(), summaries);
        assert!(rows.iter().all(|s| s.contains("0 failed; 0 ignored;")));
    }
    let root_log = std::fs::read_to_string(packet.join("root.log")).unwrap();
    assert!(root_log.contains("ship admission is closed while candidate identity and release qualification are incomplete"));
}

#[test]
fn local_diagnostic_lease_cannot_enable_production_execution_or_weaken_p0() {
    let c = contract("diagnostic-lease-local-contract.toml");
    assert_eq!(c["local_implementation_authorized"].as_bool(), Some(true));
    for flag in [
        "production_cli_or_ipc_enabled",
        "host_install_allowed",
        "host_pilot_execution_allowed",
        "qualification_allowed",
        "promotable",
        "admission_allowed",
        "external_flock_allowed",
        "automatic_retry_allowed",
        "restart_start_intent_allowed",
        "cleanup_failure_releases_reservation",
        "pid_only_exit_is_empty_cgroup",
        "deployment_or_live_cgroup_rehearsal_complete",
        "old_sealed_packets_mutable",
    ] {
        assert_eq!(c[flag].as_bool(), Some(false), "{flag}");
    }
    for (field, value) in [
        ("cell_seconds", 60),
        ("total_seconds", 300),
        ("controller_loss_seconds", 10),
        ("receipt_bytes", 16_777_216),
    ] {
        assert_eq!(c[field].as_integer(), Some(value), "{field}");
    }
    assert_eq!(
        c["reservation"]["lock"].as_str(),
        Some(".host-execution.lock")
    );
    assert_eq!(
        c["reservation"]["marker"].as_str(),
        Some("active-diagnostic.json")
    );
    assert_eq!(
        c["reservation"]["lock_held_for_workload_lifetime"].as_bool(),
        Some(false)
    );
    assert_eq!(
        c["local_test_scope"]["live_backend_or_authentication_proven"].as_bool(),
        Some(false)
    );
    let protocol =
        std::fs::read_to_string(root().join("tools/long-run-supervisor-074/src/protocol.rs"))
            .unwrap();
    let cli =
        std::fs::read_to_string(root().join("tools/long-run-supervisor-074/src/main.rs")).unwrap();
    for code in [protocol, cli] {
        assert!(!code.contains("DiagnosticCoordinator"));
        assert!(!code.contains("DiagnosticStart"));
    }
    let p0 = contract("rental-diagnostic-pilot-contract.toml");
    assert_eq!(p0["numerical_execution_allowed"].as_bool(), Some(false));
    assert_eq!(
        p0["p0_cpu_feasibility"]["minimum_usable_cpu_ns"].as_integer(),
        Some(1_000_000_000)
    );
    let packet = root().join("docs/testing/performance/0.74/local-runs/diagnostic-lease-f4f68ad7");
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(packet.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(
        manifest["implementation_source_commit"],
        "f4f68ad734bcfb454f4a5531c68d3af5fc25a7d5"
    );
    for flag in ["source_clean_before", "source_clean_after"] {
        assert_eq!(manifest[flag], true);
    }
    for flag in [
        "live_cgroup_cleanup_proven",
        "authentication_or_build_provenance_proven",
        "raw_workload_receipt_packet_sealed",
        "host_ssh_performed",
        "service_operations_performed",
        "product_workload_started",
        "pilot_executed",
        "qualification_started",
        "numerical_performance_claim_allowed",
        "promotable",
        "admission_allowed",
    ] {
        assert_eq!(manifest[flag], false, "{flag}");
    }
    let hex = |bytes: &[u8]| {
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };
    assert_eq!(
        manifest["qualification_manifest_sha256"].as_str().unwrap(),
        hex(&std::fs::read(
            root().join("docs/testing/performance/0.74/qualification-manifest.toml")
        )
        .unwrap())
    );
    assert_eq!(manifest["files"].as_array().unwrap().len(), 2);
    for file in manifest["files"].as_array().unwrap() {
        let name = file["path"].as_str().unwrap();
        assert!(["windows.log", "linux.log"].contains(&name));
        let bytes = std::fs::read(packet.join(name)).unwrap();
        assert_eq!(bytes.len() as u64, file["bytes"].as_u64().unwrap());
        assert_eq!(hex(&bytes), file["sha256"].as_str().unwrap());
    }
    for (name, expected) in [("windows.log", 23), ("linux.log", 45)] {
        let log = std::fs::read_to_string(packet.join(name)).unwrap();
        assert_eq!(
            log.lines()
                .filter(|line| line.starts_with("test ") && line.trim_end().ends_with(" ... ok"))
                .count(),
            expected
        );
        let summaries: Vec<_> = log
            .lines()
            .filter(|line| line.starts_with("test result:"))
            .collect();
        assert_eq!(summaries.len(), if name == "windows.log" { 2 } else { 3 });
        assert!(
            summaries
                .iter()
                .all(|line| line.contains("test result: ok.")
                    && line.contains("0 failed; 0 ignored;"))
        );
    }
}

#[test]
fn rental_pilot_preparation_cannot_bypass_supervisor_or_authorize_execution() {
    let c = contract("rental-pilot-coordinator-contract.toml");
    assert_eq!(c["state"].as_str(), Some("preregistered-preparation-only"));
    for flag in [
        "counting_allocator_allowed",
        "product_or_fixture_execution_allowed",
        "host_reservation_allowed",
        "remote_build_or_upload_allowed",
        "service_mutation_allowed",
        "qualification_allowed",
        "promotable",
        "admission_allowed",
        "require_ship",
        "metadata_timeout_is_workload_tree_proof",
        "hash_valid_seal_is_compilation_provenance",
        "verified_build_is_pilot_authorization",
    ] {
        assert_eq!(c[flag].as_bool(), Some(false), "{flag}");
    }
    let reservation = &c["reservation_finding"];
    for flag in [
        "external_flock_allowed",
        "external_active_marker_allowed",
        "private_uncoordinated_lock_allowed",
        "finding_is_live_fault_rehearsal",
    ] {
        assert_eq!(reservation[flag].as_bool(), Some(false), "{flag}");
    }
    assert_eq!(
        reservation["resolution_requires_explicit_service_or_control_plane_review"].as_bool(),
        Some(true)
    );
    let deadline = &c["future_workload_deadline"];
    for (field, value) in [
        ("process_seconds", 60),
        ("total_seconds", 300),
        ("receipt_bytes", 16_777_216),
    ] {
        assert_eq!(deadline[field].as_integer(), Some(value));
    }
    for flag in [
        "implementation_and_host_rehearsal_complete",
        "process_group_alone_certifies_arbitrary_tree",
        "automatic_retry_allowed",
    ] {
        assert_eq!(deadline[flag].as_bool(), Some(false), "{flag}");
    }
    for flag in [
        "owned_cgroup_and_child_tree_required",
        "controller_loss_must_not_orphan_workload",
        "partial_stdout_stderr_retained",
        "timeout_or_overflow_is_invalid",
    ] {
        assert_eq!(deadline[flag].as_bool(), Some(true), "{flag}");
    }
    let packet = root().join("docs/testing/performance/0.74/local-runs/rental-prepare-f0030b50");
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(packet.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(
        manifest["preparation_source_commit"],
        "f0030b50edb9886992dc232d33f7146055fd3d33"
    );
    for flag in ["source_clean_before", "source_clean_after"] {
        assert_eq!(manifest[flag], true);
    }
    for flag in [
        "metadata_timeout_is_workload_tree_proof",
        "compilation_provenance_proven",
        "real_linux_binary_inspected",
        "host_ssh_performed",
        "service_operations_performed",
        "product_process_started",
        "pilot_executed",
        "qualification_started",
        "promotable",
        "admission_allowed",
    ] {
        assert_eq!(manifest[flag], false, "{flag}");
    }
    let hex = |bytes: &[u8]| {
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };
    for file in manifest["files"].as_array().unwrap() {
        let name = file["path"].as_str().unwrap();
        assert!(["windows.log", "linux.log", "plan.json"].contains(&name));
        let bytes = std::fs::read(packet.join(name)).unwrap();
        assert_eq!(bytes.len() as u64, file["bytes"].as_u64().unwrap());
        assert_eq!(hex(&bytes), file["sha256"].as_str().unwrap());
    }
    assert_eq!(manifest["files"].as_array().unwrap().len(), 3);
    assert_eq!(
        hex(&std::fs::read(root().join(manifest["preparation_path"].as_str().unwrap())).unwrap()),
        manifest["preparation_sha256"].as_str().unwrap()
    );
    assert_eq!(
        hex(&std::fs::read(
            root().join("docs/testing/performance/0.74/qualification-manifest.toml")
        )
        .unwrap()),
        manifest["qualification_manifest_sha256"].as_str().unwrap()
    );
    for (name, field) in [
        ("windows.log", "windows_checks"),
        ("linux.log", "local_wsl_linux_checks"),
    ] {
        assert_eq!(manifest[field], 11);
        let log = std::fs::read_to_string(packet.join(name)).unwrap();
        assert_eq!(
            log.lines()
                .filter(|line| line.trim_end().ends_with(" ... ok"))
                .count(),
            11
        );
        assert!(log.contains("Ran 11 tests"));
        assert_eq!(
            log.lines()
                .rfind(|line| !line.trim().is_empty())
                .unwrap()
                .trim(),
            "OK"
        );
    }
    let plan: Value =
        serde_json::from_slice(&std::fs::read(packet.join("plan.json")).unwrap()).unwrap();
    assert_eq!(plan["state"], "PREPARATION_ONLY");
    assert_eq!(plan["host_reservation"], "BLOCKED_EXTERNAL_FLOCK_UNSAFE");
    for flag in [
        "metadata_commands_started",
        "build_started",
        "linux_binary_verified",
        "workload_started",
        "workload_child_tree_deadline_implemented",
        "pilot_execution_allowed",
        "promotable",
        "admission_allowed",
    ] {
        assert_eq!(plan[flag], false, "{flag}");
    }
    for (field, name) in [
        (
            "coordinator_contract_sha256",
            "rental-pilot-coordinator-contract.toml",
        ),
        (
            "pilot_contract_sha256",
            "rental-diagnostic-pilot-contract.toml",
        ),
    ] {
        assert_eq!(
            plan[field].as_str().unwrap(),
            hex(&std::fs::read(root().join("docs/testing/performance/0.74").join(name)).unwrap())
        );
    }
    let p0 = contract("rental-diagnostic-pilot-contract.toml");
    for (index, cell) in p0["p0_cpu_feasibility"]["cells"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        let receipt = &plan["configs"][index];
        assert_eq!(receipt["surface"].as_str(), cell["surface"].as_str());
        assert_eq!(
            receipt["workload_sha256_from_contract"].as_str(),
            cell["workload_sha256"].as_str()
        );
        assert_eq!(
            receipt["raw_config_sha256"].as_str().unwrap(),
            hex(&std::fs::read(
                root()
                    .join("docs/testing/performance/0.74")
                    .join(cell["config"].as_str().unwrap())
            )
            .unwrap())
        );
    }
    assert_eq!(plan["configs"].as_array().unwrap().len(), 4);
}

#[test]
fn rental_preflight_and_pilot_draft_never_open_numerical_or_host_admission() {
    let c = contract("rental-diagnostic-pilot-contract.toml");
    for flag in [
        "host_audit_is_host_reservation",
        "numerical_execution_allowed",
        "service_mutation_allowed",
        "product_changes_allowed",
        "product_candidate_execution_allowed",
        "allocator_change_allowed",
        "qualification_allowed",
        "same_box_redis_allowed",
        "invalidated_b0_retry_allowed",
        "promotable",
        "admission_allowed",
        "full_d3_completed",
        "freeze_c74_allowed",
        "supported_system_retention_proven",
    ] {
        assert_eq!(c[flag].as_bool(), Some(false), "{flag}");
    }
    assert_eq!(c["state"].as_str(), Some("prepared-not-executable"));
    let p0 = &c["p0_cpu_feasibility"];
    assert_eq!(p0["maximum_fresh_processes"].as_integer(), Some(4));
    assert_eq!(
        p0["maximum_total_execution_wall_seconds"].as_integer(),
        Some(300)
    );
    assert_eq!(
        p0["minimum_usable_cpu_ns"].as_integer(),
        Some(1_000_000_000)
    );
    assert_eq!(
        p0["minimum_usable_measurement_wall_ns"].as_integer(),
        Some(1_000_000_000)
    );
    for flag in [
        "secure_cells_allowed",
        "automatic_retry_allowed",
        "aa_noise_calibration_claim_allowed",
        "ab_comparison_allowed",
        "cross_surface_numeric_comparison_allowed",
        "allocation_or_retention_claim_allowed",
        "padding_cpu_or_lowering_quality_floors_allowed",
    ] {
        assert_eq!(p0[flag].as_bool(), Some(false), "{flag}");
    }
    for flag in [
        "stop_first_invalid",
        "retain_every_attempt",
        "exclusive_host_reservation_required",
        "binary_and_clean_source_seal_required_before_first_process",
        "child_process_tree_deadline_owner_required",
    ] {
        assert_eq!(p0[flag].as_bool(), Some(true), "{flag}");
    }
    let cells = p0["cells"].as_array().unwrap();
    assert_eq!(cells.len(), 4);
    for (index, surface) in ["embedded", "direct", "resp2", "resp3"].iter().enumerate() {
        let cell = &cells[index];
        assert_eq!(cell["order"].as_integer(), Some((index + 1) as i64));
        assert_eq!(cell["surface"].as_str(), Some(*surface));
        let input: Value = serde_json::from_slice(
            &std::fs::read(
                root()
                    .join("docs/testing/performance/0.74")
                    .join(cell["config"].as_str().unwrap()),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(input["surface"], *surface);
        assert_eq!(input["operation"], "get");
        for field in [
            "seed",
            "keyspace",
            "payload_bytes",
            "warmup_calls",
            "slots",
            "minimum_usable_cpu_ns",
            "minimum_usable_measurement_wall_ns",
        ] {
            assert_eq!(input[field].as_i64(), p0[field].as_integer(), "{field}");
        }
        assert_eq!(
            input["dataset_sha256"].as_str(),
            p0["dataset_sha256"].as_str()
        );
        assert_eq!(input["schedule"]["operations"], 10_000);
        assert_eq!(input["schedule"]["offered_rate_per_second"], 5_000);
        assert_eq!(input["schedule"]["concurrency"], 8);
        for field in [
            "maximum_queued",
            "operation_timeout_ns",
            "drain_timeout_ns",
            "slo_ns",
            "highest_trackable_ns",
        ] {
            assert_eq!(
                input["schedule"][field].as_i64(),
                p0[field].as_integer(),
                "{field}"
            );
        }
        assert_eq!(
            input["pipeline_depth"].as_i64(),
            cell["pipeline_depth"].as_integer()
        );
    }
    let packet = root().join("docs/testing/performance/0.74/local-runs/rental-preflight-62114be0");
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(packet.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(
        manifest["collector_source_commit"],
        "62114be0f5da3218706e30d7424acfb5d0579d07"
    );
    for flag in [
        "service_operations_performed",
        "product_process_started",
        "pilot_executed",
        "aa_noise_gate_pass_claimed",
        "allocator_native_active_resident_retained_proven",
        "qualification_started",
        "promotable",
        "admission_allowed",
        "signed_host_admission_claimed",
    ] {
        assert_eq!(manifest[flag], false, "{flag}");
    }
    let capture = std::fs::read(packet.join("capture.json")).unwrap();
    assert_eq!(
        capture.len() as u64,
        manifest["raw_capture"]["bytes"].as_u64().unwrap()
    );
    let hex = |bytes: &[u8]| {
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };
    assert_eq!(
        hex(&capture),
        manifest["raw_capture"]["sha256"].as_str().unwrap()
    );
    assert_eq!(
        hex(&std::fs::read(root().join(manifest["collector_path"].as_str().unwrap())).unwrap()),
        manifest["collector_sha256"].as_str().unwrap()
    );
    assert_eq!(
        hex(&std::fs::read(
            root().join("docs/testing/performance/0.74/qualification-manifest.toml")
        )
        .unwrap()),
        manifest["qualification_manifest_sha256"].as_str().unwrap()
    );
    let observed: Value = serde_json::from_slice(&capture).unwrap();
    for phase in ["lifecycle_before", "lifecycle_after"] {
        assert_eq!(observed[phase]["processes"]["complete"], true);
        assert_eq!(observed[phase]["markers"].as_array().unwrap().len(), 3);
        for marker in observed[phase]["markers"].as_array().unwrap() {
            assert_eq!(marker["status"], "absent");
        }
        assert!(observed[phase]["supervisor"]["stdout"]
            .as_str()
            .unwrap()
            .contains("MainPID=8839\nNRestarts=0"));
    }
    assert_eq!(
        observed["provisioning"]["fields"]["source_commit"],
        "543108f1ccd206ae670803c2617f07e7fa91ae62"
    );
    for (_, binary) in observed["installed_binary_hashes"].as_object().unwrap() {
        assert_eq!(
            binary["sha256"],
            observed["provisioning"]["fields"]["binary_sha256"]
        );
    }
    assert_eq!(
        observed["allocator_native_active_resident_retained_proven"],
        false
    );
    assert_eq!(observed["cpu_sample"]["noise_gate_pass_claimed"], false);
}

#[test]
fn unprofiled_timing_instrumentation_preserves_closed_cohort_and_memory_admission() {
    let c = contract("unprofiled-timing-controls-contract.toml");
    for flag in [
        "product_changes_allowed",
        "numerical_cohort_execution_allowed",
        "product_numeric_claims_allowed",
        "admission_allowed",
        "invalidated_b0_retry_allowed",
        "full_d3_completed",
        "freeze_c74_allowed",
        "counting_allocator_allowed",
        "short_measurement_is_a_pass",
        "secure_fresh_process_material_parity_proven",
        "cross_surface_numeric_comparison_allowed",
        "allocation_or_retention_claims_allowed",
    ] {
        assert_eq!(c[flag].as_bool(), Some(false), "{flag}");
    }
    assert_eq!(c["minimum_usable_cpu_ns"].as_integer(), Some(1_000_000_000));
    assert_eq!(
        c["minimum_usable_measurement_wall_ns"].as_integer(),
        Some(1_000_000_000)
    );
    assert_eq!(c["maximum_operations"].as_integer(), Some(10_000));
    let tool = root().join("tools/get-owner-scheduled-controls-074");
    let binary = std::fs::read_to_string(tool.join("src/bin/timing_controls.rs")).unwrap();
    assert!(binary.contains("timing executable refuses allocation-diagnostics builds"));
    assert!(binary.contains("timing binary seal mismatch before fixture"));
    assert!(binary.contains("source_git_identity_verified_by_binary: false"));
    for path in [
        "src/lib.rs",
        "src/timing.rs",
        "src/timing/cpu.rs",
        "src/bin/timing_controls.rs",
    ] {
        assert!(
            !std::fs::read_to_string(tool.join(path))
                .unwrap()
                .contains("#[global_allocator]"),
            "{path}"
        );
    }
    let ledger = contract("composition-ledger.toml");
    assert_eq!(ledger["accepted_candidate_count"].as_integer(), Some(0));
    assert_eq!(ledger["freeze_c74_allowed"].as_bool(), Some(false));
}

#[test]
fn secure_observer_and_memory_lane_do_not_admit_timing_or_allocator_retention() {
    let c = contract("secure-observer-memory-checks-contract.toml");
    for flag in [
        "promotable",
        "product_numeric_claims_allowed",
        "qualification_allowed",
        "invalidated_b0_retry_allowed",
        "product_changes_allowed",
    ] {
        assert_eq!(c[flag].as_bool(), Some(false), "{flag}");
    }
    for flag in [
        "production_runtime_tls_factory_required",
        "production_accept_loop_required",
        "required_client_certificate",
        "auth_before_hello_and_preload_on_every_socket",
        "shutdown_checks_zero_active_connections",
        "shared_ephemeral_pki_between_resp_and_hc2",
    ] {
        assert_eq!(c["secure_adapter"][flag].as_bool(), Some(true), "{flag}");
    }
    for flag in [
        "matched_transport_is_matched_application_authorization",
        "matched_transport_is_matched_batch_atomicity",
        "cross_surface_numeric_comparison_allowed",
        "private_keys_or_auth_tokens_in_receipts_allowed",
    ] {
        assert_eq!(c["secure_adapter"][flag].as_bool(), Some(false), "{flag}");
    }
    for flag in [
        "rss_is_heap_resident",
        "requested_live_is_allocator_retained",
        "logical_retention_is_allocator_retained",
        "allocator_active_resident_retained_unavailable_means_pass",
        "timing_metrics_allowed",
        "cpu_goodput_or_latency_claims_allowed",
        "retry_failed_attempts",
    ] {
        assert_eq!(c["memory"][flag].as_bool(), Some(false), "{flag}");
    }
    assert_eq!(c["memory"]["runtime"].as_str(), Some("current-thread"));
    assert_eq!(c["memory"]["repeats"].as_integer(), Some(3));
    assert_eq!(c["memory"]["failed_attempts_allowed"].as_integer(), Some(0));
    assert_eq!(
        c["memory"]["preload_delete_refill_workload_calls"].as_integer(),
        Some(16)
    );
    let tool = root().join("tools/get-owner-scheduled-controls-074");
    let cargo: toml::Value =
        toml::from_str(&std::fs::read_to_string(tool.join("Cargo.toml")).unwrap()).unwrap();
    assert_eq!(cargo["features"]["default"].as_array().unwrap().len(), 0);
    assert_eq!(
        cargo["bin"][0]["required-features"][0].as_str(),
        Some("allocation-diagnostics")
    );
    let binary = std::fs::read_to_string(tool.join("src/bin/memory_diagnostics.rs")).unwrap();
    assert!(binary.contains("#[global_allocator]"));
    assert!(binary.contains("unavailable-not-a-pass"));
    assert!(binary.contains("product_numeric_claims_allowed: false"));
    assert!(!std::fs::read_to_string(tool.join("src/lib.rs"))
        .unwrap()
        .contains("global_allocator"));
    let ledger = contract("composition-ledger.toml");
    assert_eq!(ledger["accepted_candidate_count"].as_integer(), Some(0));
    assert_eq!(ledger["freeze_c74_allowed"].as_bool(), Some(false));
}

#[test]
fn timing_instrumentation_receipts_bind_raw_logs_without_admitting_a_cohort() {
    let packet =
        root().join("docs/testing/performance/0.74/local-runs/timing-instrumentation-9e79b012");
    let source = "9e79b012a6046db9946da1bb972750d83b966d5b";
    for name in ["checks.json", "hosted-semantics-receipt.json"] {
        let receipt: Value =
            serde_json::from_slice(&std::fs::read(packet.join(name)).unwrap()).unwrap();
        assert_eq!(receipt["checked_source_commit"], source);
        assert_eq!(receipt["full_d3_completed"], false);
        if name == "checks.json" {
            for flag in [
                "numerical_series_started",
                "accepted_candidate",
                "admission_allowed",
                "product_numeric_claims_allowed",
                "freeze_c74_allowed",
                "full_workspace_verify_claimed",
            ] {
                assert_eq!(receipt[flag], false, "{flag}");
            }
            assert_eq!(receipt["scope"]["minimum_usable_cpu_ns"], 1_000_000_000_u64);
            assert_eq!(
                receipt["scope"]["minimum_usable_measurement_wall_ns"],
                1_000_000_000_u64
            );
            assert_eq!(
                receipt["scope"]["secure_fresh_process_pki_parity_proven"],
                false
            );
            assert_eq!(
                receipt["scope"]["independent_native_performance_floor_measured"],
                false
            );
            assert_eq!(receipt["scope"]["allocator_retention_supported"], false);
            assert_eq!(receipt["raw_log_receipts"].as_array().unwrap().len(), 14);
        } else {
            assert_eq!(receipt["promotable"], false);
            assert_eq!(receipt["qualification"], false);
            assert_eq!(receipt["native_numeric_admission"], false);
            assert_eq!(receipt["run_id"], 37742392078_u64);
            assert_eq!(receipt["attempt"], 1);
            assert_eq!(receipt["raw_log_receipts"].as_array().unwrap().len(), 9);
            let identity =
                std::fs::read_to_string(packet.join("hosted-semantics/identity.txt")).unwrap();
            assert!(identity.contains(&format!("source_commit={source}")));
            assert!(identity
                .contains("run_id=37742392078\nattempt=1\npromotable=false\nqualification=false"));
        }
        for file in receipt["raw_log_receipts"].as_array().unwrap() {
            let path = Path::new(file["path"].as_str().unwrap());
            assert_eq!(path.components().count(), 2);
            assert!(path
                .components()
                .all(|part| matches!(part, std::path::Component::Normal(_))));
            let bytes = std::fs::read(packet.join(path)).unwrap();
            assert_eq!(bytes.len() as u64, file["bytes"].as_u64().unwrap());
            assert_eq!(
                Sha256::digest(&bytes)
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>(),
                file["sha256"].as_str().unwrap()
            );
            let actual_tests: u64 = String::from_utf8(bytes)
                .unwrap()
                .lines()
                .filter_map(|line| {
                    line.strip_prefix("test result: ok. ")
                        .and_then(|value| value.split_once(" passed; 0 failed;"))
                        .map(|(count, _)| count.parse::<u64>().unwrap())
                })
                .sum();
            assert_eq!(actual_tests, file["passed_tests"].as_u64().unwrap());
            if file["name"] == "default.log" || file["name"] == "get-owner.log" {
                assert_eq!(actual_tests, 88);
            }
            if file["name"] == "ship-expected-red.log" {
                assert_eq!(file["expected_exit"], 1);
                let text = std::fs::read_to_string(packet.join(path)).unwrap();
                assert!(text.contains("ship admission is closed"));
            }
        }
    }
}

#[test]
fn observer_hosted_ci_is_only_semantics_and_never_a_product_campaign() {
    let text = std::fs::read_to_string(root().join(".github/workflows/observer-semantics-074.yml"))
        .unwrap();
    let workflow: serde_yaml::Value = serde_yaml::from_str(&text).unwrap();
    assert_eq!(
        workflow["jobs"]["semantic"]["runs-on"].as_str(),
        Some("ubuntu-24.04")
    );
    assert_eq!(
        workflow["jobs"]["semantic"]["timeout-minutes"].as_u64(),
        Some(40)
    );
    assert!(text.contains("dtolnay/rust-toolchain@1.94.0"));
    assert!(text.contains("ref: ${{ github.sha }}"));
    assert!(text.contains("--all-targets --all-features --locked -- --test-threads=1"));
    for forbidden in [
        "self-hosted",
        "workflow_run:",
        "workflow_call:",
        "systemctl",
        "cargo run",
        "performance-long-run",
        "--require-ship",
        "performance_secure_memory_074.ps1",
        "redis-benchmark",
    ] {
        assert!(!text.contains(forbidden), "{forbidden}");
    }
}

#[test]
fn embedded_scheduled_control_is_independent_public_api_not_a_b0_retry() {
    let c = contract("embedded-scheduled-controls-contract.toml");
    for flag in [
        "product_changes_allowed",
        "product_numeric_claims_allowed",
        "invalidated_b0_retry_allowed",
        "full_d3_completed",
        "freeze_c74_allowed",
        "listeners_created",
        "counting_allocator_in_library_allowed",
        "numerical_execution_allowed",
    ] {
        assert_eq!(c[flag].as_bool(), Some(false), "{flag}");
    }
    assert_eq!(
        c["same_dataset_digest_as_native_required"].as_bool(),
        Some(true)
    );
    let tool = root().join("tools/get-owner-scheduled-controls-074");
    let source = std::fs::read_to_string(tool.join("src/embedded.rs")).unwrap();
    for required in [
        "HydraCache::local()",
        ".get_encoded(",
        ".put_encoded(",
        ".remove(",
        ".flush()",
        "impl Target for EmbeddedControl",
    ] {
        assert!(source.contains(required), "{required}");
    }
    for forbidden in [
        "ClientSurfaceState",
        "TcpListener",
        "global_allocator",
        "get-owner-controls-074",
    ] {
        // Documentation may name the boundary it intentionally excludes.
        assert!(
            !source
                .lines()
                .filter(|line| !line.trim_start().starts_with("//"))
                .any(|line| line.contains(forbidden)),
            "{forbidden}"
        );
    }
}

#[test]
fn secure_memory_packet_retains_all_attempts_without_allocator_or_timing_admission() {
    let directory = root().join("docs/testing/performance/0.74/local-runs/secure-memory-d2252012");
    let summary: Value =
        serde_json::from_slice(&std::fs::read(directory.join("summary.json")).unwrap()).unwrap();
    assert_eq!(summary["completed_attempts"].as_u64(), Some(30));
    assert_eq!(summary["stopped_on_failure"].as_bool(), Some(false));
    for flag in [
        "allocator_retention_admission",
        "product_numeric_claims_allowed",
        "native_nonregression_measured",
        "full_d3_completed",
        "retry_allowed",
    ] {
        assert_eq!(summary[flag].as_bool(), Some(false), "{flag}");
    }
    let attempts = summary["attempts"].as_array().unwrap();
    assert_eq!(attempts.len(), 30);
    let mut seen = std::collections::BTreeSet::new();
    for attempt in attempts {
        let id = attempt["id"].as_str().unwrap();
        assert!(seen.insert(id));
        assert_eq!(attempt["exit_code"].as_i64(), Some(0));
        assert!(attempt["error"].is_null());
        let bytes = std::fs::read(directory.join(format!("{id}.stdout.json"))).unwrap();
        let digest = Sha256::digest(&bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert_eq!(Some(digest.as_str()), attempt["stdout_sha256"].as_str());
        assert!(std::fs::read(directory.join(format!("{id}.stderr.txt")))
            .unwrap()
            .is_empty());
        let receipt: Value = serde_json::from_slice(&bytes).unwrap();
        for flag in [
            "get_owner_feature",
            "product_numeric_claims_allowed",
            "cross_surface_numeric_comparison_allowed",
            "admission_allowed",
        ] {
            assert_eq!(receipt[flag].as_bool(), Some(false), "{flag}");
        }
        assert!(receipt["allocator_active_resident_retained"].is_null());
        assert_eq!(
            receipt["allocator_retention_status"].as_str(),
            Some("unavailable-not-a-pass")
        );
        let payload = receipt["payload_bytes"].as_u64().unwrap();
        let phases = receipt["phases"].as_array().unwrap();
        assert_eq!(phases.len(), 7);
        for (phase, name) in phases.iter().zip([
            "preload", "get", "set", "idle", "delete", "refill", "shutdown",
        ]) {
            assert_eq!(phase["name"].as_str(), Some(name));
            let entries = if matches!(name, "delete" | "shutdown") {
                0
            } else {
                16
            };
            assert_eq!(phase["logical_entries"].as_u64(), Some(entries));
            assert_eq!(
                phase["logical_value_bytes"].as_u64(),
                Some(entries * payload)
            );
            assert!(phase["process_rss_bytes"].as_u64().unwrap() > 0);
        }
    }
}

#[test]
fn scheduled_resp_fifo_and_high_concurrency_controls_do_not_claim_performance() {
    let c = contract("get-response-owner-scheduled-controls-contract.toml");
    for flag in [
        "promotable",
        "product_numeric_claims_allowed",
        "numerical_series_started",
        "accepted_product_change",
        "default_enabled",
        "complete_d3_allowed",
    ] {
        assert_eq!(c[flag].as_bool(), Some(false), "{flag}");
    }
    let resp = &c["resp"];
    for flag in [
        "real_loopback_tcp",
        "production_connection_owner",
        "pipeline_is_outstanding_ceiling_not_fixed_batch",
        "original_calendar_shared",
        "fifo_tombstone_retained_after_waiter_cancel",
        "single_use_run",
    ] {
        assert_eq!(resp[flag].as_bool(), Some(true), "{flag}");
    }
    for flag in [
        "retry_allowed",
        "secure_transport_claim_allowed",
        "numeric_comparison_allowed",
    ] {
        assert_eq!(resp[flag].as_bool(), Some(false), "{flag}");
    }
    for (field, expected) in [
        ("default_physical_connections", 1),
        ("maximum_reply_payload_bytes", 1048576),
        ("maximum_header_bytes", 128),
        ("maximum_history_records", 10256),
        ("fixture_request_frame_ceiling_bytes", 8388608),
        ("maximum_wire_drain_seconds", 5),
    ] {
        assert_eq!(resp[field].as_integer(), Some(expected), "{field}");
    }
    assert_eq!(
        resp["pipeline_limits"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_integer().unwrap())
            .collect::<Vec<_>>(),
        vec![1, 10, 50]
    );
    assert_eq!(
        c["native"]["tested_transport_client_slots"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_integer().unwrap())
            .collect::<Vec<_>>(),
        vec![1, 8, 32, 128]
    );
    for flag in [
        "new_sealed_numerical_cohort_required",
        "resp_per_response_scheduled_adapter_required",
        "hc1_hc2_nonregression_measurements_required",
        "real_transport_concurrency_32_128_required",
        "matched_mtls_resp3_required",
        "allocator_active_resident_retained_idle_refill_required",
        "feature_on_hosted_ci_receipt_required",
    ] {
        assert_eq!(c["pending"][flag].as_bool(), Some(true), "{flag}");
    }
    let tool = root().join("tools/get-owner-scheduled-controls-074");
    let source = std::fs::read_to_string(tool.join("src/resp.rs")).unwrap();
    for required in [
        "scheduled::run_at",
        "drain_pipelines(&self.pipelines).await?",
        "validate_wire(&result)?",
        "waiting_caller_cancelled",
        "slow_reader_and_partial_writes_preserve_cancelled_fifo_tombstone",
        "reads_progress_while_later_pipeline_write_is_backpressured",
    ] {
        assert!(source.contains(required), "{required}");
    }
    assert!(!source.contains("unbounded_channel"));
    assert!(!source.contains("global_allocator"));
    let native = std::fs::read_to_string(tool.join("tests/native.rs")).unwrap();
    assert!(native.contains("high_concurrency_native_clients_start_together_and_release_resources"));
    assert!(native.contains("Barrier::new(slots)"));
}

#[test]
fn scheduled_resp_connections_keep_local_order_and_do_not_admit_product_changes() {
    let c = contract("get-response-owner-scheduled-controls-contract.toml");
    let resp = &c["resp"];
    assert_eq!(
        resp["state"].as_str(),
        Some("local-resp2-get-set-semantic-adapter-only")
    );
    assert_eq!(
        resp["supported_physical_connections"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_integer().unwrap())
            .collect::<Vec<_>>(),
        vec![1, 8, 32, 128]
    );
    assert_eq!(
        resp["operations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["get", "set-same-fixed-value"]
    );
    assert_eq!(
        resp["connection_route"].as_str(),
        Some("original-sequence-modulo-physical-connections")
    );
    assert_eq!(
        resp["fifo_scope"].as_str(),
        Some("per-connection-not-global")
    );
    assert_eq!(
        resp["wire_drain_budget_scope"].as_str(),
        Some("entire-connection-group")
    );
    assert_eq!(resp["shared_store_within_control"].as_bool(), Some(true));
    for flag in [
        "cross_connection_total_order_claim_allowed",
        "secure_transport_claim_allowed",
        "numeric_comparison_allowed",
    ] {
        assert_eq!(resp[flag].as_bool(), Some(false));
    }
    assert_eq!(
        c["pending"]["matched_mtls_resp3_required"].as_bool(),
        Some(true)
    );
    assert_eq!(
        c["pending"]["new_sealed_numerical_cohort_required"].as_bool(),
        Some(true)
    );
    let tool = root().join("tools/get-owner-scheduled-controls-074");
    let source = std::fs::read_to_string(tool.join("src/resp.rs")).unwrap();
    for token in [
        "pub connection_id: usize",
        "previous_ordinals[sample.connection_id]",
        "previous_responses[sample.connection_id]",
        "independent_connections_do_not_share_cancelled_owners_or_drain_budget",
    ] {
        assert!(source.contains(token), "{token}");
    }
    let tests = std::fs::read_to_string(tool.join("tests/resp.rs")).unwrap();
    for token in [
        "multiple_resp_connections_keep_local_fifo_and_fixed_set_oracles",
        "unsupported_resp_connection_counts_fail_before_transport_setup",
        "large_fixed_set_checks_acknowledgement_and_final_bytes",
    ] {
        assert!(tests.contains(token), "{token}");
    }
}

#[test]
fn scheduled_multikey_controls_preserve_command_denominators_and_product_limits() {
    let c = contract("get-response-owner-scheduled-controls-contract.toml");
    let batch = &c["resp"]["multikey"];
    assert_eq!(
        batch["batch_sizes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_integer().unwrap())
            .collect::<Vec<_>>(),
        vec![1, 8, 32, 128]
    );
    for (field, value) in [
        ("maximum_flat_array_elements", 128),
        ("maximum_aggregate_reply_frame_bytes", 1048708),
        ("maximum_aggregate_request_frame_bytes", 1052672),
        ("unsupported_requested_batch_size", 256),
    ] {
        assert_eq!(batch[field].as_integer(), Some(value));
    }
    for flag in [
        "product_batch_limit_changed",
        "divide_latency_by_batch_size_allowed",
        "scheduled_live_delete_claim_allowed",
        "full_multikey_qualification_completed",
    ] {
        assert_eq!(batch[flag].as_bool(), Some(false));
    }
    for flag in [
        "one_sample_per_command",
        "parser_borrows_array_body",
        "independent_native_batch_measurements_required",
    ] {
        assert_eq!(batch[flag].as_bool(), Some(true));
    }
    assert_eq!(c["numerical_series_started"].as_bool(), Some(false));
    assert_eq!(
        c["pending"]["matched_mtls_resp3_required"].as_bool(),
        Some(true)
    );
    let tool = root().join("tools/get-owner-scheduled-controls-074");
    let source = std::fs::read_to_string(tool.join("src/resp.rs")).unwrap();
    for token in [
        "pub response_items: Option<usize>",
        "pub protocol_error_bytes: Option<Vec<u8>>",
        "real_batch_duplicate_order_and_oversized_mset_are_atomic",
        "concurrent_real_mset_mget_never_observes_partial_pair",
        "cancelled_fragmented_array_keeps_one_command_owner",
    ] {
        assert!(source.contains(token), "{token}");
    }
    let tests = std::fs::read_to_string(tool.join("tests/resp.rs")).unwrap();
    assert!(
        tests.contains("scheduled_multikey_commands_keep_one_original_offer_and_exact_reply_shape")
    );
    assert!(tests.contains("unsupported_batch_and_aggregate_payload_fail_before_opening_sockets"));
}

#[test]
fn independent_native_batches_preserve_denominators_and_expose_hc2_semantic_difference() {
    let c = contract("get-response-owner-scheduled-controls-contract.toml");
    let batch = &c["native"]["batch"];
    assert_eq!(
        batch["batch_sizes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_integer().unwrap())
            .collect::<Vec<_>>(),
        vec![1, 8, 32, 128]
    );
    assert_eq!(
        batch["maximum_logical_batch_bytes"].as_integer(),
        Some(1048576)
    );
    assert_eq!(
        batch["surface_dispatches_per_command"].as_integer(),
        Some(1)
    );
    assert_eq!(
        batch["hc2_dispatches_per_command"].as_str(),
        Some("batch-item-count")
    );
    assert_eq!(batch["one_sample_per_command"].as_bool(), Some(true));
    for flag in [
        "divide_latency_by_batch_size_allowed",
        "hc2_batch_is_atomic_surface_batch",
        "cross_surface_mset_equivalence_claim_allowed",
        "direct_control_uses_listener",
        "product_limits_changed",
        "scheduled_live_delete_claim_allowed",
        "native_batch_nonregression_measured",
        "full_multikey_qualification_completed",
    ] {
        assert_eq!(batch[flag].as_bool(), Some(false), "{flag}");
    }
    assert_eq!(c["numerical_series_started"].as_bool(), Some(false));
    let tool = root().join("tools/get-owner-scheduled-controls-074");
    let source = std::fs::read_to_string(tool.join("src/native.rs")).unwrap();
    for token in [
        "operation.validate(&dataset)?",
        "validate_surface_batch",
        "validate_hc2_batch",
        "direct_and_hc1_oversized_batch_put_reject_without_partial_mutation",
        "hc2_batch_limits_and_unapplied_item_are_not_atomic_surface_batch_semantics",
    ] {
        assert!(source.contains(token), "{token}");
    }
    let tests = std::fs::read_to_string(tool.join("tests/native.rs")).unwrap();
    for token in [
        "native_batches_keep_one_offer_and_separate_dispatch_semantics",
        "native_batch_concurrency_boundaries_release_all_owners",
        "unsupported_native_batch_size_and_payload_fail_before_setup",
    ] {
        assert!(tests.contains(token), "{token}");
    }
}

#[test]
fn resp3_controls_require_hello_without_waiving_security_or_numeric_guards() {
    let c = contract("get-response-owner-scheduled-controls-contract.toml");
    let resp3 = &c["resp"]["resp3"];
    for (field, expected) in [
        ("maximum_hello_frame_bytes", 4096),
        ("maximum_hello_version_bytes", 64),
        ("hello_metadata_fields", 7),
        ("maximum_hello_timeout_seconds", 5),
    ] {
        assert_eq!(resp3[field].as_integer(), Some(expected), "{field}");
    }
    for flag in [
        "hello_map_order_independent",
        "per_connection_dialect_validated",
        "mixed_hello_transition_is_separate_fixture",
        "null_and_empty_are_distinct",
    ] {
        assert_eq!(resp3[flag].as_bool(), Some(true), "{flag}");
    }
    for flag in [
        "hello_duplicate_or_unknown_fields_allowed",
        "general_recursive_map_parser",
        "hello_setup_counted_as_measured_offer",
        "resp2_null_in_resp3_allowed",
        "resp3_null_in_resp2_allowed",
        "secure_transport_claim_allowed",
        "numerical_nonregression_measured",
        "full_qualification_completed",
    ] {
        assert_eq!(resp3[flag].as_bool(), Some(false), "{flag}");
    }
    assert_eq!(c["numerical_series_started"].as_bool(), Some(false));
    assert_eq!(
        c["pending"]["matched_mtls_resp3_required"].as_bool(),
        Some(true)
    );
    let tool = root().join("tools/get-owner-scheduled-controls-074");
    let source = std::fs::read_to_string(tool.join("src/resp.rs")).unwrap();
    for token in [
        "negotiate_resp3(&mut client)",
        "sample.dialect != observation.dialect",
        "resp3_hello_metadata_is_shallow_bounded_and_map_order_independent",
        "hello_negotiation_refuses_error_disconnect_and_unsolicited_tail",
        "hello_negotiation_timeout_drops_socket_without_fallback",
        "real_hello_transitions_apply_to_the_next_pipelined_reply",
        "real_resp2_resp3_mget_preserves_null_empty_binary_and_duplicate_positions",
    ] {
        assert!(source.contains(token), "{token}");
    }
    let tests = std::fs::read_to_string(tool.join("tests/resp.rs")).unwrap();
    assert!(tests.contains("resp3_negotiates_every_socket_and_keeps_command_denominators"));
    assert!(tests.contains("large_resp3_get_set_keep_one_complete_frame_observation"));
}

#[test]
fn scheduled_native_controls_preserve_offers_without_admitting_full_d3() {
    let c = contract("get-response-owner-scheduled-controls-contract.toml");
    assert_eq!(
        c["profile_id"].as_str(),
        Some("get-owner-scheduled-controls-074-v1")
    );
    assert_eq!(
        c["state"].as_str(),
        Some("local-instrumentation-and-semantic-controls-only")
    );
    for flag in [
        "promotable",
        "product_numeric_claims_allowed",
        "numerical_series_started",
        "product_mutation_allowed",
        "expensive_workloads_allowed",
        "qualification_allowed",
        "prior_rejections_reopened",
        "accepted_product_change",
        "default_enabled",
        "complete_d3_allowed",
    ] {
        assert_eq!(c[flag].as_bool(), Some(false), "{flag}");
    }
    for flag in [
        "original_offer_timestamps_preserved",
        "overflow_must_be_counted",
        "incomplete_is_censored_not_response",
        "all_offers_accounted",
    ] {
        assert_eq!(c["driver"][flag].as_bool(), Some(true), "{flag}");
    }
    for flag in [
        "legacy_unbounded_driver_used",
        "global_counting_allocator_linked",
        "response_pacing_allowed",
        "retry_allowed",
    ] {
        assert_eq!(c["driver"][flag].as_bool(), Some(false), "{flag}");
    }
    for (key, expected) in [
        ("maximum_operations", 10000),
        ("maximum_queued", 1024),
        ("maximum_schedule_seconds", 15),
        ("maximum_operation_timeout_seconds", 5),
        ("maximum_drain_seconds", 5),
    ] {
        assert_eq!(c["driver"][key].as_integer(), Some(expected), "{key}");
    }
    assert_eq!(c["native"]["seed"].as_integer(), Some(740074));
    assert_eq!(c["native"]["surfaces"].as_array().unwrap().len(), 2);
    for flag in [
        "shared_surface_state_between_controls",
        "product_instrumentation_enabled",
        "system_service_used",
        "cross_security_context_numeric_comparison_allowed",
    ] {
        assert_eq!(c["native"][flag].as_bool(), Some(false), "{flag}");
    }
    for flag in [
        "new_sealed_numerical_cohort_required",
        "resp_per_response_scheduled_adapter_required",
        "hc1_hc2_nonregression_measurements_required",
        "real_transport_concurrency_32_128_required",
        "matched_mtls_resp3_required",
        "miss_error_slow_reader_size_transition_required",
        "allocator_active_resident_retained_idle_refill_required",
        "feature_on_hosted_ci_receipt_required",
    ] {
        assert_eq!(c["pending"][flag].as_bool(), Some(true), "{flag}");
    }
    assert_eq!(
        c["pending"]["invalidated_b0_retry_allowed"].as_bool(),
        Some(false)
    );
    assert_eq!(c["pending"]["integrated_c74"].as_str(), Some("UNRESOLVED"));
    let tool = root().join("tools/get-owner-scheduled-controls-074");
    let manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string(tool.join("Cargo.toml")).unwrap()).unwrap();
    assert!(manifest["features"]["default"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(
        manifest["features"]["get-owner"][0].as_str(),
        Some("hydracache-redis-compat/experimental-resp-get-owner-074")
    );
    assert!(manifest["dependencies"].get("hydracache-loadgen").is_none());
    assert!(!tool.join("src/main.rs").exists());
    let driver = std::fs::read_to_string(tool.join("src/scheduled.rs")).unwrap();
    assert!(!driver.contains("run_open_loop("));
    assert!(!driver.contains("unbounded_channel"));
    assert!(driver.contains("tasks.abort_all()"));
    assert!(driver.contains("incomplete_lower_bound_ns"));
    let library = std::fs::read_to_string(tool.join("src/lib.rs")).unwrap();
    assert!(library.contains("crates/hydracache-loadgen/src/rate.rs"));
    assert!(!library.contains("global_allocator"));
    let registry = contract("proposal-registry.toml");
    let proposal = registry["followup_proposals"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"].as_str() == Some("p74-get-response-owner-v1"))
        .unwrap();
    assert_eq!(
        proposal["scheduled_controls_contract"].as_str(),
        Some("get-response-owner-scheduled-controls-contract.toml")
    );
    assert_eq!(
        proposal["scheduled_controls_numerical_started"].as_bool(),
        Some(false)
    );
    assert_eq!(proposal["accepted_product_change"].as_bool(), Some(false));
}

#[test]
fn resp_security_audit_preserves_mtls_gap_without_lowering_comparison_guard() {
    let c = contract("get-response-owner-scheduled-controls-contract.toml");
    let security = &c["resp"]["security_audit"];
    assert_eq!(security["historical"].as_bool(), Some(true));
    let historical: Value = serde_json::from_str(
        &std::fs::read_to_string(root().join(
            "docs/testing/performance/0.74/local-runs/get-owner-security-audit-b747502d.json",
        ))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(historical["production_resp_mtls_implemented"], false);
    assert_eq!(
        historical["baseline_source_commit"].as_str(),
        security["audit_baseline_source_sha"].as_str()
    );
    assert_eq!(
        security["state"].as_str(),
        Some("production-resp-mtls-gap-requires-policy-decision")
    );
    for flag in [
        "production_resp_server_auth_tls_available",
        "production_resp_auth_available",
        "hc2_client_certificate_required",
        "resp_auth_identity_is_listener_bound",
        "separate_product_security_proposal_required",
    ] {
        assert_eq!(security[flag].as_bool(), Some(true), "{flag}");
    }
    for flag in [
        "production_resp_mtls_available",
        "tls_auth_is_mtls",
        "fixture_tls_wrapper_is_product_receipt",
        "matched_security_cohort_completed",
        "cross_surface_hc2_resp_numeric_comparison_allowed",
        "mtls_requirement_waived",
        "product_security_mutation_authorized_by_observer",
    ] {
        assert_eq!(security[flag].as_bool(), Some(false), "{flag}");
    }
    assert_eq!(
        c["pending"]["matched_mtls_resp3_required"].as_bool(),
        Some(true)
    );
    assert_eq!(c["product_mutation_allowed"].as_bool(), Some(false));
    assert_eq!(c["numerical_series_started"].as_bool(), Some(false));
    let resp_tls =
        std::fs::read_to_string(root().join("crates/hydracache-server/src/redis_tcp.rs")).unwrap();
    assert!(resp_tls.contains(".with_no_client_auth()"));
    assert!(!resp_tls.contains(".ca_path"));
    let hc2 = std::fs::read_to_string(root().join("crates/hydracache-server/src/hc2.rs")).unwrap();
    assert!(hc2.contains(".client_ca_root(Certificate::from_pem(ca))"));
    let resp =
        std::fs::read_to_string(root().join("crates/hydracache-redis-compat/src/lib.rs")).unwrap();
    assert!(resp.contains("connection.identity = self.identity.clone()"));
}

#[test]
fn resp_mtls_extension_is_opt_in_auth_bound_and_non_promotable() {
    let c = contract("get-response-owner-scheduled-controls-contract.toml");
    let mtls = &c["resp"]["mtls"];
    assert_eq!(
        mtls["state"].as_str(),
        Some("local-production-mtls-semantic-proof-only")
    );
    assert_eq!(
        mtls["authorization"].as_str(),
        Some("explicit-human-approval-2026-10-07")
    );
    assert_eq!(
        mtls["configuration"].as_str(),
        Some("redis_api.mtls_client_ca_path")
    );
    assert_eq!(mtls["default_client_ca_path"].as_str(), Some("none"));
    for flag in [
        "enabled_requires_redis_rediss_tls_and_auth",
        "client_certificate_required",
        "auth_and_listener_tenant_binding_preserved",
        "shutdown_joins_owned_connections",
        "production_implemented",
    ] {
        assert_eq!(mtls[flag].as_bool(), Some(true), "{flag}");
    }
    for flag in [
        "optional_client_certificate_mode",
        "global_tls_ca_inherited",
        "certificate_subject_selects_tenant",
        "matched_security_cohort_completed",
        "numerical_comparison_allowed",
        "qualification_completed",
    ] {
        assert_eq!(mtls[flag].as_bool(), Some(false), "{flag}");
    }
    for (field, bound) in [
        ("maximum_client_ca_file_bytes", 262144),
        ("maximum_client_ca_certificates", 16),
        ("maximum_handshake_seconds", 5),
        ("maximum_owned_connections", 128),
    ] {
        assert_eq!(mtls[field].as_integer(), Some(bound), "{field}");
    }
    assert_eq!(
        c["pending"]["production_resp_mtls_policy_decision_required"].as_bool(),
        Some(false)
    );
    assert_eq!(
        c["pending"]["matched_mtls_resp3_required"].as_bool(),
        Some(true)
    );
    assert_eq!(c["accepted_product_change"].as_bool(), Some(false));
    let source =
        std::fs::read_to_string(root().join("crates/hydracache-server/src/redis_tcp.rs")).unwrap();
    assert!(source.contains(".with_client_cert_verifier(verifier)"));
    assert!(source.contains("WebPkiClientVerifier::builder"));
    assert!(!source.contains("allow_unauthenticated"));
    assert!(!source.contains(".ca_path"));
    assert!(source.contains("MAX_CLIENT_CA_BYTES: u64 = 256 * 1024"));
    assert!(source.contains("MAX_CLIENT_CA_CERTIFICATES: usize = 16"));
    assert!(source.contains("MAX_MTLS_CONNECTIONS: usize = 128"));
    assert!(source.contains("finish_mtls_owners(&mut connections).await"));
    assert!(source.contains("connections.abort_all()"));
    assert!(source.contains("while connections.join_next().await.is_some()"));
}

#[test]
fn get_owner_phase_b0_separates_unprofiled_timing_without_waiving_full_d3() {
    let c = contract("get-response-owner-phase-b0-contract.toml");
    assert_eq!(
        c["profile_id"].as_str(),
        Some("get-owner-local-controls-074-v1")
    );
    for flag in [
        "promotable",
        "product_numeric_claims_allowed",
        "product_mutation_allowed",
        "expensive_workloads_allowed",
        "qualification_allowed",
        "prior_rejections_reopened",
    ] {
        assert_eq!(c[flag].as_bool(), Some(false), "{flag}");
    }
    let s = &c["screen"];
    for (key, value) in [
        ("seed", 740074),
        ("runtime_workers", 2),
        ("key_space", 16),
        ("total_fresh_process_attempts", 400),
        ("independent_aa_pairs_per_cell_per_lane", 5),
        ("independent_ab_pairs_per_cell_per_lane", 5),
        ("maximum_attempt_seconds", 60),
        ("maximum_failed_attempts", 0),
    ] {
        assert_eq!(s[key].as_integer(), Some(value), "{key}");
    }
    assert_eq!(c["cell"].as_array().unwrap().len(), 10);
    for (key, value) in [
        ("minimum_goodput_ratio", 0.98),
        ("maximum_cpu_ratio", 1.03),
        ("maximum_p99_ratio", 1.03),
        ("maximum_gross_allocation_ratio", 1.05),
    ] {
        assert_eq!(c["guards"][key].as_float(), Some(value), "{key}");
    }
    for flag in [
        "complete_d3_allowed",
        "accepted_product_change",
        "default_enabled",
    ] {
        assert_eq!(c["pending"][flag].as_bool(), Some(false), "{flag}");
    }
    for flag in [
        "hc1_hc2_controls_required",
        "matched_mtls_resp3_required",
        "concurrency_32_128_required",
        "scheduled_per_operation_latency_required",
        "miss_error_slow_reader_size_transition_required",
        "allocator_active_resident_retained_idle_refill_required",
        "feature_on_hosted_ci_receipt_required",
    ] {
        assert_eq!(c["pending"][flag].as_bool(), Some(true), "{flag}");
    }
    let tool = root().join("tools/get-owner-controls-074");
    let manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string(tool.join("Cargo.toml")).unwrap()).unwrap();
    assert!(manifest["features"]["default"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(
        manifest["features"]["get-owner"][0].as_str(),
        Some("hydracache-redis-compat/experimental-resp-get-owner-074")
    );
    assert!(manifest["dependencies"].get("hydracache-loadgen").is_none());
    let source = std::fs::read_to_string(tool.join("src/main.rs")).unwrap();
    assert!(source.contains("#[cfg(feature = \"allocation-profile\")]\n#[global_allocator]"));
    assert!(!source.contains("ProfiledTcpStream"));
    assert!(source.contains("\"closed_loop_pipeline_batch\""));
    assert!(source.contains("create_new(true)"));
    assert!(source.contains("placement_gate().await?"));
    let registry = contract("proposal-registry.toml");
    let proposal = registry["followup_proposals"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"].as_str() == Some("p74-get-response-owner-v1"))
        .unwrap();
    assert_eq!(
        proposal["phase_b0_contract"].as_str(),
        Some("get-response-owner-phase-b0-contract.toml")
    );
    assert_eq!(
        proposal["phase_b0_state"].as_str(),
        Some("invalidated-background-cpu-before-first-process")
    );
    assert_eq!(
        proposal["phase_b0_numerical_started"].as_bool(),
        Some(false)
    );
    assert_eq!(proposal["phase_b0_attempts_retained"].as_integer(), Some(1));
    let packet = root().join("docs/testing/performance/0.74/local-runs/get-owner-b0-18ee8cdc");
    for (file, expected) in [
        (
            "seal.json",
            "bc9c2ae4ea38801905ee1c96b81df9f8f59ae579a29838c22dcba03f62bcf572",
        ),
        (
            "summary.json",
            "789bd36b7f62c0f220eee304d4d0fe8c1a09d9cefb5c8956b9555be62d716cea",
        ),
        (
            "attempt-0001/attempt.json",
            "a28d1f46c06569bc936a39bd8b31d90814dd31cf9dd15cea45db79a0b6412e70",
        ),
    ] {
        let bytes = std::fs::read(packet.join(file)).unwrap();
        let hash: String = Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert_eq!(hash, expected, "{file}");
    }
    assert_eq!(proposal["accepted_product_change"].as_bool(), Some(false));
}

fn contract(name: &str) -> toml::Value {
    let path = root().join("docs/testing/performance/0.74").join(name);
    toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn response_reduction_d1_is_separate_bounded_and_cannot_authorize_a_candidate() {
    let c = contract("response-reduction-attribution-contract.toml");
    assert_eq!(
        c["profile_id"].as_str(),
        Some("response-reduction-owner-d1-074-v1")
    );
    for flag in [
        "product_mutation_allowed",
        "promotable",
        "product_numeric_claims_allowed",
        "prior_rejections_reopened",
        "expensive_workloads_allowed",
    ] {
        assert_eq!(c[flag].as_bool(), Some(false), "{flag}");
    }
    let s = &c["screen"];
    for (key, expected) in [
        ("seed", 740074),
        ("fresh_process_repeats", 3),
        ("maximum_attempts", 18),
        ("maximum_attempt_seconds", 30),
        ("warmup_operations_per_dispatch_control", 100),
        ("concurrency", 1),
        ("cache_time_ms", 1000000),
    ] {
        assert_eq!(s[key].as_integer(), Some(expected), "{key}");
    }
    assert_eq!(
        s["cpu_latency_goodput_native_nonregression_claims_allowed"].as_bool(),
        Some(false)
    );
    assert_eq!(
        s["decoder_translation_encoder_socket_scheduling_included"].as_bool(),
        Some(false)
    );
    let cells = c["cell"].as_array().unwrap();
    assert_eq!(cells.len(), 6);
    let expected = [
        ("get-empty", "get", true, 0, 10000),
        ("get-64", "get", true, 64, 10000),
        ("get-4096", "get", true, 4096, 10000),
        ("get-1048576", "get", true, 1048576, 500),
        ("get-miss", "get", false, 4096, 10000),
        ("set-4096", "set", true, 4096, 10000),
    ];
    for (cell, (id, op, hit, bytes, operations)) in cells.iter().zip(expected) {
        assert_eq!(cell["id"].as_str(), Some(id));
        assert_eq!(cell["operation"].as_str(), Some(op));
        assert_eq!(cell["hit"].as_bool(), Some(hit));
        assert_eq!(cell["payload_bytes"].as_integer(), Some(bytes));
        assert_eq!(cell["iterations"].as_integer(), Some(operations));
    }
    assert_eq!(
        c["next_boundary"]["candidate_authorized_by_this_contract"].as_bool(),
        Some(false)
    );
    assert_eq!(
        c["next_boundary"]["four_native_surfaces_and_peak_idle_memory_guards_not_waived"].as_bool(),
        Some(true)
    );
    let manifest: toml::Value = toml::from_str(
        &std::fs::read_to_string(root().join("tools/resp-response-owner-074/Cargo.toml")).unwrap(),
    )
    .unwrap();
    assert!(
        manifest.get("features").is_none(),
        "D1 tool must not forward a candidate feature"
    );
    assert_eq!(manifest["package"]["publish"].as_bool(), Some(false));
}

#[test]
fn response_reduction_d1_retains_exact_packet_without_reopening_rejected_scratch() {
    let registry = contract("proposal-registry.toml");
    let attribution = &registry["followup_attributions"].as_array().unwrap()[0];
    let source = "2562f6f7e2ff598741d4fe9a4f38ae635786e8d9";
    assert_eq!(attribution["measurement_source_sha"].as_str(), Some(source));
    for flag in [
        "product_mutation_allowed",
        "accepted_product_change",
        "promotable",
    ] {
        assert_eq!(attribution[flag].as_bool(), Some(false));
    }
    let directory = root().join("docs/testing/performance/0.74/local-runs/response-owner-2562f6f7");
    assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 38);
    let read =
        |path: &Path| -> Value { serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap() };
    let seal = read(&directory.join("seal.json"));
    let summary = read(&directory.join("summary.json"));
    assert_eq!(seal["source_commit"], source);
    assert_eq!(summary["source_commit"], source);
    assert_eq!(summary["attempts"], 18);
    assert_eq!(summary["candidate_authorized"], false);
    assert_eq!(summary["promotable"], false);
    for (index, entry) in seal["order"].as_array().unwrap().iter().enumerate() {
        let cell = entry["cell_id"].as_str().unwrap();
        let prefix = format!("{index:02}-{cell}");
        let attempt = read(&directory.join(format!("{prefix}.attempt.json")));
        let path = directory.join(format!("{prefix}.raw.json"));
        let raw = read(&path);
        assert_eq!(attempt["returncode"], 0);
        assert_eq!(attempt["failure"], Value::Null);
        assert_eq!(attempt["index"], index);
        assert_eq!(raw["source_commit"], source);
        for key in ["binary_sha256", "tool_lock_sha256", "contract_sha256"] {
            assert_eq!(raw[key], seal[key]);
        }
        let hash = Sha256::digest(std::fs::read(&path).unwrap())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert_eq!(attempt["raw_sha256"], format!("sha256:{hash}"));
    }
    let scratch = &registry["followup_proposals"].as_array().unwrap()[0];
    assert_eq!(
        scratch["state"].as_str(),
        Some("rejected-d3a-live-peak-regression-runtime-removed")
    );
    assert_eq!(scratch["accepted_product_change"].as_bool(), Some(false));
    assert_eq!(scratch["runtime_removed"].as_bool(), Some(true));
}

fn trace() -> Value {
    json!({
        "trace_sha256": "trace-a",
        "payload_corpus_sha256": "payload-a",
        "key_corpus_sha256": "keys-a",
        "seed": 740074,
        "warmup_seconds": 1,
        "measurement_seconds": 3,
        "operation_count": 1000,
        "offered_schedule_sha256": "schedule-a",
        "concurrency": 8,
        "pipeline_depth": 10,
        "security": "plaintext",
        "persistence": "off",
        "semantic_contract": "get-hit-v1"
    })
}

fn role(id: &str) -> Value {
    json!({
        "source_sha": id,
        "binary_sha256": format!("binary-{id}"),
        "binary_sha256s": [format!("binary-{id}")],
        "redis_tool_sha256": "redis-benchmark-7.2.5-a",
        "trace": trace(),
        "outcomes": {
            "completed": 1000,
            "errors": 0,
            "timeouts": 0,
            "rejections": 0,
            "late": 0,
            "incomplete": 0,
            "final_cardinality": 128,
            "final_state_sha256": "state-a"
        },
        "metrics": {
            "goodput_operations_per_second": 1000.0,
            "latency_p99_microseconds": 100.0,
            "cpu_seconds_per_operation": 0.00001
        }
    })
}

fn receipt() -> Value {
    let surfaces = [
        "resp-api",
        "native-api-hc1",
        "native-api-hc2",
        "client-surface-state",
        "embedded-hydracache-raw",
        "embedded-hydracache-typed",
    ]
    .into_iter()
    .map(|surface| {
        json!({
            "surface": surface,
            "trace_sha256": "trace-a",
            "baseline_goodput": 1000.0,
            "candidate_goodput": 1000.0
        })
    })
    .collect::<Vec<_>>();
    json!({
        "release": "0.74",
        "candidate_derived_thresholds": false,
        "final_sample_present": true,
        "declared_block_order": "baseline_candidate",
        "block_order": ["baseline", "candidate"],
        "integrated_native_guard": true,
        "baseline": role("baseline-source"),
        "candidate": role("candidate-source"),
        "surface_results": surfaces
    })
}

fn has(problems: &[String], needle: &str) -> bool {
    problems.iter().any(|problem| problem.contains(needle))
}

#[test]
fn checked_in_w0_contract_is_valid_and_non_promotable() {
    assert!(xtask::performance_contract_074::check_identities(&contract(
        "baseline-identities.toml"
    ))
    .is_empty());
    assert!(
        xtask::performance_contract_074::check_matrix(&contract("scenario-matrix.toml")).is_empty()
    );
    assert!(
        xtask::performance_contract_074::check_statistics(&contract("statistics.toml")).is_empty()
    );
    assert!(
        xtask::performance_contract_074::check_registry(&contract("proposal-registry.toml"))
            .is_empty()
    );
    assert!(xtask::performance_contract_074::check_host(&contract("host-profile.toml")).is_empty());
    assert!(
        xtask::performance_contract_074::check_local_harness(&contract("local-harness.toml"))
            .is_empty()
    );
    assert!(
        xtask::performance_contract_074::check_composition(&contract("composition-ledger.toml"))
            .is_empty()
    );
    assert!(
        xtask::performance_contract_074::check_release_admission(&contract(
            "release-admission-contract.toml"
        ))
        .is_empty()
    );
    assert!(
        xtask::performance_contract_074::check_terminal_dispositions(
            &root(),
            &contract("terminal-disposition-ledger.toml")
        )
        .is_empty()
    );
    assert!(
        xtask::performance_contract_074::check_at_root(Path::new(&root()), None)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn w11_schema_hashes_and_local_completion_flags_are_exact() {
    let controller = contract("long-run-controller-resilience-contract.toml");
    let implementation = controller["local_implementation"].as_table().unwrap();
    for (path_field, digest_field) in [
        ("packet_manifest_schema", "packet_manifest_schema_sha256"),
        (
            "seal_input_inventory_schema",
            "seal_input_inventory_schema_sha256",
        ),
        (
            "checkpoint_envelope_schema",
            "checkpoint_envelope_schema_sha256",
        ),
        ("raw_manifest_schema", "raw_manifest_schema_sha256"),
        ("start_manifest_schema", "start_manifest_schema_sha256"),
        ("host_observation_schema", "host_observation_schema_sha256"),
        ("start_bundle_schema", "start_bundle_schema_sha256"),
        (
            "role_overhead_attempt_schema",
            "role_overhead_attempt_schema_sha256",
        ),
    ] {
        let path = root().join(implementation[path_field].as_str().unwrap());
        let digest = Sha256::digest(std::fs::read(path).unwrap())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert_eq!(implementation[digest_field].as_str(), Some(digest.as_str()));
    }
    for field in [
        "raw_file_set_verification_complete",
        "campaign_manifest_binding_complete",
        "continuation_packet_digest_binding_complete",
        "strict_seal_input_resolver_complete",
        "live_seal_dispatch_complete",
        "guard_evidence_binding_complete",
        "deterministic_archive_creation_complete",
        "phase_progress_watchdog_complete",
        "supervisor_progress_loss_maintenance_complete",
        "supervisor_measurement_loss_maintenance_complete",
        "full_design_manifest_fields_complete",
        "host_observation_socket_export_complete",
        "installed_source_receipt_binding_complete",
        "host_receipt_revalidation_complete",
        "live_start_host_receipt_revalidation_complete",
        "start_evidence_import_complete",
        "live_attach_lease_admission_complete",
        "live_attach_checkpoint_refresh_complete",
        "start_bundle_builder_complete",
        "start_bundle_transport_verifier_complete",
        "supervisor_owned_start_upload_complete",
        "privileged_start_bundle_staging_complete",
        "connection_transport_failure_survival_complete",
        "startup_checkpoint_absence_maintenance_complete",
        "campaign_lifecycle_controller_loss_rehearsal_complete",
        "progress_loss_host_rehearsal_complete",
        "measurement_loss_host_rehearsal_complete",
        "lease_expiry_host_rehearsal_complete",
        "real_systemd_spawn_backend_host_rehearsal_complete",
        "systemd_confinement_complete",
        "privileged_start_bundle_host_rehearsal_complete",
        "protected_start_attach_abort_rehearsal_complete",
        "signed_socket_start_host_rehearsal_complete",
        "live_attach_host_rehearsal_complete",
        "live_abort_host_rehearsal_complete",
        "supervisor_restart_rehearsal_complete",
        "live_role_reboot_rehearsal_complete",
        "live_seal_complete",
        "live_mutating_operations_complete",
        "role_overhead_analyzer_complete",
        "role_overhead_collector_complete",
    ] {
        assert_eq!(implementation[field].as_bool(), Some(true), "{field}");
    }
    assert_eq!(
        implementation["live_service_complete"].as_bool(),
        Some(false)
    );
    assert_eq!(
        implementation["release_admission_allowed"].as_bool(),
        Some(false)
    );
}

#[test]
fn w11_live_role_reboot_is_fail_closed_and_non_promotable() {
    let controller = contract("long-run-controller-resilience-contract.toml");
    let implementation = controller["local_implementation"].as_table().unwrap();
    let relative = implementation["live_role_reboot_rehearsal"]
        .as_str()
        .unwrap();
    let evidence: Value =
        serde_json::from_slice(&std::fs::read(root().join(relative)).unwrap()).unwrap();

    assert_eq!(
        evidence["source_commit"].as_str(),
        Some("b83fd1b792cd610de9cb4817d178b5c8c9635914")
    );
    assert_eq!(
        evidence["provisioning"]["run_id"].as_u64(),
        Some(37453559280)
    );
    assert_eq!(
        evidence["start_bundle"]["run_id"].as_u64(),
        Some(37453956080)
    );
    assert_eq!(evidence["start"]["run_id"].as_u64(), Some(37454199519));
    assert_ne!(
        evidence["reboot"]["old_boot_id"].as_str(),
        evidence["reboot"]["new_boot_id"].as_str()
    );
    assert_eq!(
        evidence["reboot"]["measurement_loss_reason"].as_str(),
        Some("host-identity-drift")
    );
    assert_eq!(
        evidence["final_state"]["campaign_state"].as_str(),
        Some("FAILED_INCOMPLETE")
    );
    for field in [
        "recorded_failure",
        "harness_identity_cleared",
        "daemon_identity_cleared",
        "checkpoint_identity_cleared",
        "controller_lease_cleared",
        "active_campaign_released",
        "role_unit_absent",
        "role_process_absent",
        "supervisor_active",
        "runner_inactive",
        "runner_disabled",
    ] {
        assert_eq!(
            evidence["final_state"][field].as_bool(),
            Some(true),
            "{field}"
        );
    }
    assert_eq!(
        evidence["final_state"]["replacement_role_started"].as_bool(),
        Some(false)
    );
    assert_eq!(
        evidence["forbidden_work"]["product_candidate_started"].as_bool(),
        Some(false)
    );
    assert_eq!(
        implementation["live_role_reboot_rehearsal_complete"].as_bool(),
        Some(true)
    );
    assert_eq!(
        implementation["live_service_complete"].as_bool(),
        Some(false)
    );
}

#[test]
fn w11_lease_expiry_host_rehearsal_is_exact_and_non_promotable() {
    let controller = contract("long-run-controller-resilience-contract.toml");
    let implementation = controller["local_implementation"].as_table().unwrap();
    let relative = implementation["campaign_lease_expiry_rehearsal"]
        .as_str()
        .unwrap();
    let evidence: Value =
        serde_json::from_slice(&std::fs::read(root().join(relative)).unwrap()).unwrap();

    assert_eq!(
        evidence["source_commit"].as_str(),
        Some("187b63fb20555743f8b220ac60273e0e8e8663cc")
    );
    assert_eq!(evidence["github_run"]["id"].as_u64(), Some(37380414415));
    assert_eq!(
        evidence["observation"]["terminal_state"].as_str(),
        Some("LEASE_EXPIRED_INCOMPLETE")
    );
    assert_eq!(
        evidence["observation"]["product_candidate_started"].as_bool(),
        Some(false)
    );
    assert_eq!(
        evidence["observation"]["terminal_execution_fields_cleared"].as_bool(),
        Some(true)
    );
    assert_eq!(
        evidence["observation"]["fixture_unit_stopped"].as_bool(),
        Some(true)
    );
    assert_eq!(
        evidence["observation"]["active_campaign_released"].as_bool(),
        Some(true)
    );
    assert_eq!(
        evidence["decision"]["scheduled_drift_window_independently_sampled"].as_bool(),
        Some(false)
    );
    assert_eq!(
        evidence["decision"]["qualification_started"].as_bool(),
        Some(false)
    );
    assert_eq!(
        evidence["decision"]["release_admission_allowed"].as_bool(),
        Some(false)
    );
}

#[test]
fn w11_non_product_role_rehearsal_is_complete_but_role_budget_stays_closed() {
    let controller = contract("long-run-controller-resilience-contract.toml");
    let implementation = controller["local_implementation"].as_table().unwrap();
    let relative = implementation["supervisor_idle_overhead_evidence"]
        .as_str()
        .unwrap();
    let evidence: Value =
        serde_json::from_slice(&std::fs::read(root().join(relative)).unwrap()).unwrap();

    assert_eq!(
        evidence["tooling_source_commit"].as_str(),
        Some("1bc4823b9f0585cd7ad4f767d0fcf093a7f7576b")
    );
    assert_eq!(evidence["github_run"]["id"].as_u64(), Some(37388038717));
    assert_eq!(
        evidence["observation"]["product_candidate_started"].as_bool(),
        Some(false)
    );
    assert_eq!(
        evidence["observation"]["cpu_budget_passed"].as_bool(),
        Some(true)
    );
    assert_eq!(
        evidence["observation"]["rss_budget_passed"].as_bool(),
        Some(true)
    );
    assert_eq!(
        evidence["observation"]["io_budget_evaluated"].as_bool(),
        Some(true)
    );
    assert_eq!(
        evidence["observation"]["io_budget_passed"].as_bool(),
        Some(true)
    );
    assert_eq!(
        evidence["observation"]["idle_screen_passed"].as_bool(),
        Some(true)
    );
    assert_eq!(
        evidence["decision"]["idle_overhead_budget_complete"].as_bool(),
        Some(false)
    );
    assert_eq!(
        evidence["decision"]["release_admission_allowed"].as_bool(),
        Some(false)
    );
    assert_eq!(
        implementation["supervisor_idle_cpu_rss_screen_complete"].as_bool(),
        Some(true)
    );
    for field in [
        "supervisor_idle_io_screen_complete",
        "supervisor_idle_overhead_screen_complete",
        "io_accounting_instrumentation_staged",
        "io_accounting_instrumentation_host_rehearsal_complete",
    ] {
        assert_eq!(implementation[field].as_bool(), Some(true), "{field}");
    }
    for field in [
        "idle_overhead_budget_complete",
        "role_overhead_qualification_complete",
    ] {
        assert_eq!(implementation[field].as_bool(), Some(false), "{field}");
    }
    assert_eq!(
        implementation["non_product_role_overhead_rehearsal_complete"].as_bool(),
        Some(true)
    );
    let role_evidence: Value = serde_json::from_slice(
        &std::fs::read(
            root().join(
                implementation["non_product_role_overhead_rehearsal"]
                    .as_str()
                    .unwrap(),
            ),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        role_evidence["evidence_class"].as_str(),
        Some("non-product-role-overhead-rehearsal")
    );
    assert_eq!(role_evidence["promotable"].as_bool(), Some(false));
    assert_eq!(
        role_evidence["source_commit"].as_str(),
        Some("543108f1ccd206ae670803c2617f07e7fa91ae62")
    );
    assert_eq!(
        role_evidence["github_run"]["id"].as_u64(),
        Some(37467960862)
    );
    assert_eq!(
        role_evidence["workload_identity"]["attempt_count"].as_u64(),
        Some(20)
    );
    assert_eq!(
        role_evidence["budgets"]["non_product_rehearsal_passed"].as_bool(),
        Some(true)
    );
    assert_eq!(
        role_evidence["decision"]["role_overhead_qualification_complete"].as_bool(),
        Some(false)
    );
    assert_eq!(
        role_evidence["decision"]["release_admission_allowed"].as_bool(),
        Some(false)
    );
    assert_eq!(
        implementation["role_overhead_analyzer_complete"].as_bool(),
        Some(true)
    );
    let analyzer = std::fs::read_to_string(
        root().join(implementation["role_overhead_analyzer"].as_str().unwrap()),
    )
    .unwrap();
    assert!(analyzer.contains("non-product-role-overhead-rehearsal"));
    assert!(analyzer.contains("\"role_overhead_qualification_complete\": False"));
    assert!(analyzer.contains("\"release_admission_allowed\": False"));
    let collector = std::fs::read_to_string(
        root().join(implementation["role_overhead_collector"].as_str().unwrap()),
    )
    .unwrap();
    assert!(collector.contains("non-product-role-overhead-rehearsal"));
    assert!(collector.contains("PAIRS = 5"));
    assert!(collector.contains("CHECKPOINT_BYTES = 4_096"));
    assert!(collector.contains("active_campaign_absent"));
    assert!(collector.contains("shell=False"));
    assert!(collector.contains("RLIMIT_AS"));
    assert!(!collector.contains("/bin/sh"));
    let fixture = std::fs::read_to_string(
        root().join(implementation["role_overhead_fixture"].as_str().unwrap()),
    )
    .unwrap();
    assert!(fixture.contains("const PAIRS: u32 = 5"));
    assert!(fixture.contains("const CHECKPOINT_BYTES: usize = 4_096"));
    assert!(fixture.contains("role-overhead fixture invocation is invalid"));

    let provisioning: Value = serde_json::from_slice(
        &std::fs::read(
            root().join(
                implementation["host_provisioning_receipt"]
                    .as_str()
                    .unwrap(),
            ),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(provisioning["github_run"]["id"].as_u64(), Some(37387670191));
    assert_eq!(
        provisioning["host"]["service_io_accounting"].as_bool(),
        Some(true)
    );
    assert_eq!(
        provisioning["decision"]["release_admission_allowed"].as_bool(),
        Some(false)
    );
}

#[test]
fn w10_cannot_freeze_a_candidate_or_hide_an_accepted_proposal() {
    let mut value = contract("composition-ledger.toml");
    value["freeze_c74_allowed"] = toml::Value::Boolean(true);
    value["accepted_candidate_count"] = toml::Value::Integer(1);
    value["proposal"]
        .as_array_mut()
        .unwrap()
        .first_mut()
        .unwrap()["disposition"] = toml::Value::String("accepted".to_owned());
    let problems = xtask::performance_contract_074::check_composition(&value);
    assert!(has(&problems, "freeze_c74_allowed"));
    assert!(has(&problems, "accepted_candidate_count"));
    assert!(has(&problems, "cannot contain an accepted proposal"));
}

#[test]
fn w3_delivery_frontier_review_preserves_semantics_without_authorizing_a_candidate() {
    let review = contract("w3-delivery-frontier-review.toml");
    assert_eq!(review["schema_version"].as_integer(), Some(1));
    assert_eq!(review["release"].as_str(), Some("0.74"));
    assert_eq!(
        review["state"].as_str(),
        Some("local-feasibility-review-no-candidate")
    );
    assert_eq!(review["candidate_source_sha"].as_str(), Some("UNRESOLVED"));
    for flag in [
        "product_mutation_allowed",
        "numerical_measurement_started",
        "numerical_claims_allowed",
        "promotable",
        "prior_rejections_reopened",
    ] {
        assert_eq!(review[flag].as_bool(), Some(false), "{flag}");
    }
    let boundary = &review["delivery_boundary"];
    for flag in [
        "previous_response_write_all_complete_required",
        "previous_response_flush_complete_required",
    ] {
        assert_eq!(boundary[flag].as_bool(), Some(true), "{flag}");
    }
    for flag in [
        "first_byte_authorizes_next_mutation",
        "socket_writable_hint_authorizes_next_mutation",
        "complete_write_without_flush_authorizes_next_mutation",
        "execute_then_batch_before_write_allowed",
        "transaction_staging_or_early_ack_allowed",
    ] {
        assert_eq!(boundary[flag].as_bool(), Some(false), "{flag}");
    }
    let scope = &review["guard_scope"];
    assert_eq!(scope["first_reply_bytes"].as_integer(), Some(5));
    assert_eq!(
        scope["accepted_prefixes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_integer().unwrap())
            .collect::<Vec<_>>(),
        (0..=5).collect::<Vec<_>>()
    );
    assert_eq!(scope["mutations_before_release"].as_integer(), Some(1));
    assert_eq!(
        scope["mutations_after_successful_release"].as_integer(),
        Some(2)
    );
    for flag in [
        "complete_reply_pending_flush",
        "flush_error_stops_next_command",
        "other_connection_reads_exact_committed_state",
        "correctness_uses_explicit_future_poll",
    ] {
        assert_eq!(scope[flag].as_bool(), Some(true), "{flag}");
    }
    assert_eq!(
        scope["sleep_based_correctness_allowed"].as_bool(),
        Some(false)
    );
    let previous = contract("w3-adaptive-coalescing-contract.toml");
    for section in ["deep_pipeline_acceptance", "pipeline_one_non_regression"] {
        assert_eq!(review.get(section), previous.get(section), "{section}");
    }
    assert!(root().join(review["design"].as_str().unwrap()).is_file());
    let source =
        std::fs::read_to_string(root().join(review["semantic_tests"].as_str().unwrap())).unwrap();
    for function in [
        "every_partial_reply_and_pending_flush_preserve_the_mutation_frontier",
        "failed_flush_after_complete_reply_does_not_execute_the_next_set",
    ] {
        assert!(source.contains(&format!("async fn {function}()")));
    }
}

#[test]
fn w3_staged_execution_assessment_does_not_authorize_product_or_threshold_changes() {
    let review = contract("w3-staged-execution-review.toml");
    assert_eq!(review["schema_version"].as_integer(), Some(1));
    assert_eq!(review["release"].as_str(), Some("0.74"));
    assert_eq!(
        review["state"].as_str(),
        Some("architecture-review-complete-not-admitted")
    );
    assert_eq!(review["candidate_source_sha"].as_str(), Some("UNRESOLVED"));
    assert_eq!(
        review["architecture_assessment_allowed"].as_bool(),
        Some(true)
    );
    for flag in [
        "product_implementation_allowed",
        "numerical_measurement_started",
        "numerical_claims_allowed",
        "promotable",
        "prior_rejections_reopened",
        "changes_external_semantics",
    ] {
        assert_eq!(review[flag].as_bool(), Some(false), "{flag}");
    }
    for flag in [
        "first_mutation_visible_before_its_response_completion",
        "previous_response_complete_write_and_flush_before_next_execution",
        "commit_before_exposing_success_response",
        "native_operations_progress_while_resp_writer_pending",
        "no_store_lock_across_io",
        "no_early_tenant_quota_reservation",
        "request_time_and_conditional_result_revalidated_at_execution",
        "no_early_audit_event_or_mutation_publication",
        "no_speculative_shared_store_mutation_and_rollback",
        "no_native_cost_transfer",
        "no_acknowledgement_or_durability_change",
        "no_distributed_transactions",
    ] {
        assert_eq!(review["invariants"][flag].as_bool(), Some(true), "{flag}");
    }
    let verification = &review["verification"];
    assert_eq!(
        verification["accepted_first_reply_prefix_bytes"],
        contract("w3-delivery-frontier-review.toml")["guard_scope"]["accepted_prefixes"]
    );
    for flag in [
        "native_invalidate_changes_queued_nx_result",
        "native_put_changes_queued_get_result",
        "expiry_changes_queued_nx_result",
        "new_ttl_starts_at_queued_command_execution",
        "native_put_consumes_quota_before_queued_set",
        "quota_failure_emits_one_audit_and_no_set_mutation",
        "canonical_separate_write_positive_control",
    ] {
        assert_eq!(verification[flag].as_bool(), Some(true), "{flag}");
    }
    assert_eq!(verification["model_permutations"].as_integer(), Some(6));
    assert_eq!(
        verification["chronological_fixed_batch_schedules"].as_integer(),
        Some(3)
    );
    assert_eq!(
        verification["admitted_fixed_batch_schedules"].as_integer(),
        Some(0)
    );
    let previous = contract("w3-adaptive-coalescing-contract.toml");
    for section in ["deep_pipeline_acceptance", "pipeline_one_non_regression"] {
        assert_eq!(review.get(section), previous.get(section), "{section}");
    }
    assert!(root().join(review["design"].as_str().unwrap()).is_file());
    let source =
        std::fs::read_to_string(root().join(review["semantic_tests"].as_str().unwrap())).unwrap();
    for function in [
        "queued_set_nx_revalidates_after_intervening_native_invalidate",
        "queued_get_observes_intervening_native_put_without_blocking_native",
        "queued_set_nx_uses_expiry_and_ttl_at_its_execution_boundary",
        "queued_set_does_not_reserve_quota_ahead_of_intervening_native_put",
    ] {
        assert!(source.contains(&format!("async fn {function}()")));
    }
    let model =
        std::fs::read_to_string(root().join(review["order_model"].as_str().unwrap())).unwrap();
    assert!(model.contains(
        "fn fixed_two_reply_batch_cannot_preserve_both_commit_frontier_and_response_order()"
    ));
}

#[test]
fn w9b_large_response_attribution_is_bounded_and_does_not_reopen_product_candidates() {
    let review = contract("w9b-response-buffer-attribution-contract.toml");
    assert_eq!(
        review["state"].as_str(),
        Some("preregistered-local-d1-only")
    );
    assert_eq!(review["candidate_source_sha"].as_str(), Some("UNRESOLVED"));
    for flag in [
        "product_mutation_allowed",
        "promotable",
        "product_numeric_claims_allowed",
        "prior_w3_or_w4_rejections_reopened",
    ] {
        assert_eq!(review[flag].as_bool(), Some(false), "{flag}");
    }
    let screen = &review["screen"];
    assert_eq!(screen["seed"].as_integer(), Some(740074));
    assert_eq!(
        screen["warmup_operations_per_canonical_control"].as_integer(),
        Some(100)
    );
    assert_eq!(screen["fresh_process_repeats"].as_integer(), Some(3));
    assert_eq!(screen["maximum_attempts"].as_integer(), Some(15));
    for flag in [
        "independent_identically_initialized_canonical_controls",
        "exact_results_dispatch_mutations_and_final_cardinality_required",
    ] {
        assert_eq!(screen[flag].as_bool(), Some(true), "{flag}");
    }
    for flag in [
        "socket_io_included",
        "counting_allocator_timing_is_product_cpu_evidence",
        "expected_payload_allocation_inside_validator_allowed",
    ] {
        assert_eq!(screen[flag].as_bool(), Some(false), "{flag}");
    }
    let cells: Vec<_> = review["cell"]
        .as_array()
        .unwrap()
        .iter()
        .map(|cell| {
            (
                cell["operation"].as_str().unwrap(),
                cell["payload_bytes"].as_integer().unwrap(),
                cell["iterations"].as_integer().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        cells,
        vec![
            ("get", 256, 10000),
            ("set", 256, 10000),
            ("get", 4096, 10000),
            ("set", 4096, 10000),
            ("get", 1048576, 500)
        ]
    );
    let boundary = &review["future_candidate_boundary"];
    for flag in [
        "previous_complete_write_and_flush_before_next_command",
        "pipeline_one_retains_canonical_path",
        "release_scratch_before_waiting_for_another_read",
        "no_cross_connection_pool",
        "no_larger_input_output_or_idle_retention_bound",
        "native_surfaces_remain_separate_required_guards",
    ] {
        assert_eq!(boundary[flag].as_bool(), Some(true), "{flag}");
    }
    assert_eq!(
        boundary["minimum_affected_end_to_end_gross_allocation_reduction"].as_float(),
        Some(0.20)
    );
    assert_eq!(
        boundary["unaffected_minimum_goodput_ratio"].as_float(),
        Some(0.98)
    );
    assert_eq!(
        boundary["unaffected_maximum_cpu_or_p99_ratio"].as_float(),
        Some(1.03)
    );
    assert_eq!(
        boundary["minimum_counterbalanced_independent_pairs_for_d3"].as_integer(),
        Some(5)
    );
    let source = std::fs::read_to_string(root().join(review["source"].as_str().unwrap())).unwrap();
    assert!(source.contains(review["profile_id"].as_str().unwrap()));
    for function in [
        "canonical_controls_reconcile_dispatch_mutation_and_cardinality",
        "canonical_result_validator_adds_no_gross_allocation",
        "canonical_validator_rejects_changed_values_errors_and_frame_bytes",
    ] {
        assert!(source.contains(&format!("fn {function}()")));
    }
}

#[test]
fn w9b_owner_receipts_preserve_all_registered_cells_without_product_claims() {
    let report: Value = serde_json::from_str(
        &std::fs::read_to_string(root().join(
            "docs/testing/performance/0.74/local-runs/w9b-serial-encoder-owner-cc0b2fbf.json",
        ))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        report["source_commit"],
        "cc0b2fbfc0b7d9c4c9f37e96089f37134fdf546f"
    );
    assert_eq!(report["candidate_source_commit"], "UNRESOLVED");
    for flag in ["promotable", "product_changed", "product_performance_claim"] {
        assert_eq!(report[flag], false, "{flag}");
    }
    let review = contract("w9b-response-buffer-attribution-contract.toml");
    let receipts = report["raw_receipts"].as_array().unwrap();
    assert_eq!(receipts.len(), 15);
    let paths: std::collections::BTreeSet<_> = receipts
        .iter()
        .map(|entry| entry["path"].as_str().unwrap())
        .collect();
    assert_eq!(paths.len(), 15);
    let summaries = report["cells"].as_array().unwrap();
    assert_eq!(summaries.len(), 5);
    for cell in review["cell"].as_array().unwrap() {
        let operation = cell["operation"].as_str().unwrap();
        let payload_bytes = cell["payload_bytes"].as_integer().unwrap() as u64;
        let iterations = cell["iterations"].as_integer().unwrap() as u64;
        let summary = summaries
            .iter()
            .find(|entry| {
                entry["operation"] == operation && entry["payload_bytes"] == payload_bytes
            })
            .unwrap();
        assert_eq!(summary["iterations"], iterations);
        let matches: Vec<_> = receipts
            .iter()
            .filter(|entry| {
                entry["operation"] == operation && entry["payload_bytes"] == payload_bytes
            })
            .collect();
        assert_eq!(matches.len(), 3);
        let mut previous: Option<Value> = None;
        for repeat in 1..=3 {
            let entry = matches
                .iter()
                .find(|entry| entry["repeat"] == repeat)
                .unwrap();
            let text = std::fs::read_to_string(root().join(entry["path"].as_str().unwrap()))
                .unwrap()
                .replace("\r\n", "\n");
            let original = text.strip_suffix('\n').unwrap_or(&text);
            let digest = Sha256::digest(original.as_bytes())
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>();
            assert_eq!(entry["original_tool_output_sha256"], digest);
            let receipt: Value = serde_json::from_str(original).unwrap();
            for field in ["source_commit", "profile_id", "binary_sha256"] {
                assert_eq!(receipt[field], report[field], "{field}");
            }
            assert_eq!(receipt["operation"], operation);
            assert_eq!(receipt["payload_bytes"], payload_bytes);
            assert_eq!(receipt["iterations"], iterations);
            assert_eq!(receipt["seed"], 740074);
            assert_eq!(receipt["canonical_warmup_operations"], 100);
            assert_eq!(receipt["promotable"], false);
            assert_eq!(receipt["exact_result_validation"], true);
            for (summary_field, receipt_field) in [
                (
                    "canonical_encode_incremental_bytes_per_operation",
                    "canonical_encode_incremental_allocated_bytes_per_operation",
                ),
                (
                    "isolated_encode_incremental_bytes_per_operation",
                    "encode_incremental_allocated_bytes_per_operation",
                ),
                ("workload_sha256", "workload_sha256"),
                ("request_sha256", "request_sha256"),
                ("key_corpus_sha256", "key_corpus_sha256"),
                ("payload_corpus_sha256", "payload_corpus_sha256"),
            ] {
                if receipt[receipt_field].is_number() {
                    assert_eq!(
                        summary[summary_field].as_f64().unwrap(),
                        receipt[receipt_field].as_f64().unwrap(),
                        "{summary_field}"
                    );
                } else {
                    assert_eq!(
                        summary[summary_field], receipt[receipt_field],
                        "{summary_field}"
                    );
                }
            }
            let control = receipt["canonical_execution"]["gross_allocated_bytes_per_operation"]
                .as_f64()
                .unwrap();
            let encoded = receipt["canonical_execution_and_encode"]
                ["gross_allocated_bytes_per_operation"]
                .as_f64()
                .unwrap();
            assert_eq!(
                summary["canonical_execution_gross_bytes_per_operation"]
                    .as_f64()
                    .unwrap(),
                control
            );
            assert_eq!(
                summary["canonical_execution_and_encode_gross_bytes_per_operation"]
                    .as_f64()
                    .unwrap(),
                encoded
            );
            let increment = receipt["canonical_encode_incremental_allocated_bytes_per_operation"]
                .as_f64()
                .unwrap();
            assert!((increment - (encoded - control)).abs() < 1e-9);
            assert!(
                (summary["canonical_encode_share"].as_f64().unwrap() - increment / encoded).abs()
                    < 1e-12
            );
            for field in [
                "canonical_control_dispatches",
                "canonical_encoded_dispatches",
            ] {
                assert_eq!(receipt[field], 2 * iterations + 102);
            }
            for stage in ["canonical_execution", "canonical_execution_and_encode"] {
                assert_eq!(receipt[stage]["checksum"], iterations);
                if let Some(previous) = &previous {
                    assert_eq!(
                        receipt[stage]["gross_allocated_bytes"],
                        previous[stage]["gross_allocated_bytes"]
                    );
                }
            }
            if let Some(previous) = &previous {
                for field in [
                    "workload_sha256",
                    "request_sha256",
                    "key_corpus_sha256",
                    "payload_corpus_sha256",
                ] {
                    assert_eq!(receipt[field], previous[field], "{field}");
                }
            }
            previous = Some(receipt);
        }
    }
    assert_eq!(report["assessment"]["accepted_product_proposals"], 0);
    assert_eq!(
        report["assessment"]["end_to_end_twenty_percent_reduction_proven"],
        false
    );
}

#[test]
fn w9b_serial_scratch_proposal_preserves_independent_semantic_and_numerical_gates() {
    let proposal = contract("w9b-serial-scratch-proposal.toml");
    assert_eq!(
        proposal["feature"].as_str(),
        Some("experimental-resp-serial-scratch-074")
    );
    for flag in [
        "default_enabled",
        "accepted_product_change",
        "promotable",
        "qualification_allowed",
        "expensive_workloads_allowed",
        "existing_terminal_dispositions_reopened",
    ] {
        assert_eq!(proposal[flag].as_bool(), Some(false), "{flag}");
    }
    let activation = &proposal["activation"];
    assert_eq!(activation["minimum_payload_bytes"].as_integer(), Some(4096));
    assert_eq!(
        activation["maximum_payload_bytes"].as_integer(),
        Some(1048576)
    );
    assert_eq!(
        activation["maximum_serial_scratch_capacity_bytes"].as_integer(),
        Some(1048588)
    );
    for flag in [
        "first_response_uses_canonical_writer",
        "pipeline_one_uses_canonical_writer",
    ] {
        assert_eq!(activation[flag].as_bool(), Some(true), "{flag}");
    }
    for flag in [
        "decode_lookahead_or_early_execution",
        "read_buffer_compaction_changed",
        "public_codec_functions_changed",
    ] {
        assert_eq!(activation[flag].as_bool(), Some(false), "{flag}");
    }
    let lifetime = &proposal["lifetime"];
    assert_eq!(lifetime["idle_retained_bytes"].as_integer(), Some(0));
    for flag in [
        "release_before_non_get_execution",
        "release_on_response_shape_or_size_change",
        "release_before_protocol_error_reply",
        "release_before_next_read",
        "release_on_eof_error_cancel_quit",
        "previous_complete_write_and_flush_before_next_command",
    ] {
        assert_eq!(lifetime[flag].as_bool(), Some(true), "{flag}");
    }
    for flag in [
        "cross_connection_pool",
        "cross_principal_reuse",
        "new_lock_or_atomic_on_native_path",
    ] {
        assert_eq!(lifetime[flag].as_bool(), Some(false), "{flag}");
    }
    let d3 = &proposal["d3_boundary"];
    assert_eq!(
        d3["minimum_independent_counterbalanced_pairs"].as_integer(),
        Some(5)
    );
    assert_eq!(
        d3["minimum_affected_end_to_end_gross_allocation_reduction"].as_float(),
        Some(0.20)
    );
    assert_eq!(
        d3["unaffected_minimum_goodput_ratio"].as_float(),
        Some(0.98)
    );
    assert_eq!(
        d3["unaffected_maximum_cpu_or_p99_ratio"].as_float(),
        Some(1.03)
    );
    assert_eq!(
        d3["maximum_native_gross_allocation_ratio"].as_float(),
        Some(1.05)
    );
}

// Only the reviewed experiment's exact two source hunks may be removed to
// reconstruct the same full canonical runtime fingerprint. Unknown edits fail.
fn without_get_owner_overlay(source: &str) -> String {
    let module = "#[cfg(any(test, feature = \"experimental-resp-get-owner-074\"))]\nmod get_response_owner_074;\n\n";
    let selector = concat!(
        "        #[cfg(feature = \"experimental-resp-get-owner-074\")]\n",
        "        let reduced = get_response_owner_074::reduce(&plan, responses);\n",
        "        #[cfg(not(feature = \"experimental-resp-get-owner-074\"))]\n",
        "        let reduced = plan.reduce(&responses);\n",
        "        match reduced {\n",
    );
    if source.contains("mod get_response_owner_074;") {
        assert_eq!(source.matches(module).count(), 1);
        assert_eq!(source.matches(selector).count(), 1);
        source
            .replace(module, "")
            .replace(selector, "        match plan.reduce(&responses) {\n")
    } else {
        source.to_owned()
    }
}

#[test]
fn get_response_owner_d2_preserves_one_hypothesis_and_future_guards() {
    let p = contract("get-response-owner-proposal.toml");
    assert_eq!(p["proposal_id"].as_str(), Some("p74-get-response-owner-v1"));
    assert_eq!(
        p["owner_source_sha"].as_str(),
        Some("2562f6f7e2ff598741d4fe9a4f38ae635786e8d9")
    );
    assert_eq!(p["product_mutation_allowed"].as_bool(), Some(true));
    for flag in [
        "default_enabled",
        "accepted_product_change",
        "promotable",
        "product_numeric_claims_allowed",
        "existing_terminal_dispositions_reopened",
        "numerical_comparison_allowed_by_this_contract",
        "expensive_workloads_allowed",
        "qualification_allowed",
    ] {
        assert_eq!(p[flag].as_bool(), Some(false), "{flag}");
    }
    for flag in [
        "single_initial_get_request",
        "no_followup_plan",
        "single_actual_response",
        "successful_nonempty_value_only",
        "no_spare_response_capacity",
        "command_decode_translation_and_dispatch_unchanged",
        "public_borrowed_reducer_unchanged",
        "null_empty_error_wrong_shape_and_non_get_use_canonical_reducer",
        "no_new_protocol_version_gate_or_response_shape",
        "no_early_execution_or_decode_lookahead",
    ] {
        assert_eq!(p["activation"][flag].as_bool(), Some(true), "{flag}");
    }
    let d3 = &p["future_d3"];
    assert_eq!(
        d3["minimum_affected_end_to_end_gross_allocation_reduction"].as_float(),
        Some(0.20)
    );
    assert_eq!(
        d3["minimum_counterbalanced_independent_pairs_per_cell"].as_integer(),
        Some(5)
    );
    assert_eq!(
        d3["maximum_peak_live_above_start_ratio"].as_float(),
        Some(1.0)
    );
    assert_eq!(
        d3["maximum_next_read_or_post_close_live_increase_bytes"].as_integer(),
        Some(0)
    );
    assert_eq!(d3["minimum_native_goodput_ratio"].as_float(), Some(0.98));
    assert_eq!(d3["maximum_native_cpu_or_p99_ratio"].as_float(), Some(1.03));
    assert_eq!(
        d3["maximum_native_gross_allocation_ratio"].as_float(),
        Some(1.05)
    );
    assert_eq!(
        d3["native_surfaces"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["embedded", "client-surface", "hc1", "hc2"]
    );
    assert_eq!(
        p["semantic_gate"]["seeded_property_seed"].as_integer(),
        Some(740074)
    );
    assert_eq!(
        p["semantic_gate"]["seeded_property_cases"].as_integer(),
        Some(128)
    );
    let registry = contract("proposal-registry.toml");
    let proposals = registry["followup_proposals"].as_array().unwrap();
    assert_eq!(proposals.len(), 2);
    let current = proposals
        .iter()
        .find(|entry| entry["id"] == p["proposal_id"])
        .unwrap();
    for flag in [
        "default_enabled",
        "accepted_product_change",
        "promotable",
        "existing_terminal_dispositions_reopened",
    ] {
        assert_eq!(current[flag].as_bool(), Some(false));
    }
}

#[test]
fn get_response_owner_runtime_is_off_by_default_and_feature_semantics_are_enrolled_in_ci() {
    let p = contract("get-response-owner-proposal.toml");
    let feature = p["feature"].as_str().unwrap();
    let manifest: toml::Value = toml::from_str(
        &std::fs::read_to_string(root().join("crates/hydracache-redis-compat/Cargo.toml")).unwrap(),
    )
    .unwrap();
    assert!(manifest["features"]["default"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(manifest["features"][feature].as_array().unwrap().is_empty());
    let runtime = std::fs::read_to_string(root().join("crates/hydracache-redis-compat/src/lib.rs"))
        .unwrap()
        .replace("\r\n", "\n");
    let canonical = without_get_owner_overlay(&runtime);
    assert_ne!(
        canonical, runtime,
        "the exact two enrolled hunks must exist"
    );
    assert!(root()
        .join("crates/hydracache-redis-compat/src/get_response_owner_074.rs")
        .is_file());
    let workflow: serde_yaml::Value = serde_yaml::from_str(
        &std::fs::read_to_string(root().join(".github/workflows/ci.yml")).unwrap(),
    )
    .unwrap();
    let steps = workflow["jobs"]["rust"]["steps"].as_sequence().unwrap();
    let step = steps
        .iter()
        .find(|step| step["name"].as_str() == Some("Release 0.74 GET ownership feature semantics"))
        .unwrap();
    assert_eq!(
        step["if"].as_str(),
        Some("env.HYDRACACHE_CANDIDATE_RELEASE == '0.74'")
    );
    assert_eq!(step["run"].as_str().unwrap().trim(), concat!(
        "cargo test -p hydracache-redis-compat --features experimental-resp-get-owner-074 --locked\n",
        "cargo test -p hydracache-server --test server_lifecycle redis --features hydracache-redis-compat/experimental-resp-get-owner-074 --locked"));
}

#[test]
fn get_response_owner_d3a_is_finite_isolated_and_cannot_admit_a_candidate() {
    let p = contract("get-response-owner-d3-contract.toml");
    assert_eq!(p["proposal_id"].as_str(), Some("p74-get-response-owner-v1"));
    for flag in [
        "accepted_product_change",
        "promotable",
        "product_numeric_claims_allowed",
        "expensive_workloads_allowed",
        "qualification_allowed",
    ] {
        assert_eq!(p[flag].as_bool(), Some(false));
    }
    let a = &p["phase_a"];
    for (key, value) in [
        ("seed", 740074),
        ("concurrency", 1),
        ("independent_aa_pairs_per_cell", 5),
        ("independent_ab_pairs_per_cell", 5),
        ("total_fresh_process_attempts", 180),
        ("maximum_next_read_or_post_close_live_increase_bytes", 0),
    ] {
        assert_eq!(a[key].as_integer(), Some(value));
    }
    for (key, value) in [
        ("minimum_affected_gross_allocation_reduction", 0.20),
        ("maximum_unaffected_gross_allocation_ratio", 1.05),
        ("maximum_peak_live_above_start_ratio", 1.00),
        ("maximum_aa_allocation_or_live_relative_noise", 0.01),
    ] {
        assert_eq!(a[key].as_float(), Some(value));
    }
    assert_eq!(p["cell"].as_array().unwrap().len(), 9);
    assert_eq!(
        p["phase_b"]["phase_a_pass_cannot_accept_or_activate_candidate"].as_bool(),
        Some(true)
    );
    let manifest: toml::Value = toml::from_str(
        &std::fs::read_to_string(root().join("tools/resp-get-owner-screen-074/Cargo.toml"))
            .unwrap(),
    )
    .unwrap();
    assert!(manifest["features"]["default"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(
        manifest["features"]["get-owner"].as_array().unwrap()[0].as_str(),
        Some("hydracache-redis-compat/experimental-resp-get-owner-074")
    );
    let registry = contract("proposal-registry.toml");
    let entry = registry["followup_proposals"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["id"] == p["proposal_id"])
        .unwrap();
    assert!(entry["numerical_candidate_comparison_started"]
        .as_bool()
        .is_some());
    if entry["numerical_candidate_comparison_started"].as_bool() == Some(true) {
        assert!(root()
            .join("docs/testing/performance/0.74")
            .join(entry["screen_evidence"].as_str().unwrap())
            .is_file());
    }
}

#[test]
fn get_response_owner_d3a_retains_all_raw_hashes_without_acceptance() {
    let packet = root().join("docs/testing/performance/0.74/local-runs/get-owner-d3a-213e9e0a");
    assert_eq!(std::fs::read_dir(&packet).unwrap().count(), 362);
    let seal: Value =
        serde_json::from_slice(&std::fs::read(packet.join("seal.json")).unwrap()).unwrap();
    let summary: Value =
        serde_json::from_slice(&std::fs::read(packet.join("summary.json")).unwrap()).unwrap();
    assert_eq!(
        seal["source_commit"],
        "213e9e0a3c9c50e089e5aed2583913f8c6d10807"
    );
    assert_eq!(summary["source_commit"], seal["source_commit"]);
    assert_eq!(summary["attempts_retained"], 180);
    assert_eq!(
        summary["classification"],
        "screen-passed-full-d3-controls-still-required"
    );
    for flag in [
        "promotable",
        "accepted_product_change",
        "product_performance_claim",
    ] {
        assert_eq!(summary[flag], false);
    }
    let schedule = seal["schedule"].as_array().unwrap();
    assert_eq!(schedule.len(), 180);
    for (index, row) in schedule.iter().enumerate() {
        let ordinal = index + 1;
        let mode = row[0].as_str().unwrap();
        let pair = row[1].as_u64().unwrap();
        let cell = row[2].as_str().unwrap();
        let role = row[3].as_str().unwrap();
        let raw =
            std::fs::read(packet.join(format!("{ordinal:03}-{mode}-{pair}-{cell}-{role}.json")))
                .unwrap();
        let attempt: Value = serde_json::from_slice(
            &std::fs::read(packet.join(format!("{ordinal:03}.attempt.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(attempt["ordinal"], ordinal);
        assert_eq!(attempt["exit_code"], 0);
        assert_eq!(
            attempt["raw_receipt_sha256"],
            format!(
                "sha256:{}",
                Sha256::digest(&raw)
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            )
        );
        let receipt: Value = serde_json::from_slice(&raw).unwrap();
        let enabled = mode == "ab" && role == "on";
        assert_eq!(receipt["source_commit"], seal["source_commit"]);
        assert_eq!(receipt["get_owner_enabled"], enabled);
        assert_eq!(
            receipt["binary_sha256"],
            seal[if enabled {
                "on_binary_sha256"
            } else {
                "off_binary_sha256"
            }]
        );
    }
}

#[test]
fn w9b_rejected_serial_scratch_is_removed_and_negative_receipts_retained() {
    let proposal = contract("w9b-serial-scratch-proposal.toml");
    let registry = contract("proposal-registry.toml");
    let followups = registry["followup_proposals"].as_array().unwrap();
    assert_eq!(followups.len(), 2);
    let followup = followups
        .iter()
        .find(|entry| entry["id"] == proposal["proposal_id"])
        .unwrap();
    assert_eq!(followup["id"], proposal["proposal_id"]);
    assert_eq!(
        followup["implementation_sha"],
        proposal["candidate_source_sha"]
    );
    for flag in ["default_enabled", "accepted_product_change", "promotable"] {
        assert_eq!(followup[flag].as_bool(), Some(false), "{flag}");
    }
    assert_eq!(
        followup["numerical_candidate_comparison_started"].as_bool(),
        Some(true)
    );
    assert_eq!(followup["runtime_removed"].as_bool(), Some(true));
    assert_eq!(proposal["runtime_removed"].as_bool(), Some(true));
    assert_eq!(followup["state"], proposal["state"]);
    assert_eq!(
        proposal["state"].as_str(),
        Some("rejected-d3a-live-peak-regression-runtime-removed")
    );
    let manifest: toml::Value = toml::from_str(
        &std::fs::read_to_string(root().join("crates/hydracache-redis-compat/Cargo.toml")).unwrap(),
    )
    .unwrap();
    let feature = proposal["feature"].as_str().unwrap();
    assert!(manifest
        .get("features")
        .and_then(|features| features.get(feature))
        .is_none());
    let source = std::fs::read_to_string(root().join("crates/hydracache-redis-compat/src/lib.rs"))
        .unwrap()
        .replace("\r\n", "\n");
    assert!(!source.contains("serial_get_scratch_074"));
    assert!(!source.contains("write_serial_scratch_response"));
    assert!(!root()
        .join("crates/hydracache-redis-compat/src/serial_get_scratch_074.rs")
        .exists());
    // Exact LF-normalized canonical runtime from 4d733e31 after removing only
    // the new preregistered experiment's two exact cfg hunks. No old Git object
    // is required in shallow CI. Scratch stays removed; unknown edits fail.
    let canonical = without_get_owner_overlay(&source);
    assert_eq!(
        Sha256::digest(canonical.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
        "c98ab563d4fc9181735d432484118561ad9bf773d32fb2865f4a591dcc50e932"
    );
    let directory = root().join("docs/testing/performance/0.74/local-runs/w9b-d3a-3171d02a");
    let report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(directory.join("summary.json")).unwrap())
            .unwrap();
    assert_eq!(
        report["classification"],
        "rejected-local-allocation-memory-screen"
    );
    assert_eq!(report["attempts_retained"], 140);
    assert_eq!(report["accepted_product_change"], false);
    for result in report["results"].as_array().unwrap() {
        assert_eq!(result["gross_guard_passed"], true);
        assert_eq!(result["owner_guard_passed"], true);
        assert_eq!(
            result["peak_guard_passed"].as_bool(),
            Some(!result["affected"].as_bool().unwrap())
        );
    }
    let count = std::fs::read_dir(directory).unwrap().count();
    assert_eq!(
        count, 282,
        "all raw receipts, attempts, seal and summary must remain"
    );
}

#[test]
fn w9b_d3_screen_seals_memory_rejection_without_native_or_timing_waivers() {
    let policy = contract("w9b-serial-scratch-d3-contract.toml");
    for flag in [
        "promotable",
        "accepted_product_change",
        "product_numeric_claims_allowed",
        "expensive_workloads_allowed",
        "qualification_allowed",
    ] {
        assert_eq!(policy[flag].as_bool(), Some(false), "{flag}");
    }
    let screen = &policy["phase_a"];
    assert_eq!(
        screen["independent_aa_pairs_per_cell"].as_integer(),
        Some(5)
    );
    assert_eq!(
        screen["independent_ab_pairs_per_cell"].as_integer(),
        Some(5)
    );
    assert_eq!(
        screen["minimum_affected_gross_allocation_reduction"].as_float(),
        Some(0.20)
    );
    assert_eq!(
        screen["maximum_peak_live_above_start_ratio"].as_float(),
        Some(1.0)
    );
    assert_eq!(
        screen["maximum_next_read_or_post_close_live_increase_bytes"].as_integer(),
        Some(0)
    );
    assert_eq!(
        screen["counting_allocator_can_certify_cpu_latency_goodput"].as_bool(),
        Some(false)
    );
    assert_eq!(
        screen["oracle_corpus_precomputed_outside_counting"].as_bool(),
        Some(true)
    );
    assert_eq!(policy["cell"].as_array().unwrap().len(), 7);
    let broader = &policy["phase_b"];
    assert_eq!(
        broader["native_minimum_goodput_ratio"].as_float(),
        Some(0.98)
    );
    assert_eq!(
        broader["native_maximum_cpu_or_p99_ratio"].as_float(),
        Some(1.03)
    );
    assert_eq!(
        broader["native_maximum_gross_allocation_ratio"].as_float(),
        Some(1.05)
    );
    assert_eq!(
        broader["required_separate_native_controls"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item.as_str().unwrap())
            .collect::<Vec<_>>(),
        ["embedded", "ClientSurfaceState", "HC1", "HC2"]
    );
    assert_eq!(
        broader["phase_a_pass_cannot_accept_or_activate_candidate"].as_bool(),
        Some(true)
    );
    assert_eq!(
        broader["feature_on_hosted_ci_evidence_required_before_promotion"].as_bool(),
        Some(true)
    );
}

#[test]
fn w12_evidence_skeleton_is_exact_and_fail_closed() {
    let manifest: toml::Value = toml::from_str(
        &std::fs::read_to_string(root().join("docs/testing/release-evidence/0.74.toml")).unwrap(),
    )
    .unwrap();
    let expected = [
        "W0", "W1", "W2", "W3", "W4", "W4a", "W4b", "W4c", "W4d", "W4e", "W5", "W6", "W6a", "W6b",
        "W7", "W8", "W8a", "W8b", "W8c", "W9", "W9a", "W9b", "W9c", "W9d", "W9e", "W10", "W11",
        "W12",
    ];
    let actual = manifest["work_item"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["id"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
    assert_eq!(
        manifest["dynamic_canary_work_items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item.as_str().unwrap())
            .collect::<Vec<_>>(),
        ["W11", "W12"]
    );
    assert!(
        xtask::canary_check::check_canary_registry_for_release(&root(), "0.74")
            .unwrap()
            .is_empty()
    );
    let report = xtask::release_evidence::build_report(&root(), "0.74", None).unwrap();
    for item in report
        .work_items
        .iter()
        .filter(|item| !matches!(item.id.as_str(), "W11" | "W12"))
    {
        assert!(
            item.reasons
                .iter()
                .all(|reason| !reason.contains("dynamic canary")),
            "explicit W11/W12 canary selection must not block {}: {:?}",
            item.id,
            item.reasons
        );
    }
}

#[test]
fn w12_release_admission_contract_is_fail_closed() {
    let mut value = contract("release-admission-contract.toml");
    assert!(xtask::performance_contract_074::check_release_admission(&value).is_empty());

    value["candidate_source_sha"] = toml::Value::String("1".repeat(40));
    value["release_admission_allowed"] = toml::Value::Boolean(true);
    value["claims"]["numerical_product_performance_allowed"] = toml::Value::Boolean(true);
    value["authorization"]["expensive_runs_enabled"] = toml::Value::Boolean(true);
    value["completion"]["immutable_archive_complete"] = toml::Value::Boolean(true);
    value["unresolved"] = toml::Value::Array(Vec::new());

    let problems = xtask::performance_contract_074::check_release_admission(&value);
    for field in [
        "candidate_source_sha",
        "release_admission_allowed",
        "numerical_product_performance_allowed",
        "expensive_runs_enabled",
        "immutable_archive_complete",
        "unresolved blockers",
    ] {
        assert!(
            has(&problems, field),
            "missing problem for {field}: {problems:?}"
        );
    }
}

#[test]
fn release_admission_expected_red_canary() {
    let mut value = contract("release-admission-contract.toml");
    let defect_enabled = std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("PERF74-W12");
    if defect_enabled {
        value["authorization"]["expensive_runs_enabled"] = toml::Value::Boolean(true);
    }
    let problems = xtask::performance_contract_074::check_release_admission(&value);
    assert!(
        problems.is_empty(),
        "{}: injected unsafe release admission flag was rejected: {problems:?}",
        if defect_enabled {
            "HC-CANARY-RED:PERF74-W12"
        } else {
            "checked-in release admission contract is invalid"
        }
    );
}

#[test]
fn w12_release_note_retains_draft_claim_rollback_and_gate_boundaries() {
    let note = std::fs::read_to_string(root().join("docs/releases/0.74.0.md")).unwrap();
    for marker in [
        "pre-release draft; no product candidate is accepted or frozen",
        "no numerical product-performance",
        "no exact C74 D4 artifact",
        "Activation and rollback",
        "Remaining admission work",
        "performance-contract-check --require-ship",
        "release-evidence --require-ship",
    ] {
        assert!(note.contains(marker), "release note must retain {marker:?}");
    }
}

#[test]
fn w12_generic_evidence_report_retains_closed_ship_admission() {
    for release in ["0.74", "0.74.0"] {
        let report = xtask::release_evidence::build_report(&root(), release, None).unwrap();
        assert!(
            has(&report.reasons, "0.74 ship admission is closed"),
            "ordinary green test receipts must not substitute for qualification: {:?}",
            report.reasons
        );
        assert!(report
            .work_items
            .iter()
            .all(|item| { item.stage <= xtask::release_evidence::EvidenceStage::FastGreen }));
    }
}

#[test]
fn w2_through_w9_terminal_dispositions_are_exact_and_non_promotable() {
    let mut value = contract("terminal-disposition-ledger.toml");
    assert!(
        xtask::performance_contract_074::check_terminal_dispositions(&root(), &value).is_empty()
    );

    let receipt_path = root().join(
        "docs/testing/performance/0.74/local-runs/w2-w9-terminal-dispositions-local-20261006.json",
    );
    let receipt: Value = serde_json::from_slice(&std::fs::read(receipt_path).unwrap()).unwrap();
    let ledger_bytes = std::fs::read(
        root().join("docs/testing/performance/0.74/terminal-disposition-ledger.toml"),
    )
    .unwrap();
    let digest = Sha256::digest(ledger_bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(
        receipt["terminal_disposition_ledger_sha256"].as_str(),
        Some(digest.as_str())
    );
    assert_eq!(
        receipt["accepted_product_candidate_count"].as_u64(),
        Some(0)
    );
    assert_eq!(receipt["promotable"].as_bool(), Some(false));
    assert_eq!(receipt["release_admission_allowed"].as_bool(), Some(false));

    value["disposition"]
        .as_array_mut()
        .unwrap()
        .first_mut()
        .unwrap()["accepted_product_change"] = toml::Value::Boolean(true);
    let problems = xtask::performance_contract_074::check_terminal_dispositions(&root(), &value);
    assert!(has(&problems, "cannot accept a product change"));
}

#[test]
fn w12_workspace_fast_gate_covers_every_work_item_without_budget_drift() {
    let registry = xtask::fast_suite::load_registry(&root()).unwrap();
    assert_eq!(registry.aggregate_budget_seconds, 1_680);
    assert_eq!(
        registry
            .suite
            .iter()
            .map(|suite| suite.budget_seconds)
            .sum::<u64>(),
        1_680
    );
    let gate = registry
        .suite
        .iter()
        .find(|suite| suite.id == "fast.workspace-nextest")
        .unwrap();
    assert_eq!(gate.timeout_seconds, 2_400);
    assert_eq!(gate.budget_seconds, 840);
    assert_eq!(gate.command.program, "cargo");
    assert_eq!(
        gate.command.args,
        [
            "nextest",
            "run",
            "--workspace",
            "--profile",
            "ci",
            "--locked"
        ]
    );

    let manifest: toml::Value = toml::from_str(
        &std::fs::read_to_string(root().join("docs/testing/release-evidence/0.74.toml")).unwrap(),
    )
    .unwrap();
    let gate_items = gate
        .work_items
        .iter()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    for item in manifest["work_item"].as_array().unwrap() {
        let id = item["id"].as_str().unwrap();
        assert!(gate_items.contains(id), "workspace gate must cover {id}");
        assert_eq!(
            item["fast_gate_ids"]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| value.as_str().unwrap())
                .collect::<Vec<_>>(),
            ["fast.workspace-nextest"],
            "release evidence must bind {id} to the unchanged workspace gate"
        );
    }

    let receipt: Value = serde_json::from_slice(
        &std::fs::read(root().join(
            "docs/testing/performance/0.74/local-runs/w12-fast-gate-registration-local-20261006.json",
        ))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(receipt["gate_id"].as_str(), Some("fast.workspace-nextest"));
    assert_eq!(receipt["work_items_bound"].as_u64(), Some(28));
    assert_eq!(receipt["command_changed"].as_bool(), Some(false));
    assert_eq!(receipt["budget_changed"].as_bool(), Some(false));
    assert_eq!(receipt["gate_executed"].as_bool(), Some(false));
    assert_eq!(
        receipt["exact_commit_receipt_present"].as_bool(),
        Some(false)
    );
    assert_eq!(receipt["promotable"].as_bool(), Some(false));

    let timeout: Value = serde_json::from_slice(
        &std::fs::read(root().join(
            "docs/testing/performance/0.74/local-runs/w12-workspace-fast-gate-cold-timeout-05e7c6e9.json",
        ))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        timeout["source_commit"].as_str(),
        Some("05e7c6e957ca7c1705f22977164f1576e358f88c")
    );
    assert_eq!(timeout["outcome"].as_str(), Some("timeout"));
    assert_eq!(timeout["execution_phase"].as_str(), Some("compile"));
    assert_eq!(timeout["duration_ms"].as_u64(), Some(2_400_607));
    assert_eq!(timeout["timeout_seconds"].as_u64(), Some(2_400));
    assert_eq!(timeout["budget_seconds"].as_u64(), Some(840));
    assert_eq!(timeout["tests_started"].as_bool(), Some(false));
    assert_eq!(timeout["junit_present"].as_bool(), Some(false));
    assert_eq!(timeout["test_failure_observed"].as_bool(), Some(false));
    assert_eq!(timeout["command_changed"].as_bool(), Some(false));
    assert_eq!(timeout["budget_changed"].as_bool(), Some(false));
    assert_eq!(timeout["timeout_changed"].as_bool(), Some(false));
    assert_eq!(timeout["promotable"].as_bool(), Some(false));
    assert_eq!(timeout["release_admission_allowed"].as_bool(), Some(false));
    assert_eq!(
        timeout["command_digest"].as_str(),
        Some("c5dcee3e2ec9668589f44d012e3b0a902fe342bbb44402f14cfda5ded4283c3c")
    );
    assert_eq!(
        timeout["registry_digest"].as_str(),
        Some("1edc459e489c1c7e1f6f292e9d1fef2eb00f46f9d1b385e9100748ea9e333d60")
    );
    assert_eq!(
        timeout["input_digest"].as_str(),
        Some("32b2acb938379beafeac9ad5b476b218d77bd977b268bf3d862ef457589ed6c8")
    );

    let warm_timeout: Value = serde_json::from_slice(
        &std::fs::read(root().join(
            "docs/testing/performance/0.74/local-runs/w12-workspace-fast-gate-warm-timeout-415b01fc.json",
        ))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        warm_timeout["source_commit"].as_str(),
        Some("415b01fc59fdf0829197d50832f0963a77bf3b5b")
    );
    assert_eq!(warm_timeout["outcome"].as_str(), Some("timeout"));
    assert_eq!(warm_timeout["execution_phase"].as_str(), Some("tests"));
    assert_eq!(warm_timeout["duration_ms"].as_u64(), Some(2_626_352));
    assert_eq!(
        warm_timeout["termination_overrun_ms"].as_u64(),
        Some(226_352)
    );
    assert_eq!(warm_timeout["compile_duration_seconds"].as_u64(), Some(684));
    assert_eq!(warm_timeout["tests_started"].as_bool(), Some(true));
    assert_eq!(warm_timeout["tests_scheduled"].as_u64(), Some(3_655));
    assert_eq!(warm_timeout["test_binaries"].as_u64(), Some(477));
    assert_eq!(warm_timeout["tests_skipped"].as_u64(), Some(82));
    assert_eq!(warm_timeout["junit_present"].as_bool(), Some(false));
    assert_eq!(warm_timeout["test_failure_reported"].as_bool(), Some(false));
    assert_eq!(
        warm_timeout["test_completion_proven"].as_bool(),
        Some(false)
    );
    assert_eq!(warm_timeout["timeout_seconds"].as_u64(), Some(2_400));
    assert_eq!(warm_timeout["budget_seconds"].as_u64(), Some(840));
    assert_eq!(warm_timeout["command_changed"].as_bool(), Some(false));
    assert_eq!(warm_timeout["budget_changed"].as_bool(), Some(false));
    assert_eq!(warm_timeout["timeout_changed"].as_bool(), Some(false));
    assert_eq!(warm_timeout["promotable"].as_bool(), Some(false));
    assert_eq!(
        warm_timeout["release_admission_allowed"].as_bool(),
        Some(false)
    );
    assert_eq!(
        warm_timeout["retry_count_after_this_result"].as_u64(),
        Some(0)
    );
}

#[test]
fn w11_linux_fixture_harness_cli_is_platform_gated() {
    let source =
        std::fs::read_to_string(root().join("tools/long-run-supervisor-074/src/main.rs")).unwrap();
    assert!(source.contains(
        "#[cfg(target_os = \"linux\")]\n        [command] if command == \"campaign-fixture-harness\" => campaign_fixture_harness(),"
    ));
}

#[test]
fn published_b73_requires_exact_tag_archive_and_runtime_relationship() {
    let mut value = contract("baseline-identities.toml");
    value["predecessor_candidate"]["annotated_tag_commit_sha"] =
        toml::Value::String("0".repeat(40));
    value["predecessor_candidate"]["release_archive_verified"] = toml::Value::Boolean(false);
    value["predecessor_closure"]["tag_and_product_relationship_verified"] =
        toml::Value::Boolean(false);

    let problems = xtask::performance_contract_074::check_identities(&value);
    assert!(has(&problems, "annotated_tag_commit_sha"));
    assert!(has(&problems, "release_archive_verified"));
    assert!(has(&problems, "tag_and_product_relationship_verified"));
}

#[test]
fn local_harness_freezes_placement_abba_and_noise_policy() {
    let mut value = contract("local-harness.toml");
    value["cpu_affinity_required"] = toml::Value::Boolean(false);
    value["pairs"] = toml::Value::Integer(3);
    value["noise"]["minimum_p99_effect"] = toml::Value::Float(0.0);
    let problems = xtask::performance_contract_074::check_local_harness(&value);
    assert!(has(&problems, "cpu_affinity_required"));
    assert!(has(&problems, "pairs"));
    assert!(has(&problems, "minimum_p99_effect"));
}

#[test]
fn proposal_admission_requires_w1_attribution_and_evidence() {
    let mut value = contract("proposal-registry.toml");
    value["work_items"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|item| item["id"].as_str() == Some("W1"))
        .unwrap()["decision"] = toml::Value::String("in-progress".to_owned());
    let problems = xtask::performance_contract_074::check_registry(&value);
    assert!(has(&problems, "requires locally attributed W1"));

    let mut value = contract("proposal-registry.toml");
    let w3 = value["work_items"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|item| item["id"].as_str() == Some("W3"))
        .unwrap();
    w3["decision"] = toml::Value::String("authorized-local-candidate".to_owned());
    w3.as_table_mut().unwrap().remove("evidence");
    let problems = xtask::performance_contract_074::check_registry(&value);
    assert!(has(&problems, "requires evidence"));
}

#[test]
fn w3_adaptive_retry_preserves_pipeline_one_and_freezes_deep_thresholds() {
    let value = contract("w3-adaptive-coalescing-contract.toml");
    assert_eq!(
        value["state"].as_str(),
        Some("rejected-semantic-backpressure-before-measurement")
    );
    assert_eq!(value["pairs"].as_integer(), Some(5));
    assert_eq!(
        value["activation"]["minimum_complete_frames_in_current_read"].as_integer(),
        Some(2)
    );
    assert_eq!(
        value["activation"]["pipeline_one_uses_existing_write_response"].as_bool(),
        Some(true)
    );
    assert_eq!(value["activation"]["timer_allowed"].as_bool(), Some(false));
    assert_eq!(
        value["deep_pipeline_acceptance"]["minimum_goodput_ratio"].as_float(),
        Some(1.20)
    );
    assert_eq!(
        value["pipeline_one_non_regression"]["minimum_goodput_ratio"].as_float(),
        Some(0.98)
    );
    assert_eq!(
        value["pipeline_one_non_regression"]["minimum_passing_pairs"].as_integer(),
        Some(5)
    );
    assert_eq!(
        value["semantic_guards"]["native_surfaces_unchanged"].as_bool(),
        Some(true)
    );
    assert_eq!(
        value["negative_evidence"].as_str(),
        Some("local-runs/w3-adaptive-semantic-rejection.json")
    );
}

#[test]
fn socket_boundary_attribution_keeps_local_product_work_admitted() {
    let value = contract("proposal-registry.toml");
    assert!(
        xtask::performance_contract_074::check_registry(&value).is_empty(),
        "the checked-in open kernel gate is a terminal local W1 attribution state"
    );
}

#[test]
fn matched_receipt_is_accepted() {
    assert!(xtask::performance_contract_074::check_receipt(&receipt()).is_empty());
}

#[test]
fn trace_workload_security_and_persistence_drift_are_rejected() {
    for field in [
        "trace_sha256",
        "payload_corpus_sha256",
        "key_corpus_sha256",
        "seed",
        "measurement_seconds",
        "concurrency",
        "pipeline_depth",
        "security",
        "persistence",
        "semantic_contract",
    ] {
        let mut value = receipt();
        value["candidate"]["trace"][field] = json!(format!("changed-{field}"));
        let problems = xtask::performance_contract_074::check_receipt(&value);
        assert!(
            has(&problems, &format!("mismatch for {field}")),
            "{problems:#?}"
        );
    }
}

#[test]
fn redis_tool_mismatch_and_mixed_binary_are_rejected() {
    let mut value = receipt();
    value["candidate"]["redis_tool_sha256"] = json!("redis-other");
    value["candidate"]["binary_sha256s"] = json!(["a", "b"]);
    let problems = xtask::performance_contract_074::check_receipt(&value);
    assert!(has(&problems, "mismatched Redis tool"));
    assert!(has(&problems, "mixes more than one binary"));
}

#[test]
fn missing_outcomes_final_sample_and_nonfinite_metric_are_rejected() {
    let mut value = receipt();
    value["baseline"]["outcomes"]
        .as_object_mut()
        .unwrap()
        .remove("errors");
    value["candidate"]["metrics"]["cpu_seconds_per_operation"] = json!("NaN");
    value["final_sample_present"] = json!(false);
    let problems = xtask::performance_contract_074::check_receipt(&value);
    assert!(has(&problems, "missing the final sample"));
    assert!(has(&problems, "outcomes are missing errors"));
    assert!(has(&problems, "is not finite"));
}

#[test]
fn reordered_blocks_and_candidate_thresholds_are_rejected() {
    let mut value = receipt();
    value["block_order"] = json!(["candidate", "baseline"]);
    value["candidate_derived_thresholds"] = json!(true);
    let problems = xtask::performance_contract_074::check_receipt(&value);
    assert!(has(&problems, "reordered"));
    assert!(has(&problems, "candidate-derived thresholds"));
}

#[test]
fn pooled_or_missing_native_surface_is_rejected() {
    let mut value = receipt();
    let results = value["surface_results"].as_array_mut().unwrap();
    results.retain(|row| row["surface"] != "native-api-hc2");
    results.push(json!({
        "surface": "native",
        "trace_sha256": "trace-a",
        "baseline_goodput": 1000.0,
        "candidate_goodput": 1000.0
    }));
    let problems = xtask::performance_contract_074::check_receipt(&value);
    assert!(has(&problems, "pooled"));
    assert!(has(&problems, "omits native-api-hc2"));
}

#[test]
fn unmatched_surface_trace_and_three_percent_native_regression_are_rejected() {
    let mut value = receipt();
    let row = value["surface_results"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["surface"] == "client-surface-state")
        .unwrap();
    row["trace_sha256"] = json!("other-trace");
    row["candidate_goodput"] = json!(970.0);
    let problems = xtask::performance_contract_074::check_receipt(&value);
    assert!(has(&problems, "does not use the matched trace"));
    assert!(has(&problems, "regresses goodput beyond 2%"));
}

#[test]
fn long_run_controller_workflow_is_manual_serialized_and_signs_off_host() {
    let workflow = std::fs::read_to_string(
        root().join(".github/workflows/performance-long-run-qualification-074.yml"),
    )
    .unwrap();
    assert!(workflow.contains("workflow_call:"));
    assert!(workflow.contains("workflow_dispatch:"));
    assert!(!workflow.contains("pull_request:"));
    assert!(!workflow.contains("schedule:"));
    assert!(workflow.contains("group: long-run-074-${{ inputs.host_id }}"));
    assert!(workflow.contains("cancel-in-progress: false"));
    assert!(workflow.contains("environment: performance-reference-074"));
    assert!(workflow.contains("runs-on: ubuntu-latest"));
    assert!(workflow.contains("runs-on: [self-hosted, linux, x64, hydracache-release]"));
    assert_eq!(
        workflow
            .matches("secrets.HYDRACACHE_074_AUTH_SIGNING_KEY_HEX")
            .count(),
        1
    );
    let signing_key = workflow
        .find("secrets.HYDRACACHE_074_AUTH_SIGNING_KEY_HEX")
        .unwrap();
    let self_hosted = workflow
        .find("runs-on: [self-hosted, linux, x64, hydracache-release]")
        .unwrap();
    assert!(signing_key < self_hosted);
    assert!(workflow.contains("build-request"));
    assert!(workflow.contains("request.sha256"));
    assert!(workflow.contains("actions: read"));
    assert!(workflow.contains("start_bundle_run_id"));
    assert!(workflow.contains("start_bundle_artifact_name"));
    assert!(workflow.contains("--verify-start-bundle"));
    assert!(workflow.contains("request-start"));
    assert!(workflow.contains("inputs.expected_state_revision == '0'"));
    assert!(workflow.contains("Capture a best-effort read-only status snapshot"));
    assert!(!workflow.contains("continue-on-error: true"));

    let entry = std::fs::read_to_string(
        root().join(".github/workflows/performance-long-run-host-capability-074.yml"),
    )
    .unwrap();
    for required in [
        "- controller",
        "- controller-start-abort-rehearsal",
        "inputs.mode == 'controller'",
        "inputs.mode == 'controller-start-abort-rehearsal'",
        "uses: ./.github/workflows/performance-long-run-qualification-074.yml",
        "source_sha: ${{ inputs.controller_source_sha }}",
        "operation: ${{ inputs.controller_operation }}",
        "approval_nonce_sha256: ${{ inputs.approval_nonce_sha256 }}",
        "start_bundle_run_id: ${{ inputs.start_bundle_run_id }}",
        "start_bundle_artifact_name: ${{ inputs.start_bundle_artifact_name }}",
        "secrets: inherit",
    ] {
        assert!(
            entry.contains(required),
            "controller entrypoint omitted {required}"
        );
    }
    for required in [
        "needs: controller-rehearsal-start",
        "needs: controller-rehearsal-attach",
        "operation: start",
        "operation: attach",
        "operation: abort",
        "expected_state_revision: \"0\"",
        "expected_state_revision: \"2\"",
        "expected_state_revision: \"3\"",
        "request_id: ${{ inputs.attach_request_id }}",
        "request_id: ${{ inputs.abort_request_id }}",
    ] {
        assert!(
            entry.contains(required),
            "same-run controller rehearsal omitted {required}"
        );
    }
    assert_eq!(
        entry
            .matches("uses: ./.github/workflows/performance-long-run-qualification-074.yml")
            .count(),
        12
    );
}

#[test]
fn host_observation_workflow_is_read_only_fixed_scope_and_socket_bound() {
    let workflow = std::fs::read_to_string(
        root().join(".github/workflows/performance-long-run-host-observation-074.yml"),
    )
    .unwrap();
    let _: serde_yaml::Value = serde_yaml::from_str(&workflow).unwrap();
    for required in [
        "workflow_call:",
        "workflow_dispatch:",
        "group: long-run-074-${{ inputs.host_id }}",
        "cancel-in-progress: false",
        "runs-on: [self-hosted, linux, x64, hydracache-release]",
        "environment: performance-reference-074",
        "operation\": \"host_observation",
        "2dd3f4aa960289c385aad3988d988533197269368f851e5834f535a753cd92ba",
        "baa6e451a7248643e7a96fc5908ac07e0c97d38255e31b1bb1494ea5ac26c6ec",
        "/run/hydracache-perf/supervisor-v1.sock",
        "/opt/hydracache-perf/bin/hydracache-long-run-supervisor-074",
        "installed_source_commit",
        "active_campaign_absent",
        "installed_fixture_binary",
        "/opt/hydracache-performance/0.74/campaign-lifecycle-fixture",
        "installed_supervisor_binary_sha256",
        "peer_admission_required",
        "product_candidate_started",
        "promotable\": False",
    ] {
        assert!(workflow.contains(required), "workflow omitted {required}");
    }
    for forbidden in [
        "sudo -n",
        "systemctl stop",
        "request-start",
        "performance-long-run-qualification-074",
        "sleep ",
    ] {
        assert!(
            !workflow.contains(forbidden),
            "read-only host observation workflow contains {forbidden}"
        );
    }
    let collector = workflow
        .split_once("- name: Collect and independently bind the host receipt")
        .unwrap()
        .1;
    assert!(
        collector.contains("import re"),
        "fixture digest validation must import its regular-expression dependency"
    );

    let entry = std::fs::read_to_string(
        root().join(".github/workflows/performance-long-run-host-capability-074.yml"),
    )
    .unwrap();
    assert!(entry.contains("- host-observation"));
    assert!(entry.contains("inputs.mode == 'host-observation'"));
    assert!(
        entry.contains("uses: ./.github/workflows/performance-long-run-host-observation-074.yml")
    );
}

#[test]
fn non_product_start_bundle_workflow_is_observation_bound_and_does_not_start() {
    let workflow = std::fs::read_to_string(
        root().join(".github/workflows/performance-long-run-start-bundle-074.yml"),
    )
    .unwrap();
    let _: serde_yaml::Value = serde_yaml::from_str(&workflow).unwrap();
    for required in [
        "workflow_call:",
        "workflow_dispatch:",
        "uses: ./.github/workflows/performance-long-run-host-observation-074.yml",
        "runs-on: ubuntu-latest",
        "performance_non_product_start_bundle_074.py",
        "test_performance_non_product_start_bundle_074",
        "--verify-start-bundle",
        "long-run-074-start-bundle-${{ steps.bundle.outputs.bundle_sha256 }}",
        "long-run-074-start-bundle-evidence-${{ steps.bundle.outputs.bundle_sha256 }}",
    ] {
        assert!(workflow.contains(required), "workflow omitted {required}");
    }
    for forbidden in [
        "request-start",
        "HYDRACACHE_074_AUTH_SIGNING_KEY_HEX",
        "sudo -n",
        "systemctl stop",
    ] {
        assert!(
            !workflow.contains(forbidden),
            "bundle-only workflow contains {forbidden}"
        );
    }

    let assembler = std::fs::read_to_string(
        root().join("scripts/perf/performance_non_product_start_bundle_074.py"),
    )
    .unwrap();
    for required in [
        "non-product-start-fixture.toml",
        "/opt/hydracache-performance/0.74/campaign-lifecycle-fixture",
        "campaign-start-rehearsal-harness",
        "signed_start_dispatched\": False",
        "campaign_state_mutated\": False",
        "product_candidate_started\": False",
        "promotable\": False",
    ] {
        assert!(assembler.contains(required), "assembler omitted {required}");
    }
    let fixture = std::fs::read_to_string(
        root().join("docs/testing/performance/0.74/non-product-start-fixture.toml"),
    )
    .unwrap();
    for required in [
        "product_candidate = false",
        "promotable = false",
        "role_started = \"i74\"",
        "cleanup = \"explicit-signed-abort\"",
    ] {
        assert!(
            fixture.contains(required),
            "fixture contract omitted {required}"
        );
    }

    let entry = std::fs::read_to_string(
        root().join(".github/workflows/performance-long-run-host-capability-074.yml"),
    )
    .unwrap();
    assert!(entry.contains("- start-bundle"));
    assert!(entry.contains("inputs.mode == 'start-bundle'"));
    assert!(entry.contains("performance-long-run-start-bundle-074.yml"));
}

#[test]
fn w11_host_observation_evidence_is_exact_non_mutating_and_non_promotable() {
    let controller = contract("long-run-controller-resilience-contract.toml");
    let implementation = controller["local_implementation"].as_table().unwrap();
    let relative = implementation["host_observation_socket_evidence"]
        .as_str()
        .unwrap();
    let evidence: Value =
        serde_json::from_slice(&std::fs::read(root().join(relative)).unwrap()).unwrap();

    let source = "127ebc6cb24191d567772966586bdb827a0ae380";
    assert_eq!(evidence["source_commit"].as_str(), Some(source));
    assert_eq!(
        evidence["provisioning"]["run_id"].as_u64(),
        Some(37394441292)
    );
    assert_eq!(
        evidence["observation"]["run_id"].as_u64(),
        Some(37394832233)
    );
    assert_eq!(
        evidence["observation"]["installed_source_commit"].as_str(),
        Some(source)
    );
    assert_eq!(
        evidence["observation"]["campaign_state_mutated"].as_bool(),
        Some(false)
    );
    assert_eq!(evidence["product_candidate_started"].as_bool(), Some(false));
    assert_eq!(
        evidence["expensive_qualification_started"].as_bool(),
        Some(false)
    );
    assert_eq!(evidence["promotable"].as_bool(), Some(false));
    assert_eq!(
        evidence["decision"]["host_observation_socket_export_complete"].as_bool(),
        Some(true)
    );
    assert_eq!(
        evidence["decision"]["privileged_start_bundle_host_rehearsal_complete"].as_bool(),
        Some(false)
    );
    assert_eq!(
        evidence["post_run_host_state"]["runner_active"].as_bool(),
        Some(false)
    );
    assert_eq!(
        evidence["post_run_host_state"]["supervisor_active"].as_bool(),
        Some(true)
    );
}

#[test]
fn w9e_allocator_attribution_is_linux_only_complete_and_non_promotable() {
    let allocator = contract("w9e-linux-allocator-profile-contract.toml");
    assert_eq!(
        allocator["state"].as_str(),
        Some("preregistered-before-dedicated-linux-execution")
    );
    assert_eq!(allocator["operating_system"].as_str(), Some("linux"));
    assert_eq!(allocator["architecture"].as_str(), Some("x86_64"));
    assert_eq!(allocator["minimum_attempts"].as_integer(), Some(20));
    assert_eq!(
        allocator["counterbalanced_allocator_order_required"].as_bool(),
        Some(true)
    );
    assert_eq!(allocator["product_mutation_allowed"].as_bool(), Some(false));
    assert_eq!(
        allocator["allocator_default_change_allowed"].as_bool(),
        Some(false)
    );
    assert_eq!(
        allocator["acceptance_decision_allowed"].as_bool(),
        Some(false)
    );
    assert_eq!(allocator["promotable"].as_bool(), Some(false));
    assert_eq!(
        allocator["owner_materiality_floor_percent"].as_float(),
        Some(5.0)
    );

    let allocators = allocator["allocators"].as_array().unwrap();
    assert_eq!(allocators.len(), 3);
    for expected in ["system", "mimalloc", "jemalloc"] {
        assert!(allocators
            .iter()
            .any(|value| value.as_str() == Some(expected)));
    }

    let workflow = std::fs::read_to_string(
        root().join(".github/workflows/performance-allocator-attribution-074.yml"),
    )
    .unwrap();
    assert!(workflow.contains("workflow_dispatch:"));
    assert!(workflow.contains("feat/0.74-resp-native-throughput"));
    assert!(workflow.contains("group: performance-reference-074-host"));
    assert!(workflow.contains("cancel-in-progress: false"));
    assert!(workflow.contains("runs-on: [self-hosted, linux, x64, hydracache-release]"));
    assert!(workflow.contains("--features \"allocator-$allocator\""));
    assert!(workflow.contains("--repeats 5"));
    assert!(workflow.contains("acceptance_decision_allowed"));
    assert!(workflow.contains("promotable"));
    assert!(!workflow.contains("performance-long-run-qualification-074"));
}

#[test]
fn w9c_kernel_attribution_is_linux_only_counterbalanced_and_non_promotable() {
    let kernel = contract("w9c-linux-kernel-attribution-contract.toml");
    assert_eq!(
        kernel["state"].as_str(),
        Some("preregistered-before-dedicated-linux-execution")
    );
    assert_eq!(kernel["operating_system"].as_str(), Some("linux"));
    assert_eq!(kernel["architecture"].as_str(), Some("x86_64"));
    assert_eq!(kernel["minimum_attempts"].as_integer(), Some(30));
    assert_eq!(kernel["minimum_processes"].as_integer(), Some(60));
    assert_eq!(kernel["repeats_per_cell"].as_integer(), Some(5));
    assert_eq!(
        kernel["paired_untraced_control_required"].as_bool(),
        Some(true)
    );
    assert_eq!(
        kernel["measurement_gate_after_warmup_required"].as_bool(),
        Some(true)
    );
    assert_eq!(kernel["product_mutation_allowed"].as_bool(), Some(false));
    assert_eq!(kernel["runtime_tuning_allowed"].as_bool(), Some(false));
    assert_eq!(kernel["acceptance_decision_allowed"].as_bool(), Some(false));
    assert_eq!(kernel["promotable"].as_bool(), Some(false));

    let workflow = std::fs::read_to_string(
        root().join(".github/workflows/performance-kernel-attribution-074.yml"),
    )
    .unwrap();
    assert!(workflow.contains("workflow_dispatch:"));
    assert!(workflow.contains("feat/0.74-resp-native-throughput"));
    assert!(workflow.contains("group: performance-reference-074-host"));
    assert!(workflow.contains("cancel-in-progress: false"));
    assert!(workflow.contains("runs-on: [self-hosted, linux, x64, hydracache-release]"));
    assert!(workflow.contains("command -v strace"));
    assert!(workflow.contains("command -v ss"));
    assert!(workflow.contains("--repeats 5"));
    assert!(workflow.contains("candidate_data_present"));
    assert!(workflow.contains("promotable"));
    assert!(!workflow.contains("performance-long-run-qualification-074"));
}

#[test]
fn w11_host_capability_probe_is_read_only_and_serialized() {
    let workflow = std::fs::read_to_string(
        root().join(".github/workflows/performance-long-run-host-capability-074.yml"),
    )
    .unwrap();
    assert!(workflow.contains("workflow_dispatch:"));
    assert!(workflow.contains("group: performance-reference-074-host"));
    assert!(workflow.contains("cancel-in-progress: false"));
    assert!(workflow.contains("runs-on: [self-hosted, linux, x64, hydracache-release]"));
    assert!(workflow.contains("Collect read-only host capability receipt"));
    assert!(
        workflow.contains("if: github.event_name == 'workflow_dispatch' && inputs.mode == 'probe'")
    );
    let probe_job = workflow
        .split("\n  provision:")
        .next()
        .expect("probe job precedes mutating dispatch jobs");
    for forbidden in [
        "sudo ",
        "systemctl start",
        "systemctl stop",
        "systemctl restart",
        "systemd-run --unit",
        "performance-long-run-qualification-074",
    ] {
        assert!(
            !probe_job.contains(forbidden),
            "forbidden mutation: {forbidden}"
        );
    }
    for mode in [
        "provision",
        "systemd-smoke",
        "controller-loss-smoke",
        "campaign-lifecycle-smoke",
        "campaign-progress-loss-smoke",
        "campaign-measurement-loss-smoke",
        "campaign-lease-expiry-smoke",
        "supervisor-overhead-smoke",
        "role-overhead-smoke",
    ] {
        assert!(
            workflow.contains(&format!("inputs.mode == '{mode}'")),
            "missing isolated dispatch condition for {mode}"
        );
    }

    let probe = std::fs::read_to_string(
        root().join("scripts/perf/performance_long_run_host_capability_074.py"),
    )
    .unwrap();
    assert!(probe.contains("\"read_only\": True"));
    assert!(probe.contains("\"mutation_performed\": False"));

    let overhead = std::fs::read_to_string(
        root().join("scripts/perf/performance_long_run_supervisor_overhead_074.py"),
    )
    .unwrap();
    assert!(overhead.contains("hydracache-w11-supervisor-idle-overhead-v1"));
    assert!(overhead.contains("\"role_overhead_qualification_complete\": False"));
    assert!(overhead.contains("\"release_admission_allowed\": False"));
    assert!(workflow.contains("--duration-seconds 30"));
    assert!(workflow.contains("--maximum-cpu-percent 0.5"));
    assert!(workflow.contains("--maximum-rss-bytes 67108864"));
    assert!(workflow.contains("--maximum-io-bytes-per-second 1048576"));
    assert!(workflow.contains("performance_long_run_role_overhead_collect_074.py"));
    assert!(workflow.contains("--supervisor-pid \"$SUPERVISOR_PID\""));
    assert!(workflow.contains("--control-group \"$SUPERVISOR_CONTROL_GROUP\""));
    assert!(!workflow
        .contains("sudo -n /usr/local/sbin/hydracache-provision-host-074 --role-overhead-smoke"));
    assert!(workflow.contains("--expected-i74-source \"$INSTALLED_SOURCE_COMMIT\""));
    assert!(workflow.contains("--expected-c74-source \"$INSTALLED_SOURCE_COMMIT\""));
    assert!(workflow.contains(
        "assert receipt[\"decision\"][\"role_overhead_qualification_complete\"] is False"
    ));
    let overhead_job = workflow
        .split("\n  supervisor-overhead-smoke:")
        .nth(1)
        .expect("isolated supervisor overhead job");
    for forbidden in [
        "systemctl start",
        "systemctl stop",
        "systemctl restart",
        "--campaign-lifecycle-smoke-start",
        "performance-long-run-qualification-074",
    ] {
        assert!(
            !overhead_job.contains(forbidden),
            "overhead screen must remain read-only: {forbidden}"
        );
    }
}

#[test]
fn w12_ordinary_ci_preserves_explicit_infrastructure_boundary() {
    let read = |name: &str| {
        let text = std::fs::read_to_string(root().join(".github/workflows").join(name)).unwrap();
        serde_yaml::from_str::<serde_yaml::Value>(&text).unwrap()
    };
    let ci = read("ci.yml");
    let inputs = &ci["on"]["workflow_dispatch"]["inputs"];
    for flag in [
        "run_nightly",
        "run_reference_performance",
        "run_memory_diagnostic",
        "run_retention_soak_070",
        "run_management_candidate_soak_072",
        "run_management_ship_soak_072",
        "run_management_mixed_072",
        "run_redis_compat_release_proof",
    ] {
        assert_eq!(inputs[flag]["default"].as_bool(), Some(false), "{flag}");
    }
    assert_eq!(
        inputs["performance_0671_mode"]["default"].as_str(),
        Some("off")
    );
    for (file, job) in [
        ("performance-kernel-attribution-074.yml", "attribute"),
        ("performance-long-run-host-capability-074.yml", "probe"),
    ] {
        let workflow = read(file);
        let condition = workflow["jobs"][job]["if"].as_str().unwrap();
        assert!(condition.contains("github.event_name == 'workflow_dispatch'"));
        assert!(
            !condition.contains("||"),
            "push must not select {file}#{job}"
        );
    }
    let bundle = read("performance-long-run-start-bundle-074.yml");
    let group = bundle["concurrency"]["group"].as_str().unwrap();
    assert!(group.starts_with("long-run-074-start-bundle-"));
    for marker in ["inputs.host_id", "inputs.source_sha", "github.run_id"] {
        assert!(group.contains(marker));
    }
    assert_eq!(
        bundle["concurrency"]["cancel-in-progress"].as_bool(),
        Some(false)
    );
}

#[test]
fn w12_internal_workspace_tools_inherit_reviewed_license() {
    let workspace: toml::Value =
        toml::from_str(&std::fs::read_to_string(root().join("Cargo.toml")).unwrap()).unwrap();
    assert_eq!(
        workspace["workspace"]["package"]["license"].as_str(),
        Some("Apache-2.0")
    );
    let mut checked = 0;
    for member in workspace["workspace"]["members"].as_array().unwrap() {
        let member = member.as_str().unwrap();
        if !member.starts_with("tools/") {
            continue;
        }
        let manifest: toml::Value = toml::from_str(
            &std::fs::read_to_string(root().join(member).join("Cargo.toml")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            manifest["package"]["publish"].as_bool(),
            Some(false),
            "{member}"
        );
        assert_eq!(
            manifest["package"]
                .get("license")
                .and_then(|license| license.get("workspace"))
                .and_then(toml::Value::as_bool),
            Some(true),
            "internal tool must inherit the reviewed workspace license: {member}"
        );
        checked += 1;
    }
    assert!(checked >= 2, "both 0.74 workspace tools must be checked");
}

#[test]
fn w12_live_registries_preserve_frozen_baseline_inputs() {
    let identities: toml::Value = toml::from_str(
        &std::fs::read_to_string(root().join("docs/testing/memory/0.71/baseline-identities.toml"))
            .unwrap(),
    )
    .unwrap();
    for input in identities["scenario"]["input"].as_array().unwrap() {
        let path = input["path"].as_str().unwrap();
        let bytes = std::fs::read(root().join(path)).unwrap();
        let text = String::from_utf8(bytes).unwrap().replace("\r\n", "\n");
        assert_eq!(
            Sha256::digest(text.as_bytes())
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
            input["sha256"].as_str().unwrap(),
            "historical prerequisite must not be rewritten to register live CI: {path}"
        );
    }
}
