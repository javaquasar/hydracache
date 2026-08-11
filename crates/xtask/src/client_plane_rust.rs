use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;

const SDK_CRATE: &str = "hydracache-client-hc2";
const PEER_CRATE: &str = "hydracache-client-plane-spike";
const SERVER_CRATE: &str = "hydracache-server";
const DEFAULT_TARGET_DIR: &str = "target/hc2-rust-sdk-check";

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if !args.is_empty() {
        return Err("client-plane-rust-sdk-check does not accept arguments".into());
    }
    check_at_root(&workspace_root()?)
}

pub fn check_at_root(root: &Path) -> Result<(), Box<dyn Error>> {
    let target_dir =
        std::env::var("HC2_SHARED_TARGET_DIR").unwrap_or_else(|_| DEFAULT_TARGET_DIR.to_owned());
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
            &target_dir,
        ],
        &[],
        "separate HC/2 conformance peer",
    )?;
    run_checked(
        root,
        &[
            "build",
            "--locked",
            "-p",
            SERVER_CRATE,
            "--bin",
            SERVER_CRATE,
            "--target-dir",
            &target_dir,
        ],
        &[],
        "production HC/2 daemon for Rust interop",
    )?;
    let peer = interop_server(root, &target_dir);
    if !peer.is_file() {
        return Err(format!("HC/2 conformance peer is missing: {}", peer.display()).into());
    }
    let peer = peer.to_string_lossy().into_owned();
    let daemon = production_daemon(root, &target_dir);
    if !daemon.is_file() {
        return Err(format!("production daemon is missing: {}", daemon.display()).into());
    }
    let daemon = daemon.to_string_lossy().into_owned();
    run_checked(
        root,
        &[
            "test",
            "--locked",
            "-p",
            SDK_CRATE,
            "--target-dir",
            &target_dir,
        ],
        &[
            ("HC2_RUST_INTEROP_SERVER", &peer),
            ("HC2_RUST_PRODUCTION_DAEMON", &daemon),
        ],
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
            &target_dir,
        ],
        &[],
        "unchanged HC/1 client conformance",
    )?;
    run_checked(
        root,
        &["package", "--locked", "-p", SDK_CRATE, "--allow-dirty"],
        &[],
        "publishable Rust HC/2 SDK package",
    )?;
    println!(
        "client-plane-rust-sdk-check: OK (conformance peer + production daemon mTLS/drain + recovery/bounds + HC/1 regression + package)"
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

fn production_daemon(root: &Path, target_dir: &str) -> PathBuf {
    let executable = if cfg!(windows) {
        "hydracache-server.exe"
    } else {
        "hydracache-server"
    };
    root.join(target_dir).join("debug").join(executable)
}

fn run_checked(
    root: &Path,
    args: &[&str],
    environment: &[(&str, &str)],
    label: &str,
) -> Result<(), Box<dyn Error>> {
    let mut command = Command::new("cargo");
    command.args(args).current_dir(root);
    for (name, value) in environment {
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
