use hydracache_cluster_testkit::value_plane_extended_075::{
    execute_extended, shrink_extended, shrink_extended_bounded, ExtendedCorpus, MAX_BYTES, MAX_LINE,
};

const VECTOR:&str = "imap-extended-semantic-v2\t-9223372036854775808\t3\n7030\tput\t:00ff:~:e\t63:1\t~\t0\t0\n6330\tcapture_version\t:~:7630:p\t~\t~\t0\t0\n7230\treplace_if_version\t:aa:7630:p\t63:2\t~\t0\t0\n";

#[test]
fn common_v2_canonical_vector_executes_real_owner_and_preserves_empty_bytes() {
    let corpus = ExtendedCorpus::parse(VECTOR).unwrap();
    assert_eq!(corpus.encode().unwrap(), VECTOR);
    assert_eq!(
        ExtendedCorpus::parse(VECTOR.trim_end_matches('\n')).unwrap(),
        corpus
    );
    let observations = execute_extended(&corpus).unwrap();
    assert_eq!(observations[0]["result"]["kind"], "inserted");
    assert_eq!(observations[0]["snapshot"][0]["key"], "");
    assert_eq!(observations[0]["snapshot"][0]["value"], "00ff");
    assert_eq!(observations[1]["token_relations"]["v0=v0"], true);
    assert_eq!(observations[2]["snapshot"][0]["value"], "aa");
    assert_eq!(observations[2]["events"][0]["old_value"], "00ff");
    assert_eq!(observations[2]["owners"]["outcomes"], 2);
    assert_eq!(observations[2]["owners"]["listeners"], 1);
}

#[test]
fn v2_hostile_corpus_refuses_shape_encoding_references_and_bounds_before_owner() {
    for source in [
        VECTOR.replace("semantic-v2", "semantic-v1"),
        VECTOR.replace("\t3\n", "\t4\n"),
        VECTOR.replace("\t3\n", "\t03\n"),
        VECTOR.replace("-9223372036854775808", "-0"),
        VECTOR.replace("-9223372036854775808", "+1"),
        VECTOR.replace("-9223372036854775808", "9223372036854775808"),
        VECTOR.replace("00ff", "00FF"),
        VECTOR.replace("00ff", "0"),
        VECTOR.replace("7030", "ff"),
        VECTOR.replace("7030", "3d"),
        VECTOR.replace("7030", "20"),
        VECTOR.replace("\tput\t", "\tunknown\t"),
        VECTOR.replace("\tput\t", "\tcapture_version\t"),
        VECTOR.replace(":00ff:~:e", ":~:~:e"),
        VECTOR.replace(":~:7630:p", ":~:7630:x0"),
        VECTOR.replace(":~:7630:p", ":~:7630:x01"),
        VECTOR.replace(":~:7630:p", ":~:7630:q"),
        VECTOR.replace("63:1", "63:0"),
        VECTOR.replace("63:1", "63:9223372036854775808"),
        VECTOR.replace("63:1", "63:01"),
        VECTOR.replace("\t~\t0\t0", "\t0:1\t0\t0"),
        VECTOR.replace("\t~\t0\t0", "\t~\t1\t0"),
        VECTOR.replace("\t~\t0\t0", "\t~\t0\t2"),
        VECTOR.replace("6330", "7030"),
        VECTOR.replace(":aa:7630:p", ":aa:756e6b:p"),
        VECTOR.replace("\n", "\r\n"),
        format!("{VECTOR}\n"),
        format!("{}x", " ".repeat(MAX_BYTES)),
        format!(
            "imap-extended-semantic-v2\t0\t1\n{}",
            "a".repeat(MAX_LINE + 1)
        ),
    ] {
        assert!(
            ExtendedCorpus::parse(&source).is_err(),
            "unexpected acceptance: {}",
            source.chars().take(100).collect::<String>()
        );
    }
    let mut constructed = ExtendedCorpus::parse(VECTOR).unwrap();
    constructed.steps[0].requests[0].sequence = 0;
    assert!(execute_extended(&constructed).is_err());
    assert!(constructed.encode().is_err());
    assert_eq!(
        ExtendedCorpus::parse("imap-extended-semantic-v2\t0\t0\n")
            .unwrap()
            .steps
            .len(),
        0
    );
}

