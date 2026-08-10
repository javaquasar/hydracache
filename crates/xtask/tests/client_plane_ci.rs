use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{json, Value};

static NEXT: AtomicU64 = AtomicU64::new(1);
const COMMIT: &str = "1111111111111111111111111111111111111111";
const INTEROP_IMAGE: &str =
    "ubuntu:24.04@sha256:561618e2c15bf2397621dd04f96926663a3b5616c189cf7e38db7e82f5c538ea";

struct Scratch(PathBuf);

impl Scratch {
    fn new(label: &str) -> Self {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = workspace_root()
            .join("target/hc2-ci-test")
            .join(format!("{label}-{}-{id}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn complete_same_commit_receipts_are_admitted() {
    let scratch = Scratch::new("pass");
    write_all(&scratch.0, COMMIT, None);
    assert!(admission(&scratch.0, COMMIT).success());
}

#[test]
fn intentional_missing_and_red_canaries_are_rejected() {
    let missing = Scratch::new("missing");
    write_all(&missing.0, COMMIT, Some("fixed-host-soak"));
    assert!(!admission(&missing.0, COMMIT).success());

    let red = Scratch::new("red");
    write_all(&red.0, COMMIT, None);
    let path = red.0.join("fuzz.receipt.json");
    let mut receipt: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    receipt["outcome"] = json!("fail");
    fs::write(&path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    assert!(!admission(&red.0, COMMIT).success());
}

#[test]
fn intentional_mixed_commit_canary_is_rejected() {
    let scratch = Scratch::new("mixed");
    write_all(&scratch.0, COMMIT, None);
    let path = scratch.0.join("docker-interop.receipt.json");
    let mut receipt: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    receipt["commit"] = json!("2222222222222222222222222222222222222222");
    fs::write(&path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    assert!(!admission(&scratch.0, COMMIT).success());
}

#[test]
fn intentional_wrong_interop_image_canary_is_rejected() {
    let scratch = Scratch::new("wrong-image");
    write_all(&scratch.0, COMMIT, None);
    let path = scratch.0.join("docker-interop.receipt.json");
    let mut receipt: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    receipt["image"] = json!(format!("ubuntu:24.04@sha256:{}", "a".repeat(64)));
    fs::write(&path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    assert!(!admission(&scratch.0, COMMIT).success());
}

fn write_all(directory: &Path, commit: &str, omit: Option<&str>) {
    for lane in [
        "linux-required",
        "docker-interop",
        "fuzz",
        "fixed-host-soak",
    ] {
        if omit == Some(lane) {
            continue;
        }
        let profile = if lane == "fixed-host-soak" {
            "hc2-fixed-soak-v1"
        } else {
            "hc2-correctness-v1"
        };
        let receipt = json!({
            "schema_version": "hydracache.hc2.ci-receipt.v1",
            "lane": lane,
            "outcome": "pass",
            "commit": commit,
            "run_id": "77",
            "run_attempt": "1",
            "runner_os": "Linux",
            "runner_arch": "X64",
            "runner_name": "test-runner",
            "profile": profile,
            "seed": if lane == "fuzz" { Some(22_u64) } else { None },
            "iterations": if lane == "fixed-host-soak" { Some(8_u64) } else { None },
            "image": if lane == "docker-interop" {
                Some(INTEROP_IMAGE.to_owned())
            } else {
                None
            }
        });
        fs::write(
            directory.join(format!("{lane}.receipt.json")),
            serde_json::to_vec_pretty(&receipt).unwrap(),
        )
        .unwrap();
    }
}

fn admission(directory: &Path, commit: &str) -> std::process::ExitStatus {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args([
            "client-plane-ci-admission",
            "--receipts",
            directory.to_str().unwrap(),
            "--commit",
            commit,
        ])
        .current_dir(workspace_root())
        .status()
        .unwrap()
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}
