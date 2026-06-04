use hydracache::{cacheable, HydraCache};

fn main() {
    let cache = HydraCache::local().build();
    let _future = cacheable!(
        cache = cache,
        key = "value:1",
    );
}