#[test]
fn bounded_shrink_preserves_token_dependencies_and_propagates_replay_errors() {
    let original = ExtendedCorpus::parse(VECTOR).unwrap();
    let mut attempts = 0;
    let minimized = shrink_extended(original.clone(), 16, |c| {
        attempts += 1;
        Ok(c.steps.iter().any(|s| s.operation == "replace_if_version"))
    })
    .unwrap();
    assert!(attempts <= 16);
    assert!(minimized
        .steps
        .iter()
        .any(|s| s.operation == "capture_version"));
    assert!(minimized.validate().is_ok());
    assert!(shrink_extended(original.clone(), 0, |_| Ok(true)).is_err());
    assert!(shrink_extended(original.clone(), 17, |_| Ok(true)).is_err());
    assert!(shrink_extended(original, 1, |_| Err(
        hydracache_cluster_testkit::value_plane_differential_075::DifferentialError(
            "peer failed".into()
        )
    ))
    .is_err());
}

#[test]
fn v2_pending_leases_clock_and_checkpoint_use_actual_owned_resources() {
    let lines = [
        "73\treserve_put\t6b:76:~:e\t63:1\t~\t0\t0",
        "71\tquery_outcomes\t~\t63:1\t~\t0\t0",
        "62\tcheckpoint_restore\t~\t~\t~\t0\t0",
        "64\tdispatch_put\t6b:76:~:e\t63:1\t~\t0\t1",
        "68\thold_listener_snapshot\t~\t~\t64:1048576\t0\t0",
        "63\tcheckpoint_restore\t~\t~\t~\t0\t0",
        "78\tclose_listener_snapshot\t~\t~\t~\t0\t0",
        "72\tcheckpoint_restore\t~\t~\t~\t0\t0",
        "61\tadvance\t~\t~\t~\t9223372036854775807\t0",
        "6f\tadvance\t~\t~\t~\t1\t0",
    ];
    let corpus = ExtendedCorpus::parse(&format!(
        "imap-extended-semantic-v2\t117\t{}\n{}\n",
        lines.len(),
        lines.join("\n")
    ))
    .unwrap();
    let o = execute_extended(&corpus).unwrap();
    assert_eq!(o[0]["owners"]["pending"], 1);
    assert_eq!(o[1]["result"]["statuses"][0]["kind"], "pending");
    assert_eq!(o[2]["result"]["kind"], "error:active_owners");
    assert_eq!(o[3]["result"]["kind"], "outcome_unknown");
    assert_eq!(o[4]["owners"]["snapshot_payload"], true);
    assert_eq!(o[5]["result"]["kind"], "error:active_owners");
    assert_eq!(o[6]["owners"]["snapshot_payload"], false);
    assert_eq!(o[7]["result"]["kind"], "restored");
    assert_eq!(o[8]["tick"], i64::MAX as u64);
    assert_eq!(o[9]["result"]["kind"], "error:invalid_argument");
    assert_eq!(o[9]["tick"], i64::MAX as u64);
}

#[test]
fn terminal_listener_is_drained_once_and_post_destroy_cleanup_keeps_no_resource_owner() {
    let source="imap-extended-semantic-v2\t0\t5\n61\tput\t6b:76:~:e\t63:1\t~\t0\t0\n62\tdestroy\t~\t63:2\t~\t0\t0\n63\tquery_outcomes\t~\t63:1,63:2\t~\t0\t0\n64\tcancel_reserved\t~\t63:1\t~\t0\t0\n65\tresources\t~\t~\t~\t0\t0\n";
    let corpus = ExtendedCorpus::parse(source).unwrap();
    let observations = execute_extended(&corpus).unwrap();
    assert_eq!(observations[1]["events"][0]["kind"], "destroyed");
    for observation in &observations[1..] {
        assert_eq!(observation["owners"]["listeners"], 0);
        assert_eq!(observation["owners"]["pending"], 0);
        assert_eq!(observation["owners"]["outcomes"], 0);
        assert!(observation["snapshot"].as_array().unwrap().is_empty());
    }
    for observation in &observations[2..] {
        assert!(observation["events"].as_array().unwrap().is_empty());
    }
    assert_eq!(observations[2]["result"]["statuses"][0]["kind"], "unknown");
    assert_eq!(observations[3]["result"]["kind"], "absent");
    assert_ne!(
        observations,
        hydracache_cluster_testkit::value_plane_extended_075::execute_extended_drop_event(&corpus)
            .unwrap()
    );
}

