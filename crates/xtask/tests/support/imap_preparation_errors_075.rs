use super::*;
use crate::imap_build_process_075::BuildStage;
struct Driver {
    calls: usize,
    fail_at: usize,
}
impl Driver {
    fn step(&mut self) -> Result<(), Box<dyn Error>> {
        self.calls += 1;
        if self.calls == self.fail_at {
            Err("unit preparation command refused".into())
        } else {
            Ok(())
        }
    }
}
impl PreparationCommands for Driver {
    fn command(
        &mut self,
        _: &Path,
        stage: BuildStage,
        program: &str,
        args: &[&str],
    ) -> Result<(), Box<dyn Error>> {
        assert_eq!(program, "cargo");
        assert!(matches!(
            stage,
            BuildStage::HelperContractTests | BuildStage::SoakWorkerBuild
        ));
        assert!(args.contains(&"--locked"));
        self.step()
    }
    fn cargo(
        &mut self,
        _: &Path,
        name: &str,
        stage: BuildStage,
        command: Command,
    ) -> Result<(), Box<dyn Error>> {
        assert!(matches!(
            stage,
            BuildStage::RustConsumerTests | BuildStage::RustConsumerBuild
        ));
        assert!(name == "consumer-test" || name == "consumer-build");
        assert!(command
            .get_current_dir()
            .unwrap()
            .join("Cargo.lock")
            .is_file());
        assert!(command.get_args().any(|a| a == "--offline"));
        self.step()
    }
    fn metadata(
        &mut self,
        _: &Path,
        manifest: Option<&Path>,
        name: &str,
    ) -> Result<(), Box<dyn Error>> {
        assert_eq!(name, "consumer-metadata");
        assert!(manifest.unwrap().is_file());
        self.step()
    }
    fn consumer(&mut self, _: &Path) -> Result<(), Box<dyn Error>> {
        self.step()
    }
}
#[test]
fn each_reference_build_and_consumer_metadata_refusal_stops_before_receipt_publication() {
    for fail_at in 1..=3 {
        let root = tempfile::tempdir().unwrap();
        let mut driver = Driver { calls: 0, fail_at };
        assert!(reference_tool_builds(root.path(), &mut driver)
            .unwrap_err()
            .to_string()
            .contains("unit preparation command refused"));
        assert_eq!(driver.calls, fail_at);
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }
    for fail_at in 1..=3 {
        let root = tempfile::tempdir().unwrap();
        let source = root
            .path()
            .join("tests/rust-imap-contract-consumer-075/src");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("main.rs"), b"unit source; never compiled").unwrap();
        fs::write(
            root.path().join("Cargo.lock"),
            include_str!("../../../../Cargo.lock"),
        )
        .unwrap();
        let mut driver = Driver { calls: 0, fail_at };
        assert!(rust_consumer_using(root.path(), &mut driver)
            .unwrap_err()
            .to_string()
            .contains("unit preparation command refused"));
        assert_eq!(driver.calls, fail_at);
        assert_eq!(
            fs::read_dir(root.path().join("target/imap-075"))
                .unwrap()
                .count(),
            0
        );
        assert!(!root.path().join("receipt.json").exists());
    }
}
#[test]
fn retained_inventory_missing_inputs_and_conflicts_never_overwrite_existing_bytes() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("out");
    fs::create_dir(&directory).unwrap();
    assert!(retain_development_inventory(root.path(), &directory).is_err());
    let latest = root.path().join(crate::imap_sbom_075::LATEST);
    fs::create_dir_all(latest.parent().unwrap()).unwrap();
    fs::write(latest, b"unit bytes; not provenance").unwrap();
    let retained = retain_development_inventory(root.path(), &directory).unwrap();
    assert_eq!(
        fs::read(root.path().join(retained.path)).unwrap(),
        b"unit bytes; not provenance"
    );
    assert!(retain_development_inventory(root.path(), &directory).is_err());
    assert_eq!(
        fs::read(directory.join("development-provenance.json")).unwrap(),
        b"unit bytes; not provenance"
    );
    assert!(!root.path().join("receipt.json").exists());
}
