//! Frozen legacy differential reference plus the narrowly integrated caller.
#[path = "support/census_provider_input_legacy.rs"]
mod caller;
#[path = "../../../tools/historical-blob-batch-074/mod.rs"]
pub mod census_runtime;
#[path = "support/census_provider_input.rs"]
mod integrated;
use census_runtime::{census_caller as census, decoder, transport};
use serde_json::{json, Value};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use transport::{Output, Refusal, Replacements};

fn bytes(raw: &str) -> Vec<u8> {
    assert_eq!(raw.len() % 2, 0);
    (0..raw.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&raw[i..i + 2], 16).unwrap())
        .collect()
}
fn pin(value: &Value) -> decoder::Expected {
    decoder::Expected::Object {
        oid: value["oid"].as_str().unwrap().to_owned(),
        kind: match value["kind"].as_str().unwrap() {
            "blob" => decoder::Kind::Blob,
            "tree" => decoder::Kind::Tree,
            "commit" => decoder::Kind::Commit,
            "tag" => decoder::Kind::Tag,
            _ => panic!("kind"),
        },
        bytes: value["bytes"].as_u64().unwrap().try_into().unwrap(),
        sha256: bytes(value["sha256"].as_str().unwrap()).try_into().unwrap(),
    }
}
fn retain(folder: &Option<PathBuf>, name: &str, raw: &[u8]) {
    if let Some(folder) = folder {
        fs::File::create_new(folder.join(name))
            .unwrap()
            .write_all(raw)
            .unwrap();
    }
}

