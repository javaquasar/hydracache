//! Generic in-memory Read baseline, not a Linux File cost or authority proof.
use super::*;
use crate::counter::{capture, Counts};
use crate::{byte, mark_held, reset, rewind, Event, Scenario, SEED, STATE};

#[derive(Clone, Copy)]
struct Case {
    name: &'static str,
    length: usize,
    budget: usize,
    chunk: usize,
    interrupted: usize,
}
const CASES: [Case; 9] = [
    Case {
        name: "empty",
        length: 0,
        budget: 65,
        chunk: 65,
        interrupted: 0,
    },
    Case {
        name: "one",
        length: 1,
        budget: 65,
        chunk: 65,
        interrupted: 0,
    },
    Case {
        name: "marker",
        length: 65,
        budget: 65,
        chunk: 65,
        interrupted: 0,
    },
    Case {
        name: "marker-fragmented",
        length: 65,
        budget: 65,
        chunk: 1,
        interrupted: 0,
    },
    Case {
        name: "record-short",
        length: 256,
        budget: 2048,
        chunk: 2048,
        interrupted: 0,
    },
    Case {
        name: "record-max",
        length: 2048,
        budget: 2048,
        chunk: 2048,
        interrupted: 0,
    },
    Case {
        name: "record-fragmented",
        length: 2048,
        budget: 2048,
        chunk: 7,
        interrupted: 0,
    },
    Case {
        name: "record-interrupted",
        length: 256,
        budget: 2048,
        chunk: 11,
        interrupted: 1,
    },
    Case {
        name: "binary",
        length: 37,
        budget: 2048,
        chunk: 3,
        interrupted: 0,
    },
];
impl Case {
    fn scenario(self) -> Scenario {
        let mut s = Scenario::new(self.length, self.chunk);
        s.interrupt_at = self.interrupted;
        s
    }
}
fn check(bytes: &[u8], scenario: Scenario) {
    assert_eq!(bytes.len(), scenario.body_len);
    for (i, value) in bytes.iter().enumerate() {
        assert_eq!(
            *value,
            byte(i, scenario.seed) ^ u8::from(i == scenario.flip)
        );
    }
}
fn counts_sum(a: Counts, b: Counts) -> Counts {
    Counts {
        allocations: a.allocations + b.allocations,
        zeroed: a.zeroed + b.zeroed,
        reallocations: a.reallocations + b.reallocations,
        requested_bytes: a.requested_bytes + b.requested_bytes,
        deallocations: a.deallocations + b.deallocations,
        deallocated_bytes: a.deallocated_bytes + b.deallocated_bytes,
    }
}
fn trace() -> String {
    STATE.with(|state| {let s=state.borrow();let events=s.order[..s.used].iter().map(|e|format!("\"{}\"",e.name())).collect::<Vec<_>>().join(",");
        format!("{{\"events\":[{events}],\"reads\":{},\"bytes\":{},\"eof\":{},\"interrupted\":{},\"read_errors\":{},\"max_request\":{},\"fresh_live\":{},\"held_live\":{},\"first_error\":\"{}\"}}",
            s.reads,s.bytes,s.eof,s.interrupted,s.read_errors,s.max_request,s.fresh_live,s.held_id.is_some(),s.first_error.map_or("none",Event::name))})
}
fn outcome<T>(result: Result<T, AuthorityError>) -> &'static str {
    match result {
        Ok(_) => "ok",
        Err(AuthorityError::Invalid) => "invalid",
        Err(AuthorityError::Budget) => "budget",
        Err(AuthorityError::Io(e)) => {
            assert_eq!(e.kind(), std::io::ErrorKind::Other);
            "io"
        }
    }
}
fn leaf(s: Scenario, dir: &File, budget: usize) -> Leaf {
    reset(s);
    let leaf = Leaf::open(dir, "record".into(), 1001, 1002, budget).unwrap();
    mark_held(&leaf.file);
    leaf
}

