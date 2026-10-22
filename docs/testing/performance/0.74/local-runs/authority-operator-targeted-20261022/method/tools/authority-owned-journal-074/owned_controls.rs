use super::controls::{main, outcome};
use super::journal::*;
use super::owned::Owned;
use super::*;
use crate::counter::{capture, Counts};
use crate::{begin_round, trace, Event, SEED, STATE};

#[test]
fn owned_main() {
    for generations in GENERATIONS {
        let (reference, expected, read) = main::<Leaf>(generations);
        drop(read);
        let (candidate, actual, mut read) = main::<Owned>(generations);
        assert_eq!(actual, expected);
        assert_eq!(candidate.allocations, generations * 2);
        assert_eq!(candidate.requested_bytes, generations * 2113);
        assert_eq!(candidate.reallocations, 0);
        assert_eq!(candidate.deallocations, 0);
        assert_eq!(read.capacity(), generations * 2113);
        let originals = read.originals();
        let (_, eviction) = capture(|| read.evict());
        assert_eq!(read.capacity(), 0);
        assert_eq!(read.originals(), originals);
        assert_eq!(eviction.deallocated_bytes, generations * 2113);
        println!("HC074-OWNED-MAIN {{\"generations\":{generations},\"rounds\":16,\"reference\":{},\"candidate\":{},\"eviction\":{},\"trace\":{}}}",reference.json(),candidate.json(),eviction.json(),actual.json());
    }
}
fn execute<L: Storage>(
    generations: usize,
    fault: Fault,
    outer: Option<Event>,
) -> (&'static str, crate::Trace, Counts) {
    let mut read = Journal::<L>::build(generations);
    begin_round();
    read.revalidate(Fault::default()).unwrap();
    let originals = read.originals();
    begin_round();
    let (code, counts) = capture(|| {
        outcome(if let Some(site) = outer {
            read.outer_failure(site)
        } else {
            read.revalidate(fault)
        })
    });
    let events = trace(generations * 2);
    assert_eq!(read.originals(), originals);
    if code != "ok" {
        assert!(read.failed);
        assert_eq!(read.capacity(), 0);
        begin_round();
        let (second, counts) = capture(|| outcome(read.revalidate(Fault::default())));
        assert_eq!(second, "refused");
        assert_eq!(counts, Counts::default());
        assert_eq!(trace(generations * 2).events.len(), 0);
    }
    (code, events, counts)
}
fn pair(
    g: usize,
    fault: Fault,
    outer: Option<Event>,
) -> (&'static str, crate::Trace, Counts, Counts) {
    let (expected, events, reference) = execute::<Leaf>(g, fault, outer);
    let (actual, observed, candidate) = execute::<Owned>(g, fault, outer);
    assert_eq!(actual, expected);
    assert_eq!(observed, events);
    (actual, observed, reference, candidate)
}
#[test]
fn owned_fault_matrix() {
    let mut cases = 0;
    for g in [1, 2, 16, 128] {
        let mut positions = vec![0, g / 2, g - 1];
        positions.sort_unstable();
        positions.dedup();
        for index in positions {
            for role in 0..2 {
                for mask in 0..64 {
                    let slot = index * 2 + role;
                    let mut s = normal(slot);
                    s.faults = mask;
                    let (code, events, reference, candidate) = pair(
                        g,
                        Fault {
                            gate: None,
                            leaf: Some((slot, s)),
                        },
                        None,
                    );
                    assert_eq!(code, if mask == 0 { "ok" } else { "io" });
                    assert_eq!(candidate.allocations, 0);
                    assert_eq!(candidate.reallocations, 0);
                    assert_eq!(
                        candidate.deallocated_bytes,
                        if mask == 0 { 0 } else { g * 2113 }
                    );
                    println!("HC074-OWNED-FAULT {{\"generations\":{g},\"slot\":{slot},\"mask\":{mask},\"outcome\":\"{code}\",\"reference\":{},\"candidate\":{},\"trace\":{}}}",reference.json(),candidate.json(),events.json());
                    cases += 1;
                }
            }
        }
        for site in [
            Event::DirBefore,
            Event::InvBefore,
            Event::DirAfter,
            Event::InvAfter,
            Event::Bind,
            Event::Transition,
            Event::Publication,
            Event::External,
        ] {
            let outer = matches!(
                site,
                Event::Bind | Event::Transition | Event::Publication | Event::External
            );
            let (code, events, _, candidate) = pair(
                g,
                Fault {
                    gate: Some(site),
                    leaf: None,
                },
                if outer { Some(site) } else { None },
            );
            assert_eq!(code, "invalid");
            assert_eq!(candidate.allocations, 0);
            assert_eq!(candidate.deallocated_bytes, g * 2113);
            println!("HC074-OWNED-GATE {{\"generations\":{g},\"site\":\"{}\",\"outcome\":\"{code}\",\"candidate\":{},\"trace\":{}}}",site.name(),candidate.json(),events.json());
        }
    }
    assert_eq!(cases, 1152);
}
fn changed(mut m: crate::Metadata, field: usize) -> crate::Metadata {
    match field {
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
    };
    m
}
#[test]
fn owned_metadata_and_io() {
    let mut cases = 0;
    for slot in [2, 3] {
        let normal = normal(slot);
        for held in [false, true] {
            for field in 0..11 {
                let mut s = normal;
                if held {
                    s.held = changed(s.held, field);
                } else {
                    s.after = changed(s.after, field);
                }
                assert_eq!(
                    pair(
                        2,
                        Fault {
                            gate: None,
                            leaf: Some((slot, s))
                        },
                        None
                    )
                    .0,
                    "invalid"
                );
                cases += 1;
            }
        }
        for field in 0..6 {
            let mut s = normal;
            match field {
                0 => s.before.mode = 0o040400,
                1 => s.before.uid += 1,
                2 => s.before.gid += 1,
                3 => s.before.mode = 0o100600,
                4 => s.before.links = 2,
                5 => s.parent_dev += 1,
                _ => unreachable!(),
            }
            assert_eq!(
                pair(
                    2,
                    Fault {
                        gate: None,
                        leaf: Some((slot, s))
                    },
                    None
                )
                .0,
                if field == 5 { "budget" } else { "invalid" }
            );
            cases += 1;
        }
        for case in 0..7 {
            let mut s = normal;
            let expected = match case {
                0 => {
                    s.body_len -= 1;
                    "invalid"
                }
                1 => {
                    s.body_len += 1;
                    "invalid"
                }
                2 => {
                    s.before.len += 1;
                    "budget"
                }
                3 => {
                    s.chunk = 7;
                    s.error_at = 2;
                    "io"
                }
                4 => {
                    s.interrupt_at = 1;
                    "ok"
                }
                5 => {
                    s.body_len = 0;
                    "invalid"
                }
                6 => {
                    s.chunk = 1;
                    s.error_at = 2;
                    "io"
                }
                _ => unreachable!(),
            };
            assert_eq!(
                pair(
                    2,
                    Fault {
                        gate: None,
                        leaf: Some((slot, s))
                    },
                    None
                )
                .0,
                expected
            );
            cases += 1;
        }
        let mut seed = SEED;
        for _ in 0..256 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let mut s = normal;
            s.flip = seed as usize % s.body_len;
            assert_eq!(
                pair(
                    2,
                    Fault {
                        gate: None,
                        leaf: Some((slot, s))
                    },
                    None
                )
                .0,
                "invalid"
            );
            cases += 1;
        }
    }
    assert_eq!(cases, 582);
    println!("HC074-OWNED-DRIFT {{\"cases\":582,\"metadata\":56,\"io\":14,\"content\":512,\"seed\":\"0x074a17b3\",\"traces_equal\":true}}");
}
#[test]
fn owned_lifecycle() {
    let mut read = Journal::<Owned>::build(1);
    begin_round();
    read.revalidate(Fault::default()).unwrap();
    let mut growths = 0;
    while read.entries.len() < 128 {
        let old = read.originals();
        let capacity = read.entries.capacity();
        begin_round();
        read.append(false).unwrap();
        if read.entries.capacity() > capacity {
            growths += 1;
        }
        assert_eq!(&read.originals()[..old.len()], old);
        assert_eq!(read.capacity(), read.entries.len() * 2113);
    }
    assert!(growths > 0);
    let before = read.originals();
    begin_round();
    assert_eq!(outcome(read.append(false)), "budget");
    assert!(read.failed);
    assert_eq!(read.capacity(), 0);
    assert_eq!(read.originals(), before);
    drop(read);
    let mut read = Journal::<Owned>::build(1);
    begin_round();
    read.operator_refuse().unwrap();
    assert!(read.refused && !read.failed);
    assert_eq!(read.capacity(), 0);
    begin_round();
    read.operator_advance(2).unwrap();
    assert!(!read.refused && !read.failed);
    assert_eq!(read.epoch, 2);
    let before = read.originals();
    let generations = read.entries.len();
    let epoch = read.epoch;
    drop(read);
    let mut reopened = Journal::<Owned>::build(generations);
    reopened.epoch = epoch;
    assert_eq!(reopened.capacity(), 0);
    assert_eq!(
        before.iter().map(|(_, hash)| hash).collect::<Vec<_>>(),
        reopened
            .originals()
            .iter()
            .map(|(_, hash)| hash)
            .collect::<Vec<_>>()
    );
    assert!(before
        .iter()
        .all(|(old, _)| reopened.originals().iter().all(|(new, _)| old != new)));
    begin_round();
    reopened.revalidate(Fault::default()).unwrap();
    begin_round();
    assert_eq!(outcome(reopened.operator_advance(3)), "invalid");
    assert!(reopened.failed);
    assert_eq!(reopened.capacity(), 0);
    drop(reopened);
    let mut read = Journal::<Owned>::build(1);
    begin_round();
    read.revalidate(Fault::default()).unwrap();
    let old = read.originals();
    begin_round();
    assert_eq!(outcome(read.append(true)), "invalid");
    assert_eq!(&read.originals()[..old.len()], old);
    assert_eq!(read.entries.len(), 2);
    assert_eq!(read.capacity(), 0);
    assert!(read.failed);
    begin_round();
    assert_eq!(outcome(read.append(false)), "refused");
    drop(read);
    STATE.with(|state| {
        assert_eq!(state.borrow().held_count, 0);
        assert_eq!(state.borrow().fresh_live, 0);
    });
    println!("HC074-OWNED-LIFECYCLE {{\"append_to_generations\":128,\"growths\":{growths},\"originals_preserved\":true,\"reopen_new_tokens\":true,\"reopen_cold\":true,\"operator_state_distinct\":true,\"append_failure_sticky\":true}}");
}
#[test]
fn owned_layout_drop_and_thread_affinity() {
    use std::mem::{align_of, size_of};
    let mut read = Journal::<Owned>::build(2);
    begin_round();
    read.revalidate(Fault::default()).unwrap();
    let old = read.originals();
    let (_, eviction) = capture(|| read.evict());
    assert_eq!(read.originals(), old);
    assert_eq!(eviction.deallocated_bytes, 4226);
    begin_round();
    read.revalidate(Fault::default()).unwrap();
    let (_, dropped) = capture(|| drop(read));
    STATE.with(|s| assert_eq!(s.borrow().held_count, 0));
    trait AmbiguousSend<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousSend<()> for T {}
    struct IfSend;
    impl<T: ?Sized + Send> AmbiguousSend<IfSend> for T {}
    let _ = <Journal<Owned> as AmbiguousSend<_>>::marker;
    trait AmbiguousSync<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousSync<()> for T {}
    struct IfSync;
    impl<T: ?Sized + Sync> AmbiguousSync<IfSync> for T {}
    let _ = <Journal<Owned> as AmbiguousSync<_>>::marker;
    println!("HC074-OWNED-LAYOUT {{\"leaf\":{},\"owned_leaf\":{},\"entry\":{},\"owned_entry\":{},\"journal\":{},\"owned_journal\":{},\"owned_align\":{},\"eviction\":{},\"full_drop\":{},\"send\":false,\"sync\":false,\"synthetic_not_linux_layout\":true}}",size_of::<Leaf>(),size_of::<Owned>(),size_of::<Entry<Leaf>>(),size_of::<Entry<Owned>>(),size_of::<Journal<Leaf>>(),size_of::<Journal<Owned>>(),align_of::<Owned>(),eviction.json(),dropped.json());
}
