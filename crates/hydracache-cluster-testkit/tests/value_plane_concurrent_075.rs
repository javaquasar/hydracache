use hydracache_cluster_testkit::value_plane_concurrent_075::{
    execute_concurrent_rust, generated_schedule, parse_histories, shrink_schedule,
    validate_histories, validate_history, ConcurrentHistory, ConcurrentSchedule, MAX_HISTORY_BYTES,
    MAX_SCHEDULE_BYTES, MAX_SCHEDULE_ROUNDS,
};
use hydracache_cluster_testkit::value_plane_differential_075::DifferentialError;

#[test]
fn real_threads_and_async_admission_produce_legal_bounded_histories() {
    let schedule = generated_schedule(117, 16).unwrap();
    assert_eq!(
        ConcurrentSchedule::parse(&schedule.encode()).unwrap(),
        schedule
    );
    assert_eq!(generated_schedule(117, 16).unwrap(), schedule);
    assert_ne!(generated_schedule(118, 16).unwrap(), schedule);
    let histories = execute_concurrent_rust(&schedule).unwrap();
    validate_histories(&schedule, &histories).unwrap();
    let transcript = histories
        .iter()
        .map(|history| format!("{}\n", history.encode()))
        .collect::<String>();
    assert_eq!(
        parse_histories(transcript.as_bytes(), histories.len()).unwrap(),
        histories
    );
    assert!(histories
        .iter()
        .any(|history| history.asynchronous.starts_with("cancelled_before/")));
    assert!(histories
        .iter()
        .any(|history| history.asynchronous.starts_with("cancelled_after/")));
    assert!(histories
        .iter()
        .any(|history| history.events.contains("expired/")));
}

#[test]
fn checker_accepts_either_overlap_winner_and_enforces_real_time_edges() {
    let schedule = generated_schedule(117, 0).unwrap();
    let histories = execute_concurrent_rust(&schedule).unwrap();
    let overlap = &schedule.rounds[0];
    let mut left = histories[1].clone();
    let mut right = histories[2].clone();
    for history in [&mut left, &mut right] {
        history.id = overlap.id;
        history.order = "overlap".into();
        history.begin = [1, 2];
        history.end = [3, 4];
        validate_history(overlap, history).unwrap();
    }
    assert_ne!(left.snapshot, right.snapshot);
    left.order = "right_first".into();
    left.begin = [3, 1];
    left.end = [4, 2];
    assert!(validate_history(&schedule.rounds[2], &left).is_err());
    left.id = schedule.rounds[2].id;
    assert!(validate_history(&schedule.rounds[2], &left).is_err());
}

#[test]
fn checker_rejects_broken_values_receipts_events_resources_and_cancellation() {
    let schedule = generated_schedule(117, 0).unwrap();
    let histories = execute_concurrent_rust(&schedule).unwrap();
    let round = &schedule.rounds[0];
    let original = &histories[0];
    let mut candidates = Vec::new();
    let mut history = original.clone();
    history.id += 1;
    candidates.push(history);
    let mut history = original.clone();
    history.order = "invalid".into();
    candidates.push(history);
    let mut history = original.clone();
    history.begin = [0, 0];
    candidates.push(history);
    let mut history = original.clone();
    history.end[0] = history.begin[0];
    candidates.push(history);
    let mut history = original.clone();
    history.begin = [1, 3];
    history.end = [2, 4];
    candidates.push(history);
    let mut history = original.clone();
    history.results[0] = "replaced|00|4|10|0|1|[]".into();
    candidates.push(history);
    let mut history = original.clone();
    history.snapshot = "[]".into();
    candidates.push(history);
    let mut history = original.clone();
    history.events = "[]".into();
    candidates.push(history);
    let mut history = original.clone();
    history.replay = "not_run".into();
    candidates.push(history);
    let mut history = original.clone();
    history.query = "unknown|~|~|~|0|0|[]".into();
    candidates.push(history);
    let mut history = original.clone();
    history.resources = "[1,1,3,1,1,0]".into();
    candidates.push(history);
    for history in candidates {
        assert!(
            validate_history(round, &history).is_err(),
            "{}",
            history.encode()
        );
    }
    for index in [9, 10] {
        let mut history = histories[index].clone();
        history.asynchronous = "cancelled_before/not_applied/unknown".into();
        history.snapshot = "[]".into();
        assert!(validate_history(&schedule.rounds[index], &history).is_err());
    }
    assert!(validate_histories(&schedule, &histories[..histories.len() - 1]).is_err());
}

