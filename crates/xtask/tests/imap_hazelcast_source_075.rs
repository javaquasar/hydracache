use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use xtask::imap_hazelcast_source::{check_at_root, check_map_file};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}

#[test]
fn pinned_source_map_is_complete() {
    let problems = check_at_root(&root(), "0.75", None).unwrap();
    assert!(problems.is_empty(), "{problems:#?}");
}

#[test]
fn local_pinned_checkout_matches_every_digest_when_present() {
    let upstream = root().parent().unwrap().join("hazelcast");
    if upstream.join(".git").exists() {
        let problems = check_at_root(&root(), "0.75", Some(&upstream)).unwrap();
        assert!(problems.is_empty(), "{problems:#?}");
    }
}

#[test]
fn missing_adapted_test_id_and_non_adoption_note_fail_closed() {
    let workspace = root();
    let source = workspace.join("docs/testing/imap/0.75/hazelcast-source-map.toml");
    let text = fs::read_to_string(source).unwrap();
    let text = text.replacen(
        "adapted_test_ids = [\"HC-IMAP-075-BOUND-ADMISSION\"]",
        "adapted_test_ids = []",
        1,
    ).replacen(
        "non_adopted = \"Hazelcast property names and runtime defaults are not copied as HydraCache compatibility promises\"",
        "non_adopted = \"\"",
        1,
    );
    let temp = tempdir().unwrap();
    let map = temp.path().join("source-map.toml");
    fs::write(&map, text).unwrap();
    let problems = check_map_file(&workspace, &map, None).unwrap();
    assert!(problems
        .iter()
        .any(|problem| problem.contains("adapted test id")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("non-adopted note")));
}
