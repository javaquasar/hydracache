//! Structural fixtures exercise receipt validation only. The actual-process test below
//! creates a separate clean local fixture and never emits production/release evidence.
use hydracache_cluster_testkit::value_plane_extended_075::{
    ExtendedCorpus, ExtendedStep, MAX_OUTPUT,
};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use tempfile::TempDir;
use xtask::imap_extended_receipt_075::*;
use xtask::imap_proof_process_075::{execute, ProcessBounds};

const FAULTS: [&str; 8] = [
    "java_drop_event",
    "java_wrong_ttl",
    "java_reverse_bulk",
    "java_reuse_token",
    "java_wrong_owners",
    "java_wrong_clock",
    "java_wrong_cut",
    "rust_drop_event",
];
fn row(number: u64) -> Value {
    json!({"result":{"kind":"counted","value":null,"ttl":{"kind":"absent","remaining_ticks":0},"items":[],"statuses":[],"number":number},"snapshot":[],"events":[],"token_relations":{},"owners":{"outcomes":0,"pending":0,"listeners":0,"snapshot_payload":false},"tick":0})
}
fn corpus() -> ExtendedCorpus {
    ExtendedCorpus {
        seed: 117,
        steps: (0..2)
            .map(|index| ExtendedStep {
                id: format!("fixture-{index}"),
                operation: "resources".into(),
                entries: Vec::new(),
                requests: Vec::new(),
                limits: None,
                ticks: 0,
                lose_response: false,
            })
            .collect(),
    }
}
fn write(root: &Path, relative: &str, bytes: &[u8]) -> Artifact {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, bytes).unwrap();
    artifact(root, &path, MAX_OUTPUT).unwrap()
}
fn stream(
    root: &Path,
    relative: &str,
    corpus: &ExtendedCorpus,
    expected: bool,
    number: u64,
) -> Stream {
    let observations = vec![row(number); corpus.steps.len()];
    let mut source = format!(
        "imap-extended-observations-v2\t{}\n",
        if expected { "expected" } else { "actual" }
    );
    for (step, observation) in corpus.steps.iter().zip(&observations) {
        source.push_str(
            &hydracache_cluster_testkit::value_plane_differential_075::hex(step.id.as_bytes()),
        );
        source.push('\t');
        source.push_str(&serde_json::to_string(observation).unwrap());
        source.push('\n');
    }
    Stream {
        artifact: write(root, relative, source.as_bytes()),
        normalized_sha256: normalized(&observations).unwrap(),
    }
}
struct Fixture {
    directory: TempDir,
    receipt: ExtendedReceipt,
}
impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let corpus = corpus();
        let corpus_ref = write(
            root,
            "proof/corpus.tsv",
            corpus.encode().unwrap().as_bytes(),
        );
        let expected = stream(root, "proof/expected.tsv", &corpus, true, 0);
        let java_actual = stream(root, "proof/java.tsv", &corpus, false, 0);
        let rust_actual = stream(root, "proof/rust.tsv", &corpus, false, 0);
        let canaries = FAULTS
            .iter()
            .map(|id| Canary {
                id: (*id).into(),
                observed: stream(root, &format!("proof/{id}.tsv"), &corpus, false, 1),
            })
            .collect();
        let shrink = ShrinkProof {
            maximum_replays: 16,
            maximum_work: 4096,
            replays: 1,
            work: 2,
            budget_exhausted: false,
            corpus: write(
                root,
                "proof/minimized.tsv",
                corpus.encode().unwrap().as_bytes(),
            ),
            expected: stream(root, "proof/minimized-expected.tsv", &corpus, true, 0),
            broken_rust: stream(root, "proof/minimized-rust.tsv", &corpus, false, 1),
        };
        let rust_peer = write(
            root,
            "target/debug/structural-rust-peer",
            b"structural fixture: not an executable",
        );
        let java_classes = vec![write(
            root,
            "tests/java-imap-benchmark/target/classes/Structural.class",
            b"structural fixture: not a class",
        )];
        let runtime = RuntimeBinding {
            rust_fingerprint: "rustc structural fixture".into(),
            java_fingerprint: "java structural fixture".into(),
            java_launcher_sha256: "a".repeat(64),
            process_scope: if cfg!(windows) {
                "windows_job_kill_on_close_process_commit_limit"
            } else {
                "linux_owned_session_nested_groups_per_process_address_space_limit"
            }
            .into(),
            process_timeout_ms: 15_000,
            process_memory_bytes: if cfg!(windows) {
                512 * 1024 * 1024
            } else {
                8 * 1024 * 1024 * 1024
            },
            process_count_limit: if cfg!(windows) { Some(8) } else { None },
        };
        let receipt = ExtendedReceipt {
            schema: SCHEMA.into(),
            release: "0.75".into(),
            scope: "local_actual_owner_qualification".into(),
            source_sha: "a".repeat(40),
            corpus_format: "imap-extended-semantic-v2".into(),
            observation_format: "imap-extended-observations-v2".into(),
            seed: 117,
            steps: 2,
            corpus: corpus_ref,
            rust_peer,
            java_classpath: "tests/java-imap-benchmark/target/classes".into(),
            java_classes,
            runtime,
            expected,
            java_actual,
            rust_actual,
            canaries,
            shrink,
            passed: true,
            production_enabled: false,
        };
        Self { directory, receipt }
    }
    fn root(&self) -> &Path {
        self.directory.path()
    }
}

