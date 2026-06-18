use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate should live under crates/hydracache-db")
        .to_path_buf()
}

fn read_doc(path: impl AsRef<Path>) -> String {
    std::fs::read_to_string(path.as_ref())
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.as_ref().display()))
}

#[test]
fn release_37_adr_skeleton_exists_with_required_headings() {
    let root = repo_root();
    for file in [
        "docs/adr/0001-ownership.md",
        "docs/adr/0002-replication.md",
        "docs/adr/0003-consistency.md",
        "docs/adr/0004-transport.md",
        "docs/adr/0005-durability.md",
    ] {
        let contents = read_doc(root.join(file));
        assert!(
            contents.starts_with("# ADR-"),
            "{file} should start with an ADR title"
        );
        for heading in ["## Status", "## Context", "## Decision", "## Consequences"] {
            assert!(
                contents.contains(heading),
                "{file} is missing required heading {heading}"
            );
        }
    }
}

#[test]
fn compat_register_tracks_release_37_durable_and_wire_artifacts() {
    let contents = read_doc(repo_root().join("docs/COMPAT.md"));

    for required in [
        "CacheInvalidationFrame",
        "hydracache_invalidation_outbox",
        "Unknown future schema versions fail closed",
        "Unknown wire versions are treated as decode errors",
    ] {
        assert!(
            contents.contains(required),
            "COMPAT register is missing `{required}`"
        );
    }
}
