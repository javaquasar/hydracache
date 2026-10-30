use super::*;
use serde_json::json;
const POLICY: &str = include_str!("../../../../docs/testing/imap/0.75/coverage-profile.json");
struct Host {
    root: tempfile::TempDir,
    calls: usize,
    fail_at: usize,
    omit: Option<String>,
    bad_version: Option<String>,
    rust: Vec<Value>,
}
impl Host {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        for name in ["cargo", "java", "llvm-cov", "llvm-profdata", "rustc"] {
            let file = root.path().join("bin").join(name);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, b"unit bytes; never executed as a tool").unwrap();
        }
        for relative in [
            "release",
            "lib/modules",
            jvm_relative(std::env::consts::OS, std::env::consts::ARCH).unwrap(),
        ] {
            let file = root.path().join(relative);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, b"unit bytes; never a JVM").unwrap();
        }
        Self {
            root,
            calls: 0,
            fail_at: 0,
            omit: None,
            bad_version: None,
            rust: vec![json!({"path":"unit.rlib","sha256":"0".repeat(64),"bytes":1})],
        }
    }
    fn step(&mut self) -> Result<()> {
        self.calls += 1;
        if self.calls == self.fail_at {
            Err("unit profile input refused".into())
        } else {
            Ok(())
        }
    }
}
impl ProfileHost for Host {
    fn tools(&mut self) -> Result<BTreeMap<String, PathBuf>> {
        self.step()?;
        Ok(["cargo", "java", "llvm-cov", "llvm-profdata", "rustc"]
            .into_iter()
            .filter(|name| self.omit.as_deref() != Some(*name))
            .map(|name| (name.into(), self.root.path().join("bin").join(name)))
            .collect())
    }
    fn fingerprint(&mut self, path: &Path, args: &[&str]) -> Result<String> {
        self.step()?;
        let name = path.file_name().unwrap().to_str().unwrap();
        assert_eq!(
            args,
            match name {
                "java" => vec![
                    "-Xms16m",
                    "-Xmx64m",
                    "-XX:ActiveProcessorCount=2",
                    "--version"
                ],
                "rustc" => vec!["-vV"],
                _ => vec!["--version"],
            }
        );
        Ok(if self.bad_version.as_deref() == Some(name) {
            "foreign".into()
        } else {
            match name {
                "java" => "openjdk 21.0.1".into(),
                "rustc" => "rustc\nrelease: 1.94.0".into(),
                _ => "unit version".into(),
            }
        })
    }
    fn java(&mut self) -> Result<PathBuf> {
        self.step()?;
        Ok(self.root.path().join("bin/java"))
    }
    fn target(&mut self) -> Result<crate::imap_sbom_075::CargoTarget> {
        self.step()?;
        Ok(crate::imap_sbom_075::CargoTarget {
            triple: "unit-unqualified".into(),
            cfg: Default::default(),
        })
    }
    fn rust_materials(&mut self, host: &str) -> Result<Vec<Value>> {
        self.step()?;
        assert_eq!(host, "unit-unqualified");
        Ok(self.rust.clone())
    }
}
#[test]
fn each_fixed_profile_input_failure_stops_before_later_hashing_or_publication() {
    for fail_at in 1..=10 {
        let mut host = Host {
            fail_at,
            ..Host::new()
        };
        assert!(current_profile_using(POLICY, &mut host)
            .unwrap_err()
            .to_string()
            .contains("unit profile input refused"));
        assert_eq!(host.calls, fail_at);
        assert!(!host.root.path().join("control.json").exists());
        assert!(!host.root.path().join("report.json").exists());
    }
    let mut host = Host::new();
    let profile = current_profile_using(POLICY, &mut host).unwrap();
    assert_eq!(host.calls, 10);
    assert_eq!(profile.tools.len(), 9);
    assert!(!host.root.path().join("capture.json").exists());
}
#[test]
fn policy_parse_and_validation_fail_before_any_tool_is_observed() {
    for policy in ["{", "{}"] {
        let mut host = Host::new();
        assert!(current_profile_using(policy, &mut host).is_err());
        assert_eq!(host.calls, 0);
    }
}
#[test]
fn missing_tools_unadmitted_versions_and_incomplete_jdk_files_never_form_profiles() {
    for name in ["java", "rustc"] {
        let mut host = Host {
            omit: Some(name.into()),
            ..Host::new()
        };
        assert!(current_profile_using(POLICY, &mut host).is_err());
        let mut host = Host {
            bad_version: Some(name.into()),
            ..Host::new()
        };
        assert!(current_profile_using(POLICY, &mut host).is_err());
    }
    for path in [
        "release",
        "lib/modules",
        jvm_relative(std::env::consts::OS, std::env::consts::ARCH).unwrap(),
    ] {
        let mut host = Host::new();
        fs::remove_file(host.root.path().join(path)).unwrap();
        assert!(current_profile_using(POLICY, &mut host).is_err());
    }
    let mut host = Host::new();
    fs::remove_file(host.root.path().join("bin/cargo")).unwrap();
    assert!(current_profile_using(POLICY, &mut host).is_err());
}
#[test]
fn malformed_rust_distribution_rows_refuse_at_each_required_field() {
    for field in ["path", "sha256", "bytes"] {
        let mut host = Host::new();
        host.rust[0].as_object_mut().unwrap().remove(field);
        assert!(current_profile_using(POLICY, &mut host).is_err());
        assert_eq!(host.calls, 9);
    }
}
#[test]
fn actual_bounded_fingerprint_refuses_missing_and_failed_tools_and_retains_success_output() {
    let root = tempfile::tempdir().unwrap();
    assert!(fingerprint(&root.path().join("missing-tool"), &["--version"]).is_err());
    if cfg!(windows) {
        assert!(fingerprint(Path::new("cmd"), &["/d", "/c", "exit 7"]).is_err());
        assert_eq!(
            fingerprint(Path::new("cmd"), &["/d", "/c", "echo imap075"]).unwrap(),
            "imap075"
        );
    } else {
        assert!(fingerprint(Path::new("sh"), &["-c", "exit 7"]).is_err());
        assert_eq!(
            fingerprint(Path::new("sh"), &["-c", "printf imap075"]).unwrap(),
            "imap075"
        );
    }
}
