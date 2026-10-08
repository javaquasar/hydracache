//! All positive fixtures are SYNTHETIC: fake ELF, invented Cargo log, test key.
//! No compilation, executable correctness or real builder trust is proved here.
use ed25519_dalek::{Signer, SigningKey};
use hydracache_long_run_supervisor_074::diagnostic_artifacts::*;
use hydracache_long_run_supervisor_074::diagnostic_lease::{
    CellIntent, DiagnosticIdentity, BINARY_PATH, SOURCE_COMMIT,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

struct Synthetic {
    key: SigningKey,
    statement: BuildStatement,
    binary: Vec<u8>,
    log: Vec<u8>,
    configs: BTreeMap<String, &'static [u8]>,
}
const ROOT_LOCK: &[u8] = include_bytes!("../../../Cargo.lock");
const OBSERVER_LOCK: &[u8] = include_bytes!("../../get-owner-scheduled-controls-074/Cargo.lock");

fn artifact_event() -> serde_json::Value {
    serde_json::json!({
        "reason":"compiler-artifact",
        "package_id":"path+file:///synthetic/tools/get-owner-scheduled-controls-074#get-owner-scheduled-controls-074@0.0.0",
        "target":{"name":"timing-controls-074","kind":["bin"],"crate_types":["bin"],
            "src_path":"/synthetic/tools/get-owner-scheduled-controls-074/src/bin/timing_controls.rs"},
        "profile":{"opt_level":"3","debug_assertions":false,"test":false},
        "features":[],
        "executable":"/synthetic/tools/get-owner-scheduled-controls-074/target/x86_64-unknown-linux-gnu/release/timing-controls-074"
    })
}
fn log(events: &[serde_json::Value]) -> Vec<u8> {
    events
        .iter()
        .flat_map(|v| {
            let mut b = serde_json::to_vec(v).unwrap();
            b.push(b'\n');
            b
        })
        .collect()
}
fn final_event() -> serde_json::Value {
    serde_json::json!({"reason":"build-finished","success":true})
}

impl Synthetic {
    fn new() -> Self {
        let configs: BTreeMap<String, &'static [u8]> = BTreeMap::from([
            (
                "embedded".into(),
                include_bytes!(
                    "../../../docs/testing/performance/0.74/rental-pilot-draft/embedded.json"
                )
                .as_slice(),
            ),
            (
                "direct".into(),
                include_bytes!(
                    "../../../docs/testing/performance/0.74/rental-pilot-draft/direct.json"
                )
                .as_slice(),
            ),
            (
                "resp2".into(),
                include_bytes!(
                    "../../../docs/testing/performance/0.74/rental-pilot-draft/resp2.json"
                )
                .as_slice(),
            ),
            (
                "resp3".into(),
                include_bytes!(
                    "../../../docs/testing/performance/0.74/rental-pilot-draft/resp3.json"
                )
                .as_slice(),
            ),
        ]);
        let mut binary = vec![0; 120];
        binary[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
        binary[16..18].copy_from_slice(&2u16.to_le_bytes());
        binary[18..20].copy_from_slice(&62u16.to_le_bytes());
        binary[20..24].copy_from_slice(&1u32.to_le_bytes());
        binary[32..40].copy_from_slice(&64u64.to_le_bytes());
        binary[52..54].copy_from_slice(&64u16.to_le_bytes());
        binary[54..56].copy_from_slice(&56u16.to_le_bytes());
        binary[56..58].copy_from_slice(&1u16.to_le_bytes());
        let log = log(&[artifact_event(), final_event()]);
        let statement = BuildStatement {
            schema_version: 1,
            repository_id: 74,
            builder_id: "synthetic-test-builder".into(),
            source_commit: SOURCE_COMMIT.into(),
            source_tree: SOURCE_TREE.into(),
            source_clean_before: true,
            source_clean_after: true,
            rustc_version: "rustc 1.94.0 (4a4ef493e 2026-03-02)".into(),
            cargo_version: "cargo 1.94.0 (85eff7c80 2026-01-15)".into(),
            target: "x86_64-unknown-linux-gnu".into(),
            profile: "release".into(),
            features: vec![],
            allocator: "System".into(),
            counting_allocator: false,
            build_command: BUILD_COMMAND.iter().map(|s| s.to_string()).collect(),
            binary: ArtifactDigest::of(&binary),
            root_lock: ArtifactDigest::of(ROOT_LOCK),
            observer_lock: ArtifactDigest::of(OBSERVER_LOCK),
            build_log: ArtifactDigest::of(&log),
            configs: configs
                .iter()
                .map(|(k, v)| (k.clone(), ArtifactDigest::of(v)))
                .collect(),
        };
        Self {
            key: SigningKey::from_bytes(&[74; 32]),
            statement,
            binary,
            log,
            configs,
        }
    }
    fn trust(&self) -> BuildTrust {
        BuildTrust {
            key: self.key.verifying_key(),
            repository_id: 74,
            builder_id: "synthetic-test-builder".into(),
        }
    }
    fn receipt(&self) -> (Vec<u8>, DiagnosticIdentity) {
        let signature = self.key.sign(&signing_message(&self.statement).unwrap());
        let bytes = receipt_bytes(&SignedBuild {
            statement: self.statement.clone(),
            signature_hex: hex(&signature.to_bytes()),
        })
        .unwrap();
        let identity = DiagnosticIdentity {
            lease_id: "a".repeat(64),
            boot_id: "12345678-1234-1234-1234-123456789abc".into(),
            binary_sha256: self.statement.binary.sha256.clone(),
            build_provenance_sha256: hex(&Sha256::digest(&bytes)),
        };
        (bytes, identity)
    }
    fn verified(&self) -> VerifiedBuild {
        let (bytes, id) = self.receipt();
        verify_receipt(&bytes, &id, &self.trust()).unwrap()
    }
    fn contents(&self) -> ArtifactContents<'_> {
        ArtifactContents {
            binary: &self.binary,
            root_lock: ROOT_LOCK,
            observer_lock: OBSERVER_LOCK,
            build_log: &self.log,
            configs: self.configs.clone(),
        }
    }
    fn intent(&self) -> CellIntent {
        let (_, id) = self.receipt();
        let unit_name = format!("hydracache-diagnostic-074-{}-1.service", id.lease_id);
        CellIntent {
            lease_id: id.lease_id,
            boot_id: id.boot_id,
            surface: "embedded".into(),
            cgroup_path: format!("/system.slice/{unit_name}"),
            unit_name,
            binary_path: BINARY_PATH.into(),
            binary_sha256: id.binary_sha256,
            config_path: format!("{INSTALL_ROOT}/embedded.json"),
            config_sha256: self.statement.configs["embedded"].sha256.clone(),
            source_commit: SOURCE_COMMIT.into(),
            maximum_runtime_seconds: 60,
        }
    }
}

