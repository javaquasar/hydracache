use serde_json::{json, Value};
use xtask::imap_extended_receipt_075::Artifact;
use xtask::imap_sbom_075::*;

fn metadata() -> Value {
    json!({"packages":[
        {"id":"root-id","name":"root","version":"1.0.0","source":null,"license":"Apache-2.0","features":{"default":[],"enabled":[]},"dependencies":[{"name":"library","rename":null,"optional":false,"kind":null,"target":null},{"name":"library","rename":null,"optional":false,"kind":"build","target":"cfg(all())"}]},
        {"id":"dep-id","name":"library","version":"1.2.3","source":"registry+https://github.com/rust-lang/crates.io-index","license":null,"features":{"default":[],"enabled":[]},"dependencies":[]}],
        "resolve":{"nodes":[{"id":"root-id","deps":[{"name":"library","pkg":"dep-id","dep_kinds":[{"kind":null,"target":null},{"kind":"build","target":"cfg(all())"}]}]},{"id":"dep-id","deps":[]}]}})
}
fn transcript() -> Vec<u8> {
    let mut rows = Vec::new();
    for id in ["root-id", "dep-id"] {
        rows.push(json!({"reason":"compiler-artifact","package_id":id,"features":["default","enabled"],"target":{"kind":["lib"],"name":id,"src_path":"C:/private/workspace/src.rs"},"profile":{"test":false,"opt_level":"0"}}));
    }
    rows.push(json!({"reason":"compiler-artifact","package_id":"root-id","features":["default","enabled"],"target":{"kind":["custom-build"],"name":"build_script_build"},"profile":{"test":false,"opt_level":"0"}}));
    rows.push(json!({"reason":"build-finished","success":true}));
    rows.iter()
        .map(|r| format!("{r}\n"))
        .collect::<String>()
        .into_bytes()
}
#[test]
fn malformed_resolver_and_compiler_records_refuse_at_their_exact_boundary() {
    let target = CargoTarget {
        triple: "fixture".into(),
        cfg: Default::default(),
    };
    let project = |meta: &Value, bytes: &[u8], locked: &str| {
        cargo_projection_for_target(meta, bytes, "fixture", locked, &"b".repeat(40), &target)
    };
    for mode in 0..10 {
        let mut meta = metadata();
        let mut bytes = transcript();
        let mut locked = lock();
        let expected = match mode {
            0 => {
                locked = locked.replace(&"a".repeat(64), "invalid");
                "registry checksum missing"
            }
            1 => {
                locked.push_str(&lock().replace("version=4\n", ""));
                "duplicate locked identity"
            }
            2 => {
                bytes.extend_from_slice(b"{}\n");
                "Cargo compiler record bound"
            }
            3 => {
                bytes = b"{\"reason\":\"build-finished\",\"success\":false}\n".to_vec();
                "Cargo build terminal"
            }
            4 => {
                bytes = b"{\"reason\":\"future\"}\n".to_vec();
                "unknown Cargo compiler record"
            }
            5 => {
                meta["packages"][0]["features"]["enabled"] = json!(["absent"]);
                "unknown Cargo feature closure member"
            }
            6 => {
                let node = meta["resolve"]["nodes"][0].clone();
                meta["resolve"]["nodes"].as_array_mut().unwrap().push(node);
                "duplicate Cargo resolved node"
            }
            7 => {
                meta["packages"][0]["dependencies"][0]["name"] = json!("foreign");
                "no matching declaration"
            }
            8 => {
                meta["packages"][0]["dependencies"][0]["optional"] = Value::Null;
                "Cargo optional declaration missing"
            }
            _ => {
                meta["packages"][0]["features"]["enabled"] = json!(vec!["enabled"; 20001]);
                "Cargo feature closure work bound"
            }
        };
        let error = project(&meta, &bytes, &locked).unwrap_err().to_string();
        assert!(error.contains(expected), "{mode}: {error}");
    }
    let mut meta = metadata();
    meta["packages"][1]["name"] = meta["packages"][0]["name"].clone();
    meta["packages"][1]["version"] = meta["packages"][0]["version"].clone();
    meta["packages"][1]["source"] = Value::Null;
    assert!(project(&meta, &transcript(), &lock())
        .unwrap_err()
        .to_string()
        .contains("ambiguous compiled"));
    assert!(target
        .applies("cfg(target_arch=unknown)")
        .unwrap_err()
        .to_string()
        .contains("literal required"));
    assert!(target.applies("cfg(all(windows,,unix))").is_err());
}
#[test]
fn inventory_roots_and_compilation_instances_cannot_be_duplicated_or_omitted() {
    for mode in 0..4 {
        let mut value = inventory();
        match mode {
            0 => {
                value.roots.pop();
            }
            1 => value.roots[1] = value.roots[0].clone(),
            2 => value.compilations.push(value.compilations[0].clone()),
            _ => value.compilations[0].component = "foreign".into(),
        }
        assert!(validate_inventory(&value).is_err(), "{mode}");
    }
}
#[test]
fn maven_record_limits_tools_and_unknown_events_are_fail_closed() {
    assert!(maven_journal(&lines(&journal()), "")
        .unwrap_err()
        .to_string()
        .contains("journal bound"));
    assert!(maven_journal(&vec![b' '; 16385], "fixture")
        .unwrap_err()
        .to_string()
        .contains("record/work bound"));
    for mode in 0..2 {
        let mut rows = journal();
        rows[3] = if mode == 0 {
            rows[1].clone()
        } else {
            json!({"event":"future"})
        };
        let error = maven_journal(&lines(&rows), "fixture")
            .unwrap_err()
            .to_string();
        assert!(
            error.contains(if mode == 0 {
                "invalid Maven tool"
            } else {
                "unexpected Maven journal record"
            }),
            "{error}"
        );
    }
    assert!(run(vec![], false).is_err());
    assert!(run(vec![], true).is_err());
}
fn lock() -> String {
    format!("version=4\n[[package]]\nname='library'\nversion='1.2.3'\nsource='registry+https://github.com/rust-lang/crates.io-index'\nchecksum='{}'\n","a".repeat(64))
}
fn inventory() -> Inventory {
    let (components, edges, compilations) = cargo_projection(
        &metadata(),
        &transcript(),
        "fixture",
        &lock(),
        &"b".repeat(40),
    )
    .unwrap();
    let mut components = components;
    components.sort_by(|a, b| a.id.cmp(&b.id));
    Inventory {
        schema: "imap075-development-inventory-v1".into(),
        profile: "imap075-native-host-build-and-test-inputs-v1".into(),
        target: "x86_64-pc-windows-msvc".into(),
        source_sha: "b".repeat(40),
        cargo_lock_sha256: "c".repeat(64),
        roots: (0..5)
            .map(|i| RootArtifact {
                component: components[0].id.clone(),
                artifact: Artifact {
                    path: format!("target/{i}.exe"),
                    bytes: 1,
                    sha256: "d".repeat(64),
                },
            })
            .collect(),
        components,
        edges,
        compilations,
        tools: vec![json!({"kind":"rustc","version":"fixture"})],
    }
}
#[test]
fn actual_compiler_instances_have_features_kinds_and_unknown_license_origin() {
    let (c, e, i) = cargo_projection(
        &metadata(),
        &transcript(),
        "fixture",
        &lock(),
        &"b".repeat(40),
    )
    .unwrap();
    assert_eq!(c.len(), 2);
    assert_eq!(e.len(), 2);
    assert_eq!(i.len(), 3);
    assert!(c
        .iter()
        .any(|c| c.license_origin == "unknown" && c.licenses.is_empty()));
    assert!(i
        .iter()
        .all(|i| i.target.get("src_path").is_none() && i.features == ["default", "enabled"]));
    assert!(e
        .iter()
        .any(|e| e.kind == "build" && e.target == "cfg(all())"));
    assert_eq!(
        serde_json::to_vec(&inventory()).unwrap(),
        serde_json::to_vec(&inventory()).unwrap()
    );
}
#[test]
fn optional_feature_activation_and_test_build_contexts_define_actual_edges() {
    let mut meta = metadata();
    meta["packages"][0]["dependencies"][0]["optional"] = json!(true);
    let project = |m: &Value, t: &[u8]| {
        cargo_projection(m, t, "fixture", &lock(), &"b".repeat(40))
            .unwrap()
            .1
    };
    assert_eq!(project(&meta, &transcript()).len(), 1);
    meta["packages"][0]["features"]["enabled"] = json!(["library?/std"]);
    assert_eq!(project(&meta, &transcript()).len(), 1);
    meta["packages"][0]["features"]["enabled"] = json!(["dep:library"]);
    assert_eq!(project(&meta, &transcript()).len(), 2);
    meta["packages"][0]["features"]["enabled"] = json!(["nested"]);
    meta["packages"][0]["features"]["nested"] = json!(["library/std", "enabled"]);
    assert_eq!(project(&meta, &transcript()).len(), 2);
    meta = metadata();
    meta["packages"][0]["dependencies"][1]["kind"] = json!("dev");
    meta["resolve"]["nodes"][0]["deps"][0]["dep_kinds"][1]["kind"] = json!("dev");
    assert_eq!(project(&meta, &transcript()).len(), 1);
    let test = String::from_utf8(transcript())
        .unwrap()
        .replace("\"test\":false", "\"test\":true");
    assert_eq!(project(&meta, test.as_bytes()).len(), 2);
    let missing = String::from_utf8(transcript())
        .unwrap()
        .lines()
        .filter(|l| !l.contains("\"package_id\":\"dep-id\""))
        .map(|l| format!("{l}\n"))
        .collect::<String>();
    assert!(cargo_projection(
        &meta,
        missing.as_bytes(),
        "fixture",
        &lock(),
        &"b".repeat(40)
    )
    .is_err());
    meta["resolve"]["nodes"].as_array_mut().unwrap().pop();
    assert!(cargo_projection(&meta, &transcript(), "fixture", &lock(), &"b".repeat(40)).is_err());
}
#[test]
fn cargo_target_predicates_are_evaluated_against_actual_flags_not_metadata_presence() {
    let target = CargoTarget {
        triple: "x86_64-pc-windows-msvc".into(),
        cfg: ["windows".into(), "target_arch=\"x86_64\"".into()]
            .into_iter()
            .collect(),
    };
    for predicate in [
        "cfg(all())",
        "cfg(windows)",
        "cfg(all(windows, target_arch = \"x86_64\", not(unix)))",
        "cfg(any(unix,windows))",
        "x86_64-pc-windows-msvc",
    ] {
        assert!(target.applies(predicate).unwrap(), "{predicate}");
    }
    for predicate in ["cfg(any())", "cfg(unix)", "aarch64-unknown-linux-gnu"] {
        assert!(!target.applies(predicate).unwrap());
    }
    for predicate in [
        "cfg(not())",
        "cfg(future(windows))",
        "cfg(target_arch=1)",
        "cfg(a::b)",
        "cfg(not(windows)",
        "cfg())",
    ] {
        assert!(target.applies(predicate).is_err(), "{predicate}");
    }
    assert!(target
        .applies(&format!(
            "cfg({}windows{})",
            "not(".repeat(34),
            ")".repeat(34)
        ))
        .is_err());
    assert!(target.applies(&"x".repeat(4097)).is_err());
}
#[test]
fn absent_foreign_duplicate_and_incomplete_cargo_inputs_refuse() {
    for mode in 0..6 {
        let mut meta = metadata();
        let mut bytes = transcript();
        let mut locked = lock();
        match mode {
            0 => {
                meta["packages"].as_array_mut().unwrap().pop();
            }
            1 => {
                let duplicate = meta["packages"][0].clone();
                meta["packages"].as_array_mut().unwrap().push(duplicate);
            }
            2 => bytes = bytes[..bytes.len() - 44].to_vec(),
            3 => locked = locked.replace("1.2.3", "1.2.4"),
            4 => {
                bytes = String::from_utf8(bytes)
                    .unwrap()
                    .replace("\"enabled\"", "\"default\"")
                    .into_bytes()
            }
            _ => meta["resolve"]["nodes"][0]["deps"][0]["dep_kinds"][0]["kind"] = json!("future"),
        }
        assert!(
            cargo_projection(&meta, &bytes, "fixture", &locked, &"b".repeat(40)).is_err(),
            "{mode}"
        );
    }
    assert!(cargo_projection(
        &metadata(),
        &vec![0; 16 * 1024 * 1024 + 1],
        "fixture",
        &lock(),
        &"b".repeat(40)
    )
    .is_err());
}
#[test]
fn strict_reduction_rejects_missing_transitive_feature_source_target_and_artifact() {
    let expected = inventory();
    validate_inventory(&expected).unwrap();
    for mode in 0..10 {
        let mut actual = expected.clone();
        match mode {
            0 => {
                actual.components.pop();
            }
            1 => actual.components.push(actual.components[0].clone()),
            2 => actual.components[0].source = "foreign".into(),
            3 => actual.components[0].source_checksum = "forged".into(),
            4 => {
                actual.edges.pop();
            }
            5 => {
                actual.compilations[0].features.pop();
            }
            6 => actual.target = "aarch64-unknown-linux-gnu".into(),
            7 => actual.profile = "production".into(),
            8 => actual.roots[0].artifact.sha256 = "e".repeat(64),
            _ => actual.source_sha = "f".repeat(40),
        }
        assert!(compare_inventory(&expected, &actual).is_err(), "{mode}");
    }
    let mut invalid = expected.clone();
    invalid.components[0].licenses.clear();
    invalid.components[0].license_origin = "approved".into();
    assert!(validate_inventory(&invalid).is_err());
}
#[test]
fn cyclone_dx_is_offline_pinned_deterministic_and_schema_checked() {
    let bom = cyclone_dx(&inventory()).unwrap();
    validate_cyclone_dx(&bom).unwrap();
    assert_eq!(bom, cyclone_dx(&inventory()).unwrap());
    let mut changed = bom;
    changed["components"][0]["type"] = json!("invented-type");
    assert!(validate_cyclone_dx(&changed).is_err());
    assert!(validate_cyclone_dx(&json!({"passed":true})).is_err());
}
fn journal() -> Vec<Value> {
    vec![
        json!({"event":"init","schema":"imap075-maven-resolution-v1","nonce":"fixture","stage":"artifact-build"}),
        json!({"event":"tool","kind":"jdk","path":"bin/java","sha256":"a".repeat(64)}),
        json!({"event":"material","id":"a:b:jar::1","path":"a/b/1/b-1.jar","location":"repository","sha256":"b".repeat(64),"bytes":1}),
        json!({"event":"mojo","group":"a","name":"plugin","version":"1","goal":"test"}),
        json!({"event":"terminal","nonce":"fixture","stage":"artifact-build","complete":true,"records":4,"materials":1}),
    ]
}
fn lines(rows: &[Value]) -> Vec<u8> {
    rows.iter()
        .map(|r| format!("{r}\n"))
        .collect::<String>()
        .into_bytes()
}
#[test]
fn maven_swallowed_errors_partial_foreign_duplicate_journals_fail_closed() {
    maven_journal(&lines(&journal()), "fixture").unwrap();
    for mode in 0..7 {
        let mut rows = journal();
        match mode {
            0 => {
                rows.pop();
            }
            1 => rows[4]["complete"] = json!(false),
            2 => rows[4]["nonce"] = json!("foreign"),
            3 => rows[4]["materials"] = json!(2),
            4 => rows[3] = rows[2].clone(),
            5 => rows[2]["sha256"] = json!("fake"),
            _ => rows[2]["location"] = json!("global-cache"),
        };
        assert!(maven_journal(&lines(&rows), "fixture").is_err(), "{mode}");
    }
    assert!(maven_journal(&[], "fixture").is_err());
}
#[test]
fn dependency_policy_never_converts_unknown_or_rejected_into_passed() {
    assert_eq!(
        policy_state(
            false,
            false,
            b"bad-prefix\n{\"fields\":{\"severity\":\"error\",\"code\":\"rejected\"}}\ntruncated"
        ),
        PolicyState::Failed
    );
    assert_eq!(
        policy_state(true, true, b"{\"type\":\"summary\"}\n"),
        PolicyState::Passed
    );
    assert_eq!(
        policy_state(
            false,
            true,
            b"{\"fields\":{\"severity\":\"error\",\"code\":\"vulnerability\"}}\n"
        ),
        PolicyState::Failed
    );
    for (success, complete, bytes) in [
        (true, true, &b""[..]),
        (false, true, &b"missing tool"[..]),
        (true, false, &b"{}"[..]),
        (
            false,
            true,
            &b"{\"level\":\"ERROR\",\"fields\":{\"code\":\"index-failure\"}}"[..],
        ),
    ] {
        assert_eq!(policy_state(success, complete, bytes), PolicyState::Unknown);
    }
}

