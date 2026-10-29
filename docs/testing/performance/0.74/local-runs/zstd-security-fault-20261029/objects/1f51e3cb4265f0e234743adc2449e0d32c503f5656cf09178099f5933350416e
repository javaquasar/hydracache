use super::journal::operator::{Cut, Failures, OperatorError};
use super::journal::*;
use super::operator_controls::{clear, expected, prepare, walks, OPS};
use super::owned::Owned;
use crate::{trace, AuthorityError, Event};

fn run(read: &mut Journal<Owned>, op: &str, faults: Failures) -> &'static str {
    let result = match op {
        "refuse" => read.candidate_refuse(faults),
        "advance-less" => read.candidate_advance(6, faults),
        "advance-equal" => read.candidate_advance(7, faults),
        "advance-greater" => read.candidate_advance(8, faults),
        _ => unreachable!(),
    };
    match result {
        Ok(()) => "ok",
        Err(OperatorError::Transition) => "transition",
        Err(OperatorError::Authority(error)) => match error {
            AuthorityError::Invalid => "invalid",
            AuthorityError::Budget => "budget",
            AuthorityError::Refused => "refused",
            AuthorityError::Io(error) => {
                assert_eq!(error.kind(), std::io::ErrorKind::Other);
                "io"
            }
        },
    }
}
fn healthy(g: usize) -> Vec<(usize, Event)> {
    let mut events = vec![(256, Event::DirBefore), (256, Event::InvBefore)];
    for slot in 0..g * 2 {
        events.extend(
            [
                Event::Held,
                Event::Open,
                Event::Before,
                Event::Regular,
                Event::Parent,
                Event::Read,
                Event::After,
                Event::DropFresh,
            ]
            .map(|e| (slot, e)),
        );
    }
    events.extend([(256, Event::DirAfter), (256, Event::InvAfter)]);
    events
}
fn constructors(g: usize) -> Vec<(usize, Event)> {
    (g * 2..g * 2 + 2)
        .flat_map(|slot| {
            [
                Event::Open,
                Event::Before,
                Event::Regular,
                Event::Parent,
                Event::Read,
                Event::After,
            ]
            .map(|e| (slot, e))
        })
        .collect()
}
fn successful(g: usize) -> Vec<(usize, Event)> {
    let mut events = healthy(g);
    events.extend(healthy(g));
    events.extend(constructors(g));
    events.extend(healthy(g + 1));
    events
}
fn sticky(read: &mut Journal<Owned>) {
    let originals = read.originals();
    let state = (read.entries.len(), read.refused, read.epoch);
    assert!(read.failed);
    assert_eq!(read.capacity(), 0);
    clear();
    assert_eq!(run(read, "refuse", Failures::default()), "refused");
    assert_eq!(walks(), 0);
    assert!(trace(read.entries.len() * 2).events.is_empty());
    assert_eq!(read.originals(), originals);
    assert_eq!((read.entries.len(), read.refused, read.epoch), state);
}
#[test]
fn candidate_state_priority() {
    let mut cases = 0;
    for g in GENERATIONS {
        for failed in [false, true] {
            for refused in [false, true] {
                for op in OPS {
                    let mut read = prepare(g, failed, refused);
                    let originals = read.originals();
                    let code = run(&mut read, op, Failures::default());
                    let (wanted, rounds) = expected(failed, refused, op, g);
                    assert_eq!(code, wanted);
                    assert_eq!(walks(), rounds);
                    let observed = trace(read.entries.len() * 2);
                    let events = if failed {
                        vec![]
                    } else if code == "ok" {
                        successful(g)
                    } else {
                        healthy(g)
                    };
                    assert_eq!(observed.events, events);
                    assert_eq!(observed.fresh, 0);
                    assert_eq!(&read.originals()[..originals.len()], originals);
                    if code == "ok" {
                        assert!(!read.failed);
                        assert_eq!(read.entries.len(), g + 1);
                        assert_eq!(read.refused, op == "refuse");
                        assert_eq!(read.epoch, if op == "refuse" { 7 } else { 8 });
                        assert_eq!(
                            read.capacity(),
                            if op == "refuse" { 0 } else { (g + 1) * 2113 }
                        );
                    } else {
                        assert_eq!(
                            (read.entries.len(), read.refused, read.epoch),
                            (g, refused, 7)
                        );
                    }
                    println!("HC074-OP-STATE {{\"generations\":{g},\"failed\":{failed},\"data_refused\":{refused},\"operation\":\"{op}\",\"outcome\":\"{code}\",\"walks\":{},\"events\":{},\"entries_after\":{},\"refused_after\":{},\"epoch_after\":{},\"cache_after\":{},\"trace_hash\":\"{:016x}\"}}",walks(),observed.events.len(),read.entries.len(),read.refused,read.epoch,read.capacity(),observed.hash());
                    if code != "ok" {
                        sticky(&mut read);
                    }
                    cases += 1;
                }
            }
        }
    }
    assert_eq!(cases, 80);
}
fn fault_prefix(g: usize, slot: usize, mask: u8) -> Vec<(usize, Event)> {
    let mut events = healthy(g);
    events.truncate(2 + slot * 8);
    for (bit, event) in [
        (32, Event::Held),
        (1, Event::Open),
        (2, Event::Before),
        (4, Event::Parent),
        (8, Event::Read),
        (16, Event::After),
    ] {
        if event == Event::Parent {
            events.push((slot, Event::Regular));
        }
        events.push((slot, event));
        if mask & bit != 0 {
            if !matches!(event, Event::Held | Event::Open) {
                events.push((slot, Event::DropFresh));
            }
            return events;
        }
    }
    unreachable!()
}
#[test]
fn candidate_initial_fault_priority() {
    let mut masks = 0;
    let mut gates = 0;
    for g in [1, 16, 128] {
        for refused in [false, true] {
            for op in OPS {
                for role in 0..2 {
                    for mask in 0..64 {
                        let slot = g / 2 * 2 + role;
                        let mut s = normal(slot);
                        s.faults = mask;
                        let mut read = prepare(g, false, refused);
                        let originals = read.originals();
                        let code = run(
                            &mut read,
                            op,
                            Failures {
                                change: Fault {
                                    gate: None,
                                    leaf: Some((slot, s)),
                                },
                                ..Failures::default()
                            },
                        );
                        let (wanted, rounds) = if mask == 0 {
                            expected(false, refused, op, g)
                        } else {
                            ("io", 1)
                        };
                        assert_eq!(code, wanted);
                        assert_eq!(walks(), rounds);
                        let observed = trace(read.entries.len() * 2);
                        let events = if mask != 0 {
                            fault_prefix(g, slot, mask)
                        } else if code == "ok" {
                            successful(g)
                        } else {
                            healthy(g)
                        };
                        assert_eq!(observed.events, events);
                        assert_eq!(observed.fresh, 0);
                        assert_eq!(&read.originals()[..originals.len()], originals);
                        println!("HC074-OP-FAULT {{\"generations\":{g},\"data_refused\":{refused},\"operation\":\"{op}\",\"slot\":{slot},\"mask\":{mask},\"outcome\":\"{code}\",\"walks\":{},\"events\":{},\"cache_after\":{},\"trace_hash\":\"{:016x}\"}}",walks(),observed.events.len(),read.capacity(),observed.hash());
                        if code != "ok" {
                            sticky(&mut read);
                        }
                        masks += 1;
                    }
                }
                for site in [
                    Event::DirBefore,
                    Event::InvBefore,
                    Event::DirAfter,
                    Event::InvAfter,
                ] {
                    let mut read = prepare(g, false, refused);
                    let originals = read.originals();
                    let code = run(
                        &mut read,
                        op,
                        Failures {
                            change: Fault {
                                gate: Some(site),
                                leaf: None,
                            },
                            ..Failures::default()
                        },
                    );
                    assert_eq!(code, "invalid");
                    assert_eq!(walks(), 1);
                    let observed = trace(g * 2);
                    let mut events = healthy(g);
                    events.truncate(match site {
                        Event::DirBefore => 1,
                        Event::InvBefore => 2,
                        Event::DirAfter => events.len() - 1,
                        Event::InvAfter => events.len(),
                        _ => unreachable!(),
                    });
                    assert_eq!(observed.events, events);
                    assert_eq!(read.originals(), originals);
                    println!("HC074-OP-GATE {{\"generations\":{g},\"data_refused\":{refused},\"operation\":\"{op}\",\"site\":\"{}\",\"outcome\":\"{code}\",\"walks\":{},\"events\":{},\"cache_after\":{}}}",site.name(),walks(),observed.events.len(),read.capacity());
                    sticky(&mut read);
                    gates += 1;
                }
            }
        }
    }
    assert_eq!(masks, 3072);
    assert_eq!(gates, 96);
}
#[test]
fn candidate_publication_and_external_latch() {
    let mut cuts = 0;
    let mut latches = 0;
    for g in [1, 16, 64] {
        for op in ["refuse", "advance-greater"] {
            for cut in 0..4 {
                let refused = op != "refuse";
                let mut read = prepare(g, false, refused);
                let originals = read.originals();
                let mut faults = Failures::default();
                match cut {
                    0 => faults.publish.gate = Some(Event::InvBefore),
                    1 => faults.cut = Some(Cut::BeforeAppend),
                    2 => faults.cut = Some(Cut::AfterAppend),
                    3 => {
                        let mut s = normal(0);
                        s.faults = 32;
                        faults.post.leaf = Some((0, s));
                    }
                    _ => unreachable!(),
                }
                let code = run(&mut read, op, faults);
                assert_eq!(code, if cut == 3 { "io" } else { "invalid" });
                assert_eq!(walks(), if cut == 3 { 3 } else { 2 });
                let appended = cut >= 2;
                assert_eq!(read.entries.len(), g + usize::from(appended));
                assert_eq!(&read.originals()[..originals.len()], originals);
                assert_eq!(
                    read.refused,
                    if appended { op == "refuse" } else { refused }
                );
                assert_eq!(read.epoch, if appended && op != "refuse" { 8 } else { 7 });
                let observed = trace(read.entries.len() * 2);
                let mut events = healthy(g);
                if cut == 0 {
                    events.extend([(256, Event::DirBefore), (256, Event::InvBefore)]);
                } else {
                    events.extend(healthy(g));
                    if appended {
                        events.extend(constructors(g));
                    }
                    if cut == 3 {
                        events.extend([
                            (256, Event::DirBefore),
                            (256, Event::InvBefore),
                            (0, Event::Held),
                        ]);
                    } else {
                        events.push((256, Event::Publication));
                    }
                }
                assert_eq!(observed.events, events);
                assert_eq!(observed.fresh, 0);
                println!("HC074-OP-CUT {{\"generations\":{g},\"operation\":\"{op}\",\"cut\":{cut},\"outcome\":\"{code}\",\"walks\":{},\"events\":{},\"entries_after\":{},\"refused_after\":{},\"epoch_after\":{},\"cache_after\":{},\"trace_hash\":\"{:016x}\"}}",walks(),observed.events.len(),read.entries.len(),read.refused,read.epoch,read.capacity(),observed.hash());
                sticky(&mut read);
                cuts += 1;
            }
        }
    }
    for g in [1, 16, 128] {
        for refused in [false, true] {
            for op in OPS {
                let mut read = prepare(g, false, refused);
                let originals = read.originals();
                read.latch();
                assert_eq!(walks(), 0);
                assert!(trace(g * 2).events.is_empty());
                assert_eq!(read.capacity(), 0);
                assert_eq!(read.originals(), originals);
                assert_eq!(run(&mut read, op, Failures::default()), "refused");
                assert_eq!(walks(), 0);
                assert!(trace(g * 2).events.is_empty());
                println!("HC074-OP-EXTERNAL {{\"generations\":{g},\"data_refused\":{refused},\"operation\":\"{op}\",\"outcome\":\"refused\",\"walks\":0,\"events\":0,\"cache_after\":0}}");
                sticky(&mut read);
                latches += 1;
            }
        }
    }
    assert_eq!(cuts, 24);
    assert_eq!(latches, 24);
}
