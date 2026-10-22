//! Paired pure controls; timing is confined to the explicitly bounded getter test.
use super::*;
use crate::counter::{capture, Counts};
use crate::{events, prototype, reset};
use cost::{BOOT, ITERATIONS, WARMUP};

const HEALTHY: [crate::Event; 5] = [
    crate::Event::Span,
    crate::Event::Before,
    crate::Event::BootClock,
    crate::Event::RealClock,
    crate::Event::After,
];
fn cases() -> Vec<(&'static str, String)> {
    let mut spare = String::with_capacity(256);
    spare.push_str(BOOT);
    vec![
        ("empty", String::new()),
        ("one", "x".into()),
        ("below", "a".repeat(35)),
        ("fixed", BOOT.into()),
        ("fixed-spare", spare),
        ("above", "b".repeat(37)),
        ("utf8-generic", "é".repeat(18)),
        ("long", "c".repeat(64)),
        ("larger", "d".repeat(1024)),
    ]
}

fn candidate_point(boot: String, boot_ns: u64, real_ns: u64) -> prototype::TimePoint {
    reset(0, boot_ns, real_ns);
    prototype::Origin { boot }.sample().unwrap()
}
fn expected(length: usize, inline: bool) -> Counts {
    let allocations = usize::from(length != 0 && !(inline && length <= 36));
    Counts {
        allocations,
        requested_bytes: allocations * length,
        deallocations: allocations,
        deallocated_bytes: allocations * length,
        ..Counts::default()
    }
}
fn multiplied(c: Counts, n: usize) -> Counts {
    Counts {
        allocations: c.allocations * n,
        requested_bytes: c.requested_bytes * n,
        deallocations: c.deallocations * n,
        deallocated_bytes: c.deallocated_bytes * n,
        ..c
    }
}
fn same(point: &prototype::TimePoint, other: &TimePoint) {
    assert_eq!(point.boot_id(), other.boot_id());
    assert_eq!(point.boottime_ns(), other.boottime_ns());
    assert_eq!(point.realtime_ns, other.realtime_ns);
    assert_eq!(point.unix_seconds(), other.unix_seconds());
}

#[test]
fn inline_paired_sample_requests() {
    for (name, boot) in cases() {
        let length = boot.len();
        let input_capacity = boot.capacity();
        let mut copied = String::with_capacity(input_capacity);
        copied.push_str(&boot);
        assert_eq!(copied.capacity(), input_capacity);
        let reference = Origin { boot };
        let candidate = prototype::Origin { boot: copied };
        let (_, old) = capture(|| {
            for _ in 0..256 {
                reset(0, 90, 2_000_000_007);
                let point = std::hint::black_box(reference.sample()).unwrap();
                assert_eq!(point.boot_id(), reference.boot);
                assert_eq!(point.boottime_ns(), 90);
                assert_eq!(point.realtime_ns, 2_000_000_007);
                assert_eq!(point.unix_seconds(), 2);
                assert_eq!(events(), (HEALTHY, 5));
            }
        });
        let mut heap_capacity = 0;
        let (_, new) = capture(|| {
            for _ in 0..256 {
                reset(0, 90, 2_000_000_007);
                let point = std::hint::black_box(candidate.sample()).unwrap();
                assert_eq!(point.boot_id(), reference.boot);
                assert_eq!(point.boottime_ns(), 90);
                assert_eq!(point.realtime_ns, 2_000_000_007);
                assert_eq!(point.unix_seconds(), 2);
                assert_eq!(events(), (HEALTHY, 5));
                heap_capacity = point.heap_capacity();
                if length != 0 {
                    assert_ne!(point.boot_id().as_ptr(), candidate.boot.as_ptr());
                }
            }
        });
        assert_eq!(old, multiplied(expected(length, false), 256));
        assert_eq!(new, multiplied(expected(length, true), 256));
        assert_eq!(heap_capacity, if length <= 36 { 0 } else { length });
        println!("HC074-BOOT-INLINE-SAMPLES {{\"case\":\"{name}\",\"length\":{length},\"input_capacity\":{input_capacity},\"heap_capacity\":{heap_capacity},\"samples_per_arm\":256,\"reference\":{},\"candidate\":{},\"model_only\":true}}",old.json(),new.json());
    }
}

#[test]
fn inline_all_first_fault_prefixes() {
    let reference = Origin { boot: BOOT.into() };
    let candidate = prototype::Origin { boot: BOOT.into() };
    for faults in 0..16 {
        reset(faults, 90, 2_000_000_007);
        let (old, old_counts) = capture(|| reference.sample());
        let old_events = events();
        reset(faults, 90, 2_000_000_007);
        let (new, new_counts) = capture(|| candidate.sample());
        assert_eq!(events(), old_events);
        assert_eq!(old.as_ref().err(), new.as_ref().err());
        assert_eq!(new_counts, Counts::default());
        if faults == 0 {
            same(&new.unwrap(), &old.unwrap());
            assert_eq!(old_counts.allocations, 1);
            assert_eq!(old_counts.requested_bytes, 36);
        } else {
            assert_eq!(old_counts, Counts::default());
        }
    }
    println!("HC074-BOOT-INLINE-FAULTS {{\"masks\":16,\"refused\":15,\"same_first_error_and_events\":true,\"refused_allocations\":0,\"model_only\":true}}");
}