#[test]
fn provenance_schema_has_explicit_unsigned_scope_and_separate_unknown_policy() {
    let a = json!({"path":"target/fixture","bytes":1,"sha256":"a".repeat(64)});
    let inputs = (0..12)
        .map(|i| json!({"path":format!("target/input-{i}"),"bytes":1,"sha256":"a".repeat(64)}))
        .collect::<Vec<_>>();
    let value = json!({"schema":"imap075-development-provenance-v1","source_sha":"b".repeat(40),"source_clean":true,"production_enabled":false,"signature":"unsigned","inventory_valid":true,"capture":a,"inputs":inputs,"inventory":a,"sbom":a,"producer":a,"dependency_policy":{"state":"unknown","rust_state":"failed","java_state":"unknown","scope":"fixture","report":a}});
    validate_provenance_schema(&value).unwrap();
    for mode in 0..5 {
        let mut v = value.clone();
        match mode {
            0 => v["production_enabled"] = json!(true),
            1 => v["signature"] = json!("signed"),
            2 => v["schema"] = json!("future"),
            3 => v["inputs"] = json!([]),
            _ => v["passed"] = json!(true),
        }
        assert!(validate_provenance_schema(&v).is_err(), "{mode}");
    }
    for (source, expected) in [
        (
            include_bytes!(
                "../../../docs/testing/imap/0.75/schemas/vendor/cyclonedx-1.6/bom-1.6.schema.json"
            )
            .as_slice(),
            "1ebcb88a2c845ecb6ff7bee7aeabdff9422cb0347f3d6875b241bd444b7e098f",
        ),
        (
            include_bytes!(
                "../../../docs/testing/imap/0.75/schemas/vendor/cyclonedx-1.6/spdx.schema.json"
            )
            .as_slice(),
            "33863a360fc4d348e183d89c1ca7aa0877f481e7eb55be2d4102a30386188a10",
        ),
        (
            include_bytes!(
                "../../../docs/testing/imap/0.75/schemas/vendor/cyclonedx-1.6/jsf-0.82.schema.json"
            )
            .as_slice(),
            "2faf5eb3651f2ae5f46091a131770d8d847bbd121139d19c85fc7051bfa58c46",
        ),
    ] {
        assert_eq!(xtask::imap_qualification_075::digest(source), expected);
    }
}

