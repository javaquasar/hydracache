#[hydracache::cacheable(
    cache = cache,
    key = "value:1",
    loader = loader,
)]
async fn load_value(cache: &hydracache::HydraCache) -> Result<u64, std::io::Error> {
    Ok(1)
}

fn main() {}
