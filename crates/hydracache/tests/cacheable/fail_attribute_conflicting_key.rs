#[hydracache::cacheable(
    cache = cache,
    key = "value:1",
    key_segments = ["value", 1],
)]
async fn load_value(cache: &hydracache::HydraCache) -> Result<u64, std::io::Error> {
    Ok(1)
}

fn main() {}