#[test]
fn structural_receipt_accepts_closed_exact_artifacts_and_roundtrips_without_a_production_claim() {
    let fixture = Fixture::new();
    validate(fixture.root(), &fixture.receipt).unwrap();
    let encoded = serde_json::to_vec(&fixture.receipt).unwrap();
    let decoded: ExtendedReceipt = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded, fixture.receipt);
    assert!(!decoded.production_enabled);
}

#[test]
fn versions_claims_missing_controls_and_unknown_json_fields_fail_closed() {
    let fixture = Fixture::new();
    for field in [
        "schema",
        "release",
        "scope",
        "source_sha",
        "corpus_format",
        "observation_format",
        "steps",
        "passed",
        "production_enabled",
    ] {
        let mut value = serde_json::to_value(&fixture.receipt).unwrap();
        value[field] = match field {
            "steps" => json!(0),
            "passed" => json!(false),
            "production_enabled" => json!(true),
            _ => json!("wrong"),
        };
        let changed: ExtendedReceipt = serde_json::from_value(value).unwrap();
        assert!(validate(fixture.root(), &changed).is_err(), "{field}");
    }
    for field in ["unexpected", "runtime", "corpus", "canaries", "shrink"] {
        let mut value = serde_json::to_value(&fixture.receipt).unwrap();
        match field {
            "unexpected" => value["unexpected"] = json!(true),
            "canaries" => value["canaries"][0]["unexpected"] = json!(true),
            _ => value[field]["unexpected"] = json!(true),
        }
        assert!(
            serde_json::from_value::<ExtendedReceipt>(value).is_err(),
            "{field}"
        );
    }
    let mut missing = fixture.receipt.clone();
    missing.canaries.pop();
    assert!(validate(fixture.root(), &missing).is_err());
    let mut duplicate = fixture.receipt.clone();
    duplicate.canaries[1].id = duplicate.canaries[0].id.clone();
    assert!(validate(fixture.root(), &duplicate).is_err());
    let mut unknown = fixture.receipt.clone();
    unknown.canaries[0].id = "future_fault".into();
    assert!(validate(fixture.root(), &unknown).is_err());
}

