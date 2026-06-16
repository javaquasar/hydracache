#[hydracache::cacheable(
    cache = cache,
    key_segments = [],
    ttl_secs = 60
)]
async fn load_value(cache: &hydracache::HydraCache) -> Result<u64, std::io::Error> {
    Ok(7)
}

fn main() {}
