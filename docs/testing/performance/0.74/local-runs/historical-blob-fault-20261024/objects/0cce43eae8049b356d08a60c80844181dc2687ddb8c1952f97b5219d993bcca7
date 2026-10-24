//! Synthetic mutation characterization. Passing these tests does NOT admit a caller.
#[path = "../../../tools/historical-blob-batch-074/decoder.rs"]
mod decoder;
#[path = "../../../tools/historical-blob-batch-074/transport.rs"]
mod transport;
use decoder::{Expected, Kind};
use serde_json::{json, Value};
use std::cell::Cell;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use transport::{Output, Point, Refusal, Replacements};

fn hex(raw: &[u8]) -> String {
    raw.iter().map(|b| format!("{b:02x}")).collect()
}
fn unhex(raw: &str) -> Vec<u8> {
    assert_eq!(raw.len() % 2, 0);
    (0..raw.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&raw[at..at + 2], 16).unwrap())
        .collect()
}
fn pin(value: &Value) -> Expected {
    if value["kind"] == "missing" {
        return if value["short"] == true {
            Expected::ShortMissing {
                spec: value["spec"].as_str().unwrap().to_owned(),
            }
        } else {
            Expected::Missing {
                spec: value["spec"].as_str().unwrap().to_owned(),
            }
        };
    }
    Expected::Object {
        oid: value["oid"].as_str().unwrap().to_owned(),
        kind: match value["kind"].as_str().unwrap() {
            "blob" => Kind::Blob,
            "tree" => Kind::Tree,
            "commit" => Kind::Commit,
            "tag" => Kind::Tag,
            _ => panic!("oracle kind"),
        },
        bytes: value["bytes"].as_u64().unwrap().try_into().unwrap(),
        sha256: unhex(value["sha256"].as_str().unwrap()).try_into().unwrap(),
    }
}
fn output(value: &Output) -> Value {
    json!({"stdout_hex":hex(&value.stdout), "stderr_hex":hex(&value.stderr), "code":value.code})
}
fn reference(value: &Value) -> Output {
    Output {
        stdout: unhex(value["stdout_hex"].as_str().unwrap()),
        stderr: unhex(value["stderr_hex"].as_str().unwrap()),
        code: value["code"].as_i64().map(|v| v.try_into().unwrap()),
    }
}
struct Evidence {
    folder: Option<PathBuf>,
    serial: Cell<usize>,
}
impl Evidence {
    fn new() -> Self {
        let folder = std::env::var_os("HC074_BLOB_TRANSPORT_EVIDENCE").map(|path| {
            let path = PathBuf::from(path);
            let out = path.with_file_name(format!(
                "{}-mutations",
                path.file_name().unwrap().to_str().unwrap()
            ));
            fs::create_dir_all(out.parent().unwrap()).unwrap();
            fs::create_dir(&out).expect("first mutation attempt already exists");
            out
        });
        let result = Self {
            folder,
            serial: Cell::new(0),
        };
        result.save(
            "intent",
            json!({"scope":"disposable Git only", "admission":false,
            "seed":"hc074-collision", "server":false, "retry_to_pass":false}),
        );
        result
    }
    fn save(&self, name: &str, value: Value) {
        let serial = self.serial.get();
        self.serial.set(serial + 1);
        if let Some(folder) = &self.folder {
            fs::File::create_new(folder.join(format!("{serial:03}-{name}.json")))
                .unwrap()
                .write_all(&serde_json::to_vec(&value).unwrap())
                .unwrap();
        }
    }
    fn git(
        &self,
        root: &Path,
        policy: Replacements,
        args: &[&str],
        input: &[u8],
        deadline: Instant,
    ) -> Output {
        let value = transport::git(root, policy, args, input, deadline);
        self.save(
            "git",
            json!({"args":args,"policy":format!("{policy:?}"),"input_hex":hex(input),
                "result":match &value { Ok(v)=>output(v), Err(e)=>json!({"refusal":format!("{e:?}")}) }}),
        );
        value.unwrap()
    }
    fn checked(&self, root: &Path, args: &[&str], input: &[u8]) -> String {
        let value = self.git(
            root,
            Replacements::Disabled,
            args,
            input,
            Instant::now() + transport::BOUND,
        );
        assert_eq!(value.code, Some(0), "fixture command {args:?}");
        assert!(value.stderr.is_empty(), "fixture stderr {args:?}");
        String::from_utf8(value.stdout).unwrap().trim().to_owned()
    }
    fn read(
        &self,
        name: &str,
        root: &Path,
        spec: &str,
        pin: Expected,
        policy: Replacements,
    ) -> Result<Vec<Output>, Refusal> {
        let value = transport::read(
            root,
            &[spec.to_owned()],
            &[pin],
            policy,
            Instant::now() + transport::BOUND,
        );
        self.read_result(name, &value);
        value
    }
    fn read_result(&self, name: &str, value: &Result<Vec<Output>, Refusal>) {
        self.save(
            name,
            match value {
                Ok(v) => json!({"outputs":v.iter().map(output).collect::<Vec<_>>()}),
                Err(e) => json!({"refusal":format!("{e:?}")}),
            },
        );
    }
}