#[test]
fn artifact_mutation_path_escape_aliasing_and_missing_frames_are_rejected() {
    let fixture = Fixture::new();
    let mut escape = fixture.receipt.clone();
    escape.corpus.path = "../outside.tsv".into();
    assert!(validate(fixture.root(), &escape).is_err());
    for relative in [
        "/outside.tsv",
        "target/../outside.tsv",
        "target\\outside.tsv",
        "C:/outside.tsv",
        "target//outside.tsv",
    ] {
        let mut escaped = fixture.receipt.clone();
        escaped.corpus.path = relative.into();
        assert!(validate(fixture.root(), &escaped).is_err(), "{relative}");
    }
    let mut aliased = fixture.receipt.clone();
    aliased.rust_actual = aliased.java_actual.clone();
    assert!(validate(fixture.root(), &aliased).is_err());
    let mut missing = fixture.receipt.clone();
    missing.java_actual.artifact = write(
        fixture.root(),
        "proof/no-frames.tsv",
        b"imap-extended-observations-v2\tactual\n",
    );
    assert!(validate(fixture.root(), &missing).is_err());
    let mut seed = fixture.receipt.clone();
    seed.seed += 1;
    assert!(validate(fixture.root(), &seed).is_err());
    let mut swapped = fixture.receipt.clone();
    swapped.java_actual = stream(
        fixture.root(),
        "proof/wrong-observation.tsv",
        &corpus(),
        false,
        2,
    );
    assert!(validate(fixture.root(), &swapped).is_err());
    fs::write(
        fixture.root().join(&fixture.receipt.rust_peer.path),
        b"mutated foreign executable",
    )
    .unwrap();
    assert!(validate(fixture.root(), &fixture.receipt).is_err());
}

#[test]
fn shrink_identity_budget_and_runtime_capabilities_are_checked() {
    let fixture = Fixture::new();
    for field in ["maximum_replays", "maximum_work", "replays", "work"] {
        let mut value = serde_json::to_value(&fixture.receipt).unwrap();
        value["shrink"][field] = json!(0);
        let changed: ExtendedReceipt = serde_json::from_value(value).unwrap();
        assert!(validate(fixture.root(), &changed).is_err(), "{field}");
    }
    let mut replay = fixture.receipt.clone();
    replay.shrink.replays = 17;
    assert!(validate(fixture.root(), &replay).is_err());
    let mut no_defect = fixture.receipt.clone();
    no_defect.shrink.broken_rust =
        stream(fixture.root(), "proof/not-broken.tsv", &corpus(), false, 0);
    assert!(validate(fixture.root(), &no_defect).is_err());
    let mut changed = fixture.receipt.clone();
    let mut foreign = corpus();
    foreign.steps[0].id = "foreign-step".into();
    changed.shrink.corpus = write(
        fixture.root(),
        "proof/foreign-corpus.tsv",
        foreign.encode().unwrap().as_bytes(),
    );
    changed.shrink.expected = stream(
        fixture.root(),
        "proof/foreign-expected.tsv",
        &foreign,
        true,
        0,
    );
    changed.shrink.broken_rust =
        stream(fixture.root(), "proof/foreign-rust.tsv", &foreign, false, 1);
    assert!(validate(fixture.root(), &changed).is_err());
    for field in [
        "rust_fingerprint",
        "java_fingerprint",
        "java_launcher_sha256",
        "process_scope",
        "process_timeout_ms",
        "process_memory_bytes",
    ] {
        let mut value = serde_json::to_value(&fixture.receipt).unwrap();
        value["runtime"][field] = if field.ends_with("_ms") || field.ends_with("_bytes") {
            json!(0)
        } else {
            json!("")
        };
        let changed: ExtendedReceipt = serde_json::from_value(value).unwrap();
        assert!(validate(fixture.root(), &changed).is_err(), "{field}");
    }
    let mut caps = fixture.receipt.clone();
    caps.runtime.process_scope =
        "linux_owned_session_nested_groups_per_process_address_space_limit".into();
    caps.runtime.process_count_limit = Some(8);
    assert!(validate(fixture.root(), &caps).is_err());
    caps.runtime.process_scope = "windows_job_kill_on_close_process_commit_limit".into();
    caps.runtime.process_count_limit = None;
    assert!(validate(fixture.root(), &caps).is_err());
    caps.runtime.process_count_limit = Some(33);
    assert!(validate(fixture.root(), &caps).is_err());
}

