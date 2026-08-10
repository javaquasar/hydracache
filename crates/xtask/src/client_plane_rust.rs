use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;

const SDK_CRATE: &str = "hydracache-client-hc2";
const PEER_CRATE: &str = "hydracache-client-plane-spike";
const TARGET_DIR: &str = "target/hc2-rust-sdk-check";

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if !args.is_empty() {
        return Err("client-plane-rust-sdk-check does not accept arguments".into());
    }
    check_at_root(&workspace_root()?)
}

pub fn check_at_root(root: &Path) -> Result<(), Box<dyn Error>> {
    run_checked(
        root,
        &[
            "build",
            "--locked",
            "-p",
            PEER_CRATE,
            "--bin",
            "hc2_java_interop_server",
            "--target-dir",
            TARGET_DIR,
        ],
        None,
        "separate HC/2 conformance peer",
    )?;
    let peer = interop_server(root);
    if !peer.is_file() {
        return Err(format!("HC/2 conformance peer is missing: {}", peer.display()).into());
    }
    let peer = peer.to_string_lossy().into_owned();
    run_checked(
        root,
        &[
            "test",
            "--locked",
            "-p",
            SDK_CRATE,
            "--target-dir",
            TARGET_DIR,
        ],
        Some(("HC2_RUST_INTEROP_SERVER", &peer)),
        "native Rust HC/2 SDK",
    )?;
    run_checked(
        root,
        &[
            "test",
            "--locked",
            "-p",
            "hydracache-client",
            "--test",
            "conformance",
            "--target-dir",
            TARGET_DIR,
        ],
        None,
        "unchanged HC/1 client conformance",
    )?;
    run_checked(
        root,
        &["package", "--locked", "-p", SDK_CRATE, "--allow-dirty"],
        None,
        "publishable Rust HC/2 SDK package",
    )?;
    println!(
        "client-plane-rust-sdk-check: OK (mTLS process proof + cancellation/bounds + HC/1 regression + package)"
    );
    Ok(())
}

fn interop_server(root: &Path) -> PathBuf {
    let executable = if cfg!(windows) {
        "hc2_java_interop_server.exe"
    } else {
        "hc2_java_interop_server"
    };
    root.join(TARGET_DIR).join("debug").join(executable)
}

fn run_checked(
    root: &Path,
    args: &[&str],
    environment: Option<(&str, &str)>,
    label: &str,
) -> Result<(), Box<dyn Error>> {
    let mut command = Command::new("cargo");
    command.args(args).current_dir(root);
    if let Some((name, value)) = environment {
        command.env(name, value);
    }
    let status = command
        .status()
        .map_err(|error| format!("starting {label}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{label} failed with {status}").into())
    }
}

fn workspace_root() -> Result<PathBuf, Box<dyn Error>> {
    let mut candidate = std::env::current_dir()?;
    loop {
        if candidate.join("Cargo.toml").is_file()
            && candidate.join("crates").join(SDK_CRATE).is_dir()
        {
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
    fn production_sdk_owns_the_contract_and_hc1_remains_separate() {
        let root = workspace_root().unwrap();
        assert!(root
            .join("crates/hydracache-client-hc2/proto/hc2_contract.proto")
            .is_file());
        assert!(root.join("crates/hydracache-client/Cargo.toml").is_file());
    }
}
