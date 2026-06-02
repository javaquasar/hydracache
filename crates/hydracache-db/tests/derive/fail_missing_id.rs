use hydracache_db::HydraCacheEntity;

#[derive(HydraCacheEntity)]
#[hydracache(entity = "user")]
struct User;

fn main() {}
