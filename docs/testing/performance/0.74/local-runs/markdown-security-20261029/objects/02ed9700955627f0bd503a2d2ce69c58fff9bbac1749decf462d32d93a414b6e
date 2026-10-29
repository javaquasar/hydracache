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

use super::candidate::Scratch;

fn prime(retained: &Leaf, dir: &File, s: Scenario, budget: usize) {
    rewind(s);
    retained.revalidate(dir, 1001, 1002, budget).unwrap();
}

#[test]
fn scratch_cold_main() {
    let dir = File::directory();
    for case in CASES {
        let retained = leaf(case.scenario(), &dir, case.budget);
        let mut reference = Counts::default();
        let mut expected = String::new();
        for _ in 0..256 {
            rewind(case.scenario());
            let (code, counts) =
                capture(|| outcome(retained.revalidate(&dir, 1001, 1002, case.budget)));
            assert_eq!(code, "ok");
            reference = counts_sum(reference, counts);
            let current = trace();
            if !expected.is_empty() {
                assert_eq!(current, expected);
            }
            expected = current;
        }
        let mut scratch = Scratch::new(&retained);
        let mut candidate = Counts::default();
        for sample in 0..256 {
            rewind(case.scenario());
            let (code, counts) =
                capture(|| outcome(scratch.revalidate(&dir, 1001, 1002, case.budget)));
            assert_eq!(code, "ok");
            assert_eq!(trace(), expected);
            assert_eq!(
                counts.allocations,
                usize::from(sample == 0 && case.length != 0)
            );
            assert_eq!(counts.reallocations, 0);
            candidate = counts_sum(candidate, counts);
            assert_eq!(scratch.cached_capacity(), Some(case.length));
            check(&retained.raw, case.scenario());
        }
        let capacity = scratch.cached_capacity().unwrap();
        let (_, dropped) = capture(|| drop(scratch));
        assert_eq!(dropped.deallocated_bytes, capacity);
        println!("HC074-SCRATCH-MAIN {{\"case\":\"{}\",\"length\":{},\"samples\":256,\"cold\":true,\"reference\":{},\"candidate\":{},\"cache_capacity\":{capacity},\"cache_drop\":{},\"trace\":{expected}}}",case.name,case.length,reference.json(),candidate.json(),dropped.json());
    }
}

// Prime both arms identically; only storage lifetime differs at target entry.
fn pair(
    normal: Scenario,
    target: Scenario,
    budget: usize,
) -> (&'static str, String, Counts, Counts, Option<usize>) {
    let dir = File::directory();
    let retained = leaf(normal, &dir, budget);
    prime(&retained, &dir, normal, budget);
    rewind(target);
    let (code, reference) = capture(|| outcome(retained.revalidate(&dir, 1001, 1002, budget)));
    let expected = trace();
    let mut scratch = Scratch::new(&retained);
    rewind(normal);
    scratch.revalidate(&dir, 1001, 1002, budget).unwrap();
    rewind(target);
    let (candidate_code, candidate) =
        capture(|| outcome(scratch.revalidate(&dir, 1001, 1002, budget)));
    assert_eq!(candidate_code, code);
    assert_eq!(trace(), expected);
    let cached = scratch.cached_capacity();
    if code != "ok" {
        assert_eq!(cached, None);
    }
    check(&retained.raw, normal);
    (code, expected, reference, candidate, cached)
}

#[test]
fn scratch_all_fault_masks() {
    for mask in 0..64 {
        let normal = Scenario::new(65, 7);
        let mut target = normal;
        target.faults = mask;
        let (code, events, reference, candidate, cached) = pair(normal, target, 65);
        assert_eq!(candidate.allocations, 0);
        assert_eq!(candidate.reallocations, 0);
        assert_eq!(
            candidate.deallocated_bytes,
            if code == "ok" { 0 } else { 65 }
        );
        println!("HC074-SCRATCH-FAULT {{\"mask\":{mask},\"outcome\":\"{code}\",\"reference\":{},\"candidate\":{},\"cached\":{},\"trace\":{events}}}",reference.json(),candidate.json(),cached.map_or("null".into(),|n|n.to_string()));
    }
}

