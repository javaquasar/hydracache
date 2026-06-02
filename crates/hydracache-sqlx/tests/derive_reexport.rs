use hydracache_sqlx::{CacheEntity, HydraCacheEntity};

#[derive(HydraCacheEntity)]
#[hydracache(entity = "sqlx-user", collection = "sqlx-users", id = i64)]
struct User;

#[test]
fn hydracache_entity_derive_is_reexported_for_sqlx_users() {
    assert_eq!(User::cache_key_for(&42), "sqlx-user:42");
    assert_eq!(User::entity_tag_for(&42), "sqlx-user:42");
    assert_eq!(User::collection_tag(), Some("sqlx-users".to_owned()));
}
