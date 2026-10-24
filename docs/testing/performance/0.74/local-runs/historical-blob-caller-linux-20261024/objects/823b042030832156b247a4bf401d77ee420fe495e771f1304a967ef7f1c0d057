//! Shadow audit of an unchanged real source caller; not an integration switch.
#[path = "support/census_provider_input.rs"]
mod caller;
#[path = "../../../tools/historical-blob-batch-074/decoder.rs"]
mod decoder;
#[path = "../../../tools/historical-blob-batch-074/transport.rs"]
mod transport;
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
        br#"{"caller_integrated":false,"server":false,"measurement":false}"#,
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
    retain(&evidence,"result.json",br#"{"fixture_pin_integrity":true,"clean_shadow_match":true,"trace_counterexample":true,"caller_admitted":false,"caller_integrated":false}"#);
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
    // Invoke the actual unchanged caller, including its conditional view/helper
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