#[test]
fn stale_pins_refs_config_and_fallback_aba_are_characterized_before_caller_admission() {
    let e = Evidence::new();
    let root = tempfile::tempdir().unwrap();
    let sources = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let python = if cfg!(windows) { "python" } else { "python3" };
    let mut oracle_command = Command::new(python);
    oracle_command
        .arg(sources.join("tools/historical-blob-batch-074/oracle.py"))
        .arg(root.path())
        .arg("--fault-controls");
    let oracle = transport::run_owned(&mut oracle_command, &[], Instant::now() + transport::BOUND);
    e.save(
        "oracle",
        match &oracle {
            Ok(v) => output(v),
            Err(err) => json!({"refusal":format!("{err:?}")}),
        },
    );
    let oracle = oracle.unwrap();
    assert_eq!(oracle.code, Some(0));
    assert!(oracle.stderr.is_empty());
    let oracle: Value = serde_json::from_slice(&oracle.stdout).unwrap();
    assert_eq!(oracle["schema"], "hc074-blob-fault-synthetic-oracle-v1");
    assert_eq!(oracle["cases"].as_array().unwrap().len(), 2);
    assert_eq!(oracle["cases"][0]["root"], "plain");
    assert_eq!(oracle["cases"][0]["policy"], "disabled");
    assert_eq!(oracle["cases"][1]["root"], "replaced");
    assert_eq!(oracle["cases"][1]["policy"], "default");
    let plain = root.path().join("plain");
    let replaced = root.path().join("replaced");
    let case = &oracle["cases"][0];
    let spec = case["specs"][0].as_str().unwrap();
    let first = spec.split_once(':').unwrap().0;
    let old_blob = case["pins"][0]["oid"].as_str().unwrap();
    let old_output = || reference(&case["expected"][0]);
    let old_pin = || pin(&case["pins"][0]);
    assert_eq!(
        e.read("baseline", &plain, spec, old_pin(), Replacements::Default)
            .unwrap(),
        vec![old_output()]
    );

    // A full commit ID is unaffected by an unrelated branch and HEAD mutation.
    e.checked(&plain, &["update-ref", "refs/heads/unrelated", first], &[]);
    e.checked(
        &plain,
        &["symbolic-ref", "HEAD", "refs/heads/unrelated"],
        &[],
    );
    assert_eq!(
        e.read(
            "head-control",
            &plain,
            spec,
            old_pin(),
            Replacements::Default
        )
        .unwrap(),
        vec![old_output()]
    );

    // Same header/OID/length, different replacement body: the digest must refuse.
    let mut body = old_output().stdout;
    body[0] ^= 1;
    let changed = e.checked(&plain, &["hash-object", "-w", "--stdin"], &body);
    e.checked(&plain, &["replace", old_blob, &changed], &[]);
    assert_eq!(
        e.read("stale-blob", &plain, spec, old_pin(), Replacements::Default),
        Err(Refusal::Protocol(decoder::Refusal::Content))
    );
    assert_eq!(
        e.read(
            "disabled-blob-control",
            &plain,
            spec,
            old_pin(),
            Replacements::Disabled
        )
        .unwrap(),
        vec![old_output()]
    );
    e.checked(
        &plain,
        &["update-ref", "-d", &format!("refs/replace/{old_blob}")],
        &[],
    );

    let replacement_case = &oracle["cases"][1];
    e.checked(&replaced, &["config", "core.useReplaceRefs", "false"], &[]);
    assert_eq!(
        e.read(
            "stale-config",
            &replaced,
            spec,
            pin(&replacement_case["pins"][0]),
            Replacements::Default
        ),
        Err(Refusal::Protocol(decoder::Refusal::Header))
    );
    assert_eq!(
        e.read(
            "disabled-config-control",
            &replaced,
            spec,
            old_pin(),
            Replacements::Disabled
        )
        .unwrap(),
        vec![old_output()]
    );

    // Replacement commit introduces a previously absent path.
    let missing_spec = case["specs"][5].as_str().unwrap();
    let missing_pin = || pin(&case["pins"][5]);
    let missing_output = || reference(&case["expected"][5]);
    let tree = e.checked(
        &plain,
        &["mktree"],
        format!("100644 blob {changed}\tmissing\n").as_bytes(),
    );
    let commit = e.checked(&plain, &["hash-object", "-t", "commit", "-w", "--stdin"],
        format!("tree {tree}\nauthor Fixture <fixture@example.invalid> 946684800 +0000\ncommitter Fixture <fixture@example.invalid> 946684800 +0000\n\nsynthetic replacement\n").as_bytes());
    e.checked(&plain, &["replace", first, &commit], &[]);
    assert_eq!(
        e.read(
            "stale-absence",
            &plain,
            missing_spec,
            missing_pin(),
            Replacements::Default
        ),
        Err(Refusal::Protocol(decoder::Refusal::Header))
    );
    assert_eq!(
        e.read(
            "disabled-absence-control",
            &plain,
            missing_spec,
            missing_pin(),
            Replacements::Disabled
        )
        .unwrap(),
        vec![missing_output()]
    );
    let fresh_missing = e.git(
        &plain,
        Replacements::Default,
        &["show", missing_spec],
        &[],
        Instant::now() + transport::BOUND,
    );
    assert_eq!(fresh_missing.code, Some(0));
    e.checked(
        &plain,
        &["update-ref", "-d", &format!("refs/replace/{first}")],
        &[],
    );

    // Expected-red admission counterexample: an ABA mutation after validation.
    // A green characterization test must not be reported as a green fault gate.
    for policy in [Replacements::Default, Replacements::Disabled] {
        let deadline = Instant::now() + transport::BOUND;
        let mut cuts = vec![];
        let result = transport::read_observed(
            &plain,
            &[missing_spec.to_owned()],
            &[missing_pin()],
            policy,
            deadline,
            |point| {
                cuts.push(format!("{point:?}"));
                let args = match point {
                    Point::BatchValidated => return,
                    Point::BeforeFallback => {
                        vec!["replace".to_owned(), first.to_owned(), commit.clone()]
                    }
                    Point::AfterFallback => vec![
                        "update-ref".to_owned(),
                        "-d".to_owned(),
                        format!("refs/replace/{first}"),
                    ],
                };
                let args: Vec<_> = args.iter().map(String::as_str).collect();
                let value = e.git(&plain, Replacements::Disabled, &args, &[], deadline);
                assert_eq!(value.code, Some(0));
                assert!(value.stderr.is_empty());
            },
        );
        e.read_result(&format!("fallback-aba-{policy:?}"), &result);
        e.save(
            "fallback-cuts",
            json!({"policy":format!("{policy:?}"),"cuts":cuts,"admission":false}),
        );
        assert_eq!(cuts, ["BatchValidated", "BeforeFallback", "AfterFallback"]);
        let expected = if matches!(policy, Replacements::Default) {
            output(&fresh_missing)
        } else {
            output(&missing_output())
        };
        let result = result.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(output(&result[0]), expected);
        assert_eq!(
            e.read(
                "after-aba-absence",
                &plain,
                missing_spec,
                missing_pin(),
                Replacements::Default
            )
            .unwrap(),
            vec![missing_output()]
        );
    }

    // Installed Git's tree `show` does not quote this Unicode name according to
    // core.quotePath. Keep this negative config hypothesis, not a drift claim.
    let unicode_tree = e.checked(
        &plain,
        &["mktree"],
        format!("100644 blob {changed}\tλ\n").as_bytes(),
    );
    let parent_tree = e.checked(
        &plain,
        &["mktree"],
        format!("040000 tree {unicode_tree}\tfolder\n").as_bytes(),
    );
    let parent = e.checked(&plain, &["hash-object", "-t", "commit", "-w", "--stdin"],
        format!("tree {parent_tree}\nauthor Fixture <fixture@example.invalid> 946684800 +0000\ncommitter Fixture <fixture@example.invalid> 946684800 +0000\n\nquote fixture\n").as_bytes());
    let tree_spec = format!("{parent}:folder");
    let raw_tree = e.git(
        &plain,
        Replacements::Disabled,
        &["cat-file", "tree", &tree_spec],
        &[],
        Instant::now() + transport::BOUND,
    );
    assert_eq!(raw_tree.code, Some(0));
    use sha2::{Digest, Sha256};
    let tree_pin = || Expected::Object {
        oid: unicode_tree.clone(),
        kind: Kind::Tree,
        bytes: raw_tree.stdout.len(),
        sha256: Sha256::digest(&raw_tree.stdout).into(),
    };
    e.checked(&plain, &["config", "core.quotePath", "true"], &[]);
    let quoted = e.git(
        &plain,
        Replacements::Default,
        &["show", &tree_spec],
        &[],
        Instant::now() + transport::BOUND,
    );
    let deadline = Instant::now() + transport::BOUND;
    let rendered = transport::read_observed(
        &plain,
        std::slice::from_ref(&tree_spec),
        &[tree_pin()],
        Replacements::Default,
        deadline,
        |point| {
            if point == Point::BeforeFallback {
                let value = e.git(
                    &plain,
                    Replacements::Disabled,
                    &["config", "core.quotePath", "false"],
                    &[],
                    deadline,
                );
                assert_eq!(value.code, Some(0));
            }
        },
    );
    e.read_result("config-rendering-cut", &rendered);
    let fresh = e.git(
        &plain,
        Replacements::Default,
        &["show", &tree_spec],
        &[],
        Instant::now() + transport::BOUND,
    );
    assert_eq!(fresh.code, Some(0));
    e.save(
        "quote-path-negative-control",
        json!({"rendering_changed":fresh.stdout != quoted.stdout,
        "first_v1_wrong_expectation_retained":true,"admission":false}),
    );
    assert_eq!(fresh, quoted);
    assert_eq!(rendered.unwrap(), vec![fresh]);

    // One deterministic bounded search introduces a second COMMIT with the prefix.
    let short = &first[..4];
    let unique = e.checked(
        &plain,
        &["rev-parse", &format!("--disambiguate={short}")],
        &[],
    );
    assert_eq!(
        unique, first,
        "first four-digit prefix must initially be unique; no retry"
    );
    let raw_commit = e.git(
        &plain,
        Replacements::Disabled,
        &["cat-file", "commit", first],
        &[],
        Instant::now() + transport::BOUND,
    );
    assert_eq!(raw_commit.code, Some(0));
    let mut collision_command = Command::new(python);
    collision_command
        .arg(sources.join("tools/historical-blob-batch-074/collision.py"))
        .arg(short);
    let collision = transport::run_owned(
        &mut collision_command,
        &raw_commit.stdout,
        Instant::now() + transport::BOUND,
    );
    e.save(
        "collision-process",
        match &collision {
            Ok(v) => output(v),
            Err(err) => json!({"refusal":format!("{err:?}")}),
        },
    );
    let collision = collision.unwrap();
    assert_eq!(collision.code, Some(0));
    assert!(collision.stderr.is_empty());
    let collision: Value = serde_json::from_slice(&collision.stdout).unwrap();
    assert_eq!(collision["authority"], false);
    let colliding = e.checked(
        &plain,
        &["hash-object", "-t", "commit", "-w", "--stdin"],
        &unhex(collision["body_hex"].as_str().unwrap()),
    );
    assert_eq!(colliding, collision["oid"].as_str().unwrap());
    assert_ne!(colliding, first);
    let ids = e.checked(
        &plain,
        &["rev-parse", &format!("--disambiguate={short}")],
        &[],
    );
    let mut ids: Vec<_> = ids.lines().collect();
    ids.sort_unstable();
    let mut both = vec![first, &colliding];
    both.sort_unstable();
    assert_eq!(ids, both);
    let short_spec = format!("{short}:binary");
    let ambiguous = e.git(
        &plain,
        Replacements::Default,
        &["show", &short_spec],
        &[],
        Instant::now() + transport::BOUND,
    );
    assert_ne!(ambiguous.code, Some(0));
    assert_eq!(
        e.read(
            "ambiguous-stale-short-pin",
            &plain,
            &short_spec,
            old_pin(),
            Replacements::Default
        ),
        Err(Refusal::Process)
    );
    assert_eq!(
        e.read(
            "full-id-after-collision",
            &plain,
            spec,
            old_pin(),
            Replacements::Default
        )
        .unwrap(),
        vec![old_output()]
    );

    // Remove only an exact verified loose fixture object, keeping its bytes held.
    let object = plain
        .join(".git/objects")
        .join(&old_blob[..2])
        .join(&old_blob[2..]);
    let object = object.canonicalize().unwrap();
    assert!(object.starts_with(plain.canonicalize().unwrap().join(".git/objects")));
    assert!(fs::symlink_metadata(&object).unwrap().is_file());
    let held = root.path().join("held-loose-object");
    assert!(!held.exists());
    fs::rename(&object, &held).unwrap();
    let removed = e.git(
        &plain,
        Replacements::Default,
        &["show", spec],
        &[],
        Instant::now() + transport::BOUND,
    );
    assert_ne!(removed.code, Some(0));
    let absent_batch = e.git(
        &plain,
        Replacements::Default,
        &["cat-file", "--batch"],
        format!("{spec}\n").as_bytes(),
        Instant::now() + transport::BOUND,
    );
    assert_eq!(absent_batch.code, Some(0));
    assert!(absent_batch.stderr.is_empty());
    assert_eq!(absent_batch.stdout, format!("{spec} missing\n").as_bytes());
    assert_eq!(
        e.read(
            "missing-object-stale-pin",
            &plain,
            spec,
            old_pin(),
            Replacements::Default
        ),
        Err(Refusal::Protocol(decoder::Refusal::Header))
    );
    fs::rename(&held, &object).unwrap();
    assert_eq!(
        e.read(
            "restored-object-control",
            &plain,
            spec,
            old_pin(),
            Replacements::Default
        )
        .unwrap(),
        vec![old_output()]
    );
    // The observer cannot grant a new deadline after it consumes the old one.
    let deadline = Instant::now() + std::time::Duration::from_secs(2);
    let mut cuts = vec![];
    let expired = transport::read_observed(
        &plain,
        &[missing_spec.to_owned()],
        &[missing_pin()],
        Replacements::Default,
        deadline,
        |point| {
            cuts.push(format!("{point:?}"));
            if point == Point::BatchValidated {
                while Instant::now() < deadline {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
            }
        },
    );
    e.read_result("observer-original-deadline", &expired);
    e.save("deadline-cuts", json!({"cuts":cuts,"new_grace":false}));
    assert_eq!(cuts, ["BatchValidated"]);
    assert_eq!(expired, Err(Refusal::Deadline));
    e.save("result", json!({"characterization_passed":true,"stale_pin_refusals":5,
        "fallback_absence_aba_counterexample":true,"quote_path_rendering_hypothesis_confirmed":false,
        "fault_admission_gate":false,"old_loader_integrated":false,"measurement":false,
        "source_authority":false,"server":false,"live_checks_or_deadlines_changed":false}));
}
