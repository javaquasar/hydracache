//! Restricted state/epoch oracle; prechecked policy/context, not real publication.
use super::controls::outcome;
use super::journal::*;
use super::owned::Owned;
use crate::{begin_round, STATE};

pub(super) const OPS: [&str; 4] = ["refuse", "advance-less", "advance-equal", "advance-greater"];
pub(super) fn expected(failed: bool, refused: bool, op: &str, g: usize) -> (&'static str, usize) {
    if failed {
        return ("refused", 0);
    }
    let valid = if op == "refuse" {
        !refused
    } else {
        refused && op == "advance-greater"
    };
    if !valid {
        ("transition", 1)
    } else if g == 128 {
        ("budget", 1)
    } else {
        ("ok", 3)
    }
}
pub(super) fn prepare(g: usize, failed: bool, refused: bool) -> Journal<Owned> {
    let mut read = Journal::<Owned>::build(g);
    read.refused = refused;
    read.epoch = 7;
    begin_round();
    read.revalidate(Fault::default()).unwrap();
    if failed {
        read.latch();
    }
    clear();
    read
}
pub(super) fn clear() {
    begin_round();
    STATE.with(|s| s.borrow_mut().walks = 0);
}
pub(super) fn walks() -> usize {
    STATE.with(|s| s.borrow().walks)
}
#[test]
fn legacy_counterexamples() {
    let mut mismatches = 0;
    let mut walk_mismatches = 0;
    for failed in [false, true] {
        for refused in [false, true] {
            for op in OPS {
                let mut read = prepare(1, failed, refused);
                let raw = outcome(match op {
                    "refuse" => read.operator_refuse(),
                    "advance-less" => read.operator_advance(6),
                    "advance-equal" => read.operator_advance(7),
                    "advance-greater" => read.operator_advance(8),
                    _ => unreachable!(),
                });
                let normalized = if raw == "invalid" { "transition" } else { raw };
                let (expected, expected_walks) = expected(failed, refused, op, 1);
                let observed_walks = walks();
                let mismatch = normalized != expected;
                let walk_mismatch = observed_walks != expected_walks;
                mismatches += usize::from(mismatch);
                walk_mismatches += usize::from(walk_mismatch);
                println!("HC074-OP-LEGACY {{\"failed\":{failed},\"data_refused\":{refused},\"operation\":\"{op}\",\"raw\":\"{raw}\",\"normalized\":\"{normalized}\",\"expected\":\"{expected}\",\"walks\":{observed_walks},\"expected_walks\":{expected_walks},\"outcome_mismatch\":{mismatch},\"walk_mismatch\":{walk_mismatch}}}");
            }
        }
    }
    assert_eq!(mismatches, 6);
    assert_eq!(walk_mismatches, 8);
}
