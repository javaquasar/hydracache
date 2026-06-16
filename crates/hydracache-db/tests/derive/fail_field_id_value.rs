use hydracache_db::HydraCacheEntity;

#[derive(HydraCacheEntity)]
#[hydracache(entity = "user")]
struct User {
    #[hydracache(id = i64)]
    id: i64,
}

fn main() {}
