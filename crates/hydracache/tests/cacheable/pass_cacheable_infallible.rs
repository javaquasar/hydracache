use hydracache::{cacheable_infallible, HydraCache};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Value {
    id: u64,
}

fn main() {
    let cache = HydraCache::local().build();
    let _future = cacheable_infallible!(
        cache = cache,
        key = "value:1",
        tags = ["values", "value:1"],
        ttl_secs = 60,
        load = || async { Value { id: 1 } },
    );
}