#[test]
fn real_census_caller_shadow_preserves_the_logging_counterexample_before_integration() {
    let evidence = std::env::var_os("HC074_BLOB_TRANSPORT_EVIDENCE").map(|path| {
        let path = PathBuf::from(path);
        let path = path.with_file_name(format!(
            "{}-caller",
            path.file_name().unwrap().to_str().unwrap()
        ));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::create_dir(&path).expect("first caller attempt already exists");
        path
    });
    retain(
        &evidence,
        "intent.json",
        br#"{"scope":"frozen-legacy-batch-canary","caller_integrated":false,"server":false,"measurement":false}"#,
    );
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut producer = Command::new(if cfg!(windows) { "python" } else { "python3" });
    producer.arg(root.join("tools/historical-blob-batch-074/caller_pins.py"));
    let produced = transport::run_owned(&mut producer, &[], Instant::now() + transport::BOUND);
    retain(
        &evidence,
        "producer-process.txt",
        format!(
            "{:?}",
            produced
                .as_ref()
                .map(|v| (v.code, v.stdout.len(), v.stderr.len()))
        )
        .as_bytes(),
    );
    let produced = produced.unwrap();
    retain(&evidence, "producer.stdout.json", &produced.stdout);
    retain(&evidence, "producer.stderr.log", &produced.stderr);
    assert_eq!(produced.code, Some(0));
    assert!(produced.stderr.is_empty());
    let document: Value = serde_json::from_slice(&produced.stdout).unwrap();
    assert_eq!(document["test_fixture_pin_integrity"], true);
    assert_eq!(document["production_pin_authority"], false);
    assert_eq!(document["cases"].as_array().unwrap().len(), 2);
    let temporary = tempfile::tempdir().unwrap();
    let input = temporary.path().join("independent-pins.json");
    fs::write(&input, &produced.stdout).unwrap();
    for mode in ["clean", "trace"] {
        let mut child = Command::new(std::env::current_exe().unwrap());
        child
            .args([
                "--exact",
                "caller_child_fixture",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("HC074_CENSUS_CALLER_CHILD", mode)
            .env("HC074_CENSUS_CALLER_PINS", &input)
            .env_remove("GIT_TRACE");
        if mode == "trace" {
            child.env("GIT_TRACE", "1");
        }
        let result = transport::run_owned(&mut child, &[], Instant::now() + transport::BOUND);
        retain(
            &evidence,
            &format!("{mode}-process.txt"),
            format!(
                "{:?}",
                result
                    .as_ref()
                    .map(|v| (v.code, v.stdout.len(), v.stderr.len()))
            )
            .as_bytes(),
        );
        let result = result.unwrap();
        retain(&evidence, &format!("{mode}.stdout.log"), &result.stdout);
        retain(&evidence, &format!("{mode}.stderr.log"), &result.stderr);
        assert_eq!(result.code, Some(0), "first caller {mode} outcome");
        assert!(result.stderr.is_empty());
    }
    retain(&evidence,"result.json",br#"{"scope":"frozen-legacy-batch-canary","fixture_pin_integrity":true,"clean_shadow_match":true,"trace_counterexample":true,"caller_admitted":false,"caller_integrated":false}"#);
}

#[test]
fn caller_child_fixture() {
    let Some(mode) = std::env::var_os("HC074_CENSUS_CALLER_CHILD") else {
        return;
    };
    let mode = mode.to_str().unwrap();
    assert!(matches!(mode, "clean" | "trace"));
    let path = std::env::var_os("HC074_CENSUS_CALLER_PINS").unwrap();
    let document: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let cases = document["cases"].as_array().unwrap();
    let source = |index: usize| {
        String::from_utf8(bytes(cases[index]["stdout_hex"].as_str().unwrap()))
            .unwrap()
            .replace("\r\n", "\n")
    };
    // Invoke the exact archived legacy caller, including its conditional view/helper
    // checks. Git tracing is child-local and its stderr is ignored by that caller.
    let original = caller::original();
    let accepted = caller::accepts(&original, &source(1));
    let view = caller::view("diagnostic_artifacts_linux.rs", &source(1));
    let unrelated = caller::view("unrelated.rs", "untouched");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut native = vec![];
    for value in cases {
        let result = transport::read_pinned(
            &root,
            &[value["spec"].as_str().unwrap().to_owned()],
            &[pin(&value["pin"])],
            &[None],
            Replacements::Disabled,
            Instant::now() + transport::BOUND,
        );
        native.push(match result {
            Ok(values)=>json!({"exact":values.len()==1 && values[0].stdout==bytes(value["stdout_hex"].as_str().unwrap()) && values[0].stderr.is_empty() && values[0].code==Some(0)}),
            Err(error)=>json!({"refusal":format!("{error:?}")}),
        });
    }
    println!(
        "{}",
        json!({"mode":mode,"old_original_exact":original==source(0),"old_accepts":accepted,
        "old_view_exact":view==original,"old_unrelated_exact":unrelated=="untouched","native":native,
        "caller_admitted":false,"caller_integrated":false})
    );
    assert_eq!(original, source(0));
    assert!(accepted);
    assert_eq!(view, original);
    assert_eq!(unrelated, "untouched");
    for value in native {
        if mode == "clean" {
            assert_eq!(value["exact"], true);
        } else {
            assert_eq!(value["refusal"], "Process");
        }
    }
}

#[test]
fn shadow_cannot_use_an_unbound_missing_reference() {
    let spec = "abcd:absent".to_owned();
    let original = decoder::Expected::ShortMissing { spec: spec.clone() };
    let reference = Output {
        stdout: vec![],
        stderr: vec![],
        code: Some(128),
    };
    let _binding =
        transport::FallbackPin::from_reference(&spec, &original, Replacements::Default, &reference)
            .unwrap();
    let full = decoder::Expected::Missing {
        spec: format!("{}:absent", "a".repeat(40)),
    };
    assert_eq!(
        transport::read(
            Path::new("nonexistent-caller-preflight"),
            &[format!("{}:absent", "a".repeat(40))],
            &[full],
            Replacements::Disabled,
            Instant::now() + transport::BOUND
        ),
        Err(Refusal::FallbackRequired)
    );
}

#[test]
fn static_census_reference_keeps_exact_revision_and_path_allowlist() {
    use census_runtime::census_reference as references;
    assert_eq!(references::BASE, integrated::contract::BASE);
    assert_eq!(references::SOURCE, integrated::contract::SOURCE);
    for revision in [references::BASE, references::REVIEWED] {
        assert!(references::reference(revision, references::SOURCE).is_ok());
    }
    for (revision, path) in [
        ("HEAD", references::SOURCE),
        ("25faa7bb", references::SOURCE),
        (references::BASE, "other.rs"),
        (references::REVIEWED, "../source.rs"),
    ] {
        let failure = references::reference(revision, path).unwrap_err();
        assert_eq!(failure.reason, census::Reason::Reference);
        assert!(failure.output.is_none());
    }
}

fn output(code: Option<i32>, stdout: &[u8], stderr: &[u8]) -> Output {
    Output {
        code,
        stdout: stdout.to_vec(),
        stderr: stderr.to_vec(),
    }
}

fn panic_text(value: &(dyn std::any::Any + Send)) -> String {
    value
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| value.downcast_ref::<&str>().copied())
        .expect("string panic payload")
        .to_owned()
}
fn panic_kind(value: &(dyn std::any::Any + Send)) -> &'static str {
    if value.is::<String>() {
        "String"
    } else if value.is::<&str>() {
        "str"
    } else {
        "other"
    }
}
fn caught(call: impl FnOnce() -> String) -> Result<String, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(call))
        .map_err(|value| panic_text(value.as_ref()))
}