#[test]
fn unsigned_file_audit_has_read_only_cli_and_refuses_changed_inputs() {
    let f = Synthetic::new();
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    for (name, bytes) in [
        ("binary", f.binary.as_slice()),
        ("cargo.jsonl", f.log.as_slice()),
        ("root.lock", ROOT_LOCK),
        ("observer.lock", OBSERVER_LOCK),
    ] {
        std::fs::write(root.join(name), bytes).unwrap();
    }
    for (name, bytes) in &f.configs {
        std::fs::write(root.join(format!("{name}.json")), bytes).unwrap();
    }
    let command = || {
        let mut command =
            std::process::Command::new(env!("CARGO_BIN_EXE_hydracache-long-run-supervisor-074"));
        command
            .arg("audit-local-build")
            .arg(root.join("binary"))
            .arg(root.join("cargo.jsonl"))
            .arg(root.join("root.lock"))
            .arg(root.join("observer.lock"))
            .arg(root);
        command
    };
    let output = command().output().unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["attestation_verified"], false);
    assert_eq!(value["execution_authorized"], false);
    assert_eq!(value["admission_allowed"], false);
    assert_eq!(std::fs::read(root.join("binary")).unwrap(), f.binary);
    std::fs::write(root.join("direct.json"), b"changed").unwrap();
    let output = command().output().unwrap();
    assert_eq!(output.status.code(), Some(9));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap().trim(),
        "unsigned local build content audit refused"
    );
    std::fs::write(root.join("binary"), b"").unwrap();
    assert!(inspect_unsigned_files(
        &root.join("binary"),
        &root.join("cargo.jsonl"),
        &root.join("root.lock"),
        &root.join("observer.lock"),
        root
    )
    .is_err());
}

