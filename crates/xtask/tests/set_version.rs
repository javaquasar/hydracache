//! Tests for `cargo xtask set-version`.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use xtask::set_version::set_version;

static SCRATCH_COUNTER: AtomicU64 = AtomicU64::new(0);

const CARGO_TOML: &str = r#"[workspace]
members = []

[workspace.package]
# release version
version = "0.74.1"
rust-version = "1.88"

[workspace.dependencies]
hydracache-core = { path = "crates/hydracache-core", version = "0.74.1" }
hydracache-sim = { path = "crates/hydracache-sim" }
serde = { version = "0.74.1" }
"#;

fn scratch() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let counter = SCRATCH_COUNTER.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "hydracache_set_version_{}_{nanos}_{counter}",
        std::process::id()
    ));
    let write = |relative: &str, body: &str| {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    };
    write("Cargo.toml", CARGO_TOML);
    write(
        "Dockerfile",
        "ARG RUST_IMAGE=rust:1.94.0-bookworm@sha256:abc\n",
    );
    write(
        ".github/workflows/ci.yml",
        "jobs:\n  rust:\n    steps:\n      - uses: dtolnay/rust-toolchain@1.94.0\n",
    );
    write(
        "deploy/helm/hydracache/Chart.yaml",
        "apiVersion: v2\nname: hydracache\nversion: 0.74.1\nappVersion: \"0.74.1\"\n",
    );
    write(
        "deploy/helm/hydracache/values.yaml",
        "image:\n  repository: ghcr.io/javaquasar/hydracache-server\n  tag: \"\"\n",
    );
    write(
        "deploy/helm/hydracache/templates/statefulset.yaml",
        "image: \"{{ .Values.image.repository }}:{{ .Values.image.tag | default .Chart.AppVersion }}\"\n",
    );
    write(
        "deploy/k8s/statefulset.yaml",
        "spec:\n  template:\n    spec:\n      containers:\n        - name: hydracache\n          image: ghcr.io/javaquasar/hydracache-server:0.74.1\n",
    );
    root
}

fn read(root: &Path, relative: &str) -> String {
    fs::read_to_string(root.join(relative)).unwrap()
}

#[test]
fn bumps_every_release_pin_and_keeps_layout() {
    let root = scratch();
    let (old, edits) = set_version(&root, "0.75.0-rc.1").unwrap();
    let cargo = read(&root, "Cargo.toml");
    let chart = read(&root, "deploy/helm/hydracache/Chart.yaml");
    let k8s = read(&root, "deploy/k8s/statefulset.yaml");
    fs::remove_dir_all(&root).unwrap();

    assert_eq!(old, "0.74.1");
    assert_eq!(edits.iter().map(|edit| edit.lines).sum::<usize>(), 5);
    assert!(cargo.contains("# release version\nversion = \"0.75.0-rc.1\"\n"));
    assert!(cargo.contains(
        "hydracache-core = { path = \"crates/hydracache-core\", version = \"0.75.0-rc.1\" }"
    ));
    // A third-party dependency that happens to share the number is untouched.
    assert!(cargo.contains("serde = { version = \"0.74.1\" }"));
    assert!(cargo.contains("rust-version = \"1.88\""));
    assert!(chart.contains("version: 0.75.0-rc.1\nappVersion: \"0.75.0-rc.1\"\n"));
    assert!(k8s.contains("image: ghcr.io/javaquasar/hydracache-server:0.75.0-rc.1\n"));
}

#[test]
fn rejects_invalid_version_without_touching_files() {
    let root = scratch();
    let err = set_version(&root, "v0.75").unwrap_err().to_string();
    let cargo = read(&root, "Cargo.toml");
    fs::remove_dir_all(&root).unwrap();
    assert!(err.contains("invalid version"), "{err}");
    assert_eq!(cargo, CARGO_TOML);
}

#[test]
fn malformed_deploy_file_leaves_tree_unchanged() {
    let root = scratch();
    fs::write(root.join("deploy/k8s/statefulset.yaml"), "spec: {}\n").unwrap();
    let err = set_version(&root, "0.75.0").unwrap_err().to_string();
    let cargo = read(&root, "Cargo.toml");
    let chart = read(&root, "deploy/helm/hydracache/Chart.yaml");
    fs::remove_dir_all(&root).unwrap();
    assert!(err.contains("deploy/k8s/statefulset.yaml"), "{err}");
    assert_eq!(cargo, CARGO_TOML);
    assert!(chart.contains("appVersion: \"0.74.1\""));
}