#[test]
fn authority_main_nine_fixed_cases() {
    let dir = File::directory();
    for case in CASES {
        let mut capacity = 0;
        let (_, counts) = capture(|| {
            for _ in 0..256 {
                reset(case.scenario());
                let value =
                    std::hint::black_box(Leaf::read(&dir, "record", 1001, 1002, case.budget))
                        .unwrap();
                check(&value.2, case.scenario());
                capacity = value.2.capacity();
                STATE.with(|state| {
                    let s = state.borrow();
                    assert_eq!(s.fresh_live, 1);
                    assert_eq!(s.bytes, case.length);
                    assert!(s.max_request <= case.budget + 1);
                    assert_eq!(s.interrupted, case.interrupted);
                });
                drop(value);
                STATE.with(|state| assert_eq!(state.borrow().fresh_live, 0));
            }
        });
        let last = trace();
        println!("HC074-AUTH-READ-MAIN {{\"case\":\"{}\",\"length\":{},\"budget\":{},\"chunk\":{},\"interrupt_at\":{},\"seed\":\"0x074a17b3\",\"samples\":256,\"output_capacity\":{capacity},\"counts\":{},\"last_trace\":{last},\"model_only\":true,\"timing_claim\":false}}",case.name,case.length,case.budget,case.chunk,case.interrupted,counts.json());
    }
}

#[test]
fn authority_separate_constructor_read_return_and_drop_stages() {
    let dir = File::directory();
    for case in CASES {
        reset(case.scenario());
        let (value, whole_return) =
            capture(|| Leaf::read(&dir, "record", 1001, 1002, case.budget).unwrap());
        check(&value.2, case.scenario());
        let capacity = value.2.capacity();
        let (_, whole_drop) = capture(|| drop(value));
        reset(case.scenario());
        let mut file = crate::open_at(&dir, "record", false).unwrap();
        let before = file.metadata().unwrap();
        regular(&before, 1001, 1002, 0o400).unwrap();
        assert_eq!(before.dev(), dir.metadata().unwrap().dev());
        let stamp = Stamp::of(&before);
        let (mut raw, constructor) = capture(|| Vec::with_capacity(before.len() as usize));
        let (_, read) = capture(|| {
            (&mut file)
                .take(case.budget as u64 + 1)
                .read_to_end(&mut raw)
                .unwrap()
        });
        assert!(Stamp::of(&file.metadata().unwrap()) == stamp);
        check(&raw, case.scenario());
        assert_eq!(raw.capacity(), capacity);
        let (_, dropped) = capture(|| drop(raw));
        drop(file);
        assert_eq!(counts_sum(constructor, read), whole_return);
        assert_eq!(dropped, whole_drop);
        assert_eq!(constructor.allocations, usize::from(case.length != 0));
        assert_eq!(constructor.requested_bytes, case.length);
        assert_eq!(constructor.zeroed, 0);
        assert_eq!(constructor.reallocations, 0);
        println!("HC074-AUTH-READ-STAGES {{\"case\":\"{}\",\"samples_per_arm\":1,\"not_pooled\":true,\"output_capacity\":{capacity},\"constructor\":{},\"read\":{},\"drop\":{},\"whole_return\":{},\"whole_drop\":{},\"model_only\":true}}",case.name,constructor.json(),read.json(),dropped.json(),whole_return.json(),whole_drop.json());
    }
}

