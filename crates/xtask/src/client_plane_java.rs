use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;

const CRATE: &str = "hydracache-client-plane-spike";
const SDK_POM: &str = "sdks/java/hydracache-client-hc2/pom.xml";
const CONSUMER_POM: &str = "tests/java-hc2-consumer/pom.xml";
const DEFAULT_TARGET_DIR: &str = "target/hc2-java-sdk-check";

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if !args.is_empty() {
        return Err("client-plane-java-sdk-check does not accept arguments".into());
    }
    check_at_root(&workspace_root()?)
}

pub fn check_at_root(root: &Path) -> Result<(), Box<dyn Error>> {
    let target_dir =
        std::env::var("HC2_SHARED_TARGET_DIR").unwrap_or_else(|_| DEFAULT_TARGET_DIR.to_owned());
    run_checked(
        root,
        "cargo",
        &[
            "build",
            "--locked",
            "-p",
            CRATE,
            "--bin",
            "hc2_java_interop_server",
            "--target-dir",
            &target_dir,
        ],
        None,
        "separate Rust HC/2 Java interop server",
    )?;
    let server = interop_server(root, &target_dir);
    if !server.is_file() {
        return Err(format!("interop server binary is missing: {}", server.display()).into());
    }
    let server = server.to_string_lossy().into_owned();
    run_checked(
        root,
        maven_program(),
        &["-B", "-ntp", "-f", SDK_POM, "install"],
        Some(("HC2_JAVA_INTEROP_SERVER", &server)),
        "publishable Java HC/2 SDK",
    )?;
    run_checked(
        root,
        maven_program(),
        &["-B", "-ntp", "-f", CONSUMER_POM, "verify"],
        Some(("HC2_JAVA_INTEROP_SERVER", &server)),
        "external Java HC/2 consumer",
    )?;
    println!(
        "client-plane-java-sdk-check: OK (SDK package + separate Rust process + installed consumer)"
    );
    Ok(())
}

fn interop_server(root: &Path, target_dir: &str) -> PathBuf {
    let executable = if cfg!(windows) {
        "hc2_java_interop_server.exe"
    } else {
        "hc2_java_interop_server"
    };
    root.join(target_dir).join("debug").join(executable)
}

fn maven_program() -> &'static str {
    if cfg!(windows) {
        "mvn.cmd"
    } else {
        "mvn"
    }
}

fn run_checked(
    root: &Path,
    program: &str,
    args: &[&str],
    environment: Option<(&str, &str)>,
    label: &str,
) -> Result<(), Box<dyn Error>> {
    let mut command = Command::new(program);
    command.args(args).current_dir(root);
    if let Some((name, value)) = environment {
        command.env(name, value);
    }
    let status = command
        .status()
        .map_err(|error| format!("starting {label} with {program}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{label} failed with {status}").into())
    }
}

fn workspace_root() -> Result<PathBuf, Box<dyn Error>> {
    let mut candidate = std::env::current_dir()?;
    loop {
        if candidate.join("Cargo.toml").is_file() && candidate.join("crates").join(CRATE).is_dir() {
            return Ok(candidate);
        }
        if !candidate.pop() {
            return Err("could not locate HydraCache workspace root".into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sdk_and_consumer_are_independent_maven_projects() {
        let root = workspace_root().unwrap();
        assert!(root.join(SDK_POM).is_file());
        assert!(root.join(CONSUMER_POM).is_file());
        assert_ne!(
            root.join(SDK_POM).parent(),
            root.join(CONSUMER_POM).parent()
        );
    }
}