// Independent model of the old caller's unwrap boundary, including payload.
fn old_unwrap<T, E: std::fmt::Debug>(value: Result<T, E>) -> T {
    value.unwrap()
}

#[test]
fn census_panic_facade_preserves_payloads_and_keeps_new_refusals_explicit() {
    let legacy = |value: Output| {
        assert_eq!(value.code, Some(0), "unused test-only status check");
        String::from_utf8(value.stdout)
            .unwrap()
            .replace("\r\n", "\n")
    };
    for value in [
        output(Some(0), b"x\r\ny\r", b"trace"),
        output(Some(0), &[0xff, 0, 0xfe], b"trace"),
    ] {
        let copy = output(value.code, &value.stdout, &value.stderr);
        assert_eq!(
            caught(|| census::panic_api::frozen(census::interpret(value))),
            caught(|| legacy(copy))
        );
    }
    for code in [Some(1), Some(128), None] {
        let expected = caught(|| panic!("missing exact material fixture source"));
        assert_eq!(
            caught(|| census::panic_api::frozen(census::interpret(output(
                code,
                &[0xff],
                b"error"
            )))),
            expected
        );
    }
    for code in [2, 13] {
        let failure = census::Failure {
            reason: census::Reason::Transport(Refusal::Io("not reconstructed".to_owned())),
            output: None,
            original_io: Some(std::io::Error::from_raw_os_error(code)),
        };
        let expected = caught(|| old_unwrap(Err(std::io::Error::from_raw_os_error(code))));
        assert_eq!(caught(|| census::panic_api::frozen(Err(failure))), expected);
    }
    for reason in [
        census::Reason::Reference,
        census::Reason::Content,
        census::Reason::Transport(Refusal::Deadline),
        census::Reason::Transport(Refusal::Budget),
        census::Reason::Transport(Refusal::Cleanup {
            primary: Some(Box::new(Refusal::Deadline)),
            reason: "owned".to_owned(),
        }),
    ] {
        let failure = census::Failure {
            reason,
            output: None,
            original_io: None,
        };
        assert!(caught(|| census::panic_api::frozen(Err(failure)))
            .unwrap_err()
            .starts_with("census adapter refusal:"));
    }
    // Actual spawn/current-dir failure: preserve the original IO cause, not its Display text.
    let root = Path::new("nonexistent-census-panic-root");
    let spec = format!("{}:source.rs", "a".repeat(40));
    let pin = decoder::Expected::Object {
        oid: "b".repeat(40),
        kind: decoder::Kind::Blob,
        bytes: 0,
        sha256: [0; 32],
    };
    let reference = census::Reference::new(&spec, &pin).unwrap();
    let old = caught(|| {
        Command::new("git")
            .arg("--no-replace-objects")
            .args(["show", &spec])
            .current_dir(root)
            .output()
            .unwrap();
        unreachable!()
    });
    let new = caught(|| {
        census::panic_api::frozen(census::read(
            root,
            &spec,
            &reference,
            Instant::now() + transport::BOUND,
        ))
    });
    assert_eq!(new, old);
}

struct PanicSchedule(Schedule);
impl census::Inputs for PanicSchedule {
    type Error = census::InputError;
    fn helper(&mut self) -> Result<String, Self::Error> {
        self.0
            .helper()
            .map_err(|e| census::InputError::Helper(std::io::Error::other(e)))
    }
    fn helper_matches(&mut self, raw: &str) -> bool {
        self.0.helper_matches(raw)
    }
    fn frozen(&mut self, revision: census::Revision) -> Result<String, Self::Error> {
        self.0.frozen(revision).map_err(|_| {
            census::InputError::Frozen(census::Failure {
                reason: census::Reason::Process,
                output: Some(output(Some(128), b"", b"failed")),
                original_io: None,
            })
        })
    }
    fn body_matches(&mut self, original: &str, raw: &str) -> bool {
        self.0.body_matches(original, raw)
    }
}