#[test]
fn scratch_io_schedule_matrix() {
    let mut cases = 0;
    for declared in [0, 1, 9, 10, 11, 65, 66] {
        for body in [0, 9, 10, 11, 65, 67] {
            for chunk in [1, 7, 128] {
                for error in [0, 1, 2, 3, 4, 12] {
                    for interrupted in [0, 1] {
                        let normal = Scenario::new(10, chunk);
                        let mut target = normal;
                        target.before.len = declared;
                        target.after.len = declared;
                        target.body_len = body;
                        target.error_at = error;
                        target.interrupt_at = interrupted;
                        let (code, events, _, _, cached) = pair(normal, target, 65);
                        println!("HC074-SCRATCH-IO {{\"declared\":{declared},\"body\":{body},\"chunk\":{chunk},\"error\":{error},\"interrupted\":{interrupted},\"outcome\":\"{code}\",\"cached\":{},\"trace\":{events}}}",cached.map_or("null".into(),|n|n.to_string()));
                        cases += 1;
                    }
                }
            }
        }
    }
    assert_eq!(cases, 1512);
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
        _ => unreachable!(),
    }
    m
}
#[test]
fn scratch_metadata_and_seeded_drift() {
    let normal = Scenario::new(65, 7);
    let mut cases = 0;
    for held in [false, true] {
        for field in 0..11 {
            let mut target = normal;
            if held {
                target.held = changed(normal.held, field);
            } else {
                target.after = changed(normal.after, field);
            }
            assert_eq!(pair(normal, target, 65).0, "invalid");
            cases += 1;
        }
    }
    for field in 0..5 {
        let mut target = normal;
        match field {
            0 => target.before.mode = 0o040400,
            1 => target.before.uid += 1,
            2 => target.before.gid += 1,
            3 => target.before.mode = 0o100600,
            4 => target.before.links += 1,
            _ => unreachable!(),
        }
        assert_eq!(pair(normal, target, 65).0, "invalid");
        cases += 1;
    }
    let mut target = normal;
    target.parent_dev += 1;
    assert_eq!(pair(normal, target, 65).0, "budget");
    cases += 1;
    assert_eq!(cases, 28);
    let mut seed = SEED;
    for _ in 0..256 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut s = Scenario::new(1 + (seed as usize % 2048), 1 + (seed as usize % 31));
        s.seed = seed;
        let mut drift = s;
        drift.flip = (seed.rotate_left(7) as usize) % s.body_len;
        assert_eq!(pair(s, drift, 2048).0, "invalid");
    }
    println!("HC074-SCRATCH-DRIFT {{\"metadata\":28,\"content\":256,\"seed\":\"0x074a17b3\",\"all_refused\":true}}");
}

#[test]
fn scratch_owner_lifetime_and_layout() {
    let dir = File::directory();
    let normal = Scenario::new(65, 65);
    let retained = leaf(normal, &dir, 65);
    let (mut scratch, constructor) = capture(|| Scratch::new(&retained));
    assert_eq!(constructor, Counts::default());
    assert!(std::ptr::eq(scratch.owner(), &retained));
    rewind(normal);
    scratch.revalidate(&dir, 1001, 1002, 65).unwrap();
    assert_eq!(scratch.cached_capacity(), Some(65));
    let (_, dropped) = capture(|| drop(scratch));
    assert_eq!(dropped.deallocations, 1);
    assert_eq!(dropped.deallocated_bytes, 65);
    STATE.with(|s| assert!(s.borrow().held_id.is_some()));
    check(&retained.raw, normal);
    println!("HC074-SCRATCH-LIFETIME {{\"constructor\":{},\"cache_drop\":{},\"size\":{},\"align\":{},\"owner_alive\":true,\"owner_bytes_unchanged\":true}}",constructor.json(),dropped.json(),std::mem::size_of::<Scratch<'_>>(),std::mem::align_of::<Scratch<'_>>());
}

fn timed(candidate: bool, length: usize, budget: usize) -> (u128, usize) {
    let dir = File::directory();
    let retained = super::cost::owner(length, &dir);
    if candidate {
        let mut scratch = Scratch::new(&retained);
        super::cost::calls(|| scratch.revalidate(&dir, 1001, 1002, budget), length, 64);
        super::cost::interval(|| scratch.revalidate(&dir, 1001, 1002, budget), length)
    } else {
        super::cost::calls(|| retained.revalidate(&dir, 1001, 1002, budget), length, 64);
        super::cost::interval(|| retained.revalidate(&dir, 1001, 1002, budget), length)
    }
}
#[test]
fn scratch_paired_cost() {
    for (length, budget) in super::cost::COSTS {
        for pair in 0..7 {
            let (reference, candidate) = if pair % 2 == 0 {
                (timed(false, length, budget), timed(true, length, budget))
            } else {
                let c = timed(true, length, budget);
                (timed(false, length, budget), c)
            };
            assert_eq!(reference.1, length * 512);
            assert_eq!(candidate.1, reference.1);
            println!("HC074-SCRATCH-COST {{\"length\":{length},\"budget\":{budget},\"pair\":{pair},\"candidate_first\":{},\"warm\":64,\"calls\":512,\"checksum\":{},\"reference_ns\":{},\"candidate_ns\":{}}}",pair%2!=0,reference.1,reference.0,candidate.0);
        }
    }
}
