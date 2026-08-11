use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{
    client_plane_bakeoff, client_plane_compat, client_plane_fault, client_plane_generation,
    client_plane_java, client_plane_python, client_plane_rust,
};

const CRATE: &str = "hydracache-client-plane-spike";
const JAVA_FIXTURE: &str = "crates/hydracache-client-plane-spike/java-fixture/pom.xml";

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if !args.is_empty() {
        return Err("client-plane-spike-check does not accept arguments".into());
    }
    let root = workspace_root()?;
    client_plane_bakeoff::check_at_root(&root)?;
    client_plane_generation::check_at_root(&root)?;
    check_rust_and_fixture(&root)?;
    client_plane_java::check_at_root(&root)?;
    client_plane_python::check_at_root(&root)?;
    client_plane_rust::check_at_root(&root)?;
    client_plane_compat::check_at_root(&root, false, true)?;
    client_plane_fault::check_at_root(&root, false)?;
    println!(
        "client-plane-spike-check: OK (production daemon + transport spikes + deterministic fault replay + native Rust/Java/Python SDK + complete retained compatibility matrix)"
    );
    Ok(())
}

pub fn run_docker(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if !args.is_empty() {
        return Err("client-plane-docker-interop-check does not accept arguments".into());
    }
    let root = workspace_root()?;
    client_plane_bakeoff::check_at_root(&root)?;
    client_plane_generation::check_at_root(&root)?;
    check_rust_and_fixture(&root)?;
    client_plane_java::check_at_root(&root)?;
    client_plane_python::check_at_root(&root)?;
    client_plane_fault::check_at_root(&root, false)?;
    println!(
        "client-plane-docker-interop-check: OK (production daemon + Rust tests + Java fixture/SDK consumer + offline Python + retained fault replay)"
    );
    Ok(())
}

fn check_rust_and_fixture(root: &Path) -> Result<(), Box<dyn Error>> {
    let target_dir = std::env::var("HC2_SHARED_TARGET_DIR")
        .unwrap_or_else(|_| "target/hc2-spike-check".to_owned());
    run_checked(
        root,
        "cargo",
        &[
            "test",
            "--locked",
            "-p",
            CRATE,
            "--all-targets",
            "--target-dir",
            &target_dir,
        ],
        "Rust HC/2 spike tests",
    )?;
    run_checked(
        root,
        "cargo",
        &[
            "test",
            "--locked",
            "-p",
            "hydracache-server",
            "--lib",
            "hc2::tests",
            "--target-dir",
            &target_dir,
        ],
        "production HC/2 listener unit/socket tests",
    )?;
    run_checked(
        root,
        "cargo",
        &[
            "test",
            "--locked",
            "-p",
            "hydracache-server",
            "--test",
            "hc2_daemon_process",
            "--target-dir",
            &target_dir,
        ],
        "production HC/2 daemon process tests",
    )?;
    run_checked(
        root,
        maven_program(),
        &["-B", "-ntp", "-f", JAVA_FIXTURE, "test"],
        "generated Java HC/2 golden fixture",
    )?;
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
