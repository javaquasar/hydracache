use std::path::{Path, PathBuf};
use tempfile::tempdir;
use xtask::imap_foundation_evidence::{check_at_root, check_receipt_set_at_root};
use xtask::imap_foundation_generate::generate_at_root;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}

#[test]
fn generator_executes_and_validates_all_four_evidence_kinds() {
    let temp = tempdir().unwrap();
    let output = temp.path().join("evidence");
    let receipts = generate_at_root(&root(), &output, 117).unwrap();
    assert_eq!(receipts.len(), 4);
    for receipt in &receipts {
        assert!(receipt.is_file());
        let problems = check_at_root(&root(), "0.75", Some(receipt)).unwrap();
        assert!(problems.is_empty(), "{}: {problems:#?}", receipt.display());
    }
    let fault: serde_json::Value =
        serde_json::from_slice(&std::fs::read(output.join("fault-schedule.json")).unwrap())
            .unwrap();
    assert_eq!(fault["chaos_steps"], 128);
    assert_eq!(fault["chaos_trace_fingerprint"].as_str().unwrap().len(), 16);
    assert_eq!(
        check_receipt_set_at_root(&root(), "0.75", &output).unwrap(),
        Vec::<String>::new()
    );
}

#[test]
fn generator_never_overwrites_an_existing_evidence_directory() {
    let temp = tempdir().unwrap();
    assert!(generate_at_root(&root(), temp.path(), 117).is_err());
}

#[test]
fn incomplete_or_mixed_seed_receipt_sets_fail_closed() {
    let temp = tempdir().unwrap();
    let output = temp.path().join("evidence");
    generate_at_root(&root(), &output, 117).unwrap();
    std::fs::remove_file(output.join("rpo-rto.json")).unwrap();
    let problems = check_receipt_set_at_root(&root(), "0.75", &output).unwrap();
    assert!(problems
        .iter()
        .any(|problem| problem.contains("four canonical JSON files")));
}
