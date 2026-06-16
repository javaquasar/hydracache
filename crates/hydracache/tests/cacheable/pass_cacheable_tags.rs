use hydracache::{cacheable_loader, HydraCache, TagSet};
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
        tags = TagSet::new().tag("values").entity("value", 1),
        tag = "tenant:7",
        load = || async { Ok::<_, std::io::Error>(Value { id: 1 }) },
    );
}
