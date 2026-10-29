use hydracache_cluster_testkit::value_plane_extended_concurrent_075::{
    generated_extended_schedule, MAX_HISTORY_BYTES,
};
use xtask::imap_extended_concurrent_075::{
    execute_java, execute_rust, parse_verdicts, run, validate_history, LegalVerdict,
};
#[test]
fn verdicts_bind_exact_schedule_and_distinguish_invalid_from_search_exhaustion() {
    let schedule = generated_extended_schedule(117, 1).unwrap();
    for (word, expected) in [
        ("valid", LegalVerdict::Valid),
        ("invalid", LegalVerdict::Invalid),
        ("search_exhausted", LegalVerdict::SearchExhausted),
    ] {
        assert_eq!(
            parse_verdicts(
                format!("imap-extended-verdict-v1\t117\t1\n1\t{word}\t3\n").as_bytes(),
                &schedule
            )
            .unwrap(),
            vec![expected]
        );
    }
    for source in [
        "",
        "imap-extended-verdict-v2\t117\t1\n1\tvalid\t1\n",
        "imap-extended-verdict-v1\t0\t1\n1\tvalid\t1\n",
        "imap-extended-verdict-v1\t117\t1\n2\tvalid\t1\n",
        "imap-extended-verdict-v1\t117\t1\n1\tfuture\t1\n",
        "imap-extended-verdict-v1\t117\t1\n1\tvalid\t0\n",
        "imap-extended-verdict-v1\t117\t1\n1\tvalid\t100001\n",
        "imap-extended-verdict-v1\t117\t1\n1\tvalid\t01\n",
        "imap-extended-verdict-v1\t117\t1\n",
        "imap-extended-verdict-v1\t117\t1\n1\tvalid\t1\nextra\n",
    ] {
        assert!(parse_verdicts(source.as_bytes(), &schedule).is_err());
    }
    assert!(parse_verdicts(&[255], &schedule).is_err());
    assert!(parse_verdicts(&vec![b'x'; 4097], &schedule).is_err());
}
#[test]
fn missing_actual_peers_and_foreign_history_cannot_be_reported_as_legal() {
    let root = tempfile::tempdir().unwrap();
    let schedule = generated_extended_schedule(117, 1).unwrap();
    assert!(execute_java(root.path(), &schedule, false).is_err());
    assert!(validate_history(root.path(), &schedule, b"", 1).is_err());
    assert!(validate_history(root.path(), &schedule, b"", 0).is_err());
    assert!(validate_history(
        root.path(),
        &schedule,
        &vec![b'x'; MAX_HISTORY_BYTES + 1],
        4096
    )
    .is_err());
    let configured = std::env::var_os("CARGO_TARGET_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| root.path().join("target"));
    let target = if configured.is_absolute() {
        configured
    } else {
        root.path().join(configured)
    };
    if !target
        .join("debug")
        .join(if cfg!(windows) {
            "imap-extended-concurrent-owner-075.exe"
        } else {
            "imap-extended-concurrent-owner-075"
        })
        .is_file()
    {
        assert!(execute_rust(root.path(), &schedule, false).is_err());
    }
}
#[test]
fn invalid_cli_generation_is_refused_before_any_java_build_or_process() {
    for args in [
        vec!["--release", "0.74"],
        vec!["--release", "0.75", "--rounds", "0"],
        vec!["--release", "0.75", "--rounds", "33"],
        vec!["--release", "0.75", "--rounds", "01"],
        vec!["--release", "0.75", "--seed", "+1"],
        vec!["--release", "0.75", "--seed", "0", "--seed", "1"],
        vec!["--release", "0.75", "--future", "1"],
        vec!["--release", "0.75", "--seed"],
        vec!["--release", "0.75", "--schedule", "not-read", "--seed", "1"],
    ] {
        assert!(run(args.into_iter().map(String::from).collect()).is_err());
    }
}