#[test]
fn rust_owner_peer_refuses_unknown_arguments_and_invalid_bounded_stdin_without_observations() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let binary = env!("CARGO_BIN_EXE_imap-extended-owner-075");
    let unknown = Command::new(binary).arg("--unknown").output().unwrap();
    assert!(!unknown.status.success());
    assert!(unknown.stdout.is_empty());
    for input in [
        vec![0xff],
        b"imap-extended-semantic-v99\t0\t0\n".to_vec(),
        vec![b'x'; MAX_BYTES + 1],
    ] {
        let mut child = Command::new(binary)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(&input).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
    }
}

fn boundary(lines: &[&str]) -> Vec<serde_json::Value> {
    let script = format!(
        "imap-extended-semantic-v2\t117\t{}\n{}\n",
        lines.len(),
        lines.join("\n")
    );
    execute_extended(&ExtendedCorpus::parse(&script).unwrap()).unwrap()
}

#[test]
fn common_signed_expiry_guard_does_not_admit_or_apply_an_overflowing_point_or_reservation() {
    let o = boundary(&[
        "61\tadvance\t~\t~\t~\t9223372036854775806\t0",
        "62\tput\t6b:31:~:x2\t63:1\t~\t0\t0",
        "63\treserve_put\t6b:31:~:x2\t63:1\t~\t0\t0",
        "64\tquery_outcomes\t~\t63:1\t~\t0\t0",
        "65\tput\t6b:31:~:e\t63:1\t~\t0\t0",
        "66\tcapture_version\t6b:~:76:x9223372036854775807\t~\t~\t0\t0",
        "67\tadvance\t~\t~\t~\t1\t0",
        "68\tput\t6b:32:~:e\t63:2\t~\t0\t0",
        "69\tput\t6b:31:~:e\t63:1\t~\t0\t0",
    ]);
    for index in [1, 2] {
        assert_eq!(o[index]["result"]["kind"], "error:invalid_argument");
        assert_eq!(o[index]["owners"]["outcomes"], 0);
        assert_eq!(o[index]["owners"]["pending"], 0);
        assert_eq!(o[index]["events"], serde_json::json!([]));
        assert_eq!(o[index]["tick"], i64::MAX as u64 - 1);
    }
    assert_eq!(o[3]["result"]["statuses"][0]["kind"], "unknown");
    assert_eq!(o[4]["result"]["kind"], "inserted");
    assert_eq!(o[5]["result"]["kind"], "entry_view");
    assert_eq!(o[7]["result"]["kind"], "not_applied");
    assert_eq!(o[8]["result"]["kind"], "inserted");
    assert_eq!(o[8]["events"], serde_json::json!([]));
    assert_eq!(o[8]["owners"]["outcomes"], 1);
}

