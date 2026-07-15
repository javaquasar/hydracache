#![cfg_attr(fuzzing, no_main)]

#[cfg(fuzzing)]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    hydracache_fuzz::fuzz_snapshot_decode(data);
});

#[cfg(not(fuzzing))]
fn main() {}
