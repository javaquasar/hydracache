use hydracache_db::HydraCacheEntity;

#[derive(HydraCacheEntity)]
#[hydracache(entity = "user")]
struct User {
    #[hydracache(primary_key)]
    id: i64,
}

fn main() {}