fn expected_fault(mask: u8, revalidation: bool) -> (Option<Event>, Vec<Event>) {
    let mut events = vec![];
    if revalidation {
        events.push(Event::Held);
        if mask & 32 != 0 {
            return (Some(Event::Held), events);
        }
    }
    events.push(Event::Open);
    if mask & 1 != 0 {
        return (Some(Event::Open), events);
    }
    events.push(Event::Before);
    if mask & 2 != 0 {
        events.push(Event::DropFresh);
        return (Some(Event::Before), events);
    }
    events.extend([Event::Regular, Event::Parent]);
    if mask & 4 != 0 {
        events.push(Event::DropFresh);
        return (Some(Event::Parent), events);
    }
    events.push(Event::Read);
    if mask & 8 != 0 {
        events.push(Event::DropFresh);
        return (Some(Event::Read), events);
    }
    events.push(Event::After);
    events.push(Event::DropFresh);
    (
        if mask & 16 != 0 {
            Some(Event::After)
        } else {
            None
        },
        events,
    )
}
#[test]
fn authority_all_first_error_masks_and_fake_file_retirement() {
    let dir = File::directory();
    for revalidation in [false, true] {
        for mask in 0..if revalidation { 64 } else { 32 } {
            let normal = Scenario::new(65, 7);
            let mut scenario = normal;
            scenario.faults = mask;
            let retained = if revalidation {
                Some(leaf(normal, &dir, 65))
            } else {
                None
            };
            if revalidation {
                rewind(scenario);
            } else {
                reset(scenario);
            }
            let (code, counts) = capture(|| {
                if let Some(leaf) = &retained {
                    outcome(leaf.revalidate(&dir, 1001, 1002, 65))
                } else {
                    outcome(Leaf::read(&dir, "record", 1001, 1002, 65))
                }
            });
            let (first, expected) = expected_fault(mask, revalidation);
            STATE.with(|state| {
                let s = state.borrow();
                assert_eq!(s.first_error, first);
                assert_eq!(&s.order[..s.used], expected);
                assert_eq!(s.fresh_live, 0);
                assert_eq!(s.held_id.is_some(), revalidation);
            });
            assert_eq!(code, if first.is_none() { "ok" } else { "io" });
            if matches!(
                first,
                Some(Event::Open | Event::Before | Event::Parent | Event::Held)
            ) {
                assert_eq!(counts, Counts::default());
            }
            println!("HC074-AUTH-READ-FAULT {{\"phase\":\"{}\",\"mask\":{mask},\"outcome\":\"{code}\",\"counts\":{},\"trace\":{},\"not_pooled\":true,\"model_only\":true}}",if revalidation{"revalidate"}else{"read"},counts.json(),trace());
            drop(retained);
        }
    }
}

fn changed(mut m: Metadata, index: usize) -> Metadata {
    match index {
        0 => m.dev += 1,
        1 => m.ino += 1,
        2 => m.uid += 1,
        3 => m.gid += 1,
        4 => m.mode += 0o200,
        5 => m.links += 1,
        6 => m.len += 1,
        7 => m.mtime += 1,
        8 => m.mtime_ns += 1,
        9 => m.ctime += 1,
        10 => m.ctime_ns += 1,
        _ => panic!("bad field"),
    };
    m
}
#[test]
fn authority_metadata_predicates_and_all_stamp_fields() {
    let dir = File::directory();
    for held in [false, true] {
        for field in 0..11 {
            let normal = Scenario::new(65, 7);
            let retained = leaf(normal, &dir, 65);
            let mut drift = normal;
            if held {
                drift.held = changed(normal.held, field);
            } else {
                drift.after = changed(normal.after, field);
            }
            rewind(drift);
            assert_eq!(
                outcome(retained.revalidate(&dir, 1001, 1002, 65)),
                "invalid"
            );
            STATE.with(|state| {
                let s = state.borrow();
                assert_eq!(s.fresh_live, 0);
                assert!(s.held_id.is_some());
                if held {
                    assert_eq!(&s.order[..s.used], &[Event::Held]);
                }
            });
            check(&retained.raw, normal);
            drop(retained);
        }
    }
    for index in 0..5 {
        let mut s = Scenario::new(65, 7);
        match index {
            0 => s.before.mode = 0o040400,
            1 => s.before.uid += 1,
            2 => s.before.gid += 1,
            3 => s.before.mode = 0o100600,
            4 => s.before.links = 2,
            _ => unreachable!(),
        };
        reset(s);
        assert_eq!(
            outcome(Leaf::read(&dir, "record", 1001, 1002, 65)),
            "invalid"
        );
        STATE.with(|state| assert_eq!(state.borrow().reads, 0));
    }
    let mut s = Scenario::new(65, 7);
    s.parent_dev += 1;
    reset(s);
    assert_eq!(
        outcome(Leaf::read(&dir, "record", 1001, 1002, 65)),
        "budget"
    );
    println!("HC074-AUTH-READ-METADATA {{\"held_fields\":11,\"after_fields\":11,\"regular_predicates\":5,\"parent_device\":1,\"model_only\":true}}");
}

