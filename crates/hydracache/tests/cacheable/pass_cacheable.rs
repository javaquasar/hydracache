use hydracache::{cacheable_loader, HydraCache};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Value {
    id: u64,
}

fn main() {
    let cache = HydraCache::local().build();
    let _future = cacheable_loader!(
        cache = cache,
        key = "value:1",
        tag = "values",
        ttl_secs = 60,
        load = || async { Ok::<_, std::io::Error>(Value { id: 1 }) },
    );
}
