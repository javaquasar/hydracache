use hydracache_db::HydraCacheEntity;

#[derive(HydraCacheEntity)]
#[hydracache(entity = "user", id = i64, table = "users")]
struct User;

fn main() {}