#[test]
fn unsigned_content_inspection_never_becomes_build_or_execution_authority() {
    let f = Synthetic::new();
    let inspection = inspect_unsigned_contents(&f.contents()).unwrap();
    let value = serde_json::to_value(inspection).unwrap();
    assert_eq!(
        value["schema_version"],
        "diagnostic-unsigned-content-inspection-074-v1"
    );
    assert_eq!(value["binary"]["sha256"], f.statement.binary.sha256);
    assert_eq!(value["build_log"]["sha256"], f.statement.build_log.sha256);
    for flag in [
        "attestation_verified",
        "source_git_identity_verified",
        "installed_paths_verified",
        "execution_authorized",
        "admission_allowed",
    ] {
        assert_eq!(value[flag], false, "{flag}");
    }
    let mut contents = f.contents();
    contents.binary = b"";
    assert!(inspect_unsigned_contents(&contents).is_err());
    contents = f.contents();
    contents.root_lock = b"changed";
    assert!(inspect_unsigned_contents(&contents).is_err());
    contents = f.contents();
    contents.observer_lock = b"changed";
    assert!(inspect_unsigned_contents(&contents).is_err());
    contents = f.contents();
    contents.configs.remove("direct");
    assert!(inspect_unsigned_contents(&contents).is_err());
    contents = f.contents();
    contents.configs.insert("direct".into(), b"changed");
    assert!(inspect_unsigned_contents(&contents).is_err());
    contents = f.contents();
    contents.build_log = b"{\"reason\":\"build-finished\",\"success\":true}\n";
    assert!(inspect_unsigned_contents(&contents).is_err());
}

#[test]
fn cargo_local_path_accepts_exact_version_fragment_not_foreign_sources() {
    let mut f = Synthetic::new();
    let mut event = artifact_event();
    event["package_id"] =
        "path+file:///synthetic/tools/get-owner-scheduled-controls-074#0.0.0".into();
    f.log = log(&[event.clone(), final_event()]);
    f.statement.build_log = ArtifactDigest::of(&f.log);
    f.verified().verify_contents(&f.contents()).unwrap();
    for invalid in [
        "path+file:///synthetic/tools/get-owner-scheduled-controls-074#0.0.1",
        "path+file:///synthetic/tools/foreign#0.0.0",
        "registry+https://crates.io/get-owner-scheduled-controls-074#0.0.0",
        "git+file:///synthetic/tools/get-owner-scheduled-controls-074#get-owner-scheduled-controls-074@0.0.0",
        "path+file:///synthetic/tools/foreign#get-owner-scheduled-controls-074@0.0.0",
    ] {
        event["package_id"] = invalid.into();
        f.log = log(&[event.clone(), final_event()]);
        f.statement.build_log = ArtifactDigest::of(&f.log);
        assert!(f.verified().verify_contents(&f.contents()).is_err(), "{invalid}");
    }
}

#[test]
fn diagnostic_build_attestation_has_a_separate_domain() {
    assert_eq!(SIGNATURE_DOMAIN, b"hydracache-diagnostic-build-074-v1");
}