#[test]
fn common_signed_reserved_clock_preserves_digest_priority_and_pending_owner_before_deadline() {
    let o = boundary(&[
        "61\treserve_put\t6b:31:~:e\t63:1\t~\t0\t0",
        "62\tput\t62:32:~:x2\t63:2\t~\t0\t0",
        "63\tadvance\t~\t~\t~\t9223372036854775806\t0",
        "64\tdispatch_put\t6b:32:~:x2\t63:1\t~\t0\t0",
        "65\tdispatch_put\t62:32:~:x2\t63:2\t~\t0\t0",
        "66\tdispatch_put\t6b:31:~:e\t63:1\t~\t0\t0",
        "67\tdispatch_put\t6b:32:~:x2\t63:1\t~\t0\t0",
        "68\tquery_outcomes\t~\t63:1,63:2\t~\t0\t0",
    ]);
    assert_eq!(o[3]["result"]["kind"], "error:mutation_identity_conflict");
    assert_eq!(o[3]["owners"]["pending"], 1);
    assert_eq!(o[3]["events"], serde_json::json!([]));
    assert_eq!(o[4]["result"]["kind"], "error:invalid_argument");
    assert_eq!(o[4]["owners"]["pending"], 1);
    assert_eq!(o[5]["result"]["kind"], "inserted");
    assert_eq!(o[6]["result"]["kind"], "error:mutation_identity_conflict");
    assert_eq!(o[7]["result"]["statuses"][0]["kind"], "completed");
    assert_eq!(o[7]["result"]["statuses"][1]["kind"], "completed");
    let matching = boundary(&[
        "61\treserve_put\t6b:31:~:x2\t63:1\t~\t0\t0",
        "62\tadvance\t~\t~\t~\t9223372036854775806\t0",
        "63\tdispatch_put\t6b:31:~:x2\t63:1\t~\t0\t0",
        "64\tquery_outcomes\t~\t63:1\t~\t0\t0",
    ]);
    assert_eq!(matching[2]["result"]["kind"], "error:invalid_argument");
    assert_eq!(matching[2]["owners"]["pending"], 1);
    assert_eq!(matching[3]["result"]["statuses"][0]["kind"], "pending");
}

#[test]
fn expired_reserved_clock_checks_ttl_before_pending_conflict_but_keeps_completed_conflict_priority()
{
    let o = boundary(&[
        "61\treserve_put\t6b:31:~:e\t63:1\t~\t0\t0",
        "62\tput\t62:32:~:e\t63:2\t~\t0\t0",
        "63\tadvance\t~\t~\t~\t9223372036854775807\t0",
        "64\tdispatch_put\t6b:32:~:x1\t63:1\t~\t0\t0",
        "65\tdispatch_put\t6b:32:~:e\t63:1\t~\t0\t0",
        "66\tdispatch_put\t62:33:~:x1\t63:2\t~\t0\t0",
        "67\tdispatch_put\t6b:31:~:e\t63:1\t~\t0\t0",
        "68\tdispatch_put\t6b:31:~:x1\t63:3\t~\t0\t0",
        "69\tquery_outcomes\t~\t63:1,63:2,63:3\t~\t0\t0",
    ]);
    for index in [3, 4, 5] {
        assert_eq!(o[index]["owners"]["pending"], 1);
        assert_eq!(o[index]["events"], serde_json::json!([]));
    }
    assert_eq!(o[3]["result"]["kind"], "error:invalid_argument");
    assert_eq!(o[4]["result"]["kind"], "error:mutation_identity_conflict");
    assert_eq!(o[5]["result"]["kind"], "error:mutation_identity_conflict");
    assert_eq!(o[6]["result"]["kind"], "not_applied");
    assert_eq!(o[6]["owners"]["pending"], 0);
    assert_eq!(o[7]["result"]["kind"], "error:invalid_argument");
    assert_eq!(o[8]["result"]["statuses"][0]["kind"], "unknown");
    assert_eq!(o[8]["result"]["statuses"][1]["kind"], "completed");
    assert_eq!(o[8]["result"]["statuses"][2]["kind"], "unknown");
}