#[test]
fn authority_bounds_eof_partial_error_and_standard_interrupted() {
    let dir = File::directory();
    for case in 0..9 {
        let mut s = Scenario::new(10, 7);
        let mut budget = 65;
        let expected = match case {
            0 => {
                s.body_len = 9;
                "invalid"
            }
            1 => {
                s.body_len = 11;
                "invalid"
            }
            2 => {
                s.body_len = 67;
                "invalid"
            }
            3 => {
                s = Scenario::new(66, 7);
                "budget"
            }
            4 => {
                s = Scenario::new(65, 7);
                s.error_at = 2;
                "io"
            }
            5 => {
                s = Scenario::new(65, 7);
                s.interrupt_at = 1;
                "ok"
            }
            6 => {
                s.body_len = 0;
                "invalid"
            }
            7 => {
                s = Scenario::new(0, 1);
                budget = 0;
                "ok"
            }
            8 => {
                s = Scenario::new(0, 1);
                s.body_len = 1;
                budget = 0;
                "invalid"
            }
            _ => unreachable!(),
        };
        reset(s);
        assert_eq!(
            outcome(Leaf::read(&dir, "record", 1001, 1002, budget)),
            expected
        );
        STATE.with(|state| {
            let state = state.borrow();
            assert_eq!(state.fresh_live, 0);
            assert!(state.max_request <= budget + 1);
            if case == 2 {
                assert_eq!(state.bytes, 66);
                assert_eq!(state.eof, 0);
            }
            if case == 4 {
                assert_eq!(state.bytes, 7);
                assert_eq!(state.reads, 2);
                assert!(!state.order[..state.used].contains(&Event::After));
            }
            if case == 5 {
                assert_eq!(state.interrupted, 1);
                assert_eq!(state.bytes, 65);
            }
        });
    }
    println!("HC074-AUTH-READ-BOUNDS {{\"cases\":9,\"over_bound_bytes\":66,\"partial_error_bytes\":7,\"standard_interrupted\":1,\"application_retry\":false,\"model_only\":true}}");
}

#[test]
fn authority_seeded_owned_returns_and_fresh_content_drift() {
    let dir = File::directory();
    let mut seed = SEED;
    for _ in 0..256 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut s = Scenario::new(1 + (seed as usize % 2048), 1 + (seed as usize % 31));
        s.seed = seed;
        reset(s);
        let (file, _, mut bytes) = Leaf::read(&dir, "record", 1001, 1002, 2048).unwrap();
        drop(file);
        let independent = bytes.clone();
        reset(Scenario::new(0, 1));
        check(&bytes, s);
        bytes[0] ^= 255;
        check(&independent, s);
        let retained = leaf(s, &dir, 2048);
        let mut drift = s;
        drift.flip = (seed.rotate_left(7) as usize) % s.body_len;
        rewind(drift);
        assert_eq!(
            outcome(retained.revalidate(&dir, 1001, 1002, 2048)),
            "invalid"
        );
        check(&retained.raw, s);
        drop(retained);
    }
    println!("HC074-AUTH-READ-OWNERSHIP {{\"seed\":\"0x074a17b3\",\"samples\":256,\"owned_after_token_drop_and_receiver_reset\":true,\"independent_vec_clone\":true,\"fresh_content_drift_refused\":true,\"model_only\":true}}");
}

#[test]
fn authority_synthetic_metadata_and_drop_receivers_allocate_nothing() {
    let dir = File::directory();
    reset(Scenario::new(65, 7));
    let file = crate::open_at(&dir, "record", false).unwrap();
    let (_, metadata) = capture(|| {
        let before = file.metadata().unwrap();
        regular(&before, 1001, 1002, 0o400).unwrap();
        let parent = dir.metadata().unwrap();
        assert_eq!(before.dev(), parent.dev());
        let stamp = Stamp::of(&before);
        assert!(Stamp::of(&file.metadata().unwrap()) == stamp);
    });
    let (_, dropped) = capture(|| drop(file));
    assert_eq!(metadata, Counts::default());
    assert_eq!(dropped, Counts::default());
    println!(
        "HC074-AUTH-READ-RECEIVERS {{\"metadata\":{},\"token_drop\":{},\"model_only\":true}}",
        metadata.json(),
        dropped.json()
    );
}
