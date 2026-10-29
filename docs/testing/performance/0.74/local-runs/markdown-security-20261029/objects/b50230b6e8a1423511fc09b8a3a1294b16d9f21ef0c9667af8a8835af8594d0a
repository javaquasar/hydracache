//! Synthetic ownership and first-error controls, not host admission.
use super::*;
use crate::counter::{capture, Counts};
use crate::{events, reset, Event};
const BOOT: &str = "6a709ad1-c736-45a2-bd6a-351e07f6b8f3";
const HEALTHY: [Event; 5] = [
    Event::Span,
    Event::Before,
    Event::BootClock,
    Event::RealClock,
    Event::After,
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
fn check(point: &TimePoint, boot: &str, boot_ns: u64, real_ns: u64) {
    assert_eq!(point.boot_id(), boot);
    assert_eq!(point.boottime_ns(), boot_ns);
    assert_eq!(point.realtime_ns, real_ns);
    assert_eq!(point.unix_seconds(), real_ns / 1_000_000_000);
}

#[test]
fn sample_boot_main_owned_sample_requests() {
    for (name, boot) in cases() {
        let origin = Origin { boot };
        let length = origin.boot.len();
        let input_capacity = origin.boot.capacity();
        let mut output_capacity = 0;
        let (_, counts) = capture(|| {
            for _ in 0..256 {
                reset(0, 90, 2_000_000_007);
                let point = std::hint::black_box(origin.sample()).unwrap();
                check(&point, &origin.boot, 90, 2_000_000_007);
                assert_eq!(events(), (HEALTHY, 5));
                output_capacity = point.boot.capacity();
                if length != 0 {
                    assert_ne!(point.boot.as_ptr(), origin.boot.as_ptr());
                }
            }
        });
        let allocations = if length == 0 { 0 } else { 256 };
        assert_eq!(
            counts,
            Counts {
                allocations,
                deallocations: allocations,
                requested_bytes: length * 256,
                deallocated_bytes: length * 256,
                ..Counts::default()
            }
        );
        println!("HC074-SAMPLE-BOOT-MAIN {{\"case\":\"{name}\",\"length\":{length},\"input_capacity\":{input_capacity},\"output_capacity\":{output_capacity},\"samples\":256,\"counts\":{},\"synthetic_frame\":true,\"timing_claim\":false,\"server\":false,\"product\":false}}",counts.json());
    }
}

#[test]
fn sample_boot_stage_requests_are_separate_from_main() {
    for (name, boot) in cases() {
        let origin = Origin { boot };
        reset(0, 90, 2_000_000_007);
        let (times, frame) = capture(|| origin.stage_prelude().unwrap());
        assert_eq!(times, (90, 2_000_000_007));
        assert_eq!(events(), (HEALTHY, 5));
        let (copy, cloned) = capture(|| std::hint::black_box(origin.boot.clone()));
        let (_, dropped) = capture(|| drop(copy));
        reset(0, 90, 2_000_000_007);
        let (point, whole) = capture(|| std::hint::black_box(origin.sample()).unwrap());
        let (_, whole_drop) = capture(|| drop(point));
        assert_eq!(frame, Counts::default());
        assert_eq!(cloned, whole);
        assert_eq!(dropped, whole_drop);
        println!("HC074-SAMPLE-BOOT-STAGES {{\"case\":\"{name}\",\"length\":{},\"samples_per_arm\":1,\"not_pooled\":true,\"frame\":{},\"cloned\":{},\"dropped\":{},\"whole_returning\":{},\"whole_drop\":{}}}",origin.boot.len(),frame.json(),cloned.json(),dropped.json(),whole.json(),whole_drop.json());
    }
}

#[test]
fn sample_boot_all_fault_masks_keep_first_error_and_no_clone() {
    let origin = Origin { boot: BOOT.into() };
    let mut refused = 0;
    for faults in 0..16 {
        reset(faults, 90, 2_000_000_007);
        let (result, counts) = capture(|| origin.sample());
        let mut expected = [Event::Empty; 5];
        expected[0] = Event::Span;
        let mut used = 1;
        let mut first = None;
        for (mask, event) in [
            (1, Event::Before),
            (2, Event::BootClock),
            (4, Event::RealClock),
            (8, Event::After),
        ] {
            expected[used] = event;
            used += 1;
            if faults & mask != 0 {
                first = Some(KernelError::Fault(mask));
                break;
            }
        }
        assert_eq!(events(), (expected, used));
        match first {
            Some(error) => {
                assert_eq!(result.unwrap_err(), error);
                assert_eq!(counts, Counts::default());
                refused += 1;
            }
            None => {
                check(&result.unwrap(), BOOT, 90, 2_000_000_007);
                assert_eq!(counts.allocations, 1);
                assert_eq!(counts.requested_bytes, 36);
            }
        }
    }
    assert_eq!(refused, 15);
    println!("HC074-SAMPLE-BOOT-FAULTS {{\"masks\":16,\"refused\":15,\"refused_allocations\":0,\"first_error_prefix\":true,\"model_only\":true}}");
}

#[test]
fn sample_boot_seeded_owned_points_getters_and_point_clones() {
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
        assert_eq!(boot.len(), 36);
        let mut origin = Origin { boot: boot.clone() };
        let real_ns = if sample % 2 == 0 { u64::MAX } else { seed };
        reset(0, seed, real_ns);
        let mut first = origin.sample().unwrap();
        let second = first.clone();
        assert_ne!(first.boot.as_ptr(), second.boot.as_ptr());
        origin.boot.clear();
        drop(origin);
        check(&first, &boot, seed, real_ns);
        first.boot.clear();
        check(&second, &boot, seed, real_ns);
    }
    println!("HC074-SAMPLE-BOOT-SEEDED {{\"seed\":\"0x074b0a19\",\"samples\":256,\"owned_after_origin_drop\":true,\"independent_point_clone\":true,\"model_only\":true}}");
}

#[test]
fn sample_boot_decoder_preserves_original_bounded_shape() {
    let mut raw = [0; 38];
    raw[..36].copy_from_slice(BOOT.as_bytes());
    raw[36] = b'\n';
    assert_eq!(boot_view(&(raw, 37)).unwrap(), BOOT);
    assert_eq!(boot_value((raw, 37)).unwrap(), BOOT);
    for count in [0, 1, 36, 38, usize::MAX] {
        let expected = if count == 0 {
            "proc-document-budget"
        } else {
            "boot-document"
        };
        assert_eq!(
            boot_view(&(raw, count)),
            Err(KernelError::Invalid(expected))
        );
    }
    for (index, value) in [
        (0, b'A'),
        (0, 0xff),
        (8, b'a'),
        (13, b'_'),
        (18, b'0'),
        (23, b' '),
        (36, b'x'),
    ] {
        let mut changed = raw;
        changed[index] = value;
        assert_eq!(
            boot_view(&(changed, 37)),
            Err(KernelError::Invalid("boot-document"))
        );
    }
    println!("HC074-SAMPLE-BOOT-DECODER {{\"valid\":1,\"invalid_counts\":5,\"invalid_bytes\":7,\"model_only\":true}}");
}