#[test]
fn signed_build_statement_binds_all_fields_and_rejects_other_domains() {
    let f = Synthetic::new();
    let (bytes, mut identity) = f.receipt();
    f.verified().verify_contents(&f.contents()).unwrap();
    for domain in [
        b"hydracache-diagnostic-request-074-v1".as_slice(),
        b"hydracache-w11-provisioning-bundle-v1",
    ] {
        let mut message = domain.to_vec();
        message.push(0);
        message.extend(&signing_message(&f.statement).unwrap()[SIGNATURE_DOMAIN.len() + 1..]);
        let receipt = receipt_bytes(&SignedBuild {
            statement: f.statement.clone(),
            signature_hex: hex(&f.key.sign(&message).to_bytes()),
        })
        .unwrap();
        identity.build_provenance_sha256 = hex(&Sha256::digest(&receipt));
        assert!(verify_receipt(&receipt, &identity, &f.trust()).is_err());
    }
    let mut receipt: SignedBuild = serde_json::from_slice(&bytes).unwrap();
    receipt.statement.builder_id = "different".into();
    let receipt = receipt_bytes(&receipt).unwrap();
    identity.build_provenance_sha256 = hex(&Sha256::digest(&receipt));
    assert!(verify_receipt(&receipt, &identity, &f.trust()).is_err());
    let (_, identity) = f.receipt();
    let mut trust = f.trust();
    trust.key = SigningKey::from_bytes(&[9; 32]).verifying_key();
    assert!(verify_receipt(&bytes, &identity, &trust).is_err());
    let mut trust = f.trust();
    trust.repository_id += 1;
    assert!(verify_receipt(&bytes, &identity, &trust).is_err());
    let mut trust = f.trust();
    trust.builder_id = "untrusted".into();
    assert!(verify_receipt(&bytes, &identity, &trust).is_err());
}

#[test]
fn every_signed_top_level_field_refuses_unsigned_mutation() {
    let f = Synthetic::new();
    let (bytes, id) = f.receipt();
    let original: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    for name in original["statement"].as_object().unwrap().keys() {
        let mut changed = original.clone();
        fn mutate(value: &mut serde_json::Value) {
            match value {
                serde_json::Value::Bool(v) => *v = !*v,
                serde_json::Value::Number(v) => *v = (v.as_u64().unwrap() + 1).into(),
                serde_json::Value::String(v) => v.push('x'),
                serde_json::Value::Array(v) => v.push(serde_json::json!("extra")),
                serde_json::Value::Object(v) => mutate(v.values_mut().next().unwrap()),
                serde_json::Value::Null => unreachable!(),
            }
        }
        mutate(&mut changed["statement"][name]);
        let mut bytes = serde_json::to_vec(&changed).unwrap();
        bytes.push(b'\n');
        let mut identity = id.clone();
        identity.build_provenance_sha256 = hex(&Sha256::digest(&bytes));
        assert!(
            verify_receipt(&bytes, &identity, &f.trust()).is_err(),
            "{name}"
        );
    }
}

#[test]
fn signed_receipt_is_strict_canonical_bounded_and_lease_bound() {
    let f = Synthetic::new();
    let (bytes, id) = f.receipt();
    for mut invalid in [
        bytes[..bytes.len() - 1].to_vec(),
        b"{}\n".to_vec(),
        vec![b' '; MAX_RECEIPT_BYTES as usize + 1],
    ] {
        let mut identity = id.clone();
        identity.build_provenance_sha256 = hex(&Sha256::digest(&invalid));
        assert!(verify_receipt(&invalid, &identity, &f.trust()).is_err());
        invalid.clear();
    }
    for field in ["unknown", "signature_hex"] {
        let mut changed = bytes.clone();
        changed.splice(1..1, format!("\"{field}\":\"bad\",").bytes());
        let mut identity = id.clone();
        identity.build_provenance_sha256 = hex(&Sha256::digest(&changed));
        assert!(verify_receipt(&changed, &identity, &f.trust()).is_err());
    }
    let mut identity = id.clone();
    identity.binary_sha256 = "b".repeat(64);
    assert!(verify_receipt(&bytes, &identity, &f.trust()).is_err());
    let mut identity = id;
    identity.build_provenance_sha256 = "b".repeat(64);
    assert!(verify_receipt(&bytes, &identity, &f.trust()).is_err());
}

