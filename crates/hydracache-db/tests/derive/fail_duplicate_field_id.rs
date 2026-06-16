use hydracache_db::HydraCacheEntity;

#[derive(HydraCacheEntity)]
#[hydracache(entity = "user")]
struct User {
    #[hydracache(id)]
    id: i64,
    #[hydracache(id)]
    legacy_id: i64,
}

fn main() {}
