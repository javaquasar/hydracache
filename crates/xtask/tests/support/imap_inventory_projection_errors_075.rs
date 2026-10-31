use super::*;
use crate::imap_sbom_capture_075::CapturedStage;
struct Observations {
    home: PathBuf,
    pom_calls: usize,
}
impl InventoryObservations for Observations {
    fn target(&mut self) -> Result<CargoTarget> {
        Ok(CargoTarget {
            triple: "unit-host".into(),
            cfg: Default::default(),
        })
    }
    fn rust_tools(&mut self) -> Result<(String, String)> {
        Ok(("a".repeat(64), "b".repeat(64)))
    }
    fn rust_materials(&mut self, _: &str) -> Result<Vec<Value>> {
        Ok(vec![])
    }
    fn output(&mut self, command: Command) -> Result<Vec<u8>> {
        Ok(if command.get_program() == "rustc" {
            b"host: unit-host\n".to_vec()
        } else {
            assert_eq!(command.get_program(), "unit-maven-never-executed");
            format!("Maven home: {}\n", self.home.display()).into_bytes()
        })
    }
    fn java(&mut self) -> PathBuf {
        self.home.join("jdk/bin/java")
    }
    fn maven(&mut self, _: &Path) -> Result<Command> {
        Ok(Command::new("unit-maven-never-executed"))
    }
    fn poms(&mut self, _: &Path, _: &BTreeSet<String>) -> Result<Value> {
        self.pom_calls += 1;
        let mut result = json!({});
        for module in ["java-imap-benchmark", "java-imap-hazelcast-075"] {
            result[format!("tests/{module}/pom.xml")] =
                json!({"licenses":[],"source_url":"","license_origin":"unknown"});
        }
        Ok(result)
    }
}
fn fixture(mode: usize) -> (tempfile::TempDir, Capture, Observations) {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("Cargo.lock"),
        b"version = 4\npackage = []\n",
    )
    .unwrap();
    fs::write(root.path().join("metadata.json"), b"{\"packages\":[]}").unwrap();
    let home = root.path().join("maven");
    fs::create_dir(&home).unwrap();
    let mut stages = vec![
        CapturedStage {
            name: "root-metadata".into(),
            stdout: "metadata.json".into(),
            journal: None,
            nonce: None,
        },
        CapturedStage {
            name: "consumer-metadata".into(),
            stdout: "metadata.json".into(),
            journal: None,
            nonce: None,
        },
    ];
    if mode == 1 {
        fs::write(root.path().join("compiler.stdout"), b"{}\n").unwrap();
        stages.push(CapturedStage {
            name: "tooling-unit".into(),
            stdout: "compiler.stdout".into(),
            journal: None,
            nonce: None,
        });
    } else {
        let mut rows = vec![
            json!({"event":"init","schema":"imap075-maven-resolution-v1","nonce":"unit","stage":"artifact-build"}),
            json!({"event":"tool","kind":"unit-not-an-installed-tool","path":"unit","sha256":"0".repeat(64)}),
        ];
        for (index, bytes) in if mode == 2 {
            vec![b"first".as_slice(), b"second".as_slice()]
        } else {
            vec![b"first".as_slice()]
        }
        .into_iter()
        .enumerate()
        {
            let relative = format!("unit/material-{index}.jar");
            let file = root.path().join(REPOSITORY).join(&relative);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(&file, bytes).unwrap();
            rows.push(json!({"event":"material","id":"unit:library:jar::1","group":"unit","name":"library","version":"1","extension":"jar","classifier":"","location":"repository","path":relative,"sha256":crate::imap_daemon_probe_075::binary_digest(&file).unwrap(),"bytes":bytes.len()}));
        }
        let materials = rows.len() - 2;
        rows.push(json!({"event":"terminal","nonce":"unit","stage":"artifact-build","complete":true,"records":rows.len(),"materials":materials}));
        let journal = if mode == 0 {
            b"{}\n".to_vec()
        } else {
            rows.iter()
                .map(|r| format!("{r}\n"))
                .collect::<String>()
                .into_bytes()
        };
        fs::write(root.path().join("journal.jsonl"), journal).unwrap();
        stages.push(CapturedStage {
            name: "JavaContract".into(),
            stdout: "unused".into(),
            journal: Some("journal.jsonl".into()),
            nonce: Some("unit".into()),
        });
    }
    if mode == 4 {
        for module in ["java-imap-benchmark", "java-imap-hazelcast-075"] {
            let file = root.path().join(format!(
                "tests/{module}/target/{module}-0.75.0-SNAPSHOT.jar"
            ));
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, b"unit bytes; never a JAR").unwrap();
        }
        fs::create_dir(root.path().join("capture")).unwrap();
        for name in ["contract", "native"] {
            fs::write(
                root.path().join(format!("capture/{name}-tree.json")),
                b"{\"unit\":\"not a dependency tree\"}",
            )
            .unwrap();
        }
    }
    let capture = Capture {
        source_sha: "0".repeat(40),
        stages,
        host: "unit-host".into(),
        rust_version: "host: unit-host\n".into(),
        rustc_sha256: "a".repeat(64),
        cargo_sha256: "b".repeat(64),
        rust_inputs: vec![],
        maven_home: home.to_string_lossy().into_owned(),
        directory: "capture".into(),
    };
    (root, capture, Observations { home, pom_calls: 0 })
}
#[test]
fn reducer_propagates_journal_cargo_component_root_and_tree_errors_without_inventory() {
    assert!(lock_checksums("version = 4\n").is_err());
    for mode in 0..5 {
        let (root, capture, mut host) = fixture(mode);
        let error = reduce_using(root.path(), &capture, &"0".repeat(40), &mut host).unwrap_err();
        assert_eq!(host.pom_calls, usize::from(mode >= 2), "{mode}: {error}");
        if mode == 0 {
            assert!(error.to_string().contains("Maven"));
        }
        if mode == 2 {
            assert!(error.to_string().contains("component"), "{error}");
        }
        if mode == 3 {
            assert_eq!(
                error.downcast_ref::<std::io::Error>().unwrap().kind(),
                std::io::ErrorKind::NotFound
            );
        }
        if mode == 4 {
            assert!(error.to_string().contains("groupId"), "{error}");
        }
        assert!(!root.path().join("capture/inventory.json").exists());
        assert!(!root.path().join("capture/provenance.json").exists());
    }
}
