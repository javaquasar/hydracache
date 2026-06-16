use std::time::Duration;

use hydracache::HydraCache;

#[derive(serde::Deserialize, serde::Serialize)]
struct Value {
    id: u64,
}

#[hydracache::cacheable(
    cache = cache,
    key = format!("value:{id}"),
    tags = vec!["values".to_owned(), format!("value:{id}")],
    ttl = Duration::from_secs(30)
)]
async fn load_value(cache: &HydraCache, id: u64) -> Result<Value, std::io::Error> {
    Ok(Value { id })
}

fn main() {
    let cache = HydraCache::local().build();
    let _future = load_value(&cache, 7);
}
