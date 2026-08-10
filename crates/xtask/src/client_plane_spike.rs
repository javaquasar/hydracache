use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{client_plane_java, client_plane_python, client_plane_rust};

const CRATE: &str = "hydracache-client-plane-spike";
const JAVA_FIXTURE: &str = "crates/hydracache-client-plane-spike/java-fixture/pom.xml";

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if !args.is_empty() {
        return Err("client-plane-spike-check does not accept arguments".into());
    }
    let root = workspace_root()?;
    run_checked(
        &root,
        "cargo",
        &[
            "test",
            "--locked",
            "-p",
            CRATE,
            "--target-dir",
            "target/hc2-spike-check",
        ],
        "Rust HC/2 spike tests",
    )?;
    run_checked(
        &root,
        maven_program(),
        &["-B", "-ntp", "-f", JAVA_FIXTURE, "test"],
        "generated Java HC/2 golden fixture",
    )?;
    client_plane_java::check_at_root(&root)?;
    client_plane_python::check_at_root(&root)?;
    client_plane_rust::check_at_root(&root)?;
    println!(
        "client-plane-spike-check: OK (transport spikes + native Rust/Java/Python SDK evidence)"
    );
    Ok(())
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
    label: &str,
) -> Result<(), Box<dyn Error>> {
    let status = Command::new(program)
        .args(args)
        .current_dir(root)
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
    fn fixture_paths_are_workspace_relative_and_present() {
        let root = workspace_root().unwrap();
        assert!(root.join("crates").join(CRATE).join("Cargo.toml").is_file());
        assert!(root.join(JAVA_FIXTURE).is_file());
    }
}
