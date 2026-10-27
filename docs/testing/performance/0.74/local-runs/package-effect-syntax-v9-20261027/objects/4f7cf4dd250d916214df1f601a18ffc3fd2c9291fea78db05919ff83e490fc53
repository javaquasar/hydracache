//! Approved lock view only: no manager, worker, clock, service or server action.
#[path = "../../../tools/authority-runtime-074/gate.rs"]
mod runtime;
#[path = "../../../tools/authority-runtime-074/io.rs"]
mod runtime_io;
#[path = "../../../tools/authority-storage-source-074/contract.rs"]
mod storage_contract;

use runtime::successor::{roots, security};

fn current() -> String {
    std::fs::read_to_string(runtime::root().join(roots::PATH))
        .unwrap()
        .replace("\r\n", "\n")
}

#[test]
fn exact_producer_and_complete_predecessor_match_independent_git_inputs() {
    let raw = current();
    assert_eq!(raw, runtime::frozen(roots::PRODUCER, roots::PATH));
    let previous = runtime::frozen(roots::PREVIOUS, roots::PATH);
    assert_eq!(roots::project(roots::PATH, raw.clone()), previous);
    assert_eq!(
        roots::project(roots::PATH, raw.replace('\n', "\r\n")),
        previous
    );
}

#[test]
fn stale_partial_duplicate_and_unrelated_edits_refuse() {
    let raw = current();
    let rules: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tools/roots-lock-view-074/rules.json"
    ))
    .unwrap();
    let mut bad = vec![
        String::new(),
        runtime::frozen(roots::PREVIOUS, roots::PATH),
        format!("{raw}\n# unrelated"),
        raw.replace('\n', "\r"),
    ];
    for edit in rules["patches"].as_array().unwrap() {
        let at = edit["offset"].as_u64().unwrap() as usize;
        let new = edit["current"].as_str().unwrap();
        let old = edit["previous"].as_str().unwrap();
        let mut partial = raw.clone();
        partial.replace_range(at..at + new.len(), old);
        bad.push(partial);
        let mut duplicate = raw.clone();
        duplicate.insert_str(at, new);
        bad.push(duplicate);
    }
    for bytes in bad {
        assert_ne!(bytes, raw);
        assert!(std::panic::catch_unwind(|| roots::project(roots::PATH, bytes)).is_err());
    }
}

#[test]
fn unknown_paths_are_not_approved_or_normalized() {
    for path in [
        "./Cargo.lock",
        "../Cargo.lock",
        "cargo.lock",
        "Cargo.lock/",
        "Cargo.toml",
        "unknown",
    ] {
        let raw = "unknown\r\nbytes".to_owned();
        assert_eq!(roots::project(path, raw.clone()), raw);
    }
}

#[test]
fn successful_input_is_never_reused_for_a_later_observation() {
    let raw = current();
    let previous = roots::project(roots::PATH, raw.clone());
    assert!(std::panic::catch_unwind(|| roots::project(roots::PATH, previous)).is_err());
    assert!(std::panic::catch_unwind(|| roots::project(roots::PATH, format!("{raw}x"))).is_err());
    assert_eq!(
        roots::project(roots::PATH, raw),
        runtime::frozen(roots::PREVIOUS, roots::PATH)
    );
}

#[test]
fn composed_fresh_reader_preserves_the_existing_security_predecessor() {
    let expected = runtime::frozen(security::BASE, roots::PATH);
    assert_eq!(
        security::project(roots::PATH, roots::project(roots::PATH, current())),
        expected
    );
    assert_eq!(
        runtime_io::read_to_string(runtime::root().join(roots::PATH)).unwrap(),
        expected
    );
}

#[test]
fn existing_security_rules_reader_and_product_lock_are_not_rewritten() {
    for path in [
        "tools/security-input-view-074/rules.json",
        "tools/security-input-view-074/contract.rs",
        "tools/authority-runtime-074/io.rs",
        "tools/authority-runtime-074/gate.rs",
    ] {
        let raw = std::fs::read_to_string(runtime::root().join(path))
            .unwrap()
            .replace("\r\n", "\n");
        assert_eq!(raw, runtime::frozen(roots::PREVIOUS, path));
    }
    assert_eq!(current(), runtime::frozen(roots::PRODUCER, roots::PATH));
}

#[test]
fn seeded_byte_mutation_corpus_refuses() {
    let raw = current();
    let seed = 0x0740_2026_1027_u64;
    eprintln!("roots lock mutation seed={seed:#x}");
    let mut state = seed;
    for _ in 0..24 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let mut bytes = raw.clone().into_bytes();
        let index = (state as usize) % bytes.len();
        let mask = ((state >> 32) % 127 + 1) as u8;
        bytes[index] ^= mask;
        let changed = String::from_utf8(bytes).unwrap();
        assert!(std::panic::catch_unwind(|| roots::project(roots::PATH, changed)).is_err());
    }
}