#[test]
fn schedule_and_history_parsers_reject_ambiguous_or_unbounded_inputs() {
    let header = "imap-api-schedule-v1\t117\n";
    for source in [
        "".to_owned(),
        header.to_owned(),
        "imap-api-schedule-v2\t117\n1\tcompare\toverlap\t1\t2\n".into(),
        header.replace("117", "-1"),
        header.replace("117", "18446744073709551616"),
        "é".into(),
        header.replace('\n', "\r\n"),
        "x".repeat(MAX_SCHEDULE_BYTES + 1),
    ] {
        assert!(ConcurrentSchedule::parse(&source).is_err());
    }
    for row in [
        "1\tcompare\toverlap\t1",
        "0\tcompare\toverlap\t1\t2",
        "1\tbad\toverlap\t1\t2",
        "1\tcompare\tbad\t1\t2",
        "1\tcompare\toverlap\t0\t2",
        "1\tcompare\toverlap\t1\t1",
        "1\tcompare\toverlap\t256\t2",
        "1\tcancel_before\toverlap\t1\t2",
        "1\tcompare\toverlap\tx\t2",
        "1\tcompare\toverlap\t9223372036854775808\t2",
    ] {
        assert!(ConcurrentSchedule::parse(&format!("{header}{row}\n")).is_err());
    }
    let row = "1\tcompare\toverlap\t1\t2\n";
    assert!(ConcurrentSchedule::parse(&format!("{header}{row}{row}")).is_err());
    let too_many = (1..=65)
        .map(|id| format!("{id}\tcompare\toverlap\t1\t2\n"))
        .collect::<String>();
    assert!(ConcurrentSchedule::parse(&format!("{header}{too_many}")).is_err());
    assert!(generated_schedule(u64::MAX, 0).is_err());
    assert!(generated_schedule(117, MAX_SCHEDULE_ROUNDS).is_err());
    for line in ["", "1\tcompare", "é", "line\r", "line\n"] {
        assert!(ConcurrentHistory::parse(line).is_err());
    }
    assert!(ConcurrentHistory::parse(&"x".repeat(4097)).is_err());
    assert!(parse_histories(b"\xff", 1).is_err());
    assert!(parse_histories(b"", 1).is_err());
    assert!(parse_histories(b"", MAX_SCHEDULE_ROUNDS + 1).is_err());
    assert!(parse_histories(&vec![b'x'; MAX_HISTORY_BYTES + 1], 1).is_err());
}

#[test]
fn failing_schedule_shrinks_deterministically_with_a_hard_predicate_budget() {
    let schedule = generated_schedule(117, 45).unwrap();
    let mut calls = 0;
    let minimized = shrink_schedule(&schedule, 16, |candidate| {
        calls += 1;
        Ok(candidate.rounds.iter().any(|round| round.id == 23))
    })
    .unwrap();
    assert!(calls <= 16);
    assert_eq!(minimized.rounds.len(), 1);
    assert_eq!(minimized.rounds[0].id, 23);
    assert_eq!(minimized.rounds[0].order, "left_first");
    assert_eq!(
        (minimized.rounds[0].left, minimized.rounds[0].right),
        (1, 2)
    );
    assert_eq!(
        shrink_schedule(&schedule, 0, |_| panic!("zero budget called predicate")).unwrap(),
        schedule
    );
    assert_eq!(
        shrink_schedule(&schedule, 16, |_| Ok(false)).unwrap(),
        schedule
    );
    let mut calls = 0;
    let bounded = shrink_schedule(&schedule, 2, |_| {
        calls += 1;
        Ok(true)
    })
    .unwrap();
    assert_eq!(calls, 2);
    assert!(bounded.rounds.len() > 1);
    assert!(shrink_schedule(&schedule, 1, |_| Err(DifferentialError(
        "predicate failed".into()
    )))
    .is_err());
}
