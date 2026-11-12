use serde_json::{json, Value};
use xtask::imap_native_concurrent_075::*;
fn fixture() -> Histories {
    serde_json::from_str(include_str!(
        "../../../docs/testing/imap/0.75/native-concurrent-history-fixture-v1.json"
    ))
    .unwrap()
}
fn raw_digest(raw: &[RawEvent]) -> String {
    let mut text = String::new();
    for e in raw {
        text.push_str(&format!(
            "{}|{}|{}|{}",
            if e.phase == Phase::Invoke { "i" } else { "r" },
            e.stamp,
            e.call_id,
            e.worker
        ));
        if e.phase == Phase::Return {
            text.push_str(&format!(
                "|{}|{}",
                e.value.as_deref().unwrap_or("null"),
                e.boolean.map_or_else(|| "null".into(), |b| b.to_string())
            ));
        }
        text.push('\n');
    }
    xtask::imap_qualification_075::digest(text.as_bytes())
}
fn digest_entries(entries: &[Entry]) -> String {
    let mut entries = entries.to_vec();
    entries.sort_by(|a, b| a.key.cmp(&b.key));
    let mut bytes = Vec::new();
    for entry in entries {
        for value in [entry.key, entry.value] {
            bytes.extend_from_slice(&((value.len() / 2) as u32).to_be_bytes());
            for pair in value.as_bytes().chunks_exact(2) {
                bytes.push(u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap());
            }
        }
        bytes.push(0);
    }
    xtask::imap_qualification_075::digest(&bytes)
}
fn tiny(
    program: Vec<Command>,
    results: Vec<(Option<&str>, Option<bool>)>,
    overlap: bool,
    final_entries: Vec<Entry>,
) -> History {
    let mut raw = Vec::new();
    if overlap {
        raw.push(RawEvent {
            phase: Phase::Invoke,
            stamp: 1,
            call_id: 0,
            worker: program[0].worker,
            value: None,
            boolean: None,
        });
        raw.push(RawEvent {
            phase: Phase::Invoke,
            stamp: 2,
            call_id: 1,
            worker: program[1].worker,
            value: None,
            boolean: None,
        });
        for (id, (value, boolean)) in results.iter().enumerate() {
            raw.push(RawEvent {
                phase: Phase::Return,
                stamp: 3 + id as u64,
                call_id: id,
                worker: program[id].worker,
                value: value.map(str::to_string),
                boolean: *boolean,
            });
        }
    } else {
        for (id, (c, (value, boolean))) in program.iter().zip(&results).enumerate() {
            raw.push(RawEvent {
                phase: Phase::Invoke,
                stamp: 1 + 2 * id as u64,
                call_id: id,
                worker: c.worker,
                value: None,
                boolean: None,
            });
            raw.push(RawEvent {
                phase: Phase::Return,
                stamp: 2 + 2 * id as u64,
                call_id: id,
                worker: c.worker,
                value: value.map(str::to_string),
                boolean: *boolean,
            });
        }
    }
    History {
        seed: 117,
        attempts: 1,
        member_uuid: String::new(),
        clients: vec![],
        program,
        raw_digest: raw_digest(&raw),
        raw,
        final_digest: digest_entries(&final_entries),
        final_entries,
        overlap,
        verdict: "linearizable".into(),
        work: 1,
        witness: vec![],
        native_events: vec![],
    }
}
fn command(
    id: usize,
    worker: usize,
    position: usize,
    operation: Operation,
    key: &str,
    value: Option<&str>,
    expected: Option<&str>,
) -> Command {
    Command {
        id,
        worker,
        position,
        operation,
        key: key.into(),
        value: value.map(str::to_string),
        expected: expected.map(str::to_string),
    }
}
#[test]
fn actual_native_fixture_is_independently_linearizable_and_exhaustion_is_not_success() {
    let observed = fixture();
    validate(&observed).unwrap();
    replay(&observed, &observed).unwrap();
    for h in &observed.histories {
        assert_eq!(check(h, 100000).unwrap(), Verdict::Linearizable);
        assert_eq!(check(h, 1).unwrap(), Verdict::Exhausted);
    }
    assert!(check(&observed.histories[0], 0).is_err());
    assert!(check(&observed.histories[0], 1000001).is_err());
}
#[test]
fn final_key_worker_interval_and_operation_coverage_bounds_are_independent() {
    let mut history = fixture().histories[0].clone();
    history.final_entries = vec![
        Entry {
            key: "61".into(),
            value: "31".into()
        };
        5
    ];
    assert!(check(&history, 100000)
        .unwrap_err()
        .to_string()
        .contains("final key bound"));
    let program = vec![
        command(0, 0, 0, Operation::Get, "61", None, None),
        command(1, 0, 1, Operation::Get, "61", None, None),
    ];
    assert!(check(
        &tiny(program.clone(), vec![(None, None); 2], false, vec![]),
        100
    )
    .unwrap_err()
    .to_string()
    .contains("worker/key bound"));
    let mut program = program;
    program.push(command(2, 1, 0, Operation::Get, "61", None, None));
    let mut history = tiny(program, vec![(None, None); 3], false, vec![]);
    history.raw.swap(1, 2);
    for (index, event) in history.raw.iter_mut().enumerate() {
        event.stamp = index as u64 + 1;
    }
    history.raw_digest = raw_digest(&history.raw);
    assert!(check(&history, 100)
        .unwrap_err()
        .to_string()
        .contains("worker interval order"));
    let mut histories = fixture();
    for command in &mut histories.histories[0].program {
        command.operation = Operation::ContainsKey;
        command.value = None;
        command.expected = None;
    }
    let history = &mut histories.histories[0];
    for event in &mut history.raw {
        event.value = None;
        event.boolean = (event.phase == Phase::Return).then_some(false);
    }
    history.final_entries.clear();
    history.final_digest = digest_entries(&[]);
    history.raw_digest = raw_digest(&history.raw);
    assert_eq!(check(history, 100000).unwrap(), Verdict::Linearizable);
    assert!(validate(&histories)
        .unwrap_err()
        .to_string()
        .contains("operation coverage missing"));
}
#[test]
fn two_winners_impossible_get_and_realtime_order_are_detected_after_rehashing() {
    let commands = vec![
        command(0, 0, 0, Operation::PutIfAbsent, "61", Some("31"), None),
        command(1, 1, 0, Operation::PutIfAbsent, "61", Some("32"), None),
    ];
    let final_entries = vec![Entry {
        key: "61".into(),
        value: "31".into(),
    }];
    assert_eq!(
        check(
            &tiny(
                commands.clone(),
                vec![(None, None), (Some("31"), None)],
                true,
                final_entries.clone()
            ),
            100
        )
        .unwrap(),
        Verdict::Linearizable
    );
    assert_eq!(
        check(
            &tiny(
                commands,
                vec![(None, None), (None, None)],
                true,
                final_entries
            ),
            100
        )
        .unwrap(),
        Verdict::NonLinearizable
    );
    let commands = vec![
        command(0, 0, 0, Operation::Get, "61", None, None),
        command(1, 1, 0, Operation::Put, "61", Some("31"), None),
    ];
    let impossible = tiny(
        commands,
        vec![(Some("31"), None), (None, None)],
        false,
        vec![Entry {
            key: "61".into(),
            value: "31".into(),
        }],
    );
    assert_eq!(check(&impossible, 100).unwrap(), Verdict::NonLinearizable);
}
#[test]
fn each_native_single_call_family_preserves_previous_and_boolean_results() {
    use Operation::*;
    let script = [
        (Put, "61", Some("31"), None, None, None),
        (Get, "61", None, None, Some("31"), None),
        (Get, "62", None, None, None, None),
        (ContainsKey, "61", None, None, None, Some(true)),
        (ContainsKey, "62", None, None, None, Some(false)),
        (PutIfAbsent, "61", Some("32"), None, Some("31"), None),
        (PutIfAbsent, "62", Some("32"), None, None, None),
        (ReplaceIfPresent, "63", Some("33"), None, None, None),
        (ReplaceIfPresent, "62", Some("33"), None, Some("32"), None),
        (GetAndPut, "61", Some("34"), None, Some("31"), None),
        (
            ReplaceIfValue,
            "61",
            Some("35"),
            Some("ff"),
            None,
            Some(false),
        ),
        (
            ReplaceIfValue,
            "61",
            Some("35"),
            Some("34"),
            None,
            Some(true),
        ),
        (RemoveIfValue, "61", None, Some("ff"), None, Some(false)),
        (RemoveIfValue, "61", None, Some("35"), None, Some(true)),
        (GetAndRemove, "62", None, None, Some("33"), None),
        (GetAndRemove, "61", None, None, None, None),
    ];
    let mut commands = Vec::new();
    let mut results = Vec::new();
    let mut positions = [0; 2];
    for (id, (op, key, value, expected, previous, boolean)) in script.into_iter().enumerate() {
        let worker = usize::from(id == 1);
        commands.push(command(
            id,
            worker,
            positions[worker],
            op,
            key,
            value,
            expected,
        ));
        positions[worker] += 1;
        results.push((previous, boolean));
    }
    let h = tiny(commands, results, false, vec![]);
    assert_eq!(check(&h, 1000).unwrap(), Verdict::Linearizable);
}
#[test]
fn raw_and_program_canaries_reject_missing_duplicate_foreign_or_contradictory_calls() {
    let original = fixture().histories.remove(0);
    for mode in 0..15 {
        let mut h = original.clone();
        match mode {
            0 => {
                h.raw.pop();
            }
            1 => h.raw[0].call_id = 99,
            2 => h.raw[0].worker = 1 - h.raw[0].worker,
            3 => h.raw[0].stamp = 9,
            4 => h.raw[1] = h.raw[0].clone(),
            5 => h.raw[0].phase = Phase::Return,
            6 => h.raw[0].value = Some("00".into()),
            7 => h.program[0].id = 9,
            8 => h.program[0].worker = 9,
            9 => h.program[0].position = 8,
            10 => h.program[0].key = "f".into(),
            11 => h.program[0].value = None,
            12 => h.program[0].expected = Some("00".into()),
            13 => h.overlap = false,
            _ => h.raw[2].boolean = Some(true),
        }
        h.raw_digest = raw_digest(&h.raw);
        assert!(check(&h, 100000).is_err(), "mode {mode}");
    }
    let mut h = original.clone();
    h.raw_digest = "00".into();
    assert!(check(&h, 1000).is_err());
    let mut h = original.clone();
    h.final_entries.push(h.final_entries[0].clone());
    assert!(check(&h, 1000).is_err());
    let mut h = original;
    h.final_digest = "00".into();
    assert!(check(&h, 1000).is_err());
}
#[test]
fn profile_witness_identity_and_callback_canaries_never_qualify() {
    let original = serde_json::to_value(fixture()).unwrap();
    for (key, value) in [
        ("schema", json!("future")),
        ("scope", json!("production")),
        ("artifact_sha256", json!("00")),
        ("histories", json!([])),
    ] {
        let mut v = original.clone();
        v[key] = value;
        assert!(validate(&serde_json::from_value(v).unwrap()).is_err());
    }
    for (key, value) in [
        ("seed", json!(119)),
        ("attempts", json!(0)),
        ("member_uuid", json!("foreign")),
        ("clients", json!([])),
        ("overlap", json!(false)),
        ("verdict", json!("exhausted")),
        ("work", json!(0)),
        ("witness", json!([])),
        ("native_events", json!([])),
    ] {
        let mut v = original.clone();
        v["histories"][0][key] = value;
        assert!(
            validate(&serde_json::from_value(v).unwrap()).is_err(),
            "{key}"
        );
    }
    let mut h = fixture();
    h.histories[0].clients[1] = h.histories[0].clients[0].clone();
    assert!(validate(&h).is_err());
    let mut h = fixture();
    h.histories[0].witness.swap(0, 1);
    assert!(validate(&h).is_err());
    let mut h = fixture();
    h.histories[0].witness[1] = h.histories[0].witness[0];
    assert!(validate(&h).is_err());
    let mut h = fixture();
    let entry = h.histories[0].final_entries.first_mut().unwrap();
    entry.value = if entry.value == "ff" { "00" } else { "ff" }.into();
    h.histories[0].final_digest = digest_entries(&h.histories[0].final_entries);
    assert_eq!(
        check(&h.histories[0], 100_000).unwrap(),
        Verdict::NonLinearizable
    );
    assert!(validate(&h).is_err());
    for (key, value) in [
        ("kind", json!("invented")),
        ("key", json!("ff")),
        ("value", json!("x")),
        ("old_value", json!("x")),
    ] {
        let mut v = original.clone();
        v["histories"][0]["native_events"][0][key] = value;
        assert!(validate(&serde_json::from_value(v).unwrap()).is_err());
    }
    let mut v = original;
    v["owner_watermark"] = json!(0);
    assert!(serde_json::from_value::<Histories>(v).is_err());
}
#[test]
fn new_program_cannot_replay_as_the_old_seed_even_when_both_histories_are_legal() {
    fn rename(value: &mut String) {
        if !value.is_empty() {
            let first = u8::from_str_radix(&value[..1], 16).unwrap() ^ 1;
            value.replace_range(..1, &format!("{first:x}"));
        }
    }
    let saved = fixture();
    let mut fresh = saved.clone();
    for h in &mut fresh.histories {
        for c in &mut h.program {
            for v in [&mut c.value, &mut c.expected].into_iter().flatten() {
                rename(v);
            }
        }
        for e in &mut h.raw {
            if let Some(v) = &mut e.value {
                rename(v);
            }
        }
        for e in &mut h.final_entries {
            rename(&mut e.value);
        }
        for e in &mut h.native_events {
            for v in [&mut e.old_value, &mut e.value].into_iter().flatten() {
                rename(v);
            }
        }
        h.raw_digest = raw_digest(&h.raw);
        h.final_digest = digest_entries(&h.final_entries);
    }
    validate(&fresh).unwrap();
    assert!(replay(&saved, &fresh)
        .unwrap_err()
        .to_string()
        .contains("program/seed changed"));
    assert!(run(vec![]).is_err());
    assert!(observe(tempfile::tempdir().unwrap().path()).is_err());
}
#[test]
fn checked_in_native_history_schema_matches_actual_bounded_fixture() {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../docs/testing/imap/0.75/schemas/native-histories.schema.json"
    ))
    .unwrap();
    jsonschema::validator_for(&schema)
        .unwrap()
        .validate(&serde_json::to_value(fixture()).unwrap())
        .unwrap();
    let mut missing = serde_json::to_value(fixture()).unwrap();
    missing["histories"][0]["raw"][0]
        .as_object_mut()
        .unwrap()
        .remove("value");
    assert!(serde_json::from_value::<Histories>(missing).is_err());
}
