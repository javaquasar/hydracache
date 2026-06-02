use hydracache_db::{CacheEntity, HydraCacheEntity};

#[derive(HydraCacheEntity)]
#[hydracache(entity = "user", collection = "users", id = i64)]
struct User;

fn main() {
    assert_eq!(User::cache_key_for(&42), "user:42");
    assert_eq!(User::entity_tag_for(&42), "user:42");
    assert_eq!(User::collection_tag(), Some("users".to_owned()));
}
