//! Fixed-command capture of actual development build inputs. No receipt selects executable code.
use crate::imap_build_process_075::{BuildController, BuildStage};
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub const REPOSITORY: &str = "target/imap-075/maven-repository";
pub const SETTINGS: &str = "docs/testing/imap/0.75/maven-provenance-settings.xml";
pub const SPY: &str = "tests/java-imap-provenance-075/target/provenance-spy.jar";

/// Both original builds and clean-directory reproduction use this same owned repository/settings.
pub fn maven_command(root: &Path) -> Result<Command, Box<dyn Error>> {
    let repository = root.join(REPOSITORY);
    fs::create_dir_all(&repository)?;
    if !repository.canonicalize()?.starts_with(root.canonicalize()?) {
        return Err("foreign Maven repository refused".into());
    }
    let settings = root.join(SETTINGS);
    let mut command = Command::new(if cfg!(windows) { "mvn.cmd" } else { "mvn" });
    command
        .current_dir(root)
        .args(["-B", "-Dstyle.color=never", "--strict-checksums", "-s"])
        .arg(&settings)
        .arg("-gs")
        .arg(settings)
        .arg(format!("-Dmaven.repo.local={}", repository.display()));
    command
        .env_remove("MAVEN_ARGS")
        .env_remove("MAVEN_OPTS")
        .env("MAVEN_SKIP_RC", "1");
    Ok(command)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapturedStage {
    pub name: String,
    pub stdout: String,
    pub journal: Option<String>,
    pub nonce: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capture {
    pub source_sha: String,
    pub stages: Vec<CapturedStage>,
    pub host: String,
    pub rust_version: String,
    pub rustc_sha256: String,
    pub cargo_sha256: String,
    pub rust_inputs: Vec<serde_json::Value>,
    pub maven_home: String,
    pub directory: String,
}
fn relative(root: &Path, path: &Path) -> Result<String, Box<dyn Error>> {
    Ok(path
        .strip_prefix(root)?
        .to_string_lossy()
        .replace('\\', "/"))
}
impl Capture {
    pub fn start(root: &Path, builds: &mut BuildController) -> Result<Self, Box<dyn Error>> {
        let source_sha = crate::imap_ci_075::source_state(root)?;
        let mut command = Command::new("rustc");
        command.args(["-vV"]);
        let report = builds.run(BuildStage::ProvenanceTools, command)?;
        let rust_version = fs::read_to_string(&report.stdout_path)?;
        let host = rust_version
            .lines()
            .find_map(|line| line.strip_prefix("host: "))
            .ok_or("rustc host missing")?
            .to_owned();
        if host.len() > 128
            || !host
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        {
            return Err("rustc host bound".into());
        }
        let directory = relative(root, builds.directory())?;
        let (rustc_sha256, cargo_sha256) = rust_tools()?;
        let rust_inputs = rust_materials(&host)?;
        let mut capture = Self {
            source_sha,
            stages: vec![],
            host,
            rust_version,
            rustc_sha256,
            cargo_sha256,
            rust_inputs,
            maven_home: String::new(),
            directory,
        };
        capture.bootstrap(root, builds)?;
        Ok(capture)
    }
    fn bootstrap(
        &mut self,
        root: &Path,
        builds: &mut BuildController,
    ) -> Result<(), Box<dyn Error>> {
        let mut version = maven_command(root)?;
        version.arg("--version");
        let report = builds.run(BuildStage::ProvenanceTools, version)?;
        let output = fs::read_to_string(report.stdout_path)?;
        // Maven's version command has no settings, environment or system-property dump.
        let home = output
            .lines()
            .find_map(|line| line.strip_prefix("Maven home: "))
            .ok_or("Maven home missing")?;
        // Keep Maven's absolute spelling for Java classpaths. Windows canonicalize
        // returns a verbatim \\?\ prefix that the JVM wildcard loader does not expand.
        let maven_home = PathBuf::from(home.trim());
        if !maven_home.is_absolute() || !maven_home.is_dir() {
            return Err("Maven distribution path invalid".into());
        }
        self.maven_home = maven_home.to_string_lossy().into();
        let java = crate::imap_api_differential::java_executable();
        let jdk = java
            .parent()
            .and_then(Path::parent)
            .ok_or("JDK launcher layout")?;
        let target = builds.directory().join("provenance-classes");
        fs::create_dir(&target)?;
        fs::create_dir_all(root.join("tests/java-imap-provenance-075/target"))?;
        let package =
            root.join("tests/java-imap-provenance-075/src/main/java/io/hydracache/imap/provenance");
        let mut javac = Command::new(jdk.join(if cfg!(windows) {
            "bin/javac.exe"
        } else {
            "bin/javac"
        }));
        javac
            .args([
                "--release",
                "17",
                "-proc:none",
                "-encoding",
                "UTF-8",
                "-classpath",
            ])
            .arg(maven_home.join("lib/*"))
            .arg("-d")
            .arg(&target);
        for name in [
            "ProvenanceData075.java",
            "MavenResolutionSpy075.java",
            "MavenInventoryMain075.java",
        ] {
            javac.arg(package.join(name));
        }
        builds.run(BuildStage::ProvenanceTools, javac)?;
        let resource = target.join("META-INF/plexus");
        fs::create_dir_all(&resource)?;
        fs::copy(
            root.join(
                "tests/java-imap-provenance-075/src/main/resources/META-INF/plexus/components.xml",
            ),
            resource.join("components.xml"),
        )?;
        let mut jar = Command::new(jdk.join(if cfg!(windows) {
            "bin/jar.exe"
        } else {
            "bin/jar"
        }));
        jar.args(["--create", "--date=2026-10-29T00:00:00Z", "--file"])
            .arg(root.join(SPY))
            .arg("-C")
            .arg(target)
            .arg(".");
        builds.run(BuildStage::ProvenanceTools, jar)?;
        Ok(())
    }
    pub fn cargo(
        &mut self,
        root: &Path,
        builds: &mut BuildController,
        name: &str,
        stage: BuildStage,
        mut command: Command,
    ) -> Result<(), Box<dyn Error>> {
        command.arg("--message-format=json-render-diagnostics");
        trusted_cargo(&mut command);
        let report = builds.run(stage, command)?;
        self.stages.push(CapturedStage {
            name: name.into(),
            stdout: relative(root, Path::new(&report.stdout_path))?,
            journal: None,
            nonce: None,
        });
        Ok(())
    }
    pub fn metadata(
        &mut self,
        root: &Path,
        builds: &mut BuildController,
        manifest: Option<&Path>,
        name: &str,
    ) -> Result<(), Box<dyn Error>> {
        let mut command = Command::new("cargo");
        trusted_cargo(&mut command);
        command
            .current_dir(root)
            .args([
                "metadata",
                "--locked",
                "--format-version",
                "1",
                "--filter-platform",
            ])
            .arg(&self.host);
        if let Some(manifest) = manifest {
            command.arg("--manifest-path").arg(manifest);
        }
        let report = builds.run(BuildStage::ProvenanceMetadata, command)?;
        self.stages.push(CapturedStage {
            name: name.into(),
            stdout: relative(root, Path::new(&report.stdout_path))?,
            journal: None,
            nonce: None,
        });
        Ok(())
    }
    pub fn maven(
        &mut self,
        root: &Path,
        builds: &mut BuildController,
        name: &str,
        kind: &str,
        stage: BuildStage,
        args: &[&str],
    ) -> Result<(), Box<dyn Error>> {
        let directory = root
            .join(&self.directory)
            .join(format!("resolution-{}", self.stages.len()));
        fs::create_dir(&directory)?;
        let nonce = format!("{}-{}", std::process::id(), self.stages.len());
        let mut command = maven_command(root)?;
        command
            .arg(format!(
                "-Dmaven.ext.class.path={}",
                root.join(SPY).display()
            ))
            .arg(format!("-Dhydracache.imap075.root={}", root.display()))
            .arg(format!(
                "-Dhydracache.imap075.repository={}",
                root.join(REPOSITORY).display()
            ))
            .arg(format!(
                "-Dhydracache.imap075.output={}",
                directory.display()
            ))
            .arg(format!("-Dhydracache.imap075.nonce={nonce}"))
            .arg(format!("-Dhydracache.imap075.stage={kind}"))
            .args(args);
        let report = builds.run(stage, command)?;
        self.stages.push(CapturedStage {
            name: name.into(),
            stdout: relative(root, Path::new(&report.stdout_path))?,
            journal: Some(relative(root, &directory.join("maven-resolution.jsonl"))?),
            nonce: Some(nonce),
        });
        Ok(())
    }
    pub fn finish(
        mut self,
        root: &Path,
        builds: &mut BuildController,
    ) -> Result<(), Box<dyn Error>> {
        self.metadata(root, builds, None, "root-metadata")?;
        for (name, module) in [
            ("contract", "java-imap-benchmark"),
            ("native", "java-imap-hazelcast-075"),
        ] {
            let pom = format!("tests/{module}/pom.xml");
            let output = root.join(&self.directory).join(format!("{name}-tree.json"));
            let output_arg = format!("-DoutputFile={}", output.display());
            self.maven(
                root,
                builds,
                &format!("{name}-tree"),
                "dependency-tree",
                BuildStage::ProvenanceMetadata,
                &[
                    "-f",
                    &pom,
                    "org.apache.maven.plugins:maven-dependency-plugin:3.8.1:tree",
                    "-DoutputType=json",
                    &output_arg,
                ],
            )?;
        }
        self.maven(
            root,
            builds,
            "provenance-tests",
            "tooling-test",
            BuildStage::ProvenanceMetadata,
            &["-f", "tests/java-imap-provenance-075/pom.xml", "verify"],
        )?;
        fs::write(
            root.join(&self.directory).join("capture.json"),
            serde_json::to_vec_pretty(&self)?,
        )?;
        crate::imap_sbom_075::generate(root, &self)?;
        Ok(())
    }
}

pub fn rust_tools() -> Result<(String, String), Box<dyn Error>> {
    let mut command = Command::new("rustc");
    command.args(["--print", "sysroot"]);
    let output = crate::imap_proof_process_075::execute(command, &[], Default::default())?;
    let sysroot = PathBuf::from(String::from_utf8(output.stdout)?.trim());
    let suffix = if cfg!(windows) { ".exe" } else { "" };
    Ok((
        crate::imap_daemon_probe_075::binary_digest(&sysroot.join(format!("bin/rustc{suffix}")))?,
        crate::imap_daemon_probe_075::binary_digest(&sysroot.join(format!("bin/cargo{suffix}")))?,
    ))
}

pub fn rust_materials(host: &str) -> Result<Vec<serde_json::Value>, Box<dyn Error>> {
    if host.is_empty()
        || host.len() > 128
        || !host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err("Rust toolchain host bound".into());
    }
    let mut command = Command::new("rustc");
    command.args(["--print", "sysroot"]);
    let output = crate::imap_proof_process_075::execute(command, &[], Default::default())?;
    let sysroot = PathBuf::from(String::from_utf8(output.stdout)?.trim());
    let mut result = std::collections::BTreeMap::new();
    let mut total = 0u64;
    for directory in [
        "bin".to_owned(),
        "lib".into(),
        format!("lib/rustlib/{host}/lib"),
    ] {
        let mut examined = 0;
        for entry in fs::read_dir(sysroot.join(&directory))? {
            examined += 1;
            if examined > 1024 {
                return Err("Rust distribution directory bound".into());
            }
            let file = entry?.path();
            let name = file
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or("Rust tool filename")?;
            if !(name.ends_with(".dll")
                || name.ends_with(".dylib")
                || name.contains(".so")
                || name.ends_with(".rlib")
                || name.ends_with(".lib"))
            {
                continue;
            }
            let actual = file.canonicalize()?;
            let size = fs::metadata(&actual)?.len();
            total = total
                .checked_add(size)
                .ok_or("Rust distribution aggregate overflow")?;
            if total > 2 * 1024 * 1024 * 1024u64 || result.len() >= 512 {
                return Err("Rust distribution aggregate bound".into());
            }
            let path = format!("{directory}/{name}");
            let hash = crate::imap_daemon_probe_075::binary_digest(&actual)?;
            result.insert(path.clone(),serde_json::json!({"kind":"rust-distribution","path":path,"sha256":hash,"bytes":size}));
        }
    }
    if result.is_empty() {
        return Err("Rust compiler/stdlib inputs missing".into());
    }
    Ok(result.into_values().collect())
}
fn trusted_cargo(command: &mut Command) {
    for variable in [
        "RUSTC",
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "CARGO_BUILD_TARGET",
    ] {
        command.env_remove(variable);
    }
}
