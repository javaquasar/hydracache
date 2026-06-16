#[hydracache::cacheable(
    cache = cache,
    key_segments = ["value", id],
    ttl_secs = 60
)]
async fn load_value(cache: &hydracache::HydraCache, id: u64) -> u64 {
    id
}

fn main() {}
