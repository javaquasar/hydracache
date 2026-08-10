use hydracache::HydraCache;
use hydracache_db::DbCache;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct User {
    id: i64,
    name: String,
}

#[tokio::main]
async fn main() -> hydracache_db::Result<()> {
    // ANCHOR: database-query-caching
    let local = HydraCache::local().build();
    let queries = DbCache::new(local, "db");

    let user = queries
        .entity::<User>("user", 42)
        .collection_tag("users")
        .ttl(Duration::from_secs(60))
        .fetch_with(|| async {
            // Replace this with SQLx, Diesel, SeaORM, or repository code.
            Ok::<_, std::io::Error>(User {
                id: 42,
                name: "Ada".to_owned(),
            })
        })
        .await?;

    assert_eq!(user.name, "Ada");
    // ANCHOR_END: database-query-caching

    Ok(())
}