#[test]
fn maven_build_commands_pin_repository_settings_and_scrub_ambient_options() {
    let root = tempfile::tempdir().unwrap();
    let command = xtask::imap_sbom_capture_075::maven_command(root.path()).unwrap();
    let args = command
        .get_args()
        .map(|a| a.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert!(args.contains(&"--strict-checksums".into()));
    assert!(args.contains(&"-Xmx512m".into()));
    assert!(args.contains(&"-XX:ActiveProcessorCount=2".into()));
    assert!(args.contains(&"org.codehaus.plexus.classworlds.launcher.Launcher".into()));
    assert!(args
        .iter()
        .any(|a| a.starts_with("-Dclassworlds.conf=") && a.ends_with("maven-classworlds.conf")));
    assert!(args.contains(&"-gs".into()));
    assert!(args.iter().any(|a| a.starts_with("-Dmaven.repo.local=")));
    for key in ["MAVEN_ARGS", "MAVEN_OPTS"] {
        assert_eq!(
            command.get_envs().find(|(k, _)| k == &key).map(|(_, v)| v),
            Some(None)
        );
    }
    assert_eq!(
        command
            .get_envs()
            .find(|(k, _)| k == &"MAVEN_SKIP_RC")
            .and_then(|(_, v)| v)
            .and_then(|v| v.to_str()),
        Some("1")
    );
}
#[test]
fn maven_distribution_resolution_does_not_treat_shims_as_a_complete_toolchain() {
    let root = tempfile::tempdir().unwrap();
    let bin = root.path().join("bin");
    std::fs::create_dir(&bin).unwrap();
    let launcher = bin.join(if cfg!(windows) { "mvn.cmd" } else { "mvn" });
    std::fs::write(&launcher, b"fixture launcher, never executed").unwrap();
    assert!(xtask::imap_sbom_capture_075::maven_home_from_launcher(&launcher).is_err());
    std::fs::create_dir(root.path().join("lib")).unwrap();
    std::fs::create_dir(root.path().join("boot")).unwrap();
    assert_eq!(
        xtask::imap_sbom_capture_075::maven_home_from_launcher(&launcher)
            .unwrap()
            .canonicalize()
            .unwrap(),
        root.path().canonicalize().unwrap()
    );
    assert!(
        xtask::imap_sbom_capture_075::maven_home_from_launcher(&root.path().join("missing"))
            .is_err()
    );
}

#[test]
fn fixed_maven_version_starts_inside_the_short_proof_memory_envelope() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let mut command = xtask::imap_sbom_capture_075::maven_command(root).unwrap();
    command.arg("--version");
    let result = xtask::imap_proof_process_075::execute(command, &[], Default::default()).unwrap();
    assert!(String::from_utf8(result.stdout)
        .unwrap()
        .contains("Apache Maven"));
    assert!(result.direct_child_reaped);
    assert_eq!(result.remaining_live_members, 0);
}

