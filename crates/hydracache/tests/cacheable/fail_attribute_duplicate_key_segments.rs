#[hydracache::cacheable(
    cache = cache,
    key_segments = ["value", id],
    key_segments = ["duplicate", id],
    ttl_secs = 60
)]
async fn load_value(cache: &hydracache::HydraCache, id: u64) -> Result<u64, std::io::Error> {
    Ok(id)
}

fn main() {}
