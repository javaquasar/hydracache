use std::hint::black_box;

use gungraun::prelude::*;
use hydracache::{CacheOptions, HydraCache};
use tokio::runtime::{Builder, Runtime};

const OPERATIONS_PER_SAMPLE: usize = 64;

struct BenchState {
    cache: HydraCache,
    runtime: Runtime,
    key: &'static str,
    expected: Option<u64>,
}

fn runtime() -> Runtime {
    Builder::new_current_thread()
        .build()
        .expect("build runtime")
}

fn setup_hit() -> BenchState {
    let runtime = runtime();
    let cache = HydraCache::local().build();
    runtime
        .block_on(cache.put("instruction-hit", 42_u64, CacheOptions::new()))
        .expect("seed hit");
    BenchState {
        cache,
        runtime,
        key: "instruction-hit",
        expected: Some(42),
    }
}

fn setup_miss() -> BenchState {
    BenchState {
        cache: HydraCache::local().build(),
        runtime: runtime(),
        key: "instruction-miss",
        expected: None,
    }
}

fn exercise_get(state: BenchState) -> usize {
    let mut observed = 0_usize;
    for _ in 0..OPERATIONS_PER_SAMPLE {
        let value = state
            .runtime
            .block_on(state.cache.get::<u64>(black_box(state.key)))
            .expect("cache get");
        assert_eq!(value, state.expected);
        observed += value.is_some() as usize;
    }
    black_box(observed)
}

#[library_benchmark(setup = setup_hit)]
fn cache_get_hit(state: BenchState) -> usize {
    exercise_get(state)
}

#[library_benchmark(setup = setup_miss)]
fn cache_get_miss(state: BenchState) -> usize {
    exercise_get(state)
}

library_benchmark_group!(
    name = cache_work,
    benchmarks = [cache_get_hit, cache_get_miss]
);

main!(library_benchmark_groups = cache_work);