#[test]
fn even_resigned_source_toolchain_features_and_limits_cannot_drift() {
    type Mutation = fn(&mut BuildStatement);
    let changes: &[Mutation] = &[
        |s| s.schema_version = 2,
        |s| s.source_commit = "b".repeat(40),
        |s| s.source_tree = "b".repeat(40),
        |s| s.source_clean_before = false,
        |s| s.source_clean_after = false,
        |s| s.rustc_version.push('x'),
        |s| s.cargo_version.push('x'),
        |s| s.target = "aarch64-unknown-linux-gnu".into(),
        |s| s.profile = "debug".into(),
        |s| s.features.push("allocation-diagnostics".into()),
        |s| s.allocator = "jemalloc".into(),
        |s| s.counting_allocator = true,
        |s| s.build_command.push("--features=get-owner".into()),
        |s| s.root_lock.sha256 = "b".repeat(64),
        |s| s.observer_lock.sha256 = "b".repeat(64),
        |s| s.binary.bytes = 0,
        |s| s.binary.bytes = MAX_BINARY_BYTES + 1,
        |s| s.build_log.bytes = MAX_LOG_BYTES + 1,
        |s| s.root_lock.bytes = MAX_LOCK_BYTES + 1,
        |s| s.observer_lock.bytes = 0,
        |s| s.configs.get_mut("direct").unwrap().bytes = MAX_CONFIG_BYTES + 1,
        |s| {
            s.configs.remove("resp3");
        },
        |s| {
            s.configs
                .insert("extra".into(), s.configs["direct"].clone());
        },
    ];
    for change in changes {
        let mut f = Synthetic::new();
        change(&mut f.statement);
        let (bytes, id) = f.receipt();
        assert!(verify_receipt(&bytes, &id, &f.trust()).is_err());
    }
}

#[test]
fn every_artifact_is_hash_bound_and_p0_configs_cannot_drift() {
    let f = Synthetic::new();
    let verified = f.verified();
    for name in [
        "binary",
        "root_lock",
        "observer_lock",
        "build_log",
        "embedded",
        "direct",
        "resp2",
        "resp3",
    ] {
        let mut contents = f.contents();
        let corrupt = b"corrupt".as_slice();
        match name {
            "binary" => contents.binary = corrupt,
            "root_lock" => contents.root_lock = corrupt,
            "observer_lock" => contents.observer_lock = corrupt,
            "build_log" => contents.build_log = corrupt,
            surface => {
                contents.configs.insert(surface.into(), corrupt);
            }
        }
        assert!(verified.verify_contents(&contents).is_err(), "{name}");
    }
    let mut changed = f.contents();
    changed.configs.remove("resp3");
    assert!(verified.verify_contents(&changed).is_err());
    for surface in ["embedded", "direct", "resp2", "resp3"] {
        let mut f = Synthetic::new();
        f.statement.configs.get_mut(surface).unwrap().sha256 = "b".repeat(64);
        let (bytes, id) = f.receipt();
        assert!(verify_receipt(&bytes, &id, &f.trust()).is_err());
    }
}

#[test]
fn cargo_log_requires_one_unprofiled_binary_and_final_success() {
    let artifact = artifact_event();
    let end = final_event();
    let cases = [
        vec![artifact.clone()],
        vec![end.clone()],
        vec![artifact.clone(), artifact.clone(), end.clone()],
        vec![
            artifact.clone(),
            serde_json::json!({"reason":"build-finished","success":false}),
        ],
        vec![artifact.clone(), end.clone(), end.clone()],
        vec![artifact.clone(), end.clone(), artifact.clone()],
        vec![
            artifact.clone(),
            serde_json::json!({"reason":"compiler-message","message":{"level":"error"}}),
            end.clone(),
        ],
        vec![
            artifact.clone(),
            serde_json::json!({"reason":"unknown"}),
            end.clone(),
        ],
    ];
    for events in cases {
        let mut f = Synthetic::new();
        f.log = log(&events);
        f.statement.build_log = ArtifactDigest::of(&f.log);
        assert!(f.verified().verify_contents(&f.contents()).is_err());
    }
    for (path, value) in [
        ("features", serde_json::json!(["allocation-diagnostics"])),
        ("package_id", serde_json::json!("other")),
        ("executable", serde_json::json!("/tmp/other")),
        (
            "profile",
            serde_json::json!({"opt_level":"0","debug_assertions":true,"test":false}),
        ),
    ] {
        let mut changed = artifact.clone();
        changed[path] = value;
        let mut f = Synthetic::new();
        f.log = log(&[changed, end.clone()]);
        f.statement.build_log = ArtifactDigest::of(&f.log);
        assert!(f.verified().verify_contents(&f.contents()).is_err());
    }
    for bad in [
        b"not json\n".to_vec(),
        log(&[artifact.clone(), end.clone()])[..10].to_vec(),
        [log(&[artifact, end]), b"\n".to_vec()].concat(),
    ] {
        let mut f = Synthetic::new();
        f.log = bad;
        f.statement.build_log = ArtifactDigest::of(&f.log);
        assert!(f.verified().verify_contents(&f.contents()).is_err());
    }
}