#[test]
fn census_panic_schedule_preserves_each_guard_message_and_short_circuit() {
    for (stop, expected, events) in [
        (
            "helper",
            caught(|| old_unwrap(Err(std::io::Error::other("helper")))).unwrap_err(),
            vec!["helper"],
        ),
        (
            "helper-check",
            "unreviewed fixture control".to_owned(),
            vec!["helper", "helper-check"],
        ),
        (
            "base",
            "missing exact material fixture source".to_owned(),
            vec!["helper", "helper-check", "base"],
        ),
        (
            "reviewed",
            "missing exact material fixture source".to_owned(),
            vec!["helper", "helper-check", "base", "reviewed"],
        ),
        (
            "body-check",
            "unreviewed provider body".to_owned(),
            vec!["helper", "helper-check", "base", "reviewed", "body-check"],
        ),
    ] {
        let mut source = PanicSchedule(Schedule {
            stop: Some(stop),
            ..Schedule::default()
        });
        assert_eq!(
            caught(|| census::panic_api::view(
                &mut source,
                "diagnostic_artifacts_linux.rs",
                "reviewed"
            )),
            Err(expected)
        );
        assert_eq!(source.0.events, events);
    }
    let mut source = PanicSchedule(Schedule::default());
    assert_eq!(
        census::panic_api::view(&mut source, "other.rs", "untouched"),
        "untouched"
    );
    assert!(source.0.events.is_empty());
    assert!(!census::panic_api::accepts(&mut source, "base", "drift"));
    assert_eq!(source.0.events, ["reviewed"]);
}

#[test]
fn census_error_contract_preserves_status_before_utf8_and_successful_stderr() {
    for code in [Some(1), Some(128), None] {
        let value = output(code, &[0xff], b"diagnostic\0");
        let failure = census::interpret(value).unwrap_err();
        assert_eq!(failure.reason, census::Reason::Process);
        assert_eq!(failure.output, Some(output(code, &[0xff], b"diagnostic\0")));
    }
    let failure = census::interpret(output(Some(0), &[0xff], b"trace")).unwrap_err();
    assert_eq!(failure.reason, census::Reason::Utf8);
    assert_eq!(failure.output.unwrap().stderr, b"trace");
    let loaded = census::interpret(output(Some(0), b"x\r\ny\r\0", &[0xff])).unwrap();
    assert_eq!(loaded.text, "x\ny\r\0");
    assert_eq!(loaded.output, output(Some(0), b"x\r\ny\r\0", &[0xff]));
    assert_eq!(
        census::interpret(output(Some(0), b"", b"trace"))
            .unwrap()
            .text,
        ""
    );
}

#[test]
fn census_reference_and_deadline_refuse_before_any_command() {
    use sha2::{Digest, Sha256};
    let spec = format!("{}:source.rs", "a".repeat(40));
    let good = decoder::Expected::Object {
        oid: "b".repeat(40),
        kind: decoder::Kind::Blob,
        bytes: 0,
        sha256: Sha256::digest([]).into(),
    };
    for invalid in [
        "abcd:source.rs".to_owned(),
        format!("{}:../source.rs", "a".repeat(40)),
        format!("{}:source\n.rs", "a".repeat(40)),
        format!("{}:x\\y", "a".repeat(40)),
        format!("{}:", "a".repeat(40)),
    ] {
        assert_eq!(
            census::Reference::new(&invalid, &good).unwrap_err().reason,
            census::Reason::Reference
        );
    }
    let missing = decoder::Expected::Missing { spec: spec.clone() };
    assert_eq!(
        census::Reference::new(&spec, &missing).unwrap_err().reason,
        census::Reason::Reference
    );
    for bad in [
        decoder::Expected::Object {
            oid: "b".repeat(40),
            kind: decoder::Kind::Tree,
            bytes: 0,
            sha256: [0; 32],
        },
        decoder::Expected::Object {
            oid: "bad".to_owned(),
            kind: decoder::Kind::Blob,
            bytes: 0,
            sha256: [0; 32],
        },
    ] {
        assert_eq!(
            census::Reference::new(&spec, &bad).unwrap_err().reason,
            census::Reason::Reference
        );
    }
    let reference = census::Reference::new(&spec, &good).unwrap();
    let root = Path::new("nonexistent-census-preflight");
    let failure = census::read(root, &spec, &reference, Instant::now()).unwrap_err();
    assert_eq!(failure.reason, census::Reason::Transport(Refusal::Deadline));
    assert!(failure.output.is_none());
    let swapped = format!("{}:source.rs", "c".repeat(40));
    let failure = census::read(
        root,
        &swapped,
        &reference,
        Instant::now() + transport::BOUND,
    )
    .unwrap_err();
    assert_eq!(failure.reason, census::Reason::Reference);
    assert!(failure.output.is_none());
    let failure =
        census::read(root, &spec, &reference, Instant::now() + transport::BOUND).unwrap_err();
    assert!(matches!(
        failure.reason,
        census::Reason::Transport(Refusal::Io(_))
    ));
    assert!(failure.output.is_none());
}

