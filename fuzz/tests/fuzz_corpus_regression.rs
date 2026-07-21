use std::collections::BTreeSet;
use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;

type FuzzTarget = (&'static str, fn(&[u8]));

#[test]
fn fuzz_corpus_regression_replays_every_committed_seed_without_panic_or_unbounded_alloc() {
    let corpus = committed_corpus();
    let mut executed = 0usize;

    for (target, replay) in fuzz_targets() {
        let target_dir = corpus.join(target);
        assert!(
            target_dir.is_dir(),
            "missing committed corpus directory for {target}"
        );
        for entry in fs::read_dir(&target_dir).expect("corpus directory should be readable") {
            let entry = entry.expect("corpus entry should be readable");
            if !entry.file_type().unwrap().is_file() {
                continue;
            }
            let bytes = fs::read(entry.path()).expect("corpus seed should be readable");
            assert!(
                bytes.len() <= 16 * 1024,
                "corpus seed {} exceeds regression allocation budget",
                entry.path().display()
            );
            replay(&bytes);
            executed += 1;
        }
    }

    assert_eq!(executed, committed_seed_count());
}

#[test]
fn canary_fuzz_corpus_regression_is_not_actually_executed() {
    let executed_by_broken_runner = 0usize;
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W24") {
        assert_eq!(
            executed_by_broken_runner,
            committed_seed_count(),
            "HC-CANARY-RED:W24 committed fuzz corpus was not replayed"
        );
    }
    assert_ne!(
        executed_by_broken_runner,
        committed_seed_count(),
        "canary models loading the corpus manifest without replaying any seed"
    );
}

fn committed_corpus() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/corpus"))
}

fn committed_seed_count() -> usize {
    fuzz_targets()
        .into_iter()
        .map(|(target, _)| {
            fs::read_dir(committed_corpus().join(target))
                .expect("corpus directory should exist")
                .filter_map(Result::ok)
                .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
                .count()
        })
        .sum()
}

fn fuzz_targets() -> [FuzzTarget; 5] {
    [
        ("fuzz_config_parse", hydracache_fuzz::fuzz_config_parse),
        ("fuzz_kv_codec", hydracache_fuzz::fuzz_kv_codec),
        ("fuzz_resp_command", hydracache_fuzz::fuzz_resp_command),
        (
            "fuzz_snapshot_decode",
            hydracache_fuzz::fuzz_snapshot_decode,
        ),
        ("raft_wire_frame", hydracache_fuzz::fuzz_raft_wire_frame),
    ]
}

#[test]
fn raft_wire_frame_corpus_never_panics_or_mutates_on_reject() {
    let target_dir = committed_corpus().join("raft_wire_frame");
    let mut executed = 0usize;
    let mut names = BTreeSet::new();

    for entry in fs::read_dir(&target_dir).expect("raft wire corpus should be readable") {
        let entry = entry.expect("raft wire corpus entry should be readable");
        if !entry.file_type().unwrap().is_file() {
            continue;
        }
        let path = entry.path();
        names.insert(entry.file_name().to_string_lossy().into_owned());
        let bytes = fs::read(&path).expect("raft wire corpus seed should be readable");
        assert!(
            bytes.len() <= 16 * 1024,
            "raft wire seed {} exceeds the pure replay budget",
            path.display()
        );
        let result = catch_unwind(AssertUnwindSafe(|| {
            hydracache_fuzz::fuzz_raft_wire_frame(&bytes)
        }));
        assert!(result.is_ok(), "raft wire seed {} panicked", path.display());
        executed += 1;
    }

    assert!(
        executed > 0,
        "raft wire corpus must contain committed seeds"
    );
    let required = [
        "outer-from-mismatch.json",
        "outer-to-mismatch.json",
        "outer-term-mismatch.json",
        "malformed-metadata-snapshot.json",
        "snapshot-source-mismatch.json",
        "snapshot-index-mismatch.json",
        "read-index-without-context.json",
    ];
    let missing = required
        .into_iter()
        .filter(|name| !names.contains(*name))
        .collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "raft wire corpus is missing committed identity/snapshot cases: {missing:?}"
    );
    for name in required {
        let bytes = fs::read(target_dir.join(name)).unwrap();
        assert_eq!(
            hydracache_fuzz::replay_raft_wire_frame(&bytes),
            hydracache_fuzz::RaftWireFuzzOutcome::Rejected,
            "committed malformed raft seed {name} unexpectedly reached an accepted outcome"
        );
    }
}
