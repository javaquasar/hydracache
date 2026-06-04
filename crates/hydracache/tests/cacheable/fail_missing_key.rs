use hydracache::{cacheable, HydraCache};

fn main() {
    let cache = HydraCache::local().build();
    let _future = cacheable!(
        cache = cache,
        load = || async { Ok::<_, std::io::Error>(1_u64) },
    );
}