#[test]
fn normalization_has_stable_key_order_and_strict_depth_item_and_byte_bounds() {
    let left: Value = serde_json::from_str(r#"{"z":[1,"é"],"a":{"second":2,"first":1}}"#).unwrap();
    let right: Value =
        serde_json::from_str(r#"{"a":{"first":1,"second":2},"z":[1,"\u00e9"]}"#).unwrap();
    assert_eq!(normalized(&[left]).unwrap(), normalized(&[right]).unwrap());
    assert!(normalized(&vec![Value::Null; 257]).is_err());
    assert!(normalized(&[Value::Array(vec![Value::Null; 257])]).is_err());
    let mut deep = Value::Null;
    for _ in 0..14 {
        deep = Value::Array(vec![deep]);
    }
    assert!(normalized(&[deep]).is_err());
    let object: serde_json::Map<String, Value> = (0..4097)
        .map(|index| (index.to_string(), Value::Null))
        .collect();
    assert!(normalized(&[Value::Object(object)]).is_err());
    assert!(normalized(&[Value::String("x".repeat(MAX_OUTPUT))]).is_err());
}

#[test]
fn serialized_receipt_budget_is_checked_before_reading_a_large_class_manifest() {
    let fixture = Fixture::new();
    let mut receipt = fixture.receipt.clone();
    let original = receipt.java_classes[0].clone();
    receipt.java_classes = (0..512)
        .map(|index| {
            let mut artifact = original.clone();
            artifact.path = format!(
                "{}/{}/{index:04}{}.class",
                receipt.java_classpath,
                "x".repeat(180),
                "y".repeat(180)
            );
            assert!(artifact.path.len() <= 512);
            artifact
        })
        .collect();
    assert!(serde_json::to_vec_pretty(&receipt).unwrap().len() > 256 * 1024);
    let error = validate(fixture.root(), &receipt).unwrap_err();
    assert!(error.to_string().contains("receipt byte bound"), "{error}");
}

fn actual_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .into()
}
fn actual_peer(root: &Path) -> PathBuf {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target"));
    let target = if target.is_absolute() {
        target
    } else {
        root.join(target)
    };
    let name = if cfg!(windows) {
        "imap-extended-owner-075.exe"
    } else {
        "imap-extended-owner-075"
    };
    let selected = target.join("debug").join(name);
    if selected.is_file() {
        selected
    } else {
        root.join("target/debug").join(name)
    }
}
fn copy_classes(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let kind = entry.file_type().unwrap();
        assert!(!kind.is_symlink());
        if kind.is_dir() {
            copy_classes(&entry.path(), &to.join(entry.file_name()));
        } else if entry
            .path()
            .extension()
            .is_some_and(|value| value == "class")
        {
            fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
        }
    }
}
fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env_remove("GIT_TEMPLATE_DIR")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "local fixture git failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
fn cli(root: &Path, args: &[&str]) -> Result<(), String> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_xtask"));
    command
        .args(args)
        .current_dir(root)
        .env("CARGO_MANIFEST_DIR", root)
        .env("CARGO_TARGET_DIR", root.join("target"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        );
    execute(
        command,
        &[],
        ProcessBounds {
            timeout: Duration::from_secs(60),
            stdout_bytes: 64 * 1024,
            stderr_bytes: 64 * 1024,
            ..Default::default()
        },
    )
    .map(|_| ())
    .map_err(|error| error.to_string())
}

