use hydracache_cluster_testkit::value_plane_cluster_chaos_075::*;

#[test]
fn retained_imap_cluster_corpus_executes_or_refuses_before_unsupported_transitions() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/fuzz_imap_cluster_075");
    for (name, valid) in [
        ("point-replay.json", true),
        ("future-version.json", false),
        ("early-response.json", false),
        ("bad-hex.json", false),
    ] {
        let data = std::fs::read(root.join(name)).unwrap();
        let result = fuzz_cluster_schedule(&data);
        assert_eq!(result.is_ok(), valid, "{name}: {result:?}");
        assert!(
            !matches!(result, Err(ClusterScheduleError::Violation(_))),
            "corpus exposes an actual invariant failure"
        );
    }
}
