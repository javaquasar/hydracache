#[hydracache::cacheable(
    cache = cache,
)]
async fn load_value(cache: &hydracache::HydraCache) -> Result<u64, std::io::Error> {
    Ok(1)
}

fn main() {}
