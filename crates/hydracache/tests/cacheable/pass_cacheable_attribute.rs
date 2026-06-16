use hydracache::HydraCache;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Value {
    id: u64,
}

#[hydracache::cacheable(
    cache = cache,
    key_segments = ["value", id],
    tag_segments = [["value", id], ["values"]],
    ttl_secs = 60
)]
async fn load_value(cache: &HydraCache, id: u64) -> Result<Value, std::io::Error> {
    Ok(Value { id })
}

fn main() {
    let cache = HydraCache::local().build();
    let _future = load_value(&cache, 1);
}
