use hydracache::HydraCache;
use hydracache_sqlx::{DbCache, HydraCacheEntity};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, HydraCacheEntity)]
#[hydracache(entity = "user", collection = "users")]
struct User {
    #[hydracache(id)]
    id: i64,
    name: String,
}

#[tokio::main]
async fn main() -> hydracache_sqlx::Result<()> {
    // ANCHOR: sqlx-adapter
    let queries = DbCache::new(HydraCache::local().build(), "sqlx");

    let user = queries
        .for_entity::<User>(42)
        .fetch_with(|| async {
            // Replace this with SQLx query_as!, a transaction, or repository code.
            Ok::<_, hydracache_sqlx::sqlx::Error>(User {
                id: 42,
                name: "Ada".to_owned(),
            })
        })
        .await?;

    assert_eq!(user.id, 42);
    // ANCHOR_END: sqlx-adapter

    Ok(())
}
