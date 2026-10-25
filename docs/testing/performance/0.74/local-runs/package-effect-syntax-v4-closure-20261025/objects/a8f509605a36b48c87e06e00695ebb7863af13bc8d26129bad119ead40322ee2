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
    fn read_bound(
        &self,
        name: &str,
        root: &Path,
        request: (&str, Expected),
        policy: Replacements,
        reference: &Output,
    ) -> Result<Vec<Output>, Refusal> {
        let fallback =
            transport::FallbackPin::from_reference(request.0, &request.1, policy, reference)
                .unwrap();
        let result = transport::read_pinned(
            root,
            &[request.0.to_owned()],
            &[request.1],
            &[Some(fallback)],
            policy,
            Instant::now() + transport::BOUND,
        );
        self.read_result(name, &result);
        result
    }
}

#[test]
fn stale_pins_refs_config_and_fallback_aba_refuse_before_caller_admission() {
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
        e.read_bound(
            "stale-absence",
            &plain,
            (missing_spec, missing_pin()),
            Replacements::Default,
            &missing_output()
        ),
        Err(Refusal::Protocol(decoder::Refusal::Header))
    );
    assert_eq!(
        e.read_bound(
            "disabled-absence-control",
            &plain,
            (missing_spec, missing_pin()),
            Replacements::Disabled,
            &missing_output()
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

    // Preserve the old cut sequence; the original unsafe result is sealed in
    // historical-blob-fault-20261024. Pinned output must now refuse that ABA.
    for policy in [Replacements::Default, Replacements::Disabled] {
        let deadline = Instant::now() + transport::BOUND;
        let mut cuts = vec![];
        let fallback = transport::FallbackPin::from_reference(
            missing_spec,
            &missing_pin(),
            policy,
            &missing_output(),
        )
        .unwrap();
        let result = transport::read_pinned_observed(
            &plain,
            &[spec.to_owned(), missing_spec.to_owned()],
            &[old_pin(), missing_pin()],
            &[None, Some(fallback)],
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
        if matches!(policy, Replacements::Default) {
            assert_eq!(result, Err(Refusal::FallbackOutput));
        } else {
            assert_eq!(result.unwrap(), vec![old_output(), missing_output()]);
        }
        assert_eq!(
            e.read_bound(
                "after-aba-absence",
                &plain,
                (missing_spec, missing_pin()),
                Replacements::Default,
                &missing_output()
            )
            .unwrap(),
            vec![missing_output()]
        );
    }

    // A later mismatched fallback publishes neither an earlier blob nor an
    // earlier successful fallback. Retain duplicate order on the healthy path.
    let good = transport::FallbackPin::from_reference(
        missing_spec,
        &missing_pin(),
        Replacements::Default,
        &missing_output(),
    )
    .unwrap();
    let mut wrong_reference = missing_output();
    wrong_reference.stderr[0] ^= 1;
    let wrong = transport::FallbackPin::from_reference(
        missing_spec,
        &missing_pin(),
        Replacements::Default,
        &wrong_reference,
    )
    .unwrap();
    let duplicate_specs = [
        spec.to_owned(),
        missing_spec.to_owned(),
        missing_spec.to_owned(),
    ];
    let duplicate_pins = [old_pin(), missing_pin(), missing_pin()];
    let duplicates = transport::read_pinned(
        &plain,
        &duplicate_specs,
        &duplicate_pins,
        &[None, Some(good.clone()), Some(good.clone())],
        Replacements::Default,
        Instant::now() + transport::BOUND,
    );
    e.read_result("duplicate-fallback-control", &duplicates);
    assert_eq!(
        duplicates.unwrap(),
        vec![old_output(), missing_output(), missing_output()]
    );
    let partial = transport::read_pinned(
        &plain,
        &duplicate_specs,
        &duplicate_pins,
        &[None, Some(good), Some(wrong)],
        Replacements::Default,
        Instant::now() + transport::BOUND,
    );
    e.read_result("late-wrong-fallback-no-prefix", &partial);
    assert_eq!(partial, Err(Refusal::FallbackOutput));

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
    let fallback = transport::FallbackPin::from_reference(
        &tree_spec,
        &tree_pin(),
        Replacements::Default,
        &quoted,
    )
    .unwrap();
    let rendered = transport::read_pinned_observed(
        &plain,
        std::slice::from_ref(&tree_spec),
        &[tree_pin()],
        &[Some(fallback)],
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

    // Tree rendering ABA after raw tree validation: pin the original rendered
    // bytes, not just the tree body. Both ref and config mutations must refuse.
    let alternate_tree = e.checked(
        &plain,
        &["mktree"],
        format!("100644 blob {changed}\tother\n").as_bytes(),
    );
    for policy in [Replacements::Default, Replacements::Disabled] {
        let fallback =
            transport::FallbackPin::from_reference(&tree_spec, &tree_pin(), policy, &quoted)
                .unwrap();
        let deadline = Instant::now() + transport::BOUND;
        let tree_aba = transport::read_pinned_observed(
            &plain,
            std::slice::from_ref(&tree_spec),
            &[tree_pin()],
            &[Some(fallback)],
            policy,
            deadline,
            |point| {
                let args = match point {
                    Point::BatchValidated => return,
                    Point::BeforeFallback => vec![
                        "replace".to_owned(),
                        unicode_tree.clone(),
                        alternate_tree.clone(),
                    ],
                    Point::AfterFallback => vec![
                        "update-ref".to_owned(),
                        "-d".to_owned(),
                        format!("refs/replace/{unicode_tree}"),
                    ],
                };
                let args: Vec<_> = args.iter().map(String::as_str).collect();
                assert_eq!(
                    e.git(&plain, Replacements::Disabled, &args, &[], deadline)
                        .code,
                    Some(0)
                );
            },
        );
        e.read_result(&format!("tree-ref-aba-{policy:?}"), &tree_aba);
        if policy == Replacements::Default {
            assert_eq!(tree_aba, Err(Refusal::FallbackOutput));
        } else {
            let values = tree_aba.unwrap();
            assert_eq!(values.len(), 1);
            assert_eq!(values[0], quoted);
        }
    }
    e.checked(&plain, &["replace", &unicode_tree, &alternate_tree], &[]);
    let alternate_raw = e.git(
        &plain,
        Replacements::Disabled,
        &["cat-file", "tree", &alternate_tree],
        &[],
        Instant::now() + transport::BOUND,
    );
    assert_eq!(alternate_raw.code, Some(0));
    let replaced_pin = Expected::Object {
        oid: unicode_tree.clone(),
        kind: Kind::Tree,
        bytes: alternate_raw.stdout.len(),
        sha256: Sha256::digest(&alternate_raw.stdout).into(),
    };
    let replacement_output = e.git(
        &plain,
        Replacements::Default,
        &["show", &tree_spec],
        &[],
        Instant::now() + transport::BOUND,
    );
    assert_eq!(replacement_output.code, Some(0));
    assert_ne!(replacement_output.stdout, quoted.stdout);
    let fallback = transport::FallbackPin::from_reference(
        &tree_spec,
        &replaced_pin,
        Replacements::Default,
        &replacement_output,
    )
    .unwrap();
    let deadline = Instant::now() + transport::BOUND;
    let config_aba = transport::read_pinned_observed(
        &plain,
        std::slice::from_ref(&tree_spec),
        &[replaced_pin],
        &[Some(fallback)],
        Replacements::Default,
        deadline,
        |point| {
            let flag = match point {
                Point::BatchValidated => return,
                Point::BeforeFallback => "false",
                Point::AfterFallback => "true",
            };
            assert_eq!(
                e.git(
                    &plain,
                    Replacements::Disabled,
                    &["config", "core.useReplaceRefs", flag],
                    &[],
                    deadline
                )
                .code,
                Some(0)
            );
        },
    );
    e.read_result("tree-config-aba", &config_aba);
    assert_eq!(config_aba, Err(Refusal::FallbackOutput));
    e.checked(
        &plain,
        &["update-ref", "-d", &format!("refs/replace/{unicode_tree}")],
        &[],
    );

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
    let fallback = transport::FallbackPin::from_reference(
        missing_spec,
        &missing_pin(),
        Replacements::Default,
        &missing_output(),
    )
    .unwrap();
    let expired = transport::read_pinned_observed(
        &plain,
        &[missing_spec.to_owned()],
        &[missing_pin()],
        &[Some(fallback)],
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
        "fallback_absence_aba_refused":true,"quote_path_rendering_hypothesis_confirmed":false,
        "pinned_output_component_gate":true,"caller_admission_gate":false,"old_loader_integrated":false,"measurement":false,
        "source_authority":false,"server":false,"live_checks_or_deadlines_changed":false}));
}

fn synthetic_reference() -> (String, Expected, Output) {
    (
        format!("{}:tree", "a".repeat(40)),
        Expected::Object {
            oid: "b".repeat(40),
            kind: Kind::Tree,
            bytes: 0,
            sha256: [0; 32],
        },
        Output {
            stdout: b"tree\0\r\nbytes".to_vec(),
            stderr: b"diagnostic\xff\0".to_vec(),
            code: Some(0),
        },
    )
}

#[test]
fn exact_output_pin_binds_each_stream_length_bytes_and_terminal_code() {
    let (spec, original, reference) = synthetic_reference();
    let binding =
        transport::FallbackPin::from_reference(&spec, &original, Replacements::Default, &reference)
            .unwrap();
    assert_eq!(
        binding.verify_output(&reference, Instant::now() + transport::BOUND),
        Ok(())
    );
    for mutation in 0..7 {
        let mut value = Output {
            stdout: reference.stdout.clone(),
            stderr: reference.stderr.clone(),
            code: reference.code,
        };
        match mutation {
            0 => value.stdout[0] ^= 1,
            1 => value.stderr[0] ^= 1,
            2 => value.stdout.push(0),
            3 => {
                value.stderr.pop();
            }
            4 => value.code = Some(1),
            5 => value.code = None,
            6 => std::mem::swap(&mut value.stdout, &mut value.stderr),
            _ => unreachable!(),
        }
        assert_eq!(
            binding.verify_output(&value, Instant::now() + transport::BOUND),
            Err(Refusal::FallbackOutput),
            "mutation {mutation}"
        );
    }
    assert_eq!(
        binding.verify_output(&reference, Instant::now()),
        Err(Refusal::Deadline)
    );
}

#[test]
fn missing_or_misaligned_fallback_bindings_refuse_before_git_or_observer() {
    let absent = Path::new("nonexistent-fallback-binding-fixture-must-not-be-opened");
    let (spec, original, reference) = synthetic_reference();
    let binding =
        transport::FallbackPin::from_reference(&spec, &original, Replacements::Default, &reference)
            .unwrap();
    let read = |specs: &[String],
                pins: &[Expected],
                fallbacks: &[Option<transport::FallbackPin>],
                policy| {
        transport::read_pinned_observed(
            absent,
            specs,
            pins,
            fallbacks,
            policy,
            Instant::now() + transport::BOUND,
            |_| panic!("preflight must precede observation"),
        )
    };
    assert_eq!(
        transport::read(
            absent,
            std::slice::from_ref(&spec),
            std::slice::from_ref(&original),
            Replacements::Default,
            Instant::now() + transport::BOUND
        ),
        Err(Refusal::FallbackRequired)
    );
    assert_eq!(
        transport::read_observed(
            absent,
            std::slice::from_ref(&spec),
            std::slice::from_ref(&original),
            Replacements::Default,
            Instant::now() + transport::BOUND,
            |_| panic!("unbound fallback must not start")
        ),
        Err(Refusal::FallbackRequired)
    );
    assert_eq!(
        read(
            std::slice::from_ref(&spec),
            std::slice::from_ref(&original),
            &[None],
            Replacements::Default
        ),
        Err(Refusal::FallbackRequired)
    );
    assert_eq!(
        read(
            std::slice::from_ref(&spec),
            std::slice::from_ref(&original),
            &[],
            Replacements::Default
        ),
        Err(Refusal::FallbackIdentity)
    );
    assert_eq!(
        read(
            &[format!("{}:other", "a".repeat(40))],
            std::slice::from_ref(&original),
            &[Some(binding.clone())],
            Replacements::Default
        ),
        Err(Refusal::FallbackIdentity)
    );
    assert_eq!(
        read(
            std::slice::from_ref(&spec),
            std::slice::from_ref(&original),
            &[Some(binding.clone())],
            Replacements::Disabled
        ),
        Err(Refusal::FallbackIdentity)
    );
    for field in 0..4 {
        let mut changed = original.clone();
        if let Expected::Object {
            oid,
            kind,
            bytes,
            sha256,
        } = &mut changed
        {
            match field {
                0 => *oid = "c".repeat(40),
                1 => *kind = Kind::Tag,
                2 => *bytes = 1,
                3 => sha256[0] ^= 1,
                _ => unreachable!(),
            }
        }
        assert_eq!(
            read(
                std::slice::from_ref(&spec),
                &[changed],
                &[Some(binding.clone())],
                Replacements::Default
            ),
            Err(Refusal::FallbackIdentity)
        );
    }
    let mut blob = original.clone();
    if let Expected::Object { kind, .. } = &mut blob {
        *kind = Kind::Blob;
    }
    assert_eq!(
        read(
            std::slice::from_ref(&spec),
            &[blob],
            &[Some(binding)],
            Replacements::Default
        ),
        Err(Refusal::FallbackIdentity)
    );
}

#[test]
fn invalid_output_references_and_original_identity_are_not_admitted() {
    let (spec, original, mut reference) = synthetic_reference();
    for kind in [Kind::Tree, Kind::Commit, Kind::Tag] {
        let mut raw = original.clone();
        if let Expected::Object { kind: field, .. } = &mut raw {
            *field = kind;
        }
        assert!(transport::FallbackPin::from_reference(
            &spec,
            &raw,
            Replacements::Default,
            &reference
        )
        .is_ok());
    }
    for code in [None, Some(-1), Some(256)] {
        reference.code = code;
        assert!(matches!(
            transport::FallbackPin::from_reference(
                &spec,
                &original,
                Replacements::Default,
                &reference
            ),
            Err(Refusal::FallbackIdentity)
        ));
    }
    reference.code = Some(0);
    let missing = Expected::Missing { spec: spec.clone() };
    assert!(matches!(
        transport::FallbackPin::from_reference(&spec, &missing, Replacements::Default, &reference),
        Err(Refusal::FallbackIdentity)
    ));
    reference.code = Some(128);
    assert!(transport::FallbackPin::from_reference(
        &spec,
        &missing,
        Replacements::Default,
        &reference
    )
    .is_ok());
    assert!(matches!(
        transport::FallbackPin::from_reference(
            &format!("{}:other", "a".repeat(40)),
            &missing,
            Replacements::Default,
            &reference
        ),
        Err(Refusal::FallbackIdentity)
    ));
    assert!(matches!(
        transport::FallbackPin::from_reference(
            "HEAD:p",
            &original,
            Replacements::Default,
            &reference
        ),
        Err(Refusal::FallbackIdentity)
    ));
    let mut blob = original.clone();
    if let Expected::Object { kind, .. } = &mut blob {
        *kind = Kind::Blob;
    }
    assert!(matches!(
        transport::FallbackPin::from_reference(&spec, &blob, Replacements::Default, &reference),
        Err(Refusal::FallbackIdentity)
    ));
    let mut corrupt = original;
    if let Expected::Object { oid, .. } = &mut corrupt {
        *oid = "bad".to_owned();
    }
    assert!(matches!(
        transport::FallbackPin::from_reference(&spec, &corrupt, Replacements::Default, &reference),
        Err(Refusal::Protocol(decoder::Refusal::Reference))
    ));
}

#[test]
fn fallback_reference_and_aggregate_budgets_refuse_before_git() {
    let (spec, original, mut reference) = synthetic_reference();
    reference.stdout = vec![0; decoder::MAX_STREAM + 1];
    assert!(matches!(
        transport::FallbackPin::from_reference(&spec, &original, Replacements::Default, &reference),
        Err(Refusal::Budget)
    ));
    reference.stdout.clear();
    reference.stderr = vec![0; 64 * 1024 + 1];
    assert!(matches!(
        transport::FallbackPin::from_reference(&spec, &original, Replacements::Default, &reference),
        Err(Refusal::Budget)
    ));
    for stream in ["stdout", "stderr"] {
        reference.stdout.clear();
        reference.stderr.clear();
        if stream == "stdout" {
            reference.stdout = vec![0; decoder::MAX_STREAM / 2 + 1];
        } else {
            reference.stderr = vec![0; 32 * 1024 + 1];
        }
        let binding = transport::FallbackPin::from_reference(
            &spec,
            &original,
            Replacements::Default,
            &reference,
        )
        .unwrap();
        assert_eq!(
            transport::read_pinned(
                Path::new("nonexistent-budget-fixture"),
                &[spec.clone(), spec.clone()],
                &[original.clone(), original.clone()],
                &[Some(binding.clone()), Some(binding)],
                Replacements::Default,
                Instant::now() + transport::BOUND
            ),
            Err(Refusal::Budget)
        );
    }
}