#[derive(Default)]
struct Schedule {
    events: Vec<&'static str>,
    stop: Option<&'static str>,
}
impl census::Inputs for Schedule {
    type Error = &'static str;
    fn helper(&mut self) -> Result<String, Self::Error> {
        self.events.push("helper");
        if self.stop == Some("helper") {
            Err("helper")
        } else {
            Ok("helper".to_owned())
        }
    }
    fn helper_matches(&mut self, _: &str) -> bool {
        self.events.push("helper-check");
        self.stop != Some("helper-check")
    }
    fn frozen(&mut self, revision: census::Revision) -> Result<String, Self::Error> {
        let name = match revision {
            census::Revision::Base => "base",
            census::Revision::Reviewed => "reviewed",
        };
        self.events.push(name);
        if self.stop == Some(name) {
            Err(name)
        } else {
            Ok(name.to_owned())
        }
    }
    fn body_matches(&mut self, _: &str, _: &str) -> bool {
        self.events.push("body-check");
        self.stop != Some("body-check")
    }
}

#[test]
fn census_schedule_keeps_each_conditional_read_and_first_failure() {
    let name = "diagnostic_artifacts_linux.rs";
    for (stop, expected, events) in [
        (
            None,
            Ok("base".to_owned()),
            vec!["helper", "helper-check", "base", "reviewed", "body-check"],
        ),
        (
            Some("helper"),
            Err(census::ViewError::Input("helper")),
            vec!["helper"],
        ),
        (
            Some("helper-check"),
            Err(census::ViewError::Helper),
            vec!["helper", "helper-check"],
        ),
        (
            Some("base"),
            Err(census::ViewError::Input("base")),
            vec!["helper", "helper-check", "base"],
        ),
        (
            Some("reviewed"),
            Err(census::ViewError::Input("reviewed")),
            vec!["helper", "helper-check", "base", "reviewed"],
        ),
        (
            Some("body-check"),
            Err(census::ViewError::Provider),
            vec!["helper", "helper-check", "base", "reviewed", "body-check"],
        ),
    ] {
        let mut source = Schedule {
            stop,
            ..Schedule::default()
        };
        assert_eq!(census::view(&mut source, name, "reviewed"), expected);
        assert_eq!(source.events, events);
    }
    let mut source = Schedule::default();
    assert_eq!(
        census::view(&mut source, "other.rs", "untouched"),
        Ok("untouched".to_owned())
    );
    assert!(source.events.is_empty());
    assert_eq!(
        census::view(&mut source, name, "drift"),
        Err(census::ViewError::Provider)
    );
    assert_eq!(
        source.events,
        ["helper", "helper-check", "base", "reviewed"]
    );
    source.events.clear();
    assert_eq!(census::accepts(&mut source, "base", "drift"), Ok(false));
    assert_eq!(source.events, ["reviewed"]);
}

