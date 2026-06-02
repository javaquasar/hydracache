use hydracache_db::HydraCacheEntity;

#[derive(HydraCacheEntity)]
#[hydracache(id = i64)]
struct User;

fn main() {}
