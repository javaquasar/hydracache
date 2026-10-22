//! Explicit pure getter clock, never admission time or a kernel observation.
use super::*;
use crate::counter::{capture, Counts};
pub(super) const ITERATIONS: usize = 262_144;
pub(super) const WARMUP: usize = 32_768;
pub(super) const BOOT: &str = "6a709ad1-c736-45a2-bd6a-351e07f6b8f3";
pub(super) fn reference_point() -> KernelTimePoint {
    crate::reset(0, 90, 2_000_000_007);
    Origin { boot: BOOT.into() }.sample().unwrap()
}
pub(super) fn getter_reference(point: &TimePoint, iterations: usize) -> u64 {
    let mut checksum = 0_u64;
    for _ in 0..iterations {
        checksum = checksum.wrapping_add(std::hint::black_box(point).boot_id().len() as u64);
    }
    std::hint::black_box(checksum)
}
pub(super) fn time_reference(point: &TimePoint) -> (u128, u64) {
    let start = std::time::Instant::now();
    let checksum = getter_reference(point, ITERATIONS);
    (start.elapsed().as_nanos(), checksum)
}
#[test]
fn reference_cost_preflight() {
    let point = reference_point();
    let (_, cloned) = capture(|| {
        for _ in 0..256 {
            let value = std::hint::black_box(point.clone());
            assert_eq!(value.boot_id(), BOOT);
        }
    });
    assert_eq!(
        cloned,
        Counts {
            allocations: 256,
            requested_bytes: 9216,
            deallocations: 256,
            deallocated_bytes: 9216,
            ..Counts::default()
        }
    );
    let (checksum, getters) = capture(|| getter_reference(&point, ITERATIONS));
    assert_eq!(getters, Counts::default());
    assert_eq!(checksum, 9_437_184);
    println!("HC074-BOOT-INLINE-REFERENCE {{\"point_size\":{},\"point_align\":{},\"clone_calls\":256,\"clone_counts\":{},\"getter_counts\":{},\"iterations\":262144,\"warmup\":32768,\"checksum\":9437184,\"model_only\":true}}",std::mem::size_of::<TimePoint>(),std::mem::align_of::<TimePoint>(),cloned.json(),getters.json());
    assert_eq!(getter_reference(&point, WARMUP), 36 * WARMUP as u64);
    for sample in 0..7 {
        let (wall_ns, checksum) = time_reference(&point);
        assert_eq!(checksum, 9_437_184);
        println!("HC074-BOOT-INLINE-PREFLIGHT {{\"sample\":{sample},\"iterations\":262144,\"checksum\":{checksum},\"reference_wall_ns\":{wall_ns},\"allocator_capture_active\":false,\"model_only\":true}}");
    }
}
