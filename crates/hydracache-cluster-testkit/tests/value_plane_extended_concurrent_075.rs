use hydracache_cluster_testkit::value_plane_extended_concurrent_075::*;
#[test]
fn real_extended_workers_cover_all_families_and_emit_replayable_intervals() {
    for seed in [0, 117, -1, i64::MIN] {
        let schedule = generated_extended_schedule(seed, 15).unwrap();
        assert_eq!(
            ExtendedSchedule::parse(&schedule.encode().unwrap()).unwrap(),
            schedule
        );
        let histories = execute_extended_concurrent(&schedule, false).unwrap();
        assert_eq!(histories.len(), 15);
        let output = encode_extended_histories(seed, &histories).unwrap();
        assert!(output.starts_with(&format!("{HISTORY_HEADER}\t{seed}\t15\n")));
        for history in histories {
            assert_eq!(
                history.calls.len(),
                history.round.program(0).len() + history.round.program(1).len()
            );
            assert!(history.calls.iter().all(|call| call.start < call.end));
            assert!(history.final_state.ends_with("/0/0/0/0"));
        }
    }
}
#[test]
fn actual_refreshing_version_defect_changes_the_right_first_aba_owner_result() {
    let schedule =
        ExtendedSchedule::parse("imap-extended-schedule-v1\t117\n1\tbulk_aba\tright_first\n")
            .unwrap();
    let good = execute_extended_concurrent(&schedule, false).unwrap();
    let bad = execute_extended_concurrent(&schedule, true).unwrap();
    assert!(good[0].calls[0].result.contains("mismatch"));
    assert!(bad[0].calls[0].result.contains("replaced,replaced"));
    assert_ne!(good[0].final_state, bad[0].final_state);
}
#[test]
fn closed_concurrent_schedule_refuses_unsupported_or_unbounded_input() {
    for source in [
        "",
        "imap-extended-schedule-v1\t-0\n1\tbulk_aba\toverlap\n",
        "imap-extended-schedule-v2\t0\n1\tbulk_aba\toverlap\n",
        "imap-extended-schedule-v1\t0\n0\tbulk_aba\toverlap\n",
        "imap-extended-schedule-v1\t0\n1\tunknown\toverlap\n",
        "imap-extended-schedule-v1\t0\n1\tbulk_aba\tunknown\n",
        "imap-extended-schedule-v1\t0\n1\tbulk_aba\toverlap\n1\tbulk_aba\toverlap\n",
    ] {
        assert!(ExtendedSchedule::parse(source).is_err());
    }
    assert!(ExtendedSchedule::parse(&"x".repeat(MAX_SCHEDULE_BYTES + 1)).is_err());
    assert!(generated_extended_schedule(0, 0).is_err());
    assert!(generated_extended_schedule(0, 33).is_err());
    assert!(ExtendedSchedule {
        seed: 0,
        rounds: vec![]
    }
    .encode()
    .is_err());
    assert!(ExtendedSchedule {
        seed: 0,
        rounds: vec![ExtendedRound {
            id: 1,
            family: "x".repeat(MAX_SCHEDULE_BYTES),
            order: "overlap".into()
        }]
    }
    .encode()
    .is_err());
    assert!(encode_extended_histories(0, &[]).is_err());
    let schedule = generated_extended_schedule(0, 1).unwrap();
    let mut histories = execute_extended_concurrent(&schedule, false).unwrap();
    let original = histories[0].clone();
    histories[0].calls[0].result = "x".repeat(1025);
    assert!(encode_extended_histories(0, &histories).is_err());
    histories[0] = original.clone();
    histories[0].calls[0].start = 0;
    assert!(encode_extended_histories(0, &histories).is_err());
    histories[0] = original.clone();
    histories[0].final_state = "x".repeat(8193);
    assert!(encode_extended_histories(0, &histories).is_err());
    histories[0] = original;
    histories[0].round.family = "x".repeat(MAX_HISTORY_BYTES);
    assert!(encode_extended_histories(0, &histories).is_err());
}
