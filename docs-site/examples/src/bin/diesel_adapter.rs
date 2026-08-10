use hydracache::HydraCache;
use hydracache_diesel::{DieselCache, DieselQueryExt};

#[tokio::main]
async fn main() -> hydracache_diesel::Result<()> {
    // ANCHOR: diesel-adapter
    let queries = DieselCache::new(HydraCache::local().build(), "diesel");

    let user_name = queries
        .entity::<String>("user", 42)
        .collection_tag("users")
        .diesel_one(move || {
            // Acquire or use a Diesel connection inside this blocking closure.
            Ok::<_, hydracache_diesel::diesel::result::Error>("Ada".to_owned())
        })
        .await?;

    assert_eq!(user_name, "Ada");
    // ANCHOR_END: diesel-adapter

    Ok(())
}
