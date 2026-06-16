#[hydracache::cacheable(
    cache = cache,
    key_segments = ["value", id],
    tag_segments = [[]],
    ttl_secs = 60
)]
async fn load_value(cache: &hydracache::HydraCache, id: u64) -> Result<u64, std::io::Error> {
    Ok(id)
}

fn main() {}