struct RealInputs<'a> {
    root: &'a Path,
    cases: &'a [Value],
    deadline: Instant,
    events: Vec<String>,
    diagnostics: Vec<usize>,
    outputs: Vec<Value>,
}
impl census::Inputs for RealInputs<'_> {
    type Error = census::InputError;
    fn helper(&mut self) -> Result<String, Self::Error> {
        self.events.push("helper".to_owned());
        fs::read_to_string(self.root.join(caller::contract::CONTROL))
            .map_err(census::InputError::Helper)
    }
    fn helper_matches(&mut self, helper: &str) -> bool {
        self.events.push("helper-check".to_owned());
        caller::contract::helper_matches(helper)
    }
    fn frozen(&mut self, revision: census::Revision) -> Result<String, Self::Error> {
        let index = usize::from(revision == census::Revision::Reviewed);
        self.events.push(format!("{revision:?}"));
        let value = &self.cases[index];
        let spec = value["spec"].as_str().unwrap();
        let reference = census::Reference::new(spec, &pin(&value["pin"])).unwrap();
        match census::read(self.root, spec, &reference, self.deadline) {
            Ok(value) => {
                self.outputs.push(
                    json!({"code":value.output.code,"stderr_hex":hex_bytes(&value.output.stderr),
                    "stdout_bytes":value.output.stdout.len(),"pinned_stdout":true}),
                );
                self.diagnostics.push(value.output.stderr.len());
                Ok(value.text)
            }
            Err(value) => {
                self.outputs.push(json!({"reason":format!("{:?}",value.reason),"output":value.output.as_ref().map(|v|
                    json!({"code":v.code,"stdout_hex":hex_bytes(&v.stdout),"stderr_hex":hex_bytes(&v.stderr)}))}));
                Err(census::InputError::Frozen(value))
            }
        }
    }
    fn body_matches(&mut self, original: &str, raw: &str) -> bool {
        self.events.push("body-check".to_owned());
        caller::contract::accepts(original, raw)
    }
}

fn hex_bytes(raw: &[u8]) -> String {
    raw.iter().map(|b| format!("{b:02x}")).collect()
}

fn starts(trace: &Path) -> Vec<Vec<String>> {
    fs::read_to_string(trace)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let v: Value = serde_json::from_str(line).unwrap();
            (v["event"] == "start").then(|| {
                v["argv"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap().to_owned())
                    .collect()
            })
        })
        .collect()
}

