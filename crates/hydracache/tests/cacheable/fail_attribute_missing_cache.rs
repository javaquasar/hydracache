#[hydracache::cacheable(
    key = "value:1",
)]
async fn load_value() -> Result<u64, std::io::Error> {
    Ok(1)
}

fn main() {}
