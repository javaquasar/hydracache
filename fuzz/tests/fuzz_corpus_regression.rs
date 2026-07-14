use std::fs;
use std::path::Path;

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

fn fuzz_targets() -> [(&'static str, fn(&[u8])); 4] {
    [
        ("fuzz_config_parse", hydracache_fuzz::fuzz_config_parse),
        ("fuzz_kv_codec", hydracache_fuzz::fuzz_kv_codec),
        ("fuzz_resp_command", hydracache_fuzz::fuzz_resp_command),
        (
            "fuzz_snapshot_decode",
            hydracache_fuzz::fuzz_snapshot_decode,
        ),
    ]
}
