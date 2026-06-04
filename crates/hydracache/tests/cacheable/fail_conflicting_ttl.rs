use hydracache::{cacheable, HydraCache};

fn main() {
    let cache = HydraCache::local().build();
    let _future = cacheable!(
        cache = cache,
        key = "value:1",
        ttl = std::time::Duration::from_secs(60),
        ttl_secs = 60,
        load = || async { Ok::<_, std::io::Error>(1_u64) },
    );
}