#[test]
#[ignore = "requires actual managed Cargo captures; IMAP075_CARGO_FIXTURE is an owned relative build directory"]
fn actual_host_filtered_cargo_graph_replays_compiler_feature_and_dependency_closure() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let directory = std::env::var("IMAP075_CARGO_FIXTURE").expect("explicit real fixture required");
    assert!(directory.starts_with("target/imap-075/builds/"));
    let read = |file: &str| {
        xtask::imap_qualification_075::read_relative(
            root,
            &format!("{directory}/{file}"),
            8 * 1024 * 1024,
        )
        .unwrap()
    };
    let root_metadata: Value =
        serde_json::from_slice(&read("17-ProvenanceMetadata.stdout")).unwrap();
    let consumer_metadata: Value =
        serde_json::from_slice(&read("9-ProvenanceMetadata.stdout")).unwrap();
    let locked = std::fs::read_to_string(root.join("Cargo.lock")).unwrap();
    for (name, metadata) in [
        ("5-ServerTests.stdout", &root_metadata),
        ("6-HelperContractTests.stdout", &root_metadata),
        ("7-RustConsumerTests.stdout", &consumer_metadata),
        ("8-RustConsumerBuild.stdout", &consumer_metadata),
        ("10-SoakWorkerBuild.stdout", &root_metadata),
    ] {
        let (components, edges, instances) =
            cargo_projection(metadata, &read(name), name, &locked, &"a".repeat(40))
                .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(components.len() > 10);
        assert!(!edges.is_empty());
        assert!(instances.len() >= components.len());
    }
}
#[test]
#[ignore = "requires a current real preparation; mutations are unqualified test fixtures only"]
fn actual_provenance_reconstruction_rejects_archive_and_policy_substitution() {
    use xtask::imap_extended_receipt_075::artifact;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let path = std::env::var("IMAP075_PROVENANCE_FIXTURE")
        .expect("explicit actual provenance fixture required");
    assert!(path.starts_with("target/imap-075/"));
    let original: Provenance = serde_json::from_slice(
        &xtask::imap_qualification_075::read_relative(root, &path, 8 * 1024 * 1024).unwrap(),
    )
    .unwrap();
    let scratch = tempfile::tempdir_in(root.join("target/imap-075")).unwrap();
    let fixture = scratch.path().join("provenance.json");
    let write = |name: &str, value: &Value| {
        let path = scratch.path().join(name);
        std::fs::write(&path, serde_json::to_vec(value).unwrap()).unwrap();
        artifact(root, &path, 8 * 1024 * 1024).unwrap()
    };
    // This test binary is a distinct verifier. Its fixtures explicitly bind its own
    // bytes, and never replace or qualify the real producer's provenance archive.
    let mut fixture_base = original;
    fixture_base.producer =
        artifact(root, &std::env::current_exe().unwrap(), 512 * 1024 * 1024).unwrap();
    std::fs::write(&fixture, serde_json::to_vec(&fixture_base).unwrap()).unwrap();
    verify(root, &fixture).unwrap();
    for mode in 0..8 {
        let mut changed = fixture_base.clone();
        let expected = match mode {
            0 => {
                changed.source_sha = "0".repeat(40);
                "foreign/stale/production"
            }
            1 => {
                let mut capture: Value = serde_json::from_slice(
                    &std::fs::read(root.join(&changed.capture.path)).unwrap(),
                )
                .unwrap();
                capture["source_sha"] = json!("0".repeat(40));
                changed.capture = write("changed-capture.json", &capture);
                "build-start source differs"
            }
            2 => {
                changed.inputs.pop();
                "missing/substituted capture inputs"
            }
            3 => {
                changed.inventory.sha256 = "0".repeat(64);
                "provenance input checksum/size"
            }
            4 => {
                let mut bom: Value =
                    serde_json::from_slice(&std::fs::read(root.join(&changed.sbom.path)).unwrap())
                        .unwrap();
                bom["components"][0]["name"] = json!("substituted-component");
                changed.sbom = write("changed-bom.json", &bom);
                "CycloneDX does not represent"
            }
            5 => {
                let mut policy: Value = serde_json::from_slice(
                    &std::fs::read(root.join(&changed.dependency_policy.report.path)).unwrap(),
                )
                .unwrap();
                policy["stdout"] = json!("substituted raw bytes");
                changed.dependency_policy.report = write("changed-policy.json", &policy);
                "policy raw report hash differs"
            }
            6 => {
                changed.dependency_policy.java_state = PolicyState::Passed;
                "policy promoted unknown/failure"
            }
            _ => {
                changed.dependency_policy.state = PolicyState::Passed;
                "policy promoted unknown/failure"
            }
        };
        std::fs::write(&fixture, serde_json::to_vec(&changed).unwrap()).unwrap();
        let error = verify(root, &fixture).unwrap_err().to_string();
        assert!(error.contains(expected), "{mode}: {error}");
    }
}