fn owned_case(boot: String, boot_ns: u64, real_ns: u64) {
    let mut reference = Origin { boot: boot.clone() };
    let mut candidate = prototype::Origin { boot: boot.clone() };
    reset(0, boot_ns, real_ns);
    let mut old = reference.sample().unwrap();
    reset(0, boot_ns, real_ns);
    let mut new = candidate.sample().unwrap();
    same(&new, &old);
    assert_eq!(format!("{new:?}"), format!("{old:?}"));
    assert_eq!(format!("{new:#?}"), format!("{old:#?}"));
    let old_clone = old.clone();
    let new_clone = new.clone();
    if !boot.is_empty() {
        assert_ne!(new.boot_id().as_ptr(), new_clone.boot_id().as_ptr());
        assert_ne!(new.boot_id().as_ptr(), candidate.boot.as_ptr());
    }
    reference.boot.clear();
    candidate.boot.clear();
    drop(reference);
    drop(candidate);
    same(&new, &old);
    assert_eq!(new.boot_id(), boot);
    old.boot.clear();
    new.replace_boot("");
    same(&new_clone, &old_clone);
    assert_eq!(new_clone.boot_id(), boot);
}

#[test]
fn inline_ownership_generic_boundaries_seed_and_debug() {
    for (_, boot) in cases() {
        owned_case(boot, u64::MAX, u64::MAX);
    }
    let mut generic = 0;
    for unit in ["a", "é", "€", "🦀", "\0", "\"\\\n\t"] {
        for n in 0..=40 {
            owned_case(unit.repeat(n), n as u64, n as u64 * 1_000_000_007);
            generic += 1;
        }
    }
    let mut seed = 0x074b_0a19_u64;
    for sample in 0..256 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut raw = [0; 38];
        for (i, byte) in raw[..36].iter_mut().enumerate() {
            *byte = if [8, 13, 18, 23].contains(&i) {
                b'-'
            } else {
                b"0123456789abcdef"[(seed.rotate_left(i as u32) & 15) as usize]
            };
        }
        raw[36] = b'\n';
        let boot = boot_value((raw, 37)).unwrap();
        owned_case(boot, seed, if sample % 2 == 0 { u64::MAX } else { seed });
    }
    assert_eq!(generic, 246);
    println!("HC074-BOOT-INLINE-OWNERSHIP {{\"corpus\":9,\"generic\":246,\"seeded\":256,\"seed\":\"0x074b0a19\",\"debug_and_pretty_debug_equal\":true,\"owned_and_independent\":true,\"model_only\":true}}");
}

fn getter_candidate(point: &prototype::TimePoint, iterations: usize) -> u64 {
    let mut checksum = 0_u64;
    for _ in 0..iterations {
        checksum = checksum.wrapping_add(std::hint::black_box(point).boot_id().len() as u64);
    }
    std::hint::black_box(checksum)
}
fn time_candidate(point: &prototype::TimePoint) -> (u128, u64) {
    let start = std::time::Instant::now();
    let checksum = getter_candidate(point, ITERATIONS);
    (start.elapsed().as_nanos(), checksum)
}

#[test]
fn inline_layout_clone_and_getter_cost_controls() {
    let old = cost::reference_point();
    let new = candidate_point(BOOT.into(), 90, 2_000_000_007);
    let (_, old_clones) = capture(|| {
        for _ in 0..256 {
            assert_eq!(std::hint::black_box(old.clone()).boot_id(), BOOT);
        }
    });
    let (_, new_clones) = capture(|| {
        for _ in 0..256 {
            assert_eq!(std::hint::black_box(new.clone()).boot_id(), BOOT);
        }
    });
    assert_eq!(old_clones, multiplied(expected(36, false), 256));
    assert_eq!(new_clones, Counts::default());
    let (old_sum, old_getters) = capture(|| cost::getter_reference(&old, ITERATIONS));
    let (new_sum, new_getters) = capture(|| getter_candidate(&new, ITERATIONS));
    assert_eq!((old_sum, new_sum), (9_437_184, 9_437_184));
    assert_eq!(
        (old_getters, new_getters),
        (Counts::default(), Counts::default())
    );
    println!("HC074-BOOT-INLINE-LAYOUT {{\"reference_size\":{},\"reference_align\":{},\"candidate_size\":{},\"candidate_align\":{},\"clone_calls\":256,\"reference_clones\":{},\"candidate_clones\":{},\"reference_getters\":{},\"candidate_getters\":{},\"iterations\":262144,\"checksum\":9437184,\"physical_copy_claim\":false,\"model_only\":true}}",std::mem::size_of::<TimePoint>(),std::mem::align_of::<TimePoint>(),std::mem::size_of::<prototype::TimePoint>(),std::mem::align_of::<prototype::TimePoint>(),old_clones.json(),new_clones.json(),old_getters.json(),new_getters.json());
}

#[test]
fn inline_paired_getter_intervals() {
    let old = cost::reference_point();
    let new = candidate_point(BOOT.into(), 90, 2_000_000_007);
    assert_eq!(cost::getter_reference(&old, WARMUP), 36 * WARMUP as u64);
    assert_eq!(getter_candidate(&new, WARMUP), 36 * WARMUP as u64);
    for sample in 0..7 {
        let (reference, candidate, order) = if sample % 2 == 0 {
            let reference = cost::time_reference(&old);
            (reference, time_candidate(&new), "reference-first")
        } else {
            let candidate = time_candidate(&new);
            (cost::time_reference(&old), candidate, "candidate-first")
        };
        assert_eq!((reference.1, candidate.1), (9_437_184, 9_437_184));
        println!("HC074-BOOT-INLINE-GETTERS {{\"sample\":{sample},\"order\":\"{order}\",\"iterations\":262144,\"warmup\":32768,\"checksum\":9437184,\"reference_wall_ns\":{},\"candidate_wall_ns\":{},\"allocator_capture_active\":false,\"model_only\":true}}",reference.0,candidate.0);
    }
}