#[test]
fn actual_local_fixture_produces_replays_and_rejects_changed_source_binary_and_artifacts() {
    let source = actual_root();
    let peer = actual_peer(&source);
    assert!(
        peer.is_file(),
        "actual Rust owner prerequisite missing: {}",
        peer.display()
    );
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path();
    fs::create_dir_all(root.join("docs/plans")).unwrap();
    fs::write(
        root.join(".gitignore"),
        "target/\ntests/java-imap-benchmark/target/\n",
    )
    .unwrap();
    fs::write(
        root.join("README.md"),
        "Isolated local receipt fixture only. This is not release or production evidence.\n",
    )
    .unwrap();
    fs::write(
        root.join("docs/plans/releases.toml"),
        "# Local fixture root marker, not a release manifest.\n",
    )
    .unwrap();
    git(root, &["-c", "init.defaultBranch=main", "init", "--quiet"]);
    git(
        root,
        &[
            "add",
            "--",
            ".gitignore",
            "README.md",
            "docs/plans/releases.toml",
        ],
    );
    git(
        root,
        &[
            "-c",
            "user.name=IMap Local Fixture",
            "-c",
            "user.email=imap-fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--quiet",
            "-m",
            "local fixture",
        ],
    );
    let target = root.join("target/debug");
    fs::create_dir_all(&target).unwrap();
    let bound_peer = target.join(peer.file_name().unwrap());
    fs::copy(&peer, &bound_peer).unwrap();
    copy_classes(
        &source.join("tests/java-imap-benchmark/target/classes"),
        &root.join("tests/java-imap-benchmark/target/classes"),
    );
    let output = root.join("target/evidence");
    let output = output.to_str().unwrap();
    cli(
        root,
        &[
            "imap-extended-evidence",
            "--release",
            "0.75",
            "--seed",
            "117",
            "--output",
            output,
        ],
    )
    .unwrap();
    let path = root.join("target/evidence/receipt.json");
    let path_text = path.to_str().unwrap();
    let bytes = fs::read(&path).unwrap();
    let receipt: ExtendedReceipt = serde_json::from_slice(&bytes).unwrap();
    assert!(!receipt.production_enabled);
    assert_eq!(receipt.scope, "local_actual_owner_qualification");
    assert_eq!(receipt.canaries.len(), 8);
    cli(
        root,
        &[
            "imap-extended-evidence-check",
            "--release",
            "0.75",
            "--receipt",
            path_text,
        ],
    )
    .unwrap();
    fs::write(root.join("README.md"), "changed tracked fixture source\n").unwrap();
    assert!(cli(
        root,
        &[
            "imap-extended-evidence-check",
            "--release",
            "0.75",
            "--receipt",
            path_text
        ]
    )
    .is_err());
    git(root, &["checkout", "--", "README.md"]);
    let mut different_runtime = receipt.clone();
    different_runtime.runtime.java_launcher_sha256 =
        if receipt.runtime.java_launcher_sha256 == "a".repeat(64) {
            "b".repeat(64)
        } else {
            "a".repeat(64)
        };
    validate(root, &different_runtime).unwrap();
    fs::write(&path, serde_json::to_vec(&different_runtime).unwrap()).unwrap();
    let mismatch = cli(
        root,
        &[
            "imap-extended-evidence-check",
            "--release",
            "0.75",
            "--receipt",
            path_text,
        ],
    )
    .unwrap_err();
    assert!(mismatch.contains("runtime"), "{mismatch}");
    fs::write(&path, &bytes).unwrap();
    let mut foreign = receipt.clone();
    foreign.rust_peer = write(root, "target/foreign-peer", b"different bound executable");
    fs::write(&path, serde_json::to_vec(&foreign).unwrap()).unwrap();
    assert!(cli(
        root,
        &[
            "imap-extended-evidence-check",
            "--release",
            "0.75",
            "--receipt",
            path_text
        ]
    )
    .is_err());
    fs::write(&path, &bytes).unwrap();
    fs::write(
        root.join(&receipt.canaries[0].observed.artifact.path),
        b"changed retained canary artifact",
    )
    .unwrap();
    assert!(cli(
        root,
        &[
            "imap-extended-evidence-check",
            "--release",
            "0.75",
            "--receipt",
            path_text
        ]
    )
    .is_err());
}