#[test]
fn census_inherited_environment_and_error_compatibility_profiles() {
    let evidence = std::env::var_os("HC074_BLOB_TRANSPORT_EVIDENCE").map(|path| {
        let path = PathBuf::from(path);
        let folder = path.with_file_name(format!(
            "{}-census-contract",
            path.file_name().unwrap().to_str().unwrap()
        ));
        fs::create_dir_all(folder.parent().unwrap()).unwrap();
        fs::create_dir(&folder).expect("first census contract attempt already exists");
        folder
    });
    retain(
        &evidence,
        "intent.json",
        br#"{"caller_integrated":true,"batch_changed":false,"server":false,"retry":false}"#,
    );
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut producer = Command::new(if cfg!(windows) { "python" } else { "python3" });
    producer.arg(root.join("tools/historical-blob-batch-074/caller_pins.py"));
    let produced =
        transport::run_owned(&mut producer, &[], Instant::now() + transport::BOUND).unwrap();
    retain(&evidence, "producer.stdout.json", &produced.stdout);
    retain(&evidence, "producer.stderr.log", &produced.stderr);
    assert_eq!(produced.code, Some(0));
    assert!(produced.stderr.is_empty());
    let temporary = tempfile::tempdir().unwrap();
    let input = temporary.path().join("pins.json");
    fs::write(&input, &produced.stdout).unwrap();
    for mode in [
        "clean",
        "trace",
        "config",
        "config-error",
        "repository-error",
    ] {
        let trace = temporary.path().join(format!("{mode}.jsonl"));
        let mut child = Command::new(std::env::current_exe().unwrap());
        child
            .args([
                "--exact",
                "census_contract_child_fixture",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("HC074_CENSUS_CONTRACT_CHILD", mode)
            .env("HC074_CENSUS_CALLER_PINS", &input)
            .env("HC074_CENSUS_CONTRACT_TRACE", &trace);
        // Define child-only controls, never mutate this process's ambient state.
        for (key, _) in std::env::vars_os() {
            let name = key.to_string_lossy().to_ascii_uppercase();
            if name.starts_with("GIT_") {
                child.env_remove(key);
            }
        }
        child
            .env("GIT_TRACE2_EVENT", &trace)
            .env("GIT_NO_LAZY_FETCH", "1");
        match mode {
            "trace" => {
                child.env("GIT_TRACE", "1");
            }
            "config" => {
                child
                    .env("GIT_CONFIG_COUNT", "1")
                    .env("GIT_CONFIG_KEY_0", "core.quotePath")
                    .env("GIT_CONFIG_VALUE_0", "false");
            }
            "config-error" => {
                child.env("GIT_CONFIG_COUNT", "invalid");
            }
            "repository-error" => {
                child.env("GIT_DIR", temporary.path().join("absent.git"));
            }
            _ => {}
        }
        let result = transport::run_owned(&mut child, &[], Instant::now() + transport::BOUND);
        retain(
            &evidence,
            &format!("{mode}-process.txt"),
            format!(
                "{:?}",
                result
                    .as_ref()
                    .map(|v| (v.code, v.stdout.len(), v.stderr.len()))
            )
            .as_bytes(),
        );
        let result = result.unwrap();
        retain(&evidence, &format!("{mode}.stdout.log"), &result.stdout);
        retain(&evidence, &format!("{mode}.stderr.log"), &result.stderr);
        retain(
            &evidence,
            &format!("{mode}.trace.jsonl"),
            &fs::read(&trace).unwrap_or_default(),
        );
        assert_eq!(result.code, Some(0), "first census contract {mode}");
        assert!(result.stderr.is_empty());
    }
    retain(&evidence, "result.json", br#"{"finite_profile_compatibility":true,"conditional_schedule":true,"caller_integrated":true,"general_unbounded_equivalence":false,"production_authority":false}"#);
}

#[test]
fn census_contract_child_fixture() {
    use census::Inputs;
    let Some(mode) = std::env::var_os("HC074_CENSUS_CONTRACT_CHILD") else {
        return;
    };
    let mode = mode.to_str().unwrap();
    assert!(matches!(
        mode,
        "clean" | "trace" | "config" | "config-error" | "repository-error"
    ));
    let document: Value = serde_json::from_slice(
        &fs::read(std::env::var_os("HC074_CENSUS_CALLER_PINS").unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(document["test_fixture_pin_integrity"], true);
    assert_eq!(document["production_pin_authority"], false);
    let cases = document["cases"].as_array().unwrap();
    let raw = String::from_utf8(bytes(cases[1]["stdout_hex"].as_str().unwrap())).unwrap();
    let base = String::from_utf8(bytes(cases[0]["stdout_hex"].as_str().unwrap())).unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let trace = PathBuf::from(std::env::var_os("HC074_CENSUS_CONTRACT_TRACE").unwrap());
    let failed = mode.ends_with("error");
    let mut receipts = vec![];
    let deadline = Instant::now() + transport::BOUND; // One bound, never renewed between reads.
    for operation in ["original", "accepts", "view", "unrelated", "mismatch"] {
        let before = starts(&trace).len();
        let previous_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {})); // Suppress only the expected old caller panic.
        let old = std::panic::catch_unwind(|| match operation {
            "original" => caller::original(),
            "accepts" => caller::accepts(&base, &raw).to_string(),
            "view" => caller::view("diagnostic_artifacts_linux.rs", &raw),
            "unrelated" => caller::view("other.rs", "untouched"),
            _ => caller::accepts(&base, "mismatch").to_string(),
        });
        std::panic::set_hook(previous_hook);
        let old_starts = starts(&trace)[before..].to_vec();
        let before = starts(&trace).len();
        let mut inputs = RealInputs {
            root: &root,
            cases,
            deadline,
            events: vec![],
            diagnostics: vec![],
            outputs: vec![],
        };
        let new = match operation {
            "original" => inputs
                .frozen(census::Revision::Base)
                .map_err(|e| format!("{e:?}")),
            "accepts" => census::accepts(&mut inputs, &base, &raw)
                .map(|v| v.to_string())
                .map_err(|e| format!("{e:?}")),
            "view" => census::view(&mut inputs, "diagnostic_artifacts_linux.rs", &raw)
                .map_err(|e| format!("{e:?}")),
            "unrelated" => {
                census::view(&mut inputs, "other.rs", "untouched").map_err(|e| format!("{e:?}"))
            }
            _ => census::accepts(&mut inputs, &base, "mismatch")
                .map(|v| v.to_string())
                .map_err(|e| format!("{e:?}")),
        };
        let new_starts = starts(&trace)[before..].to_vec();
        assert_eq!(
            old_starts, new_starts,
            "literal argv and conditional Git read order"
        );
        let old_error = old.as_ref().err().map(|e| {
            e.downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| e.downcast_ref::<&str>().copied())
                .unwrap_or("non-string panic")
                .to_owned()
        });
        let before = starts(&trace).len();
        let mut facade_inputs = RealInputs {
            root: &root,
            cases,
            deadline,
            events: vec![],
            diagnostics: vec![],
            outputs: vec![],
        };
        let previous_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let facade = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match operation {
            "original" => census::panic_api::original(&mut facade_inputs),
            "accepts" => census::panic_api::accepts(&mut facade_inputs, &base, &raw).to_string(),
            "view" => {
                census::panic_api::view(&mut facade_inputs, "diagnostic_artifacts_linux.rs", &raw)
            }
            "unrelated" => census::panic_api::view(&mut facade_inputs, "other.rs", "untouched"),
            _ => census::panic_api::accepts(&mut facade_inputs, &base, "mismatch").to_string(),
        }));
        std::panic::set_hook(previous_hook);
        let facade_error = facade.as_ref().err().map(|e| panic_text(e.as_ref()));
        let old_kind = old.as_ref().err().map(|e| panic_kind(e.as_ref()));
        let facade_kind = facade.as_ref().err().map(|e| panic_kind(e.as_ref()));
        let facade_starts = starts(&trace)[before..].to_vec();
        assert_eq!(facade_starts, old_starts);
        assert_eq!(facade_inputs.events, inputs.events);
        assert_eq!(facade_error, old_error);
        assert_eq!(facade_kind, old_kind);
        if let Ok(value) = &old {
            assert_eq!(facade.as_ref().unwrap(), value);
        }
        let before = starts(&trace).len();
        let previous_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let current = std::panic::catch_unwind(|| match operation {
            "original" => integrated::original(),
            "accepts" => integrated::accepts(&base, &raw).to_string(),
            "view" => integrated::view("diagnostic_artifacts_linux.rs", &raw),
            "unrelated" => integrated::view("other.rs", "untouched"),
            _ => integrated::accepts(&base, "mismatch").to_string(),
        });
        std::panic::set_hook(previous_hook);
        let current_error = current.as_ref().err().map(|e| panic_text(e.as_ref()));
        let current_kind = current.as_ref().err().map(|e| panic_kind(e.as_ref()));
        let current_starts = starts(&trace)[before..].to_vec();
        assert_eq!(current_starts, old_starts, "no extra guard Git reads");
        assert_eq!(current_error, old_error);
        assert_eq!(current_kind, old_kind);
        if let Ok(value) = &old {
            assert_eq!(current.as_ref().unwrap(), value);
        }
        if failed && operation != "unrelated" {
            assert_eq!(
                old_error.as_deref(),
                Some("missing exact material fixture source")
            );
            assert!(new.as_ref().unwrap_err().contains("Process"));
        } else {
            assert_eq!(new.as_ref().unwrap(), old.as_ref().unwrap());
            let expected = match operation {
                "view" => 2,
                "unrelated" => 0,
                _ => 1,
            };
            assert_eq!(old_starts.len(), expected);
            if mode == "trace" {
                assert!(inputs.diagnostics.iter().all(|n| *n > 0));
            }
        }
        receipts.push(
            json!({"operation":operation,"old_success":old.is_ok(),"old_error":old_error,
            "candidate":new,"old_git_argv":old_starts,"candidate_git_argv":new_starts,
            "candidate_events":inputs.events,"retained_stderr_bytes":inputs.diagnostics,"outputs":inputs.outputs,
            "facade_success":facade.is_ok(),"facade_error":facade_error,"facade_git_argv":facade_starts,
            "old_panic_kind":old_kind,"facade_panic_kind":facade_kind,
            "integrated_success":current.is_ok(),"integrated_error":current_error,"integrated_panic_kind":current_kind,
            "integrated_git_argv":current_starts}),
        );
    }
    // A content pin is an extra guard, never refreshed from the new command.
    if !failed {
        let spec = cases[0]["spec"].as_str().unwrap();
        let mut wrong = pin(&cases[0]["pin"]);
        if let decoder::Expected::Object { sha256, .. } = &mut wrong {
            sha256[0] ^= 1;
        }
        let reference = census::Reference::new(spec, &wrong).unwrap();
        let failure = census::read(&root, spec, &reference, deadline).unwrap_err();
        assert_eq!(failure.reason, census::Reason::Content);
        assert_eq!(
            failure.output.unwrap().stdout,
            bytes(cases[0]["stdout_hex"].as_str().unwrap())
        );
    }
    println!(
        "{}",
        json!({"mode":mode,"operations":receipts,"caller_integrated":true})
    );
}
