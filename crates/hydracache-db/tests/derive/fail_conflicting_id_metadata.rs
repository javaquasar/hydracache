use hydracache_db::HydraCacheEntity;

#[derive(HydraCacheEntity)]
#[hydracache(entity = "user", id = i64)]
struct User {
    #[hydracache(id)]
    id: i64,
}

fn main() {}
