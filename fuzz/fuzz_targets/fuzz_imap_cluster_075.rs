#![cfg_attr(fuzzing, no_main)]

#[cfg(fuzzing)]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    use hydracache_cluster_testkit::value_plane_cluster_chaos_075::{
        fuzz_cluster_schedule, ClusterScheduleError,
    };
    // Invalid fixtures and an explicitly exhausted search budget are expected.
    // An invariant violation over actual service decisions is a counterexample.
    if let Err(ClusterScheduleError::Violation(reason)) = fuzz_cluster_schedule(data) {
        panic!("actual IMap cluster invariant: {reason}");
    }
});

#[cfg(not(fuzzing))]
fn main() {}