#[test]
fn every_version_bulk_action_uses_signed_expiry_guard_before_any_item_and_valid_boundary_can_expire(
) {
    let o = boundary(&[
        "61\tput\t6b:31:~:e\t63:1\t~\t0\t0",
        "62\tcapture_version\t6b:~:76:p\t~\t~\t0\t0",
        "63\tadvance\t~\t~\t~\t9223372036854775806\t0",
        "64\tbulk_replace\t6b:32:76:x2\t63:2\t~\t0\t0",
        "65\tbulk_remove\t6b:~:76:x2\t63:3\t~\t0\t0",
        "66\tbulk_set_ttl\t6b:~:76:x2\t63:4\t~\t0\t0",
        "67\tquery_outcomes\t~\t63:2,63:3,63:4\t~\t0\t0",
        "68\tbulk_set_ttl\t6b:~:76:x1\t63:5\t~\t0\t0",
        "69\tadvance\t~\t~\t~\t1\t0",
    ]);
    for index in [3, 4, 5] {
        assert_eq!(o[index]["result"]["kind"], "error:invalid_argument");
        assert_eq!(o[index]["owners"]["outcomes"], 1);
        assert_eq!(o[index]["snapshot"][0]["value"], "31");
        assert_eq!(o[index]["events"], serde_json::json!([]));
    }
    for status in o[6]["result"]["statuses"].as_array().unwrap() {
        assert_eq!(status["kind"], "unknown");
    }
    assert_eq!(o[7]["result"]["items"][0]["kind"], "replaced");
    assert_eq!(o[7]["snapshot"][0]["ttl"]["remaining_ticks"], 1);
    assert_eq!(o[8]["events"][0]["kind"], "expired");
    assert!(o[8]["snapshot"].as_array().unwrap().is_empty());
}

#[test]
fn shared_java_signed_clock_corpus_files_execute_the_actual_rust_owner() {
    for source in [
        include_str!("../../../tests/java-imap-benchmark/src/test/resources/imap-v2-boundaries/signed-point.tsv"),
        include_str!("../../../tests/java-imap-benchmark/src/test/resources/imap-v2-boundaries/reserved-predeadline.tsv"),
        include_str!("../../../tests/java-imap-benchmark/src/test/resources/imap-v2-boundaries/reserved-matching.tsv"),
        include_str!("../../../tests/java-imap-benchmark/src/test/resources/imap-v2-boundaries/reserved-expired.tsv"),
        include_str!("../../../tests/java-imap-benchmark/src/test/resources/imap-v2-boundaries/signed-bulk.tsv"),
    ] {
        let corpus=ExtendedCorpus::parse(source).unwrap();assert_eq!(corpus.encode().unwrap(),source);
        let observations=execute_extended(&corpus).unwrap();assert_eq!(observations.len(),corpus.steps.len());
        assert!(observations.iter().any(|o|o["result"]["kind"]=="error:invalid_argument"));
    }
}

#[test]
fn v2_bounded_shrink_verifies_original_failure_and_reports_replay_and_work_exhaustion() {
    let original = ExtendedCorpus::parse(VECTOR).unwrap();
    let mut calls = 0;
    assert!(shrink_extended_bounded(original.clone(), 16, 4096, |_| {
        calls += 1;
        Ok(false)
    })
    .is_err());
    assert_eq!(calls, 1);
    let mut calls = 0;
    assert!(
        shrink_extended_bounded(original.clone(), 16, original.steps.len() - 1, |_| {
            calls += 1;
            Ok(true)
        })
        .is_err()
    );
    assert_eq!(calls, 0);
    for (replays, work) in [(0, 100), (17, 100), (1, 0), (1, 4097)] {
        assert!(shrink_extended_bounded(original.clone(), replays, work, |_| Ok(true)).is_err());
    }
    let replay = shrink_extended_bounded(original.clone(), 1, 4096, |_| Ok(true)).unwrap();
    assert_eq!(replay.corpus, original);
    assert_eq!(replay.replays, 1);
    assert_eq!(replay.work, 3);
    assert!(replay.budget_exhausted);
    let work = shrink_extended_bounded(original.clone(), 16, 3, |_| Ok(true)).unwrap();
    assert_eq!(work.corpus, original);
    assert_eq!(work.replays, 1);
    assert_eq!(work.work, 3);
    assert!(work.budget_exhausted);
    let report = shrink_extended_bounded(original.clone(), 16, 4096, |c| {
        Ok(c.steps
            .iter()
            .any(|step| step.operation == "replace_if_version"))
    })
    .unwrap();
    assert!(report.replays <= 16);
    assert!(report.work <= 4096);
    assert!(!report.budget_exhausted);
    assert_eq!(report.corpus.seed, original.seed);
    assert!(report.corpus.validate().is_ok());
    assert!(report
        .corpus
        .steps
        .iter()
        .any(|step| step.operation == "capture_version"));
}