#[test]
fn elf_checks_are_structural_only_and_refuse_truncated_or_wrong_architecture() {
    for (offset, value) in [
        (0, 0),
        (4, 1),
        (5, 2),
        (6, 2),
        (16, 1),
        (18, 3),
        (20, 2),
        (52, 0),
        (54, 0),
        (56, 0),
    ] {
        let mut f = Synthetic::new();
        f.binary[offset] = value;
        f.statement.binary = ArtifactDigest::of(&f.binary);
        assert!(f.verified().verify_contents(&f.contents()).is_err());
    }
    for len in [1, 63, 64, 119] {
        let mut f = Synthetic::new();
        f.binary.truncate(len);
        f.statement.binary = ArtifactDigest::of(&f.binary);
        assert!(f.verified().verify_contents(&f.contents()).is_err());
    }
    let mut f = Synthetic::new();
    f.binary[32..40].copy_from_slice(&u64::MAX.to_le_bytes());
    f.statement.binary = ArtifactDigest::of(&f.binary);
    assert!(f.verified().verify_contents(&f.contents()).is_err());
}

#[test]
fn checked_statement_binds_exact_model_intent_without_enrolling_execution() {
    let f = Synthetic::new();
    let verified = f.verified();
    verified.bind_intent(&f.intent()).unwrap();
    type Mutation = fn(&mut CellIntent);
    let changes: &[Mutation] = &[
        |i| i.lease_id = "b".repeat(64),
        |i| i.boot_id.push('x'),
        |i| i.surface = "resp3".into(),
        |i| i.unit_name.push('x'),
        |i| i.cgroup_path.push('x'),
        |i| i.binary_path = "/tmp/binary".into(),
        |i| i.binary_sha256 = "b".repeat(64),
        |i| i.config_path.push('x'),
        |i| i.config_sha256 = "b".repeat(64),
        |i| i.source_commit = "b".repeat(40),
        |i| i.maximum_runtime_seconds = 0,
        |i| i.maximum_runtime_seconds = 61,
    ];
    for change in changes {
        let mut intent = f.intent();
        change(&mut intent);
        assert!(verified.bind_intent(&intent).is_err());
    }
}

