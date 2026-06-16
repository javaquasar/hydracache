#[hydracache::cacheable(
    cache = cache,
    key = "value:1",
    ttl = std::time::Duration::from_secs(60),
    ttl_secs = 60,
)]
async fn load_value(cache: &hydracache::HydraCache) -> Result<u64, std::io::Error> {
    Ok(1)
}

fn main() {}