#[cfg(target_os = "linux")]
mod linux_tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::{symlink, PermissionsExt};
    use std::path::PathBuf;
    use tempfile::TempDir;

    struct Fixture {
        temporary: TempDir,
        root: PathBuf,
        f: Synthetic,
    }
    fn mode(path: &std::path::Path, mode: u32) {
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
    }
    fn ids() -> (u32, u32) {
        // SAFETY: identity getters have no pointer arguments or side effects.
        unsafe { (libc::getuid(), libc::getgid()) }
    }
    impl Fixture {
        fn new() -> Self {
            let temporary = TempDir::new().unwrap();
            let root = temporary.path().join("bundle");
            let f = Synthetic::new();
            fs::create_dir(&root).unwrap();
            for (name, bytes) in [
                ("timing-controls-074", f.binary.as_slice()),
                ("build-log.jsonl", f.log.as_slice()),
                ("Cargo.lock.root", ROOT_LOCK),
                ("Cargo.lock.observer", OBSERVER_LOCK),
            ] {
                fs::write(root.join(name), bytes).unwrap();
            }
            fs::write(root.join("build-receipt-v1.json"), f.receipt().0).unwrap();
            for (surface, bytes) in &f.configs {
                fs::write(root.join(format!("{surface}.json")), bytes).unwrap();
            }
            for entry in fs::read_dir(&root).unwrap() {
                mode(&entry.unwrap().path(), 0o444);
            }
            mode(&root.join("timing-controls-074"), 0o555);
            mode(&root, 0o555);
            Self { temporary, root, f }
        }
        fn inspect(&self) -> Result<linux::BundleSnapshot, ArtifactError> {
            let (uid, gid) = ids();
            linux::inspect_fixture(&self.root, uid, gid, &self.f.receipt().1, &self.f.trust())
        }
        fn writable(&self) {
            mode(&self.root, 0o755);
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            // Only this test's tempfile tree; restoring directory mode permits cleanup.
            if self.root.exists() {
                mode(&self.root, 0o755);
            }
            let old = self.temporary.path().join("old-bundle");
            if old.exists() {
                mode(&old, 0o755);
            }
        }
    }

    #[test]
    fn descriptor_bundle_checks_size_empty_and_special_mode_bounds() {
        for name in [
            "timing-controls-074",
            "build-log.jsonl",
            "embedded.json",
            "Cargo.lock.root",
            "build-receipt-v1.json",
        ] {
            let f = Fixture::new();
            let path = f.root.join(name);
            mode(&path, 0o644);
            let maximum = match name {
                "timing-controls-074" => MAX_BINARY_BYTES,
                "build-log.jsonl" => MAX_LOG_BYTES,
                "Cargo.lock.root" => MAX_LOCK_BYTES,
                _ => MAX_CONFIG_BYTES,
            };
            fs::OpenOptions::new()
                .write(true)
                .open(&path)
                .unwrap()
                .set_len(maximum + 1)
                .unwrap();
            mode(
                &path,
                if name == "timing-controls-074" {
                    0o555
                } else {
                    0o444
                },
            );
            assert!(f.inspect().is_err());
        }
        let f = Fixture::new();
        let path = f.root.join("direct.json");
        mode(&path, 0o644);
        fs::write(&path, []).unwrap();
        mode(&path, 0o444);
        assert!(f.inspect().is_err());
        let f = Fixture::new();
        mode(&f.root.join("timing-controls-074"), 0o4555);
        assert!(f.inspect().is_err());
        let f = Fixture::new();
        mode(&f.root.join("embedded.json"), 0o555);
        assert!(f.inspect().is_err());
    }

    #[test]
    fn independent_parallel_inspections_keep_their_own_descriptors() {
        let f = Fixture::new();
        std::thread::scope(|scope| {
            let handles: Vec<_> = (0..4)
                .map(|_| {
                    scope.spawn(|| {
                        let mut snapshot = f.inspect().unwrap();
                        snapshot.revalidate().unwrap();
                    })
                })
                .collect();
            for handle in handles {
                handle.join().unwrap();
            }
        });
    }

    #[test]
    fn descriptor_bundle_refuses_links_writes_owner_drift_and_extra_files() {
        let f = Fixture::new();
        let mut snapshot = f.inspect().unwrap();
        assert!(snapshot.is_fixture());
        snapshot.revalidate().unwrap();
        snapshot.build().bind_intent(&f.f.intent()).unwrap();
        let (uid, gid) = ids();
        assert!(linux::inspect_fixture(
            &f.root,
            uid.wrapping_add(1),
            gid,
            &f.f.receipt().1,
            &f.f.trust()
        )
        .is_err());
        assert!(linux::inspect_fixture(
            &f.root,
            uid,
            gid.wrapping_add(1),
            &f.f.receipt().1,
            &f.f.trust()
        )
        .is_err());
        for name in [
            "timing-controls-074",
            "embedded.json",
            "build-receipt-v1.json",
            "Cargo.lock.root",
        ] {
            let f = Fixture::new();
            mode(&f.root.join(name), 0o644);
            assert!(f.inspect().is_err());
        }
        let f = Fixture::new();
        f.writable();
        assert!(f.inspect().is_err());
        let f = Fixture::new();
        f.writable();
        fs::write(f.root.join("extra"), b"x").unwrap();
        mode(&f.root, 0o555);
        assert!(f.inspect().is_err());
        let f = Fixture::new();
        f.writable();
        fs::hard_link(
            f.root.join("embedded.json"),
            f.temporary.path().join("linked"),
        )
        .unwrap();
        mode(&f.root, 0o555);
        assert!(f.inspect().is_err());
        let f = Fixture::new();
        f.writable();
        let path = f.root.join("embedded.json");
        fs::remove_file(&path).unwrap();
        symlink("direct.json", path).unwrap();
        mode(&f.root, 0o555);
        assert!(f.inspect().is_err());
    }

    #[test]
    fn descriptor_bundle_refuses_missing_corrupt_special_and_symlinked_root() {
        let f = Fixture::new();
        f.writable();
        fs::remove_file(f.root.join("resp3.json")).unwrap();
        mode(&f.root, 0o555);
        assert!(f.inspect().is_err());
        let f = Fixture::new();
        let path = f.root.join("embedded.json");
        mode(&path, 0o644);
        fs::write(&path, b"wrong").unwrap();
        mode(&path, 0o444);
        assert!(f.inspect().is_err());
        let f = Fixture::new();
        let alias = f.temporary.path().join("alias");
        symlink(&f.root, &alias).unwrap();
        let (uid, gid) = ids();
        assert!(linux::inspect_fixture(&alias, uid, gid, &f.f.receipt().1, &f.f.trust()).is_err());
        let f = Fixture::new();
        f.writable();
        let path = f.root.join("resp3.json");
        fs::remove_file(&path).unwrap();
        use std::os::unix::ffi::OsStrExt;
        let name = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
        // SAFETY: valid NUL-terminated path in this test's private temporary tree.
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o444) }, 0);
        mode(&f.root, 0o555);
        assert!(f.inspect().is_err()); // O_NONBLOCK prevents FIFO hang.
    }

    #[test]
    fn pinned_descriptor_revalidation_detects_path_replacement_and_in_place_changes() {
        let f = Fixture::new();
        let mut snapshot = f.inspect().unwrap();
        f.writable();
        let path = f.root.join("embedded.json");
        fs::rename(&path, f.temporary.path().join("old-config")).unwrap();
        fs::write(&path, f.f.configs["embedded"]).unwrap();
        mode(&path, 0o444);
        mode(&f.root, 0o555);
        assert!(snapshot.revalidate().is_err());
        let f = Fixture::new();
        let mut snapshot = f.inspect().unwrap();
        let path = f.root.join("embedded.json");
        mode(&path, 0o644);
        let mut bytes = f.f.configs["embedded"].to_vec();
        bytes[0] ^= 1;
        fs::write(&path, bytes).unwrap();
        mode(&path, 0o444);
        assert!(snapshot.revalidate().is_err());
        let f = Fixture::new();
        let mut snapshot = f.inspect().unwrap();
        fs::rename(&f.root, f.temporary.path().join("old-bundle")).unwrap();
        fs::create_dir(&f.root).unwrap();
        mode(&f.root, 0o555);
        assert!(snapshot.revalidate().is_err());
    }

    #[test]
    fn revalidation_refuses_mode_and_entry_drift_after_success() {
        let f = Fixture::new();
        let mut snapshot = f.inspect().unwrap();
        mode(&f.root.join("timing-controls-074"), 0o755);
        assert!(snapshot.revalidate().is_err());
        let f = Fixture::new();
        let mut snapshot = f.inspect().unwrap();
        f.writable();
        fs::write(f.root.join("extra"), b"x").unwrap();
        mode(&f.root, 0o555);
        assert!(snapshot.revalidate().is_err());
    }
}
